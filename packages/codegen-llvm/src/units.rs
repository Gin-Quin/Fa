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
use fa_analyzer::ids::FileId;
use fa_analyzer::session::Session;
use inkwell::attributes::{Attribute, AttributeLoc};
use inkwell::context::Context;
use inkwell::module::{Linkage, Module};
use inkwell::passes::PassBuilderOptions;
use inkwell::targets::{FileType, TargetMachine};
use inkwell::values::FunctionValue;
use inkwell::{AddressSpace, GlobalVisibility};

use crate::artifact::{Artifact, Timings};
use crate::error::Error;
use crate::link;
use crate::lower::Lowering;
use crate::options::Libraries;
use crate::runtime::Runtime;
use crate::split::{EntryInputs, SplitOptions, UnitOutput, UnitSymbols};
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
const VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), "-split-1");

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

/// The options that change an object.
fn options_key(target: Target, options: &Options, split: &SplitOptions, hasher: &mut StableHasher) {
	VERSION.hash(hasher);
	target.name().hash(hasher);
	format!("{:?}", options.opt_level).hash(hasher);
	options.size.hash(hasher);
	options.overflow_checks.hash(hasher);
	options.leak_check.hash(hasher);
	split.lto.hash(hasher);
}

/// The target machine and the pointer-sized integer of a module.
fn prepare_module(
	context: &Context,
	machine: &TargetMachine,
	module: &Module<'_>,
	target: Target,
) {
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

/// The extension of the objects of a build.
fn object_extension(split: &SplitOptions) -> &'static str {
	if split.lto { "bc" } else { "o" }
}

