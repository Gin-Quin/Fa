//! Programs compiled per unit (design/parallel-pipeline.md §3.3, step 6).
//!
//! - [`compile_unit`]: the files of one unit of the analysis
//!   (`fa_analyzer::pipeline`) into an object of their own, in an LLVM
//!   context of their own, on the thread that analyzed them (`lower/units.rs`).
//!   Development builds keep the objects in a cache, keyed by the unit's
//!   sources, the interfaces it reads and the options.
//! - [`link_program`]: the entry object (`main`: the initializations in the
//!   program's order, the entry's top level, the drops at a normal exit, the
//!   switches over the ids of union members), the runtime object (compiled
//!   once per target and options, and cached), and the link: ThinLTO in
//!   release builds (`ld.lld --thinlto-jobs`, `wasm-ld`), plain objects with
//!   `--gc-sections` in development.

use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::time::Instant;

use fa_analyzer::Severity;
use fa_analyzer::ids::{DeclKey, FileId};
use fa_analyzer::session::Session;
use inkwell::attributes::{Attribute, AttributeLoc};
use inkwell::context::Context;
use inkwell::module::{Linkage, Module};
use inkwell::passes::PassBuilderOptions;
use inkwell::targets::{FileType, TargetMachine};
use inkwell::values::FunctionValue;
use inkwell::{AddressSpace, GlobalVisibility};
use rustc_hash::FxHashSet;

use crate::artifact::{Artifact, Timings};
use crate::error::Error;
use crate::link;
use crate::lower::Lowering;
use crate::options::Libraries;
use crate::runtime::Runtime;
use crate::split::{
	EntryInputs, GenericEntryRequest, GenericInstance, GenericProducer, GenericTables,
	InstanceRequest, PackageOutcome, SplitOptions, UnitOutput, UnitSymbols,
};
use crate::target::{Options, Target};

unsafe extern "C" {
	/// `shim/thin_bitcode.cpp`.
	fn fa_write_thin_bitcode(
		module: llvm_sys::prelude::LLVMModuleRef,
		path: *const std::ffi::c_char,
	) -> std::ffi::c_int;
}

/// The version of the objects: a cached object of another version of the
/// compiler is never reused.
const VERSION: &str = concat!(
	env!("CARGO_PKG_VERSION"),
	"-split-7-platform-1-effects-1-errors-1-regions-1-packed-capacity-1-frame-1-sync-payload-1-region-string-view-1-region-edges-1-inline-length-1-template-size-1-coroutine-decimal-1-array-capacities-1-reference-results-5"
);

/// A file name made of a module path.
fn file_name(module: &str) -> String {
	module
		.chars()
		.map(|character| match character {
			'/' | '\\' | ':' => '~',
			other => other,
		})
		.collect()
}

/// A stable hash (the cache keys): FNV-1a over the parts.
#[derive(Default)]
struct StableHasher(u64, u64);

impl Hasher for StableHasher {
	fn finish(&self) -> u64 {
		self.0 ^ self.1.rotate_left(29)
	}

	fn write(&mut self, bytes: &[u8]) {
		if self.0 == 0 {
			self.0 = 0xcbf2_9ce4_8422_2325;
			self.1 = 0x8422_2325_cbf2_9ce4;
		}
		for &byte in bytes {
			self.0 = (self.0 ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3);
			self.1 = (self.1 ^ u64::from(byte)).wrapping_mul(0x0000_0001_0000_01b3 ^ 0x2000);
		}
	}
}

/// The options that change an object. Not the whole-program facts: a
/// unit's object depends on the facts its lowering consulted only (checked
/// by `cached_unit` against its `.facts` file), the runtime on none.
fn options_key(target: Target, options: &Options, split: &SplitOptions, hasher: &mut StableHasher) {
	VERSION.hash(hasher);
	// the runtime's IR, so that editing `fa_runtime.c` invalidates the objects
	crate::runtime::tier1_text(target).hash(hasher);
	target.name().hash(hasher);
	target.triple().as_str().to_bytes().hash(hasher);
	format!("{:?}", options.opt_level).hash(hasher);
	options.size.hash(hasher);
	options.overflow_checks.hash(hasher);
	options.leak_check.hash(hasher);
	split.lto.hash(hasher);
	split.full_lto.hash(hasher);
	// a C library: its wrappers, and the dynamic model of its thread-local
	// variables (`lower/c_library.rs`)
	split.c_library.hash(hasher);
	if split.c_library.is_some() {
		"dynamic-tls".hash(hasher);
	}
	split.facts.is_some().hash(hasher);
	// emulated thread-local variables (`jit.rs`)
	split.in_memory.hash(hasher);
}

/// The target machine of an object of the program: with emulated
/// thread-local variables when the JIT loads it.
fn unit_machine(
	target: Target,
	options: &Options,
	split: &SplitOptions,
) -> Result<TargetMachine, Error> {
	let machine = target.machine(options)?;
	if split.in_memory {
		crate::target::use_emulated_tls(&machine);
	}
	Ok(machine)
}

/// A program compiled per unit: linked, or loaded in this process.
enum Linked {
	Executable(Artifact),
	Loaded(Box<dyn crate::LoadedProgram>),
}

/// The target machine and the pointer-sized integer of a module.
fn prepare_module(context: &Context, machine: &TargetMachine, module: &Module<'_>, target: Target) {
	module.set_triple(&target.triple());
	module.set_data_layout(&machine.get_target_data().get_data_layout());
	let _ = context;
}

/// Optimizes a module and writes it: an object, or ThinLTO bitcode.
fn emit(
	module: &Module<'_>,
	machine: &TargetMachine,
	options: &Options,
	split: &SplitOptions,
	path: &Path,
	timings: &mut Timings,
) -> Result<(), Error> {
	let start = Instant::now();
	if split.c_library.is_some() {
		crate::lower::c_library::use_dynamic_tls(module);
	}
	if !options.size {
		// the size attributes of the runtime's IR must not override the
		// program's speed pipeline (as `compile.rs` does)
		for function in module.get_functions() {
			for name in ["optsize", "minsize"] {
				function.remove_enum_attribute(
					AttributeLoc::Function,
					Attribute::get_named_enum_kind_id(name),
				);
			}
		}
	}
	crate::compile::keep_huge_functions_unoptimized(module);
	let pipeline = if split.lto {
		if options.size {
			"thinlto-pre-link<Oz>"
		} else {
			match options.opt_level {
				crate::OptLevel::O0 | crate::OptLevel::O1 => "thinlto-pre-link<O1>",
				crate::OptLevel::O2 => "thinlto-pre-link<O2>",
				crate::OptLevel::O3 => "thinlto-pre-link<O3>",
			}
		}
	} else {
		options.pipeline()
	};
	module.run_passes(pipeline, machine, PassBuilderOptions::create())?;
	crate::optimize::expose_tail_recursion(module, machine, options)?;
	if split.lto && module.get_name().to_bytes() == b"runtime" {
		keep_runtime_at_home(module);
	}
	timings.optimize += start.elapsed();
	let start = Instant::now();
	let partial = crate::compile::with_suffix(path, ".partial");
	if split.lto {
		// the link's code generator reads the CPU and the features of each
		// function (the wasm features: no `memcpy` calls, `memory.copy`)
		let cpu = machine.get_cpu().to_string_lossy().into_owned();
		let features = machine.get_feature_string().to_string_lossy().into_owned();
		let context = module.get_context();
		for function in module.get_functions() {
			if function.count_basic_blocks() == 0 {
				continue;
			}
			function.add_attribute(
				AttributeLoc::Function,
				context.create_string_attribute("target-cpu", &cpu),
			);
			if !features.is_empty() {
				function.add_attribute(
					AttributeLoc::Function,
					context.create_string_attribute("target-features", &features),
				);
			}
		}
		// the runtime stays ThinLTO bitcode under full LTO too: in its own
		// partition, its functions are not inlined into the program's (a
		// 100-file program: 444 KB, 590 KB merged)
		if split.full_lto && module.get_name().to_bytes() != b"runtime" {
			// no summary: one module optimized as a whole by the linker
			if !module.write_bitcode_to_path(&partial) {
				return Err(Error::Llvm(format!("can't write `{}`", partial.display())));
			}
			fs::rename(&partial, path)?;
			timings.emit += start.elapsed();
			return Ok(());
		}
		let text = std::ffi::CString::new(partial.as_os_str().as_encoded_bytes())
			.map_err(|_| Error::Llvm("a path with a NUL byte".into()))?;
		// SAFETY: the module is alive, the path a C string
		let failed = unsafe { fa_write_thin_bitcode(module.as_mut_ptr(), text.as_ptr()) };
		if failed != 0 {
			return Err(Error::Llvm(format!("can't write `{}`", partial.display())));
		}
	} else {
		machine.write_to_file(module, FileType::Object, &partial)?;
	}
	fs::rename(&partial, path)?;
	timings.emit += start.elapsed();
	Ok(())
}

/// The functions of the runtime larger than this (instructions, once
/// optimized) are not imported by the units at the link (`noinline`):
/// the allocator's slow paths inlined into every unit made a program 2.4
/// times larger. Its small functions are, and are inlined where they pay.
const RUNTIME_IMPORT_LIMIT: usize = 40;

