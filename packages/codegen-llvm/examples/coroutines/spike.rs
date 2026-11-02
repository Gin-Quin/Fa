//! The coroutine spike of `design/async-spike.md`: LLVM 18 switched-resume
//! coroutines built by hand with the IR builder (inkwell, plus `llvm-sys` for
//! the intrinsics that take a `token`, which inkwell cannot represent),
//! lowered by the `coro-*` passes of the new pass manager, and compared with
//! hand-made state machines.
//!
//! The module holds, for each benchmark `bench_<name>(n) -> i64`:
//!
//! - `gen_opaque`: a generator yielding `1..=n` (`gen_new`, not inlined),
//!   drained through `llvm.coro.resume` / `done` / `promise` by another
//!   function (`gen_drain`, not inlined): `n` switches on a heap frame.
//! - `gen_inline`: the same generator, created and drained in one function,
//!   so that `coro-elide` can put its frame on the stack.
//! - `sm_direct`, `sm_indirect`, `sm_inline`: the generator as a state
//!   machine (a `switch` on a state field of an explicit frame), called
//!   directly, through a function pointer stored in its frame (as a task
//!   scheduler would), or inlined.
//! - `task_yield`: a task awaiting `n` child tasks, each of which yields once
//!   to the run queue of `driver.c` before returning; the child wakes its
//!   awaiter by pushing it on the queue.
//! - `task_sync`: the same with children that return without suspending,
//!   awaited right after their creation (can `coro-elide` remove their frame?).
//! - `task_spawn`: a task creating `n` yielding children, then awaiting them
//!   all (the "spawn 100k tasks and await them" of `benchmarks/async/`).
//!
//! The frames are allocated by `fa_alloc` and freed by `fa_free` (a size-class
//! allocator in `driver.c`), as the Fa runtime would.

use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::Command;

use inkwell::attributes::{Attribute, AttributeLoc};
use inkwell::basic_block::BasicBlock;
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::intrinsics::Intrinsic;
use inkwell::module::{Linkage, Module};
use inkwell::passes::PassBuilderOptions;
use inkwell::targets::{
	CodeModel, FileType, InitializationConfig, RelocMode, Target, TargetMachine, TargetTriple,
};
use inkwell::types::{BasicMetadataTypeEnum, BasicType, BasicTypeEnum, StructType};
use inkwell::values::{AsValueRef, BasicValue, FunctionValue, IntValue, PointerValue};
use inkwell::{AddressSpace, IntPredicate, OptimizationLevel};
use llvm_sys::core::{
	LLVMBuildCall2, LLVMConstNull, LLVMGlobalGetValueType, LLVMTokenTypeInContext,
};
use llvm_sys::prelude::LLVMValueRef;

pub type Result<T> = std::result::Result<T, Box<dyn Error>>;

/// The benchmarks, in the order `driver.c` and `bench.mjs` run them.
pub const BENCHMARKS: [&str; 8] = [
	"gen_opaque",
	"gen_inline",
	"sm_direct",
	"sm_indirect",
	"sm_inline",
	"task_yield",
	"task_sync",
	"task_spawn",
];