/// Compiles the files of one analyzed unit into an object in `directory`
/// (or takes it from the cache). `entry`: the program's entry file, when it
/// is one of `files`. The files must have no error diagnostics.
pub fn compile_unit(
	session: &Session,
	files: &[FileId],
	entry: Option<FileId>,
	target: Target,
	options: &Options,
	split: &SplitOptions,
	directory: &Path,
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
	// the cache of development builds
	let cached = split.cache.as_ref().filter(|_| !split.lto).map(|cache| {
		let mut hasher = StableHasher::default();
		options_key(target, options, split, &mut hasher);
		entry.map(|entry| session.module(entry)).hash(&mut hasher);
		let mut read = std::collections::BTreeSet::new();
		for &file in files {
			let module = session.module(file);
			module.hash(&mut hasher);
			session.text(file).hash(&mut hasher);
			if let Some(scan) = session.scan_of(module) {
				read.extend(scan.dependencies.iter().cloned());
			}
		}
		// the interfaces the unit reads
		for module in read {
			module.hash(&mut hasher);
			if let Some(id) = session.types().existing_module(&module)
				&& let Some(interface) = session.interface(id)
			{
				interface.hash(session.types()).hash(&mut hasher);
			}
		}
		cache.join(format!("{}-{:016x}", file_name(&name), hasher.finish()))
	});
	if let Some(key) = &cached {
		let object = key.with_extension(extension);
		if let Ok(text) = fs::read_to_string(key.with_extension("symbols"))
			&& let Some(symbols) = UnitSymbols::from_text(&text)
			&& object.is_file()
		{
			return Ok(UnitOutput {
				name,
				object,
				symbols,
				cached: true,
			});
		}
	}
	let machine = target.machine(options)?;
	let context = Context::create();
	let module = crate::runtime::declarations_module(&context, target, &name)?;
	prepare_module(&context, &machine, &module, target);
	let usize = context.ptr_sized_int_type(&machine.get_target_data(), None);
	let paths: Vec<(FileId, String)> = files
		.iter()
		.map(|&file| (file, session.display_path(file)))
		.collect();
	let mut timings = Timings::default();
	let start = Instant::now();
	let symbols = Lowering::new(session, &paths, first, Vec::new(), &context, &module, target, usize)
		.with_overflow_checks(options.overflow_checks)
		.with_leak_check(options.leak_check)
		.with_asynchronous(false, options.dev())
		.into_unit(name.clone(), files, entry)
		.lower_unit()?;
	timings.build = start.elapsed();
	module.verify()?;
	if target != Target::Wasm && !split.lto {
		function_sections(&module);
	}
	fs::create_dir_all(directory)?;
	let object = match &cached {
		Some(key) => {
			if let Some(parent) = key.parent() {
				fs::create_dir_all(parent)?;
			}
			key.with_extension(extension)
		}
		None => directory.join(format!("{}.{extension}", file_name(&name))),
	};
	emit(&module, &machine, options, split, &object, &mut timings)?;
	if let Some(key) = &cached {
		let partial = key.with_extension("symbols.partial");
		fs::write(&partial, symbols.to_text())?;
		fs::rename(&partial, key.with_extension("symbols"))?;
	}
	Ok(UnitOutput {
		name,
		object,
		symbols,
		cached: false,
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
	let machine = target.machine(options)?;
	let context = Context::create();
	let module = context.create_module("runtime");
	prepare_module(&context, &machine, &module, target);
	let usize = context.ptr_sized_int_type(&machine.get_target_data(), None);
	let runtime = Runtime::new(&context, &module, target, usize).with_leak_check(options.leak_check);
	runtime.link_tier1()?;
	// multi-file programs keep the eager representation of characters
	if let Some(flag) = module.get_global("fa_compact_characters") {
		flag.set_initializer(&context.i32_type().const_zero());
		flag.set_constant(true);
		flag.set_linkage(Linkage::Internal);
	}
	runtime.define_platform()?;
	// the platform layer, which the units call
	for name in [
		"fa_write",
		"fa_free",
		"fa_exit",
		"fa_alloc",
		"fa_pages",
		"fa_clock",
		"fa_nanosleep",
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
	// a process-unique name first: several builds may write the cache at once
	let unique = crate::compile::with_suffix(&path, &format!(".{}", std::process::id()));
	emit(&module, &machine, options, split, &unique, &mut Timings::default())?;
	fs::rename(&unique, &path)?;
	Ok(path)
}

/// The entry object: `main` (`_start` on wasm), `fa.drops_at_exit`, and the
/// switches over the ids of union members.
fn entry_object(
	target: Target,
	options: &Options,
	inputs: &EntryInputs,
	path: &Path,
) -> Result<(), Error> {
	let machine = target.machine(options)?;
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
	// `main(argc, argv)`, `_start()` on wasm
	let main = match target {
		Target::Wasm => module.add_function("_start", void.fn_type(&[], false), Some(Linkage::External)),
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
				message: format!("the initialization of a value of `{module_path}` (split build)"),
				location: String::new(),
			});
		}
		let init = declare(&format!("fa.init.{module_path}.{node}"), void.fn_type(&[], false));
		builder.build_call(init, &[], "")?;
		initialized.push((module_path.as_str(), *node));
	}
	let code = match units.iter().find_map(|unit| unit.symbols.entry.as_ref()) {
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
	// the module values, dropped in the reverse order when a drop does
	// more than free memory (or the leaks are counted)
	if effectful || options.leak_check {
		for (module_path, node) in initialized.iter().rev() {
			if defined_drops.contains(&(*module_path, *node)) {
				let drop = declare(&format!("fa.drop.{module_path}.{node}"), void.fn_type(&[], false));
				builder.build_call(drop, &[], "")?;
			}
		}
	}
	if options.leak_check {
		builder.build_call(runtime.function("fa_leak_check")?, &[], "")?;
	}
	builder.build_call(runtime.function("fa_flush")?, &[], "")?;
	let exit = runtime.exit()?;
	let exits = context.append_basic_block(main, "exit");
	let returns = context.append_basic_block(main, "return");
	let nonzero = builder.build_int_compare(inkwell::IntPredicate::NE, code, i32.const_zero(), "")?;
	builder.build_conditional_branch(nonzero, exits, returns)?;
	builder.position_at_end(exits);
	builder.build_call(exit, &[code.into()], "")?;
	builder.build_unreachable()?;
	builder.position_at_end(returns);
	match target {
		Target::Wasm => builder.build_return(None)?,
		_ => builder.build_return(Some(&i32.const_zero()))?,
	};
	// the switches over the ids of union members: each id to a unit that
	// registered it (the first, in the order of the units)
	let mut owners: std::collections::BTreeMap<u64, &str> = std::collections::BTreeMap::new();
	for unit in units {
		for &id in &unit.symbols.boxes {
			owners.entry(id).or_insert(unit.name.as_str());
		}
	}
	let switches: [(&str, inkwell::types::FunctionType<'_>); 3] = [
		("fa.drop_box", void.fn_type(&[pointer.into()], false)),
		("fa.copy_box", pointer.fn_type(&[pointer.into()], false)),
		("fa_format_union", void.fn_type(&[pointer.into(), i32.into()], false)),
	];
	for (name, ty) in switches {
		let function = match module.get_function(name) {
			Some(function) => function,
			None => module.add_function(name, ty, Some(Linkage::External)),
		};
		function.set_linkage(Linkage::External);
		if name != "fa_format_union" {
			function.as_global_value().set_visibility(GlobalVisibility::Hidden);
		}
		runtime.add_attributes(function, &["nounwind"]);
		let box_pointer = function.get_nth_param(0).expect("a box").into_pointer_value();
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
		builder.build_switch(id, end, &cases)?;
		builder.position_at_end(end);
		if name == "fa.copy_box" {
			builder.build_return(Some(&box_pointer))?;
		} else {
			builder.build_return(None)?;
		}
	}
	module.verify()?;
	let entry_options = SplitOptions::default();
	emit(&module, &machine, options, &entry_options, path, &mut Timings::default())
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
	if target == Target::Wasm && output.extension().is_none_or(|extension| extension != "wasm") {
		output = crate::compile::with_suffix(&output, ".wasm");
	}
	if let Some(parent) = output.parent()
		&& !parent.as_os_str().is_empty()
	{
		fs::create_dir_all(parent)?;
	}
	fs::create_dir_all(directory)?;
	let start = Instant::now();
	let entry = directory.join("$entry.o");
	entry_object(target, options, inputs, &entry)?;
	let runtime = runtime_object(target, options, split, directory)?;
	timings.emit = start.elapsed();
	let mut objects: Vec<PathBuf> = inputs.units.iter().map(|unit| unit.object.clone()).collect();
	objects.push(entry);
	objects.push(runtime);
	let start = Instant::now();
	link::link_objects(target, options, split, &objects, &output, libraries)?;
	timings.link = start.elapsed();
	let loader = if target == Target::Wasm {
		let imports = link::wasm_imports(&output)?;
		Some(link::write_loader(&output, &imports)?)
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
		timings,
	})
}