/// Marks the larger functions of the runtime `noinline` (ThinLTO doesn't
/// import them).
fn keep_runtime_at_home(module: &Module<'_>) {
	let context = module.get_context();
	let kind = |name: &str| Attribute::get_named_enum_kind_id(name);
	for function in module.get_functions() {
		if function.count_basic_blocks() == 0
			|| function
				.get_enum_attribute(AttributeLoc::Function, kind("alwaysinline"))
				.is_some()
		{
			continue;
		}
		let mut count = 0;
		for block in function.get_basic_block_iter() {
			count += block.get_instructions().count();
		}
		if count > RUNTIME_IMPORT_LIMIT {
			function.add_attribute(
				AttributeLoc::Function,
				context.create_enum_attribute(kind("noinline"), 0),
			);
		}
	}
}

/// The extension of the objects of a build.
fn object_extension(split: &SplitOptions) -> &'static str {
	if split.lto { "bc" } else { "o" }
}

/// Compiles the files of one analyzed unit into an object in `directory`,
/// or at `cache` (the object cache key of a development build). `entry`:
/// the program's entry file, when it is one of `files`; `reached`: the
/// declarations the program reaches (the others are not lowered; `None`:
/// all). The files must have no error diagnostics.
#[allow(clippy::too_many_arguments)]
pub fn compile_unit(
	session: &Session,
	files: &[FileId],
	entry: Option<FileId>,
	target: Target,
	options: &Options,
	split: &SplitOptions,
	directory: &Path,
	reached: Option<&FxHashSet<String>>,
	cache: Option<&Path>,
) -> Result<UnitOutput, Error> {
	let Some(&first) = files.first() else {
		return Err(Error::Llvm("a unit without files".into()));
	};
	let mut rendered = String::new();
	for &file in files {
		if session
			.diagnostics(file)
			.iter()
			.any(|diagnostic| diagnostic.severity == Severity::Error)
		{
			rendered.push_str(&session.render_diagnostics(file, &session.display_path(file)));
		}
	}
	if !rendered.is_empty() {
		return Err(Error::Diagnostics(rendered));
	}
	let name = session.module(first).to_string();
	let extension = object_extension(split);
	let object = match cache {
		Some(key) => crate::compile::with_suffix(key, &format!(".{extension}")),
		None => directory.join(format!("{}.{extension}", file_name(&name))),
	};
	let log = std::sync::Arc::new(crate::facts::FactLog::default());
	let (symbols, requests) = lower_and_emit(
		session,
		files,
		&name,
		target,
		options,
		split,
		&object,
		|lowering| {
			lowering
				.with_fact_log(std::sync::Arc::clone(&log))
				.into_unit(name.clone(), files, entry, reached.cloned())
				.lower_unit()
		},
	)?;
	// an object with instance requests or generic function values holds
	// types of this run: not cached
	if let Some(key) = cache
		&& requests.is_empty()
		&& symbols.generic_values.is_empty()
	{
		// the facts it consulted: the object stays valid while they hold
		let facts = crate::compile::with_suffix(key, ".facts");
		fs::write(&facts, log.to_text())?;
		let partial = crate::compile::with_suffix(key, ".symbols.partial");
		fs::write(&partial, symbols.to_text())?;
		fs::rename(&partial, crate::compile::with_suffix(key, ".symbols"))?;
	}
	Ok(UnitOutput {
		name,
		object,
		symbols,
		cached: false,
		requests,
	})
}

/// What a lowering gives: the symbols of the unit, and the instances of
/// other units' generic functions it requests (global types).
type Lowered = (UnitSymbols, Vec<InstanceRequest>);

/// Lowers files of a session into a module with `lower`, verifies it and
/// writes it at `object`; the requests' types merged into the global store.
#[allow(clippy::too_many_arguments)]
fn lower_and_emit(
	session: &Session,
	files: &[FileId],
	name: &str,
	target: Target,
	options: &Options,
	split: &SplitOptions,
	object: &Path,
	lower: impl FnOnce(Lowering<'_, '_>) -> Result<Lowered, Error>,
) -> Result<Lowered, Error> {
	let machine = unit_machine(target, options, split)?;
	let context = Context::create();
	let module = crate::runtime::declarations_module(&context, target, name)?;
	prepare_module(&context, &machine, &module, target);
	let usize = context.ptr_sized_int_type(&machine.get_target_data(), None);
	let paths: Vec<(FileId, String)> = files
		.iter()
		.map(|&file| (file, session.display_path(file)))
		.collect();
	let mut timings = Timings::default();
	let start = Instant::now();
	let lowering = Lowering::new(
		session,
		&paths,
		files[0],
		Vec::new(),
		&context,
		&module,
		target,
		usize,
	)
	.with_overflow_checks(options.overflow_checks)
	.with_leak_check(options.leak_check)
	.with_asynchronous(options.asynchronous == Some(true), options.dev())
	.with_program_facts(split.facts.clone())
	.with_c_library(split.c_library.clone());
	let (symbols, requests) = lower(lowering)?;
	timings.build = start.elapsed();
	module.verify()?;
	if target != Target::Wasm && !split.lto {
		function_sections(&module);
	}
	if let Some(parent) = object.parent() {
		fs::create_dir_all(parent)?;
	}
	emit(&module, &machine, options, split, object, &mut timings)?;
	// the types of the requests, in the global store: read by other sessions
	let requests = match requests.is_empty() {
		true => Vec::new(),
		false => {
			let module_id = session.module_id(files[0]);
			let mut merger = session.types().merger(Some(module_id));
			requests
				.into_iter()
				.map(
					|InstanceRequest {
					     key,
					     arguments,
					     callbacks,
					     methods,
					 }| InstanceRequest {
						key: merger.decl_key(key),
						arguments: arguments
							.into_iter()
							.map(|argument| merger.ty(argument))
							.collect(),
						callbacks: callbacks
							.into_iter()
							.map(|(index, ty)| (index, merger.ty(ty)))
							.collect(),
						methods: methods
							.into_iter()
							.map(|method| method.merged(&mut merger))
							.collect(),
					},
				)
				.collect()
		}
	};
	Ok((symbols, requests))
}

/// Compiles the instances of generic functions of a module other units
/// requested (an instance wave): `session` holds the module analyzed again.
#[allow(clippy::too_many_arguments)]
fn compile_instances(
	session: &Session,
	files: &[FileId],
	requests: &[InstanceRequest],
	entries: &[GenericEntryRequest],
	wave: usize,
	target: Target,
	options: &Options,
	split: &SplitOptions,
	directory: &Path,
) -> Result<UnitOutput, Error> {
	let Some(&first) = files.first() else {
		return Err(Error::Llvm("an instance wave without files".into()));
	};
	let name = format!("{}.inst{wave}", session.module(first));
	let object = directory.join(format!("{}.{}", file_name(&name), object_extension(split)));
	let wanted = requests.to_vec();
	let (symbols, requests) = lower_and_emit(
		session,
		files,
		&name,
		target,
		options,
		split,
		&object,
		|lowering| {
			lowering
				.into_unit(name.clone(), files, None, None)
				.lower_instances(&wanted, entries)
		},
	)?;
	Ok(UnitOutput {
		name,
		object,
		symbols,
		cached: false,
		requests,
	})
}

/// One section per function (ELF), so that `--gc-sections` drops the
/// functions nothing calls.
fn function_sections(module: &Module<'_>) {
	for function in module.get_functions() {
		if function.count_basic_blocks() > 0 {
			let name = function.get_name().to_string_lossy().into_owned();
			function.set_section(Some(&format!(".text.{name}")));
		}
	}
}

/// The runtime object of a target and options: compiled once, then taken
/// from `cache` (else written into `directory`).
fn runtime_object(
	target: Target,
	options: &Options,
	split: &SplitOptions,
	directory: &Path,
) -> Result<PathBuf, Error> {
	let mut hasher = StableHasher::default();
	// the runtime reads no fact of the program: compiled once per target and options
	options_key(target, options, split, &mut hasher);
	"runtime".hash(&mut hasher);
	crate::runtime::tier1_text(target).hash(&mut hasher);
	let extension = object_extension(split);
	let path = match &split.runtime_cache {
		Some(cache) => {
			fs::create_dir_all(cache)?;
			cache.join(format!(
				"runtime-{}-{:016x}.{extension}",
				target.name(),
				hasher.finish()
			))
		}
		None => directory.join(format!("$runtime.{extension}")),
	};
	if path.is_file() {
		return Ok(path);
	}
	let machine = unit_machine(target, options, split)?;
	let context = Context::create();
	let module = context.create_module("runtime");
	prepare_module(&context, &machine, &module, target);
	let usize = context.ptr_sized_int_type(&machine.get_target_data(), None);
	let runtime =
		Runtime::new(&context, &module, target, usize).with_leak_check(options.leak_check);
	runtime.link_tier1()?;
	// multi-file programs keep the eager representation of characters
	if let Some(flag) = module.get_global("fa_compact_characters") {
		flag.set_initializer(&context.i32_type().const_zero());
		flag.set_constant(true);
		flag.set_linkage(Linkage::Internal);
	}
	runtime.define_platform()?;
	// the clock of `std.time.now` (a lowering defines it on use otherwise)
	runtime.now()?;
	// the platform layer, which the units call
	for name in [
		"fa_now",
		"fa_write",
		"fa_free",
		"fa_exit",
		"fa_alloc",
		"fa_pages",
		"fa_clock",
		"fa_nanosleep",
		"fa_cstring_length",
	] {
		if let Some(function) = module.get_function(name)
			&& function.count_basic_blocks() > 0
		{
			function.set_linkage(Linkage::External);
		}
	}
	module.verify()?;
	if target != Target::Wasm && !split.lto {
		function_sections(&module);
	}
	// a unique name first: several builds (and threads of one) may write
	// the cache at once
	static WRITES: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
	let write = WRITES.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
	let unique = crate::compile::with_suffix(&path, &format!(".{}-{write}", std::process::id()));
	emit(
		&module,
		&machine,
		options,
		split,
		&unique,
		&mut Timings::default(),
	)?;
	fs::rename(&unique, &path)?;
	Ok(path)
}