/// The result every benchmark must return for `n`.
pub fn expected(name: &str, n: i64) -> i64 {
	if name.starts_with("task") {
		n * (n - 1)
	} else {
		n * (n + 1) / 2
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpikeTarget {
	Native,
	Wasm,
}

impl SpikeTarget {
	pub fn name(self) -> &'static str {
		match self {
			SpikeTarget::Native => "native",
			SpikeTarget::Wasm => "wasm32",
		}
	}
}

/// A linked spike program, with what was measured while building it.
pub struct Built {
	pub target: SpikeTarget,
	pub executable: PathBuf,
	/// The coroutine frames left after the passes (`<function>.Frame`) and their sizes.
	pub frame_sizes: Vec<(String, u64)>,
	/// The size of the code of the functions of the object, in bytes.
	pub code_sizes: Vec<(String, u64)>,
	/// The calls to `llvm.coro.*` intrinsics left after the passes (must be 0).
	pub coro_calls_left: usize,
	/// The functions created by `coro-split` (`.resume`, `.destroy`, `.cleanup`).
	pub split_functions: Vec<String>,
}

/// One benchmark run.
#[derive(Debug)]
pub struct Measure {
	pub name: String,
	pub n: i64,
	pub result: i64,
	pub allocations: u64,
	pub nanoseconds: f64,
}

fn llvm_bin(tool: &str) -> PathBuf {
	Path::new(env!("FA_LLVM_PREFIX")).join("bin").join(tool)
}

fn machine(target: SpikeTarget) -> Result<TargetMachine> {
	static INITIALIZE: std::sync::Once = std::sync::Once::new();
	INITIALIZE.call_once(|| {
		let config = InitializationConfig::default();
		#[cfg(target_arch = "x86_64")]
		Target::initialize_x86(&config);
		#[cfg(target_arch = "aarch64")]
		Target::initialize_aarch64(&config);
		Target::initialize_webassembly(&config);
	});
	let (triple, cpu, features, reloc) = match target {
		SpikeTarget::Native => (
			TargetMachine::get_default_triple(),
			if cfg!(target_arch = "x86_64") {
				"x86-64"
			} else {
				"generic"
			},
			"",
			RelocMode::PIC,
		),
		SpikeTarget::Wasm => (
			TargetTriple::create("wasm32-unknown-unknown"),
			"generic",
			"+bulk-memory,+mutable-globals,+sign-ext,+nontrapping-fptoint",
			RelocMode::Static,
		),
	};
	Target::from_triple(&triple)?
		.create_target_machine(
			&triple,
			cpu,
			features,
			OptimizationLevel::Aggressive,
			reloc,
			CodeModel::Default,
		)
		.ok_or_else(|| format!("no target machine for {triple}").into())
}

/// Builds the module for `target`, optimizes it with `pipeline` (e.g.
/// `default<O3>`), and links it with `driver.c` into `directory`: an
/// executable on native, `spike.wasm` on wasm (run by `bench.mjs`).
pub fn build(target: SpikeTarget, directory: &Path, pipeline: &str) -> Result<Built> {
	std::fs::create_dir_all(directory)?;
	let machine = machine(target)?;
	let context = Context::create();
	let module = context.create_module("coroutines");
	module.set_triple(&machine.get_triple());
	// The frame layout of `coro-split` follows the data layout: set it first.
	let data = machine.get_target_data();
	module.set_data_layout(&data.get_data_layout());
	Spike::new(&context, &module).build_all()?;
	module.verify().map_err(|error| error.to_string())?;
	// Without the `coro-*` passes, code generation aborts the process
	// (`LLVM ERROR`): keep the input to try `llc` on it.
	std::fs::write(
		directory.join(format!("spike.{}.input.ll", target.name())),
		module.print_to_string().to_string(),
	)?;
	module.run_passes(pipeline, &machine, PassBuilderOptions::create())?;
	module.verify().map_err(|error| error.to_string())?;

	let text = module.print_to_string().to_string();
	std::fs::write(directory.join(format!("spike.{}.ll", target.name())), &text)?;
	let coro_calls_left = text
		.lines()
		.filter(|line| line.contains("call") && line.contains("@llvm.coro."))
		.count();
	let split_functions = module
		.get_functions()
		.map(|function| function.get_name().to_string_lossy().into_owned())
		.filter(|name| {
			[".resume", ".destroy", ".cleanup"]
				.iter()
				.any(|suffix| name.ends_with(suffix))
		})
		.collect();
	let mut frame_sizes = Vec::new();
	for name in [
		"gen_new",
		"gen_new_inline",
		"leaf_yield",
		"leaf_sync",
		"parent_yield",
		"parent_sync",
		"parent_spawn",
	] {
		if let Some(frame) = module.get_struct_type(&format!("{name}.Frame")) {
			frame_sizes.push((name.to_string(), data.get_abi_size(&frame)));
		}
	}

	let object = directory.join(format!("spike.{}.o", target.name()));
	machine.write_to_file(&module, FileType::Object, &object)?;
	let code_sizes = code_sizes(&object, target)?;
	let driver = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/coroutines/driver.c");
	let executable = match target {
		SpikeTarget::Native => {
			let driver_object = directory.join("driver.native.o");
			let compiler = std::env::var_os("CC").unwrap_or_else(|| "cc".into());
			checked(
				Command::new(&compiler)
					.args(["-O2", "-c"])
					.arg(&driver)
					.arg("-o")
					.arg(&driver_object),
			)?;
			let executable = directory.join("spike");
			checked(
				Command::new(&compiler)
					.arg(&driver_object)
					.arg(&object)
					.arg("-o")
					.arg(&executable),
			)?;
			executable
		}
		SpikeTarget::Wasm => {
			let driver_object = directory.join("driver.wasm32.o");
			checked(
				Command::new(llvm_bin("clang"))
					.args(["--target=wasm32-unknown-unknown", "-O2", "-nostdlib", "-c"])
					.arg(&driver)
					.arg("-o")
					.arg(&driver_object),
			)?;
			let executable = directory.join("spike.wasm");
			checked(
				Command::new(llvm_bin("wasm-ld"))
					.args(["--no-entry", "--export-all"])
					.arg(&driver_object)
					.arg(&object)
					.arg("-o")
					.arg(&executable),
			)?;
			executable
		}
	};
	Ok(Built {
		target,
		executable,
		frame_sizes,
		code_sizes,
		coro_calls_left,
		split_functions,
	})
}

/// Whether the wasm side can be built and run: clang of the LLVM install (for
/// `driver.c`) and node.
pub fn wasm_available() -> bool {
	llvm_bin("clang").is_file()
		&& Command::new("node")
			.arg("--version")
			.output()
			.is_ok_and(|output| output.status.success())
}

/// Runs the benchmarks (or the one named `only`) with `n`.
pub fn run(built: &Built, n: i64, only: Option<&str>) -> Result<Vec<Measure>> {
	let mut command = match built.target {
		SpikeTarget::Native => Command::new(&built.executable),
		SpikeTarget::Wasm => {
			let mut command = Command::new("node");
			command
				.arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/coroutines/bench.mjs"))
				.arg(&built.executable);
			command
		}
	};
	command.arg(n.to_string());
	if let Some(only) = only {
		command.arg(only);
	}
	let output = checked(&mut command)?;
	let mut measures = Vec::new();
	for line in String::from_utf8(output.stdout)?.lines() {
		let fields: Vec<&str> = line.split_whitespace().collect();
		if fields.len() != 6 {
			return Err(format!("unexpected line `{line}`").into());
		}
		measures.push(Measure {
			name: fields[0].to_string(),
			n: fields[1].parse()?,
			result: fields[2].parse()?,
			allocations: fields[3].parse()?,
			nanoseconds: fields[5].parse()?,
		});
	}
	Ok(measures)
}

/// The size of each function of an object: `llvm-nm --print-size` on ELF;
/// on wasm, where symbols have no size, the distance to the next function of
/// the code section (`llvm-objdump -t` and `-h`).
fn code_sizes(object: &Path, target: SpikeTarget) -> Result<Vec<(String, u64)>> {
	let mut sizes = Vec::new();
	if target == SpikeTarget::Wasm {
		let headers = checked(Command::new(llvm_bin("llvm-objdump")).arg("-h").arg(object))?;
		let code_size = String::from_utf8(headers.stdout)?
			.lines()
			.find_map(|line| {
				let fields: Vec<&str> = line.split_whitespace().collect();
				(fields.get(1) == Some(&"CODE")).then(|| u64::from_str_radix(fields[2], 16))
			})
			.ok_or("no code section")??;
		let symbols = checked(Command::new(llvm_bin("llvm-objdump")).arg("-t").arg(object))?;
		let mut starts = Vec::new();
		let symbols = String::from_utf8(symbols.stdout)?;
		for line in symbols.lines() {
			let fields: Vec<&str> = line.split_whitespace().collect();
			if fields.len() >= 4 && fields[fields.len() - 2] == "CODE" {
				starts.push((
					u64::from_str_radix(fields[0], 16)?,
					fields[fields.len() - 1],
				));
			}
		}
		starts.sort();
		for (index, (start, name)) in starts.iter().enumerate() {
			let end = starts.get(index + 1).map_or(code_size, |next| next.0);
			sizes.push((name.to_string(), end - start));
		}
	} else {
		let output = checked(
			Command::new(llvm_bin("llvm-nm"))
				.args(["--print-size", "--defined-only"])
				.arg(object),
		)?;
		for line in String::from_utf8(output.stdout)?.lines() {
			let fields: Vec<&str> = line.split_whitespace().collect();
			if let [_, size, kind, name] = fields[..]
				&& matches!(kind, "T" | "t")
			{
				sizes.push((name.to_string(), u64::from_str_radix(size, 16)?));
			}
		}
	}
	sizes.sort();
	Ok(sizes)
}

fn checked(command: &mut Command) -> Result<std::process::Output> {
	let output = command.output()?;
	if !output.status.success() {
		return Err(format!(
			"{command:?} failed: {}{}",
			String::from_utf8_lossy(&output.stdout),
			String::from_utf8_lossy(&output.stderr)
		)
		.into());
	}
	Ok(output)
}

/// A coroutine being built: what its suspension points need.
struct Coroutine<'ctx> {
	function: FunctionValue<'ctx>,
	handle: PointerValue<'ctx>,
	/// The promise (an `alloca` given to `llvm.coro.id`): what the coroutine
	/// shares with the outside, at a fixed offset of its frame.
	promise: PointerValue<'ctx>,
	/// Frees the frame (`llvm.coro.free`), then goes to `end`.
	cleanup: BasicBlock<'ctx>,
	/// `llvm.coro.end`, then returns the handle: where the ramp function and
	/// every suspension return to the caller.
	end: BasicBlock<'ctx>,
}