/// The entry object: `main` (`_start` on wasm), `fa.drops_at_exit`, and the
/// switches over the ids of union members.
fn entry_object(
	target: Target,
	options: &Options,
	split: &SplitOptions,
	inputs: &EntryInputs,
	path: &Path,
	library: bool,
) -> Result<(), Error> {
	let machine = unit_machine(target, options, split)?;
	let context = Context::create();
	let module = crate::runtime::declarations_module(&context, target, "entry")?;
	prepare_module(&context, &machine, &module, target);
	let usize = context.ptr_sized_int_type(&machine.get_target_data(), None);
	let runtime = Runtime::new(&context, &module, target, usize).declaring_only();
	let i32 = context.i32_type();
	let i64 = context.i64_type();
	let pointer = context.ptr_type(AddressSpace::default());
	let void = context.void_type();
	let builder = context.create_builder();
	let units = &inputs.units;
	let effectful = units.iter().any(|unit| unit.symbols.effectful_drops);
	let flag = module.add_global(context.bool_type(), None, "fa.drops_at_exit");
	flag.set_initializer(
		&context
			.bool_type()
			.const_int(u64::from(effectful || options.leak_check), false),
	);
	flag.set_constant(true);
	flag.set_visibility(GlobalVisibility::Hidden);
	let declare = |name: &str, ty| declare_hidden(&module, name, ty);
	// the tables and indices of the generic function values
	for (symbol, value) in &inputs.generic_tables.indices {
		let global = module.add_global(usize, None, symbol);
		global.set_initializer(&usize.const_int(*value, false));
		global.set_constant(true);
		global.set_visibility(GlobalVisibility::Hidden);
	}
	for (symbol, entries) in &inputs.generic_tables.tables {
		let entries: Vec<inkwell::values::PointerValue<'_>> = entries
			.iter()
			.map(|entry| {
				declare(entry, void.fn_type(&[], false))
					.as_global_value()
					.as_pointer_value()
			})
			.collect();
		let global = module.add_global(pointer.array_type(entries.len() as u32), None, symbol);
		global.set_initializer(&pointer.const_array(&entries));
		global.set_constant(true);
		global.set_visibility(GlobalVisibility::Hidden);
	}
	let defined_inits: std::collections::HashSet<(&str, u32)> = units
		.iter()
		.flat_map(|unit| unit.symbols.inits.iter())
		.map(|(module, node)| (module.as_str(), *node))
		.collect();
	let defined_drops: std::collections::HashSet<(&str, u32)> = units
		.iter()
		.flat_map(|unit| unit.symbols.drops.iter())
		.map(|(module, node)| (module.as_str(), *node))
		.collect();
	if library {
		library_entry(
			&module,
			&runtime,
			target,
			options,
			inputs,
			&defined_inits,
			&defined_drops,
			effectful,
		)?;
	} else {
		// `main(argc, argv)`, `_start()` on wasm
		let main = match target {
			Target::Wasm => {
				module.add_function("_start", void.fn_type(&[], false), Some(Linkage::External))
			}
			_ => module.add_function(
				"main",
				i32.fn_type(&[i32.into(), pointer.into()], false),
				Some(Linkage::External),
			),
		};
		runtime.add_attributes(main, &["nounwind"]);
		builder.position_at_end(context.append_basic_block(main, "start"));
		if target == Target::Native {
			builder.build_call(runtime.function("fa_guard_stack")?, &[], "")?;
		}
		if units.iter().any(|unit| unit.symbols.uses_arguments) {
			let (argc, argv) = match target {
				Target::Wasm => (i32.const_zero().into(), pointer.const_null().into()),
				_ => (
					main.get_nth_param(0).expect("argc").into(),
					main.get_nth_param(1).expect("argv").into(),
				),
			};
			builder.build_call(runtime.function("fa_arguments_init")?, &[argc, argv], "")?;
		}
		let mut seen = std::collections::HashSet::new();
		let mut initialized = Vec::new();
		for (module_path, node) in &inputs.init_order {
			if !seen.insert((module_path.as_str(), *node)) {
				continue;
			}
			if !defined_inits.contains(&(module_path.as_str(), *node)) {
				return Err(Error::Unsupported {
					message: format!(
						"the initialization of a value of `{module_path}` (split build)"
					),
					location: String::new(),
				});
			}
			let init = declare(
				&format!("fa.init.{module_path}.{node}"),
				void.fn_type(&[], false),
			);
			builder.build_call(init, &[], "")?;
			initialized.push((module_path.as_str(), *node));
		}
		let entry = units.iter().find_map(|unit| unit.symbols.entry.as_ref());
		// the module values, dropped in the reverse order when a drop does
		// more than free memory (or the leaks are counted)
		let drops = |builder: &inkwell::builder::Builder<'_>| -> Result<(), Error> {
			if effectful || options.leak_check {
				for (module_path, node) in initialized.iter().rev() {
					if defined_drops.contains(&(*module_path, *node)) {
						let drop = declare(
							&format!("fa.drop.{module_path}.{node}"),
							void.fn_type(&[], false),
						);
						builder.build_call(drop, &[], "")?;
					}
				}
			}
			Ok(())
		};
		if options.asynchronous == Some(true) {
			// the top level is a coroutine (`lower/units.rs`), run by the event loop
			let top_type = pointer.fn_type(&[i32.into()], false);
			let root = match entry {
				Some(entry) => {
					let top = declare(&format!("fa.main.{entry}"), top_type);
					top.set_call_conventions(crate::lower::FAST_CALLING_CONVENTION);
					let call = builder.build_call(top, &[i32.const_zero().into()], "root")?;
					call.set_call_convention(crate::lower::FAST_CALLING_CONVENTION);
					call.try_as_basic_value()
						.basic()
						.expect("a frame")
						.into_pointer_value()
				}
				None => pointer.const_null(),
			};
			let dev = i32.const_int(u64::from(options.dev()), false);
			let status = units.iter().any(|unit| unit.symbols.exit_status).then(|| {
				let global = module.add_global(i32, None, "fa.exit_status");
				global.set_visibility(GlobalVisibility::Hidden);
				global
			});
			// the end of the program: drops, flush, leaks, exit code
			let finish = |builder: &inkwell::builder::Builder<'_>| -> Result<(), Error> {
				drops(builder)?;
				builder.build_call(runtime.function("fa_flush")?, &[], "")?;
				if options.leak_check {
					builder.build_call(runtime.function("fa_leak_check")?, &[], "")?;
				}
				if let Some(status) = status {
					let code = builder.build_load(i32, status.as_pointer_value(), "")?;
					builder.build_call(runtime.exit()?, &[code.into()], "")?;
				}
				Ok(())
			};
			if target == Target::Wasm {
				// the host's event loop drives the executor (`tasks.rs`, step W1)
				let finished = module.add_function(
					"fa.finish",
					void.fn_type(&[], false),
					Some(Linkage::Internal),
				);
				runtime.add_attributes(finished, &["nounwind"]);
				let ending = context.create_builder();
				ending.position_at_end(context.append_basic_block(finished, "entry"));
				finish(&ending)?;
				ending.build_return(None)?;
				let run =
					module.add_function("fa_run", i32.fn_type(&[], false), Some(Linkage::External));
				let other = context.create_builder();
				other.position_at_end(context.append_basic_block(run, "entry"));
				let status = other
					.build_call(runtime.function("fa_wasm_run")?, &[], "")?
					.try_as_basic_value()
					.basic()
					.expect("a status");
				other.build_return(Some(&status))?;
				let wake = module.add_function(
					"fa_wake",
					void.fn_type(&[pointer.into()], false),
					Some(Linkage::External),
				);
				other.position_at_end(context.append_basic_block(wake, "entry"));
				let token = wake.get_nth_param(0).expect("a token");
				other.build_call(runtime.function("fa_wasm_wake")?, &[token.into()], "")?;
				other.build_return(None)?;
				// the exports of the loader
				for (name, function) in [("fa_run", run), ("fa_wake", wake)] {
					function.add_attribute(
						AttributeLoc::Function,
						context.create_string_attribute("wasm-export-name", name),
					);
					runtime.add_attributes(function, &["nounwind"]);
				}
				let finished = finished.as_global_value().as_pointer_value();
				builder.build_call(
					runtime.function("fa_wasm_start")?,
					&[root.into(), dev.into(), finished.into()],
					"",
				)?;
				builder.build_return(None)?;
			} else {
				builder.build_call(
					runtime.function("fa_run_main")?,
					&[root.into(), dev.into()],
					"",
				)?;
				finish(&builder)?;
				builder.build_return(Some(&i32.const_zero()))?;
			}
		} else {
			let code = match entry {
				Some(entry) => {
					let body = declare(&format!("fa.main.{entry}"), i32.fn_type(&[], false));
					builder
						.build_call(body, &[], "code")?
						.try_as_basic_value()
						.basic()
						.expect("an exit code")
						.into_int_value()
				}
				None => i32.const_zero(),
			};
			drops(&builder)?;
			if options.leak_check {
				builder.build_call(runtime.function("fa_leak_check")?, &[], "")?;
			}
			builder.build_call(runtime.function("fa_flush")?, &[], "")?;
			let exit = runtime.exit()?;
			let exits = context.append_basic_block(main, "exit");
			let returns = context.append_basic_block(main, "return");
			let nonzero =
				builder.build_int_compare(inkwell::IntPredicate::NE, code, i32.const_zero(), "")?;
			builder.build_conditional_branch(nonzero, exits, returns)?;
			builder.position_at_end(exits);
			builder.build_call(exit, &[code.into()], "")?;
			builder.build_unreachable()?;
			builder.position_at_end(returns);
			match target {
				Target::Wasm => builder.build_return(None)?,
				_ => builder.build_return(Some(&i32.const_zero()))?,
			};
		}
	}
	// `fa_resume` and `fa_destroy`, which the runtime calls (`tasks.rs`):
	// the first and the second pointer of a frame, called `fastcc`
	for (name, index) in [("fa_resume", 0u64), ("fa_destroy", 1)] {
		let Some(function) = module.get_function(name) else {
			continue;
		};
		runtime.add_attributes(function, &["nounwind"]);
		let shim = context.create_builder();
		shim.position_at_end(context.append_basic_block(function, "entry"));
		let frame = function
			.get_nth_param(0)
			.expect("a frame")
			.into_pointer_value();
		// SAFETY: the pointers at the start of a frame
		let slot = unsafe {
			shim.build_in_bounds_gep(pointer, frame, &[usize.const_int(index, false)], "")?
		};
		let target_function = shim.build_load(pointer, slot, "")?.into_pointer_value();
		let ty = void.fn_type(&[pointer.into()], false);
		let call = shim.build_indirect_call(ty, target_function, &[frame.into()], "")?;
		call.set_call_convention(crate::lower::FAST_CALLING_CONVENTION);
		shim.build_return(None)?;
	}
	// the switches over the ids of union members: each id to a unit that
	// registered it (the first, in the order of the units)
	let mut owners: std::collections::BTreeMap<u64, &str> = std::collections::BTreeMap::new();
	for unit in units {
		for &id in &unit.symbols.boxes {
			owners.entry(id).or_insert(unit.name.as_str());
		}
	}
	let switches: [(&str, inkwell::types::FunctionType<'_>); 4] = [
		("fa.drop_box", void.fn_type(&[pointer.into()], false)),
		("fa.copy_box", pointer.fn_type(&[pointer.into()], false)),
		(
			"fa_format_union",
			void.fn_type(&[pointer.into(), i32.into()], false),
		),
		(
			"fa.equal_box",
			i32.fn_type(&[pointer.into(), pointer.into()], false),
		),
	];
	for (name, ty) in switches {
		let function = match module.get_function(name) {
			Some(function) => function,
			None => module.add_function(name, ty, Some(Linkage::External)),
		};
		function.set_linkage(Linkage::External);
		if name != "fa_format_union" {
			function
				.as_global_value()
				.set_visibility(GlobalVisibility::Hidden);
		}
		runtime.add_attributes(function, &["nounwind"]);
		let box_pointer = function
			.get_nth_param(0)
			.expect("a box")
			.into_pointer_value();
		let arguments: Vec<inkwell::values::BasicMetadataValueEnum<'_>> =
			function.get_params().into_iter().map(Into::into).collect();
		builder.position_at_end(context.append_basic_block(function, "entry"));
		let end = context.append_basic_block(function, "end");
		let present = context.append_basic_block(function, "present");
		let null = builder.build_is_null(box_pointer, "")?;
		builder.build_conditional_branch(null, end, present)?;
		builder.position_at_end(present);
		let id = builder.build_load(i64, box_pointer, "id")?.into_int_value();
		let switch_block = builder.get_insert_block().expect("a block");
		// `==`: a member no unit registers has no data, its id is its value
		let unowned = if name == "fa.equal_box" {
			let unowned = context.append_basic_block(function, "unowned");
			builder.position_at_end(unowned);
			let other = function
				.get_nth_param(1)
				.expect("a box")
				.into_pointer_value();
			let compare = context.append_basic_block(function, "compare_ids");
			let different = context.append_basic_block(function, "different");
			let null = builder.build_is_null(other, "")?;
			builder.build_conditional_branch(null, different, compare)?;
			builder.position_at_end(different);
			builder.build_return(Some(&i32.const_zero()))?;
			builder.position_at_end(compare);
			let other_id = builder.build_load(i64, other, "")?.into_int_value();
			let same = builder.build_int_compare(inkwell::IntPredicate::EQ, id, other_id, "")?;
			let same = builder.build_int_z_extend(same, i32, "")?;
			builder.build_return(Some(&same))?;
			unowned
		} else {
			end
		};
		let mut cases = Vec::new();
		for (&box_id, unit) in &owners {
			let case = context.append_basic_block(function, "case");
			builder.position_at_end(case);
			let target_function = declare(&format!("{name}.{unit}"), ty);
			let call = builder.build_call(target_function, &arguments, "")?;
			match call.try_as_basic_value().basic() {
				Some(value) => builder.build_return(Some(&value))?,
				None => builder.build_return(None)?,
			};
			cases.push((i64.const_int(box_id, false), case));
		}
		builder.position_at_end(switch_block);
		builder.build_switch(id, unowned, &cases)?;
		builder.position_at_end(end);
		if name == "fa.copy_box" {
			builder.build_return(Some(&box_pointer))?;
		} else if name == "fa.equal_box" {
			// a null box: equal to itself only
			let other = function
				.get_nth_param(1)
				.expect("a box")
				.into_pointer_value();
			let same =
				builder.build_int_compare(inkwell::IntPredicate::EQ, box_pointer, other, "")?;
			let same = builder.build_int_z_extend(same, i32, "")?;
			builder.build_return(Some(&same))?;
		} else {
			builder.build_return(None)?;
		}
	}
	module.verify()?;
	// bitcode in release builds: what nothing reaches from `main` is dropped
	let entry_options = SplitOptions {
		lto: split.lto,
		full_lto: split.full_lto,
		..SplitOptions::default()
	};
	emit(
		&module,
		&machine,
		options,
		&entry_options,
		path,
		&mut Timings::default(),
	)
}

/// The load and unload functions of a C library compiled per unit
/// (`lower/c_library.rs`): `fa.library.init` (in `llvm.global_ctors`) runs
/// the initializations in the program's order and the API file's top
/// level, `fa.library.fini` (in `llvm.global_dtors`) the drops of the
/// module values (when one does more than free memory, or the leaks are
/// counted), the leak report and the flush.
#[allow(clippy::too_many_arguments)]
fn library_entry<'c>(
	module: &Module<'c>,
	runtime: &Runtime<'c, '_>,
	target: Target,
	options: &Options,
	inputs: &EntryInputs,
	defined_inits: &std::collections::HashSet<(&str, u32)>,
	defined_drops: &std::collections::HashSet<(&str, u32)>,
	effectful: bool,
) -> Result<(), Error> {
	let context = module.get_context();
	let void = context.void_type();
	let i32 = context.i32_type();
	let builder = context.create_builder();
	let declare = |name: &str, ty| declare_hidden(module, name, ty);
	let init = module.add_function(
		"fa.library.init",
		void.fn_type(&[], false),
		Some(Linkage::Internal),
	);
	runtime.add_attributes(init, &["nounwind"]);
	builder.position_at_end(context.append_basic_block(init, "start"));
	let mut seen = std::collections::HashSet::new();
	let mut initialized = Vec::new();
	for (module_path, node) in &inputs.init_order {
		if !seen.insert((module_path.as_str(), *node)) {
			continue;
		}
		if !defined_inits.contains(&(module_path.as_str(), *node)) {
			return Err(Error::Unsupported {
				message: format!("the initialization of a value of `{module_path}` (split build)"),
				location: String::new(),
			});
		}
		let init = declare(
			&format!("fa.init.{module_path}.{node}"),
			void.fn_type(&[], false),
		);
		builder.build_call(init, &[], "")?;
		initialized.push((module_path.as_str(), *node));
	}
	if let Some(entry) = inputs
		.units
		.iter()
		.find_map(|unit| unit.symbols.entry.as_ref())
	{
		let body = declare(&format!("fa.main.{entry}"), i32.fn_type(&[], false));
		builder.build_call(body, &[], "")?;
	}
	builder.build_return(None)?;
	let fini = module.add_function(
		"fa.library.fini",
		void.fn_type(&[], false),
		Some(Linkage::Internal),
	);
	runtime.add_attributes(fini, &["nounwind"]);
	builder.position_at_end(context.append_basic_block(fini, "start"));
	if effectful || options.leak_check {
		for (module_path, node) in initialized.iter().rev() {
			if defined_drops.contains(&(*module_path, *node)) {
				let drop = declare(
					&format!("fa.drop.{module_path}.{node}"),
					void.fn_type(&[], false),
				);
				builder.build_call(drop, &[], "")?;
			}
		}
	}
	if options.leak_check {
		builder.build_call(runtime.function("fa_leak_check")?, &[], "")?;
	}
	builder.build_call(runtime.function("fa_flush")?, &[], "")?;
	builder.build_return(None)?;
	crate::lower::c_library::library_hooks(module, runtime, target, init, fini)?;
	Ok(())
}