struct Spike<'a, 'ctx> {
	context: &'ctx Context,
	module: &'a Module<'ctx>,
	builder: Builder<'ctx>,
	/// `{ ptr waiter, i64 result }`: the promise of a task.
	task: StructType<'ctx>,
	/// `{ ptr next, i32 state, i64 n, i64 i, i64 value }`: a state machine.
	machine: StructType<'ctx>,
}

impl<'a, 'ctx> Spike<'a, 'ctx> {
	fn new(context: &'ctx Context, module: &'a Module<'ctx>) -> Self {
		let ptr = context.ptr_type(AddressSpace::default());
		let i64 = context.i64_type();
		Spike {
			context,
			module,
			builder: context.create_builder(),
			task: context.struct_type(&[ptr.into(), i64.into()], false),
			machine: context.struct_type(
				&[
					ptr.into(),
					context.i32_type().into(),
					i64.into(),
					i64.into(),
					i64.into(),
				],
				false,
			),
		}
	}

	fn build_all(&self) -> Result<()> {
		let i64 = self.context.i64_type();
		let ptr = self.ptr();
		let void = self.context.void_type();
		self.module
			.add_function("fa_alloc", ptr.fn_type(&[i64.into()], false), None);
		self.module
			.add_function("fa_free", void.fn_type(&[ptr.into()], false), None);
		self.module
			.add_function("fa_queue_push", void.fn_type(&[ptr.into()], false), None);
		self.module
			.add_function("fa_queue_pop", ptr.fn_type(&[], false), None);

		// generators
		let gen_new = self.build_generator("gen_new", true)?;
		let gen_new_inline = self.build_generator("gen_new_inline", false)?;
		let gen_drain = self.function("gen_drain", i64.into(), ptr.into(), true);
		let handle = gen_drain.get_nth_param(0).unwrap().into_pointer_value();
		let sum = self.emit_drain(gen_drain, handle)?;
		self.builder.build_return(Some(&sum))?;

		let bench = self.bench("gen_opaque");
		let n = bench.get_nth_param(0).unwrap();
		let handle = self.call(gen_new, &[n.as_value_ref()]);
		let sum = self.call(gen_drain, &[handle]);
		self.builder
			.build_return(Some(&unsafe { IntValue::new(sum) }))?;

		let bench = self.bench("gen_inline");
		let n = bench.get_nth_param(0).unwrap();
		let handle = self.call(gen_new_inline, &[n.as_value_ref()]);
		let sum = self.emit_drain(bench, unsafe { PointerValue::new(handle) })?;
		self.builder.build_return(Some(&sum))?;

		// state machines
		let sm_next = self.build_state_machine_next("sm_next", true)?;
		let sm_next_inline = self.build_state_machine_next("sm_next_inline", false)?;
		let sm_new = self.function("sm_new", ptr.into(), i64.into(), true);
		let n = sm_new.get_nth_param(0).unwrap().into_int_value();
		let size = self.machine.size_of().unwrap();
		let frame = self.call(
			self.module.get_function("fa_alloc").unwrap(),
			&[size.as_value_ref()],
		);
		let frame = unsafe { PointerValue::new(frame) };
		self.init_state_machine(frame, n, sm_next)?;
		self.builder.build_return(Some(&frame))?;
		for (name, indirect) in [("direct", false), ("indirect", true)] {
			let drain = self.function(&format!("sm_drain_{name}"), i64.into(), ptr.into(), true);
			let frame = drain.get_nth_param(0).unwrap().into_pointer_value();
			let sum = self.emit_state_machine_drain(drain, frame, sm_next, indirect)?;
			self.call(
				self.module.get_function("fa_free").unwrap(),
				&[frame.as_value_ref()],
			);
			self.builder.build_return(Some(&sum))?;

			let bench = self.bench(&format!("sm_{name}"));
			let n = bench.get_nth_param(0).unwrap();
			let frame = self.call(sm_new, &[n.as_value_ref()]);
			let sum = self.call(drain, &[frame]);
			self.builder
				.build_return(Some(&unsafe { IntValue::new(sum) }))?;
		}
		let bench = self.bench("sm_inline");
		let n = bench.get_nth_param(0).unwrap().into_int_value();
		let frame = self.builder.build_alloca(self.machine, "frame")?;
		self.init_state_machine(frame, n, sm_next_inline)?;
		let sum = self.emit_state_machine_drain(bench, frame, sm_next_inline, false)?;
		self.builder.build_return(Some(&sum))?;

		// tasks
		let leaf_yield = self.build_leaf("leaf_yield", true)?;
		let leaf_sync = self.build_leaf("leaf_sync", false)?;
		for (name, leaf, spawn_all) in [
			("yield", leaf_yield, false),
			("sync", leaf_sync, false),
			("spawn", leaf_yield, true),
		] {
			let parent = self.build_parent(&format!("parent_{name}"), leaf, spawn_all)?;
			let bench = self.bench(&format!("task_{name}"));
			let n = bench.get_nth_param(0).unwrap();
			let handle = self.call(parent, &[n.as_value_ref()]);
			let handle = unsafe { PointerValue::new(handle) };
			self.emit_run_queue(bench)?;
			let promise = self.promise_of(handle);
			let result = self
				.builder
				.build_struct_gep(self.task, promise, 1, "result")?;
			let result = self.builder.build_load(i64, result, "result")?;
			self.intrinsic("llvm.coro.destroy", &[], &[handle.as_value_ref()]);
			self.builder.build_return(Some(&result))?;
		}
		Ok(())
	}