/// A function of another object, hidden.
fn declare_hidden<'c>(
	module: &Module<'c>,
	name: &str,
	ty: inkwell::types::FunctionType<'c>,
) -> FunctionValue<'c> {
	module.get_function(name).unwrap_or_else(|| {
		let function = module.add_function(name, ty, Some(Linkage::External));
		function
			.as_global_value()
			.set_visibility(GlobalVisibility::Hidden);
		function
	})
}

/// The memory of a full LTO link, estimated from the bitcode it reads
/// (units, entry and runtime): measured on the generated packages (100
/// and 1,000 files: 1.96 MB → 116 MB, 16.5 MB → 373 MB) and on 50,000
/// lines of functions (1.69 MB → 112 MB), about 80 MB and 18 times the
/// bitcode.
fn full_lto_memory(bitcode: u64) -> u64 {
	const RUNTIME_AND_ENTRY: u64 = 320 << 10;
	(80 << 20) + 18 * (bitcode + RUNTIME_AND_ENTRY)
}

/// The LTO of a release link (design/decisions.md, "Full LTO when it fits
/// the memory budget"): full LTO when its estimated memory fits
/// `FA_LLVM_MEMORY` (512 MB by default), ThinLTO above it (`FA_LTO=full`
/// or `thin` forces one). The units' ThinLTO bitcode is written again
/// without its summary for full LTO. Deterministic: the bitcode is.
fn choose_lto(split: &SplitOptions, inputs: &EntryInputs) -> Result<SplitOptions, Error> {
	let mut bitcode = 0;
	for unit in &inputs.units {
		bitcode += fs::metadata(&unit.object)?.len();
	}
	let estimate = full_lto_memory(bitcode);
	let budget = llvm_budget() << 20;
	let forced = std::env::var("FA_LTO").ok();
	let full = match forced.as_deref() {
		Some("thin") => false,
		Some("full") => true,
		_ => estimate <= budget,
	};
	if split.verbose {
		eprintln!(
			"fa build: {} (full LTO estimated at {} MB for {} KB of bitcode; the budget is {} MB, FA_LLVM_MEMORY)",
			if full {
				"linked with full LTO"
			} else {
				"linked with ThinLTO"
			},
			estimate >> 20,
			bitcode >> 10,
			budget >> 20,
		);
	}
	if !full {
		return Ok(split.clone());
	}
	// the units without their summaries: one module optimized as a whole
	for unit in &inputs.units {
		let context = Context::create();
		let buffer = inkwell::memory_buffer::MemoryBuffer::create_from_file(&unit.object)?;
		let module = context.create_module_from_ir(buffer)?;
		let partial = crate::compile::with_suffix(&unit.object, ".partial");
		if !module.write_bitcode_to_path(&partial) {
			return Err(Error::Llvm(format!("can't write `{}`", partial.display())));
		}
		fs::rename(&partial, &unit.object)?;
	}
	Ok(SplitOptions {
		full_lto: true,
		..split.clone()
	})
}

/// The memory budget of the LLVM work, in MB (`FA_LLVM_MEMORY`, 512 by
/// default).
fn llvm_budget() -> u64 {
	std::env::var("FA_LLVM_MEMORY")
		.ok()
		.and_then(|value| value.parse().ok())
		.unwrap_or(512)
}

/// The objects of a program: its units, its entry and the runtime.
fn program_objects(
	target: Target,
	options: &Options,
	split: &SplitOptions,
	inputs: &EntryInputs,
	directory: &Path,
	library: bool,
) -> Result<Vec<PathBuf>, Error> {
	fs::create_dir_all(directory)?;
	let entry = directory.join(format!("$entry.{}", object_extension(split)));
	entry_object(target, options, split, inputs, &entry, library)?;
	let runtime = runtime_object(target, options, split, directory)?;
	let mut objects: Vec<PathBuf> = inputs
		.units
		.iter()
		.map(|unit| unit.object.clone())
		.collect();
	objects.push(entry);
	objects.push(runtime);
	Ok(objects)
}

/// Loads the units of a program in this process ([`SplitOptions::in_memory`]).
#[cfg(unix)]
fn load_program(
	target: Target,
	options: &Options,
	split: &SplitOptions,
	inputs: &EntryInputs,
	directory: &Path,
) -> Result<Box<dyn crate::LoadedProgram>, Error> {
	let mut timings = Timings::default();
	let start = Instant::now();
	let objects = program_objects(target, options, split, inputs, directory, false)?;
	timings.emit = start.elapsed();
	crate::jit::load_objects(&objects, timings)
}

/// Without `fork`, no program is loaded in memory.
#[cfg(not(unix))]
fn load_program(
	_target: Target,
	_options: &Options,
	_split: &SplitOptions,
	_inputs: &EntryInputs,
	_directory: &Path,
) -> Result<Box<dyn crate::LoadedProgram>, Error> {
	Err(Error::Jit("no JIT on this system".to_string()))
}

/// Links the units of a program into an executable (native) or a `.wasm`
/// module and its JavaScript loader (wasm), written at `output`.
pub fn link_program(
	target: Target,
	options: &Options,
	split: &SplitOptions,
	inputs: &EntryInputs,
	directory: &Path,
	output: &Path,
	libraries: &Libraries,
) -> Result<Artifact, Error> {
	let mut timings = Timings::default();
	let mut output = output.to_path_buf();
	if target == Target::Wasm
		&& output
			.extension()
			.is_none_or(|extension| extension != "wasm")
	{
		output = crate::compile::with_suffix(&output, ".wasm");
	}
	if let Some(parent) = output.parent()
		&& !parent.as_os_str().is_empty()
	{
		fs::create_dir_all(parent)?;
	}
	let start = Instant::now();
	// release: full LTO when its memory fits the budget, else ThinLTO
	let chosen;
	let split = if split.lto && !split.full_lto {
		chosen = choose_lto(split, inputs)?;
		&chosen
	} else {
		split
	};
	let objects = program_objects(
		target,
		options,
		split,
		inputs,
		directory,
		libraries.c_library.is_some(),
	)?;
	timings.emit = start.elapsed();
	let start = Instant::now();
	if let Some(library) = &libraries.c_library {
		// a C library: its shared and static libraries (the header is
		// written by `link_units`)
		let folder = output.parent().unwrap_or(Path::new(".")).to_path_buf();
		let shared = folder.join(library.output_name(target));
		let archive = folder.join(library.static_name());
		let mut exports: Vec<String> = inputs
			.units
			.iter()
			.flat_map(|unit| unit.symbols.c_exports.iter().cloned())
			.collect();
		exports.sort();
		exports.dedup();
		if target == Target::Wasm {
			exports.extend(
				crate::lower::c_library::WASM_EXPORTS
					.iter()
					.map(|name| (*name).to_string()),
			);
		}
		link::link_c_library(
			target,
			options,
			Some(split),
			&objects,
			&exports,
			&shared,
			&archive,
			libraries,
		)?;
		timings.link = start.elapsed();
		let loader = if target == Target::Wasm {
			Some(link::write_library_loader(&shared, options.leak_check)?)
		} else {
			None
		};
		return Ok(Artifact {
			target,
			options: *options,
			output_size: fs::metadata(&shared)?.len(),
			output: shared,
			loader,
			ir: None,
			c_library: Some(crate::artifact::CLibraryArtifacts {
				archive: (target == Target::Native).then_some(archive),
				header: PathBuf::new(),
			}),
			timings,
		});
	}
	link::link_objects(target, options, split, &objects, &output, libraries)?;
	timings.link = start.elapsed();
	let loader = if target == Target::Wasm {
		Some(link::write_loader(&output, options.leak_check)?)
	} else {
		None
	};
	Ok(Artifact {
		target,
		options: *options,
		output_size: fs::metadata(&output)?.len(),
		output,
		loader,
		ir: None,
		c_library: None,
		timings,
	})
}

/// The LLVM jobs that may run at once: `min(cores, budget / 96 MB)`, the
/// budget `FA_LLVM_MEMORY` (MB, default 512).
fn llvm_jobs() -> usize {
	let cores = std::thread::available_parallelism().map_or(1, usize::from);
	cores.min(llvm_budget() as usize / 96).max(1)
}

/// Records the units of the run (their modules, tests left out): they are
/// compiled once the run is over and the declarations the entry reaches
/// are known.
#[derive(Default)]
struct UnitRecorder {
	units: std::sync::Mutex<Vec<Vec<String>>>,
	/// The whole-program facts of the units so far, merged (`facts.rs`).
	facts: std::sync::Mutex<crate::facts::ProgramFacts>,
	map_facts: std::sync::Mutex<crate::map_facts::MapFacts>,
}

impl fa_analyzer::pipeline::UnitSink for UnitRecorder {
	fn unit(&self, session: &mut Session, files: &[FileId]) {
		self.map_facts
			.lock()
			.unwrap_or_else(|poisoned| poisoned.into_inner())
			.collect(session, files);
		let facts = crate::facts::ProgramFacts::of_unit(session, files);
		self.facts
			.lock()
			.unwrap_or_else(|poisoned| poisoned.into_inner())
			.merge(facts);
		let modules: Vec<String> = files
			.iter()
			.filter(|&&file| session.role(file) != fa_analyzer::FileRole::Test)
			.map(|&file| session.module(file).to_string())
			.collect();
		if !modules.is_empty() {
			self.units
				.lock()
				.unwrap_or_else(|poisoned| poisoned.into_inner())
				.push(modules);
		}
	}
}

/// The declarations the program reaches: their symbols, and their modules.
#[derive(Default)]
struct Reach {
	symbols: FxHashSet<String>,
	modules: FxHashSet<String>,
}

/// The declarations the entry reaches, from the call graph the interfaces
/// record (`Declared::uses`: what a declaration names or calls directly):
/// the entry's uses and its entry function, then what they use. A type
/// reached reaches its functions (methods called through a constraint or
/// a function value), conservatively.
fn reach(
	root: &Session,
	report: &fa_analyzer::pipeline::Report,
	entry: &str,
	asynchronous: bool,
	library: bool,
) -> Reach {
	let types = root.types();
	let mut stack: Vec<DeclKey> = Vec::new();
	if let Some(interface) = report.interface(root, entry) {
		stack.extend(interface.uses.iter().copied());
		stack.extend(interface.export.iter().map(|export| export.key));
		// the API file of a C library: C calls every declaration of it
		if library {
			stack.extend(interface.keys.keys().copied());
		}
	}
	let mut seen: FxHashSet<DeclKey> = FxHashSet::default();
	// the functions that may suspend, whose async instances an asynchronous
	// program may await or start
	let mut suspending: Vec<DeclKey> = Vec::new();
	while let Some(key) = stack.pop() {
		if !seen.insert(key) {
			continue;
		}
		let Some(interface) = report.shared.interface(types.decl_key_data(key).module) else {
			continue;
		};
		let Some(keyed) = interface.keys.get(&key) else {
			continue;
		};
		if asynchronous
			&& interface
				.async_summaries
				.get(&keyed.node)
				.is_some_and(|summary| summary.suspends || summary.starts)
		{
			suspending.push(key);
		}
		if let Some(declared) = interface.declared.get(&keyed.node) {
			// a namespace's uses are its members': they are reached one by one
			if declared.kind != fa_analyzer::symbols::SymbolKind::Namespace {
				stack.extend(declared.uses.iter().copied());
			}
			if declared.kind == fa_analyzer::symbols::SymbolKind::Type {
				for function in interface
					.type_functions
					.get(&keyed.node)
					.into_iter()
					.flatten()
				{
					stack.push(function.key);
				}
			}
		}
	}
	let mut reach = Reach::default();
	for key in suspending {
		let symbol = crate::lower::units::mangle(types, key);
		reach.symbols.insert(format!("{symbol}.async"));
	}
	for key in seen {
		reach
			.symbols
			.insert(crate::lower::units::mangle(types, key));
		reach.modules.insert(
			types
				.module_path(types.decl_key_data(key).module)
				.to_string(),
		);
	}
	reach
}

/// The key of a unit in the object cache of development builds: its
/// modules' sources, the hashes of the interfaces they read, the entry,
/// the declarations of the unit the program reaches, and the options.
#[allow(clippy::too_many_arguments)]
fn unit_cache_key(
	root: &Session,
	cache: &Path,
	modules: &[String],
	entry: &str,
	reached: &FxHashSet<String>,
	target: Target,
	options: &Options,
	split: &SplitOptions,
) -> PathBuf {
	let mut hasher = StableHasher::default();
	options_key(target, options, split, &mut hasher);
	let mut read = std::collections::BTreeSet::new();
	for module in modules {
		module.hash(&mut hasher);
		(module == entry).hash(&mut hasher);
		root.resolver()
			.and_then(|resolver| resolver.source(module))
			.as_deref()
			.hash(&mut hasher);
		if let Some(scan) = root.scan_of(module) {
			read.extend(scan.dependencies.iter().cloned());
		}
		// the unit's declarations the program reaches, in order
		let prefix = format!("fa.{module}.");
		let mut own: Vec<&String> = reached
			.iter()
			.filter(|symbol| symbol.starts_with(&prefix))
			.collect();
		own.sort();
		own.hash(&mut hasher);
	}
	for module in read {
		module.hash(&mut hasher);
		if let Some(id) = root.types().existing_module(&module)
			&& let Some(interface) = root.interface(id)
		{
			interface.hash(root.types()).hash(&mut hasher);
		}
	}
	cache.join(format!(
		"{}-{:016x}",
		file_name(&modules[0]),
		hasher.finish()
	))
}

/// What the units are compiled with.
struct Settings<'a> {
	target: Target,
	options: Options,
	split: &'a SplitOptions,
	directory: &'a Path,
}

/// Compiles units the entry reaches, in parallel (at most [`llvm_jobs`] at
/// once, on threads with large stacks for the analysis): each is analyzed
/// again against the published interfaces, unless the object cache has it.
fn compile_units(
	root: &mut Session,
	report: &fa_analyzer::pipeline::Report,
	units: &[Vec<String>],
	entry: &str,
	reached: &FxHashSet<String>,
	settings: &Settings<'_>,
) -> Result<Vec<UnitOutput>, Error> {
	let keys: Vec<Option<PathBuf>> = units
		.iter()
		.map(|modules| {
			let cache = settings
				.split
				.cache
				.as_ref()
				.filter(|_| !settings.split.lto)?;
			Some(unit_cache_key(
				root,
				cache,
				modules,
				entry,
				reached,
				settings.target,
				&settings.options,
				settings.split,
			))
		})
		.collect();
	let reanalyzer = report.reanalyzer(root);
	let next = std::sync::atomic::AtomicUsize::new(0);
	let outputs: std::sync::Mutex<Vec<UnitOutput>> = std::sync::Mutex::new(Vec::new());
	let failure: std::sync::Mutex<Option<Error>> = std::sync::Mutex::new(None);
	let work = || {
		loop {
			let index = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
			let Some(modules) = units.get(index) else {
				return;
			};
			if failure
				.lock()
				.unwrap_or_else(|poisoned| poisoned.into_inner())
				.is_some()
			{
				return;
			}
			let compiled = (|| {
				if let Some(key) = &keys[index]
					&& let Some(cached) = cached_unit(key, &modules[0], settings.split)
				{
					return Ok(cached);
				}
				let paths: Vec<&str> = modules.iter().map(String::as_str).collect();
				let (session, files) = reanalyzer.analyze(&paths);
				let entry_file = files
					.iter()
					.copied()
					.find(|&file| session.module(file) == entry);
				compile_unit(
					&session,
					&files,
					entry_file,
					settings.target,
					&settings.options,
					settings.split,
					settings.directory,
					Some(reached),
					keys[index].as_deref(),
				)
			})();
			match compiled {
				Ok(unit) => outputs
					.lock()
					.unwrap_or_else(|poisoned| poisoned.into_inner())
					.push(unit),
				Err(error) => {
					failure
						.lock()
						.unwrap_or_else(|poisoned| poisoned.into_inner())
						.get_or_insert(error);
				}
			}
		}
	};
	std::thread::scope(|scope| {
		for _ in 0..llvm_jobs().min(units.len()) {
			let spawned = std::thread::Builder::new()
				// the analysis recurses deeply (design/parallel-pipeline.md P8)
				.stack_size(64 << 20)
				.spawn_scoped(scope, work);
			if spawned.is_err() {
				work();
			}
		}
	});
	if let Some(error) = failure
		.into_inner()
		.unwrap_or_else(|poisoned| poisoned.into_inner())
	{
		return Err(error);
	}
	Ok(outputs
		.into_inner()
		.unwrap_or_else(|poisoned| poisoned.into_inner()))
}