	fn ptr(&self) -> inkwell::types::PointerType<'ctx> {
		self.context.ptr_type(AddressSpace::default())
	}

	/// Adds an internal function `return_type name(param)` and positions the
	/// builder in its entry block.
	fn function(
		&self,
		name: &str,
		return_type: BasicTypeEnum<'ctx>,
		param: BasicMetadataTypeEnum<'ctx>,
		noinline: bool,
	) -> FunctionValue<'ctx> {
		let function = self.module.add_function(
			name,
			return_type.fn_type(&[param], false),
			Some(Linkage::Internal),
		);
		if noinline {
			self.add_attribute(function, "noinline");
		}
		let entry = self.context.append_basic_block(function, "entry");
		self.builder.position_at_end(entry);
		function
	}

	/// Adds the exported `i64 bench_<name>(i64 n)`.
	fn bench(&self, name: &str) -> FunctionValue<'ctx> {
		let i64 = self.context.i64_type();
		let function = self.module.add_function(
			&format!("bench_{name}"),
			i64.fn_type(&[i64.into()], false),
			None,
		);
		let entry = self.context.append_basic_block(function, "entry");
		self.builder.position_at_end(entry);
		function
	}

	fn add_attribute(&self, function: FunctionValue<'ctx>, name: &str) {
		let kind = Attribute::get_named_enum_kind_id(name);
		assert_ne!(kind, 0, "unknown attribute {name}");
		function.add_attribute(
			AttributeLoc::Function,
			self.context.create_enum_attribute(kind, 0),
		);
	}

	/// Calls `function` with raw values (some intrinsics take a `token`, which
	/// inkwell cannot represent).
	fn call(&self, function: FunctionValue<'ctx>, arguments: &[LLVMValueRef]) -> LLVMValueRef {
		let mut arguments = arguments.to_vec();
		unsafe {
			LLVMBuildCall2(
				self.builder.as_mut_ptr(),
				LLVMGlobalGetValueType(function.as_value_ref()),
				function.as_value_ref(),
				arguments.as_mut_ptr(),
				arguments.len() as u32,
				c"".as_ptr(),
			)
		}
	}

	fn intrinsic(
		&self,
		name: &str,
		overloads: &[BasicTypeEnum<'ctx>],
		arguments: &[LLVMValueRef],
	) -> LLVMValueRef {
		let declaration = Intrinsic::find(name)
			.unwrap_or_else(|| panic!("no intrinsic {name}"))
			.get_declaration(self.module, overloads)
			.unwrap_or_else(|| panic!("no declaration of {name}"));
		self.call(declaration, arguments)
	}

	fn token_none(&self) -> LLVMValueRef {
		unsafe { LLVMConstNull(LLVMTokenTypeInContext(self.context.raw())) }
	}

	fn i1(&self, value: bool) -> LLVMValueRef {
		self.context
			.bool_type()
			.const_int(value as u64, false)
			.as_value_ref()
	}

	/// `llvm.coro.promise(handle, 8, false)`: the promise in a frame.
	fn promise_of(&self, handle: PointerValue<'ctx>) -> PointerValue<'ctx> {
		let alignment = self.context.i32_type().const_int(8, false);
		let promise = self.intrinsic(
			"llvm.coro.promise",
			&[],
			&[
				handle.as_value_ref(),
				alignment.as_value_ref(),
				self.i1(false),
			],
		);
		unsafe { PointerValue::new(promise) }
	}

	/// Starts a coroutine `ptr name(i64)` whose promise has type `promise`:
	/// the frame allocation (`llvm.coro.alloc`, which `coro-elide` turns to
	/// false when the frame goes on the caller's stack), `llvm.coro.begin`,
	/// and the cleanup and end blocks. The builder ends in the body.
	fn begin_coroutine(
		&self,
		name: &str,
		promise: BasicTypeEnum<'ctx>,
		noinline: bool,
	) -> Result<Coroutine<'ctx>> {
		let i64 = self.context.i64_type();
		let ptr = self.ptr();
		let function = self.function(name, ptr.into(), i64.into(), noinline);
		self.add_attribute(function, "presplitcoroutine");
		let entry = function.get_first_basic_block().unwrap();
		let allocate = self.context.append_basic_block(function, "allocate");
		let begin = self.context.append_basic_block(function, "begin");
		let cleanup = self.context.append_basic_block(function, "cleanup");
		let free = self.context.append_basic_block(function, "free");
		let end = self.context.append_basic_block(function, "end");

		let promise = self.builder.build_alloca(promise, "promise")?;
		promise
			.as_instruction_value()
			.unwrap()
			.set_alignment(8)
			.map_err(|error| error.to_string())?;
		let alignment = self.context.i32_type().const_int(8, false);
		let null = ptr.const_null();
		let id = self.intrinsic(
			"llvm.coro.id",
			&[],
			&[
				alignment.as_value_ref(),
				promise.as_value_ref(),
				null.as_value_ref(),
				null.as_value_ref(),
			],
		);
		let needed = self.intrinsic("llvm.coro.alloc", &[], &[id]);
		self.builder
			.build_conditional_branch(unsafe { IntValue::new(needed) }, allocate, begin)?;

		self.builder.position_at_end(allocate);
		let size = self.intrinsic("llvm.coro.size", &[i64.into()], &[]);
		let memory = self.call(self.module.get_function("fa_alloc").unwrap(), &[size]);
		let memory = unsafe { PointerValue::new(memory) };
		self.builder.build_unconditional_branch(begin)?;

		self.builder.position_at_end(begin);
		let phi = self.builder.build_phi(ptr, "memory")?;
		phi.add_incoming(&[(&null, entry), (&memory, allocate)]);
		let handle = self.intrinsic(
			"llvm.coro.begin",
			&[],
			&[id, phi.as_basic_value().as_value_ref()],
		);
		let handle = unsafe { PointerValue::new(handle) };

		self.builder.position_at_end(cleanup);
		let frame = self.intrinsic("llvm.coro.free", &[], &[id, handle.as_value_ref()]);
		let frame = unsafe { PointerValue::new(frame) };
		let allocated = self.builder.build_is_not_null(frame, "allocated")?;
		self.builder
			.build_conditional_branch(allocated, free, end)?;
		self.builder.position_at_end(free);
		self.call(
			self.module.get_function("fa_free").unwrap(),
			&[frame.as_value_ref()],
		);
		self.builder.build_unconditional_branch(end)?;

		self.builder.position_at_end(end);
		self.intrinsic(
			"llvm.coro.end",
			&[],
			&[handle.as_value_ref(), self.i1(false), self.token_none()],
		);
		self.builder.build_return(Some(&handle))?;

		self.builder.position_at_end(begin);
		Ok(Coroutine {
			function,
			handle,
			promise,
			cleanup,
			end,
		})
	}

	/// `llvm.coro.suspend`: returns to the caller (or resumer) through `end`.
	/// A normal suspension positions the builder where the coroutine resumes;
	/// the final one leaves it nowhere (resuming it is undefined).
	fn suspend(&self, coroutine: &Coroutine<'ctx>, last: bool) -> Result<()> {
		let state = self.intrinsic(
			"llvm.coro.suspend",
			&[],
			&[self.token_none(), self.i1(last)],
		);
		let state = unsafe { IntValue::new(state) };
		let i8 = self.context.i8_type();
		let resume = self.context.append_basic_block(
			coroutine.function,
			if last { "unreachable" } else { "resume" },
		);
		self.builder.build_switch(
			state,
			coroutine.end,
			&[
				(i8.const_int(0, false), resume),
				(i8.const_int(1, false), coroutine.cleanup),
			],
		)?;
		self.builder.position_at_end(resume);
		if last {
			self.builder.build_unreachable()?;
		}
		Ok(())
	}

	/// `ptr name(i64 n)`: a generator storing `1..=n` in its promise (`i64`).
	fn build_generator(&self, name: &str, noinline: bool) -> Result<FunctionValue<'ctx>> {
		let i64 = self.context.i64_type();
		let coroutine = self.begin_coroutine(name, i64.into(), noinline)?;
		let function = coroutine.function;
		let n = function.get_nth_param(0).unwrap().into_int_value();
		let begin = self.builder.get_insert_block().unwrap();
		let head = self.context.append_basic_block(function, "loop");
		self.builder.build_unconditional_branch(head)?;
		self.builder.position_at_end(head);
		let index = self.builder.build_phi(i64, "index")?;
		let current = index.as_basic_value().into_int_value();
		self.builder.build_store(coroutine.promise, current)?;
		self.suspend(&coroutine, false)?;
		let next = self
			.builder
			.build_int_add(current, i64.const_int(1, false), "next")?;
		let finished = self
			.builder
			.build_int_compare(IntPredicate::UGT, next, n, "finished")?;
		let resumed = self.builder.get_insert_block().unwrap();
		let last = self.context.append_basic_block(function, "last");
		self.builder
			.build_conditional_branch(finished, last, head)?;
		index.add_incoming(&[(&i64.const_int(1, false), begin), (&next, resumed)]);
		self.builder.position_at_end(last);
		self.suspend(&coroutine, true)?;
		Ok(function)
	}

	/// Resumes the generator `handle` until it is done, summing what it
	/// yields, then destroys it; returns the sum.
	fn emit_drain(
		&self,
		function: FunctionValue<'ctx>,
		handle: PointerValue<'ctx>,
	) -> Result<IntValue<'ctx>> {
		let i64 = self.context.i64_type();
		let before = self.builder.get_insert_block().unwrap();
		let head = self.context.append_basic_block(function, "drain");
		let body = self.context.append_basic_block(function, "yielded");
		let exit = self.context.append_basic_block(function, "drained");
		self.builder.build_unconditional_branch(head)?;
		self.builder.position_at_end(head);
		let sum = self.builder.build_phi(i64, "sum")?;
		let done = self.intrinsic("llvm.coro.done", &[], &[handle.as_value_ref()]);
		self.builder
			.build_conditional_branch(unsafe { IntValue::new(done) }, exit, body)?;
		self.builder.position_at_end(body);
		let promise = self.promise_of(handle);
		let value = self
			.builder
			.build_load(i64, promise, "value")?
			.into_int_value();
		let total =
			self.builder
				.build_int_add(sum.as_basic_value().into_int_value(), value, "total")?;
		self.intrinsic("llvm.coro.resume", &[], &[handle.as_value_ref()]);
		self.builder.build_unconditional_branch(head)?;
		sum.add_incoming(&[(&i64.const_zero(), before), (&total, body)]);
		self.builder.position_at_end(exit);
		self.intrinsic("llvm.coro.destroy", &[], &[handle.as_value_ref()]);
		Ok(sum.as_basic_value().into_int_value())
	}

	/// Stores the initial state of a state machine in `frame`.
	fn init_state_machine(
		&self,
		frame: PointerValue<'ctx>,
		n: IntValue<'ctx>,
		next: FunctionValue<'ctx>,
	) -> Result<()> {
		let next_field = self
			.builder
			.build_struct_gep(self.machine, frame, 0, "next")?;
		self.builder
			.build_store(next_field, next.as_global_value().as_pointer_value())?;
		let state = self
			.builder
			.build_struct_gep(self.machine, frame, 1, "state")?;
		self.builder
			.build_store(state, self.context.i32_type().const_zero())?;
		let n_field = self.builder.build_struct_gep(self.machine, frame, 2, "n")?;
		self.builder.build_store(n_field, n)?;
		Ok(())
	}

	/// `i1 name(ptr frame)`: the generator as a state machine; stores the next
	/// value in the frame and returns true, or returns false when done.
	fn build_state_machine_next(&self, name: &str, noinline: bool) -> Result<FunctionValue<'ctx>> {
		let i64 = self.context.i64_type();
		let i32 = self.context.i32_type();
		let bool = self.context.bool_type();
		let function = self.module.add_function(
			name,
			bool.fn_type(&[self.ptr().into()], false),
			Some(Linkage::Internal),
		);
		if noinline {
			self.add_attribute(function, "noinline");
		}
		let block = |name| self.context.append_basic_block(function, name);
		let (entry, start, step, check, yielded, finish, done) = (
			block("entry"),
			block("start"),
			block("step"),
			block("check"),
			block("yielded"),
			block("finish"),
			block("done"),
		);
		let frame = function.get_nth_param(0).unwrap().into_pointer_value();
		self.builder.position_at_end(entry);
		let state_field = self
			.builder
			.build_struct_gep(self.machine, frame, 1, "state")?;
		let n_field = self.builder.build_struct_gep(self.machine, frame, 2, "n")?;
		let index_field = self
			.builder
			.build_struct_gep(self.machine, frame, 3, "index")?;
		let value_field = self
			.builder
			.build_struct_gep(self.machine, frame, 4, "value")?;
		let state = self
			.builder
			.build_load(i32, state_field, "state")?
			.into_int_value();
		self.builder.build_switch(
			state,
			done,
			&[
				(i32.const_int(0, false), start),
				(i32.const_int(1, false), step),
			],
		)?;
		self.builder.position_at_end(start);
		self.builder.build_unconditional_branch(check)?;
		self.builder.position_at_end(step);
		let index = self
			.builder
			.build_load(i64, index_field, "index")?
			.into_int_value();
		let next = self
			.builder
			.build_int_add(index, i64.const_int(1, false), "next")?;
		self.builder.build_unconditional_branch(check)?;
		self.builder.position_at_end(check);
		let current = self.builder.build_phi(i64, "current")?;
		current.add_incoming(&[(&i64.const_int(1, false), start), (&next, step)]);
		let current = current.as_basic_value().into_int_value();
		let n = self.builder.build_load(i64, n_field, "n")?.into_int_value();
		let finished = self
			.builder
			.build_int_compare(IntPredicate::UGT, current, n, "finished")?;
		self.builder
			.build_conditional_branch(finished, finish, yielded)?;
		self.builder.position_at_end(yielded);
		self.builder.build_store(index_field, current)?;
		self.builder.build_store(value_field, current)?;
		self.builder
			.build_store(state_field, i32.const_int(1, false))?;
		self.builder.build_return(Some(&bool.const_int(1, false)))?;
		self.builder.position_at_end(finish);
		self.builder
			.build_store(state_field, i32.const_int(2, false))?;
		self.builder.build_unconditional_branch(done)?;
		self.builder.position_at_end(done);
		self.builder.build_return(Some(&bool.const_zero()))?;
		Ok(function)
	}

	/// Calls `next` (directly, or through the pointer in the frame) until it
	/// returns false, summing the values; returns the sum.
	fn emit_state_machine_drain(
		&self,
		function: FunctionValue<'ctx>,
		frame: PointerValue<'ctx>,
		next: FunctionValue<'ctx>,
		indirect: bool,
	) -> Result<IntValue<'ctx>> {
		let i64 = self.context.i64_type();
		let before = self.builder.get_insert_block().unwrap();
		let head = self.context.append_basic_block(function, "drain");
		let body = self.context.append_basic_block(function, "yielded");
		let exit = self.context.append_basic_block(function, "drained");
		self.builder.build_unconditional_branch(head)?;
		self.builder.position_at_end(head);
		let sum = self.builder.build_phi(i64, "sum")?;
		let more = if indirect {
			let pointer = self
				.builder
				.build_load(self.ptr(), frame, "next")?
				.into_pointer_value();
			self.builder
				.build_indirect_call(next.get_type(), pointer, &[frame.into()], "more")?
		} else {
			self.builder.build_call(next, &[frame.into()], "more")?
		};
		let more = unsafe { IntValue::new(more.as_value_ref()) };
		self.builder.build_conditional_branch(more, body, exit)?;
		self.builder.position_at_end(body);
		let value_field = self
			.builder
			.build_struct_gep(self.machine, frame, 4, "value")?;
		let value = self
			.builder
			.build_load(i64, value_field, "value")?
			.into_int_value();
		let total =
			self.builder
				.build_int_add(sum.as_basic_value().into_int_value(), value, "total")?;
		self.builder.build_unconditional_branch(head)?;
		sum.add_incoming(&[(&i64.const_zero(), before), (&total, body)]);
		self.builder.position_at_end(exit);
		Ok(sum.as_basic_value().into_int_value())
	}

	/// At the end of a task: wakes its awaiter (the `waiter` of its promise,
	/// pushed on the run queue) and suspends for the last time.
	fn finish_task(&self, coroutine: &Coroutine<'ctx>) -> Result<()> {
		let waiter = self
			.builder
			.build_load(self.ptr(), coroutine.promise, "waiter")?
			.into_pointer_value();
		let waiting = self.builder.build_is_not_null(waiter, "waiting")?;
		let wake = self.context.append_basic_block(coroutine.function, "wake");
		let last = self.context.append_basic_block(coroutine.function, "last");
		self.builder.build_conditional_branch(waiting, wake, last)?;
		self.builder.position_at_end(wake);
		self.call(
			self.module.get_function("fa_queue_push").unwrap(),
			&[waiter.as_value_ref()],
		);
		self.builder.build_unconditional_branch(last)?;
		self.builder.position_at_end(last);
		self.suspend(coroutine, true)
	}

	/// `ptr name(i64 x)`: a task returning `2 * x`, after yielding once to
	/// the run queue when `yields`.
	fn build_leaf(&self, name: &str, yields: bool) -> Result<FunctionValue<'ctx>> {
		let i64 = self.context.i64_type();
		let coroutine = self.begin_coroutine(name, self.task.into(), false)?;
		let x = coroutine
			.function
			.get_nth_param(0)
			.unwrap()
			.into_int_value();
		self.builder
			.build_store(coroutine.promise, self.ptr().const_null())?;
		if yields {
			self.call(
				self.module.get_function("fa_queue_push").unwrap(),
				&[coroutine.handle.as_value_ref()],
			);
			self.suspend(&coroutine, false)?;
		}
		let result = self
			.builder
			.build_int_mul(x, i64.const_int(2, false), "result")?;
		let result_field =
			self.builder
				.build_struct_gep(self.task, coroutine.promise, 1, "result")?;
		self.builder.build_store(result_field, result)?;
		self.finish_task(&coroutine)?;
		Ok(coroutine.function)
	}

	/// `await child`: if the child is not done, registers the current task as
	/// its waiter and suspends until the child wakes it; then reads the
	/// child's result and destroys it.
	fn emit_await(
		&self,
		coroutine: &Coroutine<'ctx>,
		child: PointerValue<'ctx>,
	) -> Result<IntValue<'ctx>> {
		let wait = self.context.append_basic_block(coroutine.function, "wait");
		let after = self
			.context
			.append_basic_block(coroutine.function, "awaited");
		let finished = self.intrinsic("llvm.coro.done", &[], &[child.as_value_ref()]);
		self.builder
			.build_conditional_branch(unsafe { IntValue::new(finished) }, after, wait)?;
		self.builder.position_at_end(wait);
		let child_promise = self.promise_of(child);
		self.builder.build_store(child_promise, coroutine.handle)?;
		self.suspend(coroutine, false)?;
		self.builder.build_unconditional_branch(after)?;
		self.builder.position_at_end(after);
		let child_promise = self.promise_of(child);
		let result_field = self
			.builder
			.build_struct_gep(self.task, child_promise, 1, "result")?;
		let result = self
			.builder
			.build_load(self.context.i64_type(), result_field, "result")?
			.into_int_value();
		self.intrinsic("llvm.coro.destroy", &[], &[child.as_value_ref()]);
		Ok(result)
	}

	/// Emits `for index in 0..n { body }` in the current coroutine or
	/// function; `body` gets the index and the running sum and returns the
	/// new sum. Returns the final sum.
	fn emit_sum_loop(
		&self,
		function: FunctionValue<'ctx>,
		n: IntValue<'ctx>,
		body: impl FnOnce(IntValue<'ctx>, IntValue<'ctx>) -> Result<IntValue<'ctx>>,
	) -> Result<IntValue<'ctx>> {
		let i64 = self.context.i64_type();
		let before = self.builder.get_insert_block().unwrap();
		let head = self.context.append_basic_block(function, "loop");
		let start = self.context.append_basic_block(function, "body");
		let done = self.context.append_basic_block(function, "done");
		self.builder.build_unconditional_branch(head)?;
		self.builder.position_at_end(head);
		let index = self.builder.build_phi(i64, "index")?;
		let sum = self.builder.build_phi(i64, "sum")?;
		let index_value = index.as_basic_value().into_int_value();
		let sum_value = sum.as_basic_value().into_int_value();
		let more = self
			.builder
			.build_int_compare(IntPredicate::ULT, index_value, n, "more")?;
		self.builder.build_conditional_branch(more, start, done)?;
		self.builder.position_at_end(start);
		let total = body(index_value, sum_value)?;
		let next = self
			.builder
			.build_int_add(index_value, i64.const_int(1, false), "next")?;
		let end = self.builder.get_insert_block().unwrap();
		self.builder.build_unconditional_branch(head)?;
		index.add_incoming(&[(&i64.const_zero(), before), (&next, end)]);
		sum.add_incoming(&[(&i64.const_zero(), before), (&total, end)]);
		self.builder.position_at_end(done);
		Ok(sum_value)
	}

	/// `ptr name(i64 n)`: a task (not inlined, as a spawned task would be)
	/// returning the sum of the results of `leaf(i)` for `i` in `0..n`. It
	/// awaits each child right after creating it, or, when `spawn_all`,
	/// creates (spawns) all of them first, then awaits them in order.
	fn build_parent(
		&self,
		name: &str,
		leaf: FunctionValue<'ctx>,
		spawn_all: bool,
	) -> Result<FunctionValue<'ctx>> {
		let i64 = self.context.i64_type();
		let ptr = self.ptr();
		let coroutine = self.begin_coroutine(name, self.task.into(), true)?;
		let function = coroutine.function;
		let n = function.get_nth_param(0).unwrap().into_int_value();
		self.builder
			.build_store(coroutine.promise, ptr.const_null())?;
		let sum = if spawn_all {
			let bytes = self
				.builder
				.build_int_mul(n, i64.const_int(8, false), "bytes")?;
			let handles = self.call(
				self.module.get_function("fa_alloc").unwrap(),
				&[bytes.as_value_ref()],
			);
			let handles = unsafe { PointerValue::new(handles) };
			self.emit_sum_loop(function, n, |index, sum| {
				let child = self.call(leaf, &[index.as_value_ref()]);
				let slot = unsafe { self.builder.build_gep(ptr, handles, &[index], "slot")? };
				self.builder
					.build_store(slot, unsafe { PointerValue::new(child) })?;
				Ok(sum)
			})?;
			let sum = self.emit_sum_loop(function, n, |index, sum| {
				let slot = unsafe { self.builder.build_gep(ptr, handles, &[index], "slot")? };
				let child = self
					.builder
					.build_load(ptr, slot, "child")?
					.into_pointer_value();
				let result = self.emit_await(&coroutine, child)?;
				Ok(self.builder.build_int_add(sum, result, "total")?)
			})?;
			self.call(
				self.module.get_function("fa_free").unwrap(),
				&[handles.as_value_ref()],
			);
			sum
		} else {
			self.emit_sum_loop(function, n, |index, sum| {
				let child = self.call(leaf, &[index.as_value_ref()]);
				let result = self.emit_await(&coroutine, unsafe { PointerValue::new(child) })?;
				Ok(self.builder.build_int_add(sum, result, "total")?)
			})?
		};
		let result_field =
			self.builder
				.build_struct_gep(self.task, coroutine.promise, 1, "result")?;
		self.builder.build_store(result_field, sum)?;
		self.finish_task(&coroutine)?;
		Ok(function)
	}

	/// The event loop: resumes the handles of the run queue until it is empty.
	fn emit_run_queue(&self, function: FunctionValue<'ctx>) -> Result<()> {
		let head = self.context.append_basic_block(function, "run");
		let body = self.context.append_basic_block(function, "resume");
		let exit = self.context.append_basic_block(function, "idle");
		self.builder.build_unconditional_branch(head)?;
		self.builder.position_at_end(head);
		let handle = self.call(self.module.get_function("fa_queue_pop").unwrap(), &[]);
		let handle = unsafe { PointerValue::new(handle) };
		let empty = self.builder.build_is_null(handle, "empty")?;
		self.builder.build_conditional_branch(empty, exit, body)?;
		self.builder.position_at_end(body);
		self.intrinsic("llvm.coro.resume", &[], &[handle.as_value_ref()]);
		self.builder.build_unconditional_branch(head)?;
		self.builder.position_at_end(exit);
		Ok(())
	}
}