/// A unit of the object cache, if its object and symbols are there.
fn cached_unit(key: &Path, name: &str, split: &SplitOptions) -> Option<UnitOutput> {
	let object = crate::compile::with_suffix(key, &format!(".{}", object_extension(split)));
	// the facts its lowering consulted still have the same answers
	let consulted = fs::read_to_string(crate::compile::with_suffix(key, ".facts")).ok()?;
	if !crate::facts::FactLog::still_holds(&consulted, split.facts.as_deref()) {
		return None;
	}
	let text = fs::read_to_string(crate::compile::with_suffix(key, ".symbols")).ok()?;
	let symbols = UnitSymbols::from_text(&text)?;
	object.is_file().then(|| UnitOutput {
		name: name.to_string(),
		object,
		symbols,
		cached: true,
		requests: Vec::new(),
	})
}

/// Analyzes a package on the pipeline (`fa_analyzer::pipeline`, the entry
/// `run.entry`), then compiles the units holding what the entry reaches
/// (each analyzed again, in parallel: at most `min(cores, budget / 96 MB)`
/// LLVM jobs at once), the instances of generic functions they request,
/// and links the program: the executable (native) or `.wasm` module
/// (wasm) at `output`, the objects in `objects` (removed after the link).
#[allow(clippy::too_many_arguments)]
pub fn compile_package(
	root: &mut Session,
	run: &fa_analyzer::pipeline::Options,
	target: Target,
	options: &Options,
	split: &SplitOptions,
	objects: &Path,
	output: &Path,
	libraries: &Libraries,
) -> (fa_analyzer::pipeline::Report, PackageOutcome) {
	let recorder = UnitRecorder::default();
	let report = fa_analyzer::pipeline::analyze_package(root, run, None, &recorder);
	let units = recorder
		.units
		.into_inner()
		.unwrap_or_else(|poisoned| poisoned.into_inner());
	// every unit's facts, merged, for every lowering (`facts.rs`)
	let mut facts = recorder
		.facts
		.into_inner()
		.unwrap_or_else(|poisoned| poisoned.into_inner());
	facts.merge(
		recorder
			.map_facts
			.into_inner()
			.unwrap_or_else(|poisoned| poisoned.into_inner())
			.resolve(root.types()),
	);
	facts.finish();
	let split = &SplitOptions {
		facts: Some(std::sync::Arc::new(facts)),
		c_library: libraries.c_library.clone(),
		..split.clone()
	};
	// a C library runs in synchronous mode (`lower/c_library.rs`)
	let options = &Options {
		asynchronous: match libraries.c_library {
			Some(_) => Some(false),
			None => options.asynchronous,
		},
		..*options
	};
	let settings = Settings {
		target,
		options: *options,
		split,
		directory: objects,
	};
	let outcome = match link_units(root, &report, units, &settings, output, libraries) {
		Ok(Linked::Executable(artifact)) => PackageOutcome::Built(artifact),
		Ok(Linked::Loaded(program)) => PackageOutcome::Loaded(program),
		Err(None) => PackageOutcome::Errors,
		Err(Some(error @ Error::Unsupported { .. })) => {
			PackageOutcome::Unsupported(error.to_string())
		}
		Err(Some(error)) => PackageOutcome::Failed(error),
	};
	// `FA_KEEP_OBJECTS=1` keeps them (to study the link)
	if std::env::var_os("FA_KEEP_OBJECTS").is_none() {
		let _ = fs::remove_dir_all(objects);
	}
	(report, outcome)
}

/// The end of [`compile_package`]: the units the entry reaches compiled,
/// the instances they request, and the link (`Err(None)`: the analysis
/// found errors).
fn link_units(
	root: &mut Session,
	report: &fa_analyzer::pipeline::Report,
	mut units: Vec<Vec<String>>,
	settings: &Settings<'_>,
	output: &Path,
	libraries: &Libraries,
) -> Result<Linked, Option<Error>> {
	if report.has_errors() {
		return Err(None);
	}
	let unsupported = |message: &str| {
		Some(Error::Unsupported {
			message: message.to_string(),
			location: String::new(),
		})
	};
	let Some(entry) = report.entry.clone() else {
		return Err(unsupported("a package without an entry"));
	};
	// the asynchronous mode (design/async.md A6): configured, else detected
	let asynchronous = settings
		.options
		.asynchronous
		.unwrap_or_else(|| entry_is_asynchronous(root, report, &entry));
	let settings = &Settings {
		options: Options {
			asynchronous: Some(asynchronous),
			..settings.options
		},
		..*settings
	};
	let mut reached = reach(
		root,
		report,
		&entry,
		asynchronous,
		settings.split.c_library.is_some(),
	);
	units.sort();
	// the units holding the entry or a declaration it reaches
	let holds = |modules: &Vec<String>, wanted: &FxHashSet<String>| {
		modules
			.iter()
			.any(|module| *module == entry || wanted.contains(module))
	};
	let mut pending: Vec<Vec<String>> = units
		.iter()
		.filter(|modules| holds(modules, &reached.modules))
		.cloned()
		.collect();
	let mut outputs: std::collections::BTreeMap<String, UnitOutput> = Default::default();
	let mut seen: std::collections::HashSet<(String, String)> = std::collections::HashSet::new();
	let mut wave = 0;
	for _round in 0..8 {
		if std::env::var_os("FA_SPLIT_TRACE").is_some() {
			eprintln!(
				"split: {} units, {} reached declarations",
				pending.len(),
				reached.symbols.len()
			);
		}
		let trace = std::env::var_os("FA_SPLIT_TRACE").is_some();
		let started = Instant::now();
		let compiled = compile_units(root, report, &pending, &entry, &reached.symbols, settings)
			.map_err(Some)?;
		if trace {
			let built: Vec<&str> = compiled
				.iter()
				.filter(|unit| !unit.cached)
				.map(|unit| unit.name.as_str())
				.collect();
			eprintln!(
				"split: units in {} ms, {} compiled: {built:?}",
				started.elapsed().as_millis(),
				built.len()
			);
		}
		let started = Instant::now();
		let requests: Vec<InstanceRequest> = compiled
			.iter()
			.flat_map(|unit| unit.requests.iter().cloned())
			.collect();
		for unit in compiled {
			outputs.insert(unit.name.clone(), unit);
		}
		// the instances of generic functions requested across units, by waves
		let instances =
			instance_waves(root, report, settings, requests, &mut seen, &mut wave).map_err(Some)?;
		for unit in instances {
			outputs.insert(unit.name.clone(), unit);
		}
		// the entries of the tables of generic function values, by waves of
		// the producers' modules (they may request instances, and more)
		generic_waves(root, report, settings, &mut outputs, &mut seen, &mut wave).map_err(Some)?;
		if trace {
			eprintln!(
				"split: instance waves in {} ms",
				started.elapsed().as_millis()
			);
		}
		// what a unit calls that no unit defines (the reachability of the
		// interfaces missed it): its unit compiled again, reaching it
		let exported: FxHashSet<&str> = outputs
			.values()
			.flat_map(|unit| unit.symbols.exports.iter().map(String::as_str))
			.collect();
		let missing: Vec<(String, String)> = outputs
			.values()
			.flat_map(|unit| unit.symbols.needs.iter())
			.filter(|(_, symbol)| !exported.contains(symbol.as_str()))
			.cloned()
			.collect();
		if missing.is_empty() {
			let mut units: Vec<UnitOutput> = outputs.into_values().collect();
			units.sort_by(|a, b| a.name.cmp(&b.name));
			if !units
				.iter()
				.any(|unit| unit.symbols.entry.as_deref() == Some(entry.as_str()))
			{
				return Err(unsupported("the entry module was not compiled"));
			}
			// the values a compiled unit initializes, in the program's order
			let defined: FxHashSet<(&str, u32)> = units
				.iter()
				.flat_map(|unit| unit.symbols.inits.iter())
				.map(|(module, node)| (module.as_str(), *node))
				.collect();
			let init_order = report
				.init_order
				.iter()
				.map(|(module, node)| (module.clone(), node.0))
				.filter(|(module, node)| defined.contains(&(module.as_str(), *node)))
				.collect();
			let generic_tables = generic_tables(&units).map_err(Some)?;
			let started = Instant::now();
			let inputs = EntryInputs {
				init_order,
				units,
				generic_tables,
			};
			// loaded by the JIT; an object it can't load is linked (the
			// emulated thread-local variables link too)
			if settings.split.in_memory {
				match load_program(
					settings.target,
					&settings.options,
					settings.split,
					&inputs,
					settings.directory,
				) {
					Ok(program) => {
						if trace {
							eprintln!(
								"split: entry and load in {} ms",
								started.elapsed().as_millis()
							);
						}
						return Ok(Linked::Loaded(program));
					}
					// linked below (`FA_JIT_TRACE=1` says why)
					Err(Error::Jit(_)) => {}
					Err(error) => return Err(Some(error)),
				}
			}
			let linked = link_program(
				settings.target,
				&settings.options,
				settings.split,
				&inputs,
				settings.directory,
				output,
				libraries,
			)
			.map_err(Some);
			if trace {
				eprintln!(
					"split: entry and link in {} ms",
					started.elapsed().as_millis()
				);
			}
			let mut artifact = linked?;
			// the header of a C library, from its API file's interface
			if let (Some(library), Some(mut extra)) =
				(&libraries.c_library, artifact.c_library.clone())
			{
				let interface = report
					.interface(root, &entry)
					.ok_or_else(|| unsupported("the API file of the C library was not analyzed"))?;
				let text = crate::header::c_header(root.types(), interface, library).map_err(
					|message| {
						Some(Error::Unsupported {
							message: format!("the header of the C library: {message}"),
							location: library.api.clone(),
						})
					},
				)?;
				let directory = artifact
					.output
					.parent()
					.map(Path::to_path_buf)
					.unwrap_or_default();
				let header = directory.join(&library.header);
				fs::write(&header, text).map_err(|error| Some(Error::from(error)))?;
				extra.header = header;
				artifact.c_library = Some(extra);
			}
			return Ok(Linked::Executable(artifact));
		}
		if std::env::var_os("FA_SPLIT_TRACE").is_some() {
			eprintln!("split: missing {missing:?}");
		}
		let mut modules = FxHashSet::default();
		for (module, symbol) in missing {
			reached.symbols.insert(symbol);
			modules.insert(module);
		}
		pending = units
			.iter()
			.filter(|unit| unit.iter().any(|module| modules.contains(module)))
			.cloned()
			.collect();
		if pending.is_empty() {
			return Err(unsupported("a declaration of no unit (split build)"));
		}
	}
	Err(unsupported(
		"declarations reached through too many rounds (split build)",
	))
}

/// The instance waves (design/parallel-pipeline.md §3.3): the instances of
/// generic functions units request from other units are made by the units
/// declaring them, analyzed again; the requests of these instances make
/// the next wave, until none is new.
fn instance_waves(
	root: &mut Session,
	report: &fa_analyzer::pipeline::Report,
	settings: &Settings<'_>,
	mut pending: Vec<InstanceRequest>,
	seen: &mut std::collections::HashSet<(String, String)>,
	wave: &mut usize,
) -> Result<Vec<UnitOutput>, Error> {
	let mut units = Vec::new();
	while !pending.is_empty() {
		*wave += 1;
		if *wave > 64 {
			return Err(Error::Unsupported {
				message: "instances of generic functions nested this deep (split build)"
					.to_string(),
				location: String::new(),
			});
		}
		// by declaring module, each instance once
		let mut groups: std::collections::BTreeMap<String, Vec<InstanceRequest>> =
			std::collections::BTreeMap::new();
		for request in pending.drain(..) {
			let types = root.types();
			let module = types
				.module_path(types.decl_key_data(request.key).module)
				.to_string();
			let symbol = crate::lower::units::callback_instance_symbol(
				types,
				request.key,
				&request.arguments,
				&request.callbacks,
				&request.methods,
			);
			if seen.insert((module.clone(), symbol)) {
				groups.entry(module).or_default().push(request);
			}
		}
		for (module, requests) in groups {
			let (session, files) = report.reanalyze(root, &[module.as_str()]);
			if files.is_empty() {
				return Err(Error::Unsupported {
					message: format!("instances of generic functions of `{module}` (split build)"),
					location: String::new(),
				});
			}
			let unit = compile_instances(
				&session,
				&files,
				&requests,
				&[],
				*wave,
				settings.target,
				&settings.options,
				settings.split,
				settings.directory,
			)?;
			pending.extend(unit.requests.iter().cloned());
			units.push(unit);
		}
	}
	Ok(units)
}

/// The classes of generic function types the units call, each with its
/// instances by key (sorted: the indices), and the producers of values,
/// by table.
type GenericClasses = (
	std::collections::BTreeMap<String, std::collections::BTreeMap<String, GenericInstance>>,
	std::collections::BTreeMap<String, GenericProducer>,
);

fn generic_classes(units: &[&UnitOutput]) -> Result<GenericClasses, Error> {
	let mut classes: std::collections::BTreeMap<
		String,
		std::collections::BTreeMap<String, GenericInstance>,
	> = Default::default();
	let mut producers: std::collections::BTreeMap<String, GenericProducer> = Default::default();
	for unit in units {
		let needs = &unit.symbols.generic_values;
		for instance in &needs.instances {
			classes
				.entry(instance.class.clone())
				.or_default()
				.entry(instance.key.clone())
				.or_insert_with(|| instance.clone());
		}
		for producer in &needs.producers {
			classes.entry(producer.class.clone()).or_default();
			producers
				.entry(producer.table.clone())
				.or_insert_with(|| producer.clone());
		}
	}
	// a class that keeps growing: a polymorphic recursion (as in one module)
	if classes
		.values()
		.any(|instances| instances.len() > crate::lower::generic_values::MOST_INSTANCES)
	{
		return Err(Error::Diagnostics(
			"error: a generic function type is called at a new type in each of its instances: a polymorphic recursion has no end of instances to compile; call it at types that don't grow\n".to_string(),
		));
	}
	Ok((classes, producers))
}

/// The waves making the entries of the tables of generic function values
/// (design/decisions.md, "Generic function tables per unit"): each producer
/// gets an entry for each instance of its class, made by its module
/// analyzed again; what these entries call makes instance waves, and the
/// instances may call generic function values at new types, until nothing
/// is new.
fn generic_waves(
	root: &mut Session,
	report: &fa_analyzer::pipeline::Report,
	settings: &Settings<'_>,
	outputs: &mut std::collections::BTreeMap<String, UnitOutput>,
	seen: &mut std::collections::HashSet<(String, String)>,
	wave: &mut usize,
) -> Result<(), Error> {
	let mut requested: FxHashSet<String> = FxHashSet::default();
	loop {
		let units: Vec<&UnitOutput> = outputs.values().collect();
		let (classes, producers) = generic_classes(&units)?;
		let defined: FxHashSet<&str> = units
			.iter()
			.flat_map(|unit| unit.symbols.generic_values.entries.iter())
			.map(String::as_str)
			.collect();
		// by producer's module, in a fixed order
		let mut groups: std::collections::BTreeMap<String, Vec<GenericEntryRequest>> =
			Default::default();
		for producer in producers.values() {
			for instance in classes[&producer.class].values() {
				let symbol = producer.entry_symbol(instance);
				if defined.contains(symbol.as_str()) || !requested.insert(symbol) {
					continue;
				}
				groups
					.entry(producer.module.clone())
					.or_default()
					.push(GenericEntryRequest {
						producer: producer.clone(),
						instance: instance.clone(),
					});
			}
		}
		if groups.is_empty() {
			return Ok(());
		}
		*wave += 1;
		if *wave > 64 {
			return Err(Error::Unsupported {
				message: "generic function values nested this deep (split build)".to_string(),
				location: String::new(),
			});
		}
		let mut requests = Vec::new();
		for (module, entries) in groups {
			let (session, files) = report.reanalyze(root, &[module.as_str()]);
			if files.is_empty() {
				return Err(Error::Unsupported {
					message: format!("generic function values of `{module}` (split build)"),
					location: String::new(),
				});
			}
			let unit = compile_instances(
				&session,
				&files,
				&[],
				&entries,
				*wave,
				settings.target,
				&settings.options,
				settings.split,
				settings.directory,
			)?;
			requests.extend(unit.requests.iter().cloned());
			outputs.insert(unit.name.clone(), unit);
		}
		for unit in instance_waves(root, report, settings, requests, seen, wave)? {
			outputs.insert(unit.name.clone(), unit);
		}
	}
}

/// The tables and the indices the entry object defines: an index is the
/// position of its instance in its class (sorted by key), a table holds
/// its producer's entry for each.
fn generic_tables(units: &[UnitOutput]) -> Result<GenericTables, Error> {
	let units: Vec<&UnitOutput> = units.iter().collect();
	let (classes, producers) = generic_classes(&units)?;
	let mut tables = GenericTables::default();
	for instances in classes.values() {
		for (position, instance) in instances.values().enumerate() {
			tables
				.indices
				.push((instance.index_symbol(), position as u64));
		}
	}
	for producer in producers.values() {
		let entries = classes[&producer.class]
			.values()
			.map(|instance| producer.entry_symbol(instance))
			.collect();
		tables.tables.push((producer.table.clone(), entries));
	}
	Ok(tables)
}

/// Whether the program runs in asynchronous mode (design/async.md A6): its
/// entry's top level or entry function starts a task, or awaits something
/// that may suspend.
fn entry_is_asynchronous(
	root: &Session,
	report: &fa_analyzer::pipeline::Report,
	entry: &str,
) -> bool {
	let Some(interface) = report.interface(root, entry) else {
		return false;
	};
	let top_level = fa_analyzer::ids::NodeId(u32::MAX);
	let mut keys = vec![top_level];
	if let Some(export) = &interface.export {
		keys.push(export.decl);
	}
	keys.iter().any(|key| {
		interface
			.async_summaries
			.get(key)
			.is_some_and(|summary| summary.starts || summary.suspends)
	})
}
