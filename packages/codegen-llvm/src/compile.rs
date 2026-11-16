use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;
use std::sync::Arc;
use std::time::Instant;

use fa_analyzer::Severity;
use fa_analyzer::modules::Program;

use fa_analyzer::ids::FileId;
use fa_analyzer::session::Session;
use inkwell::attributes::{Attribute, AttributeLoc};
use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::passes::PassBuilderOptions;
use inkwell::targets::FileType;
use inkwell::values::BasicValue;

use crate::Source;
pub use crate::artifact::{Artifact, Timings};
use crate::error::Error;
use crate::link;
use crate::lower::Lowering;
use crate::options::Libraries;
use crate::target::{Options, Target};

/// The number of instructions above which a function is not optimized: the
/// passes of LLVM take a time and a memory that grow faster than the size of
/// a function (the module-level code of a 10 000-line file takes minutes and
/// gigabytes), and such a function is straight-line initialization code.
const MAX_OPTIMIZED_INSTRUCTIONS: usize = 20_000;

/// Marks the functions larger than [`MAX_OPTIMIZED_INSTRUCTIONS`] `optnone`.
pub(crate) fn keep_huge_functions_unoptimized(module: &Module) {
	let context = module.get_context();
	let kind = |name: &str| Attribute::get_named_enum_kind_id(name);
	for function in module.get_functions() {
		let mut count = 0;
		for block in function.get_basic_block_iter() {
			count += block.get_instructions().count();
			if count > MAX_OPTIMIZED_INSTRUCTIONS {
				break;
			}
		}
		if count <= MAX_OPTIMIZED_INSTRUCTIONS {
			continue;
		}
		// `optnone` needs `noinline`, and excludes these
		for name in ["alwaysinline", "inlinehint", "minsize", "optsize"] {
			function.remove_enum_attribute(AttributeLoc::Function, kind(name));
		}
		for name in ["optnone", "noinline"] {
			function.add_attribute(
				AttributeLoc::Function,
				context.create_enum_attribute(kind(name), 0),
			);
		}
	}
}

/// Compiles an analyzed file of a session into an executable (native) or a
/// `.wasm` module and its JavaScript loader (wasm), written at `output`
/// (`.wasm` is added on wasm when missing). `path` names the file in the
/// locations of stops. The file must have no error diagnostics.
pub fn compile(
	session: &Session,
	file: FileId,
	path: &str,
	target: Target,
	options: &Options,
	output: &Path,
) -> Result<Artifact, Error> {
	compile_linking(
		session,
		file,
		path,
		target,
		options,
		output,
		&Libraries::default(),
	)
}

/// [`compile`], linking C libraries (foreign code).
pub fn compile_linking(
	session: &Session,
	file: FileId,
	path: &str,
	target: Target,
	options: &Options,
	output: &Path,
	libraries: &Libraries,
) -> Result<Artifact, Error> {
	let source = Source::File { file, path };
	compile_source(session, &source, target, options, output, libraries)
}

/// Compiles an analyzed program of several files (advanced brief §15.7)
/// into one executable or `.wasm` module: every file of `program.files`, the
/// module-level values initialized in `program.init_order`, the top level
/// of the entry file, then its entry function. The stops name each file by
/// `Session::display_path`.
pub fn compile_program(
	session: &Session,
	program: &Program,
	target: Target,
	options: &Options,
	output: &Path,
) -> Result<Artifact, Error> {
	compile_program_linking(
		session,
		program,
		target,
		options,
		output,
		&Libraries::default(),
	)
}

/// [`compile_program`], linking C libraries (foreign code: `build.link`).
pub fn compile_program_linking(
	session: &Session,
	program: &Program,
	target: Target,
	options: &Options,
	output: &Path,
	libraries: &Libraries,
) -> Result<Artifact, Error> {
	let source = Source::Program(program);
	compile_source(session, &source, target, options, output, libraries)
}

/// The executable of the tests of a program (`fa test --target=native`):
/// every file of the program and of its tests; run with the index of a test
/// as its first argument, it runs that test (`lower/testing.rs`).
pub fn compile_tests_linking(
	session: &Session,
	program: &Program,
	target: Target,
	options: &Options,
	output: &Path,
	libraries: &Libraries,
) -> Result<Artifact, Error> {
	let source = Source::Tests(program);
	compile_source(session, &source, target, options, output, libraries)
}

/// What [`prepare`] reads from a [`Source`].
pub(crate) struct Prepared {
	pub files: Vec<(FileId, String)>,
	pub entry: FileId,
	pub init_order: Vec<(FileId, fa_analyzer::ids::NodeId)>,
	pub tests: Vec<(FileId, fa_analyzer::ids::NodeId)>,
	/// The options, with the asynchronous mode of a program.
	pub options: Options,
}

/// The files of a source, its entry, its initialization order and its tests.
pub(crate) fn prepare(
	session: &Session,
	source: &Source<'_>,
	options: &Options,
) -> Result<Prepared, Error> {
	let missing = |what: &str| Error::Unsupported {
		message: what.to_string(),
		location: String::new(),
	};
	let of_program = |program: &Program| Options {
		asynchronous: options.asynchronous.or(Some(program.asynchronous)),
		..*options
	};
	let shown = |&file: &FileId| (file, session.display_path(file));
	match *source {
		Source::File { file, path } => Ok(Prepared {
			files: vec![(file, path.to_string())],
			entry: file,
			init_order: Vec::new(),
			tests: Vec::new(),
			options: *options,
		}),
		Source::Program(program) => Ok(Prepared {
			files: program.files.iter().map(shown).collect(),
			entry: program
				.entry
				.ok_or_else(|| missing("a program without an entry"))?,
			init_order: program.init_order.clone(),
			tests: Vec::new(),
			options: of_program(program),
		}),
		Source::Tests(program) => {
			let tests = crate::program_tests(session, program);
			let &(first, _) = tests
				.first()
				.ok_or_else(|| missing("a program without tests"))?;
			Ok(Prepared {
				files: program
					.files
					.iter()
					.chain(&program.tests)
					.map(shown)
					.collect(),
				entry: program.entry.unwrap_or(first),
				init_order: program.init_order.clone(),
				tests,
				options: of_program(program),
			})
		}
	}
}

fn compile_source(
	session: &Session,
	source: &Source<'_>,
	target: Target,
	options: &Options,
	output: &Path,
	libraries: &Libraries,
) -> Result<Artifact, Error> {
	let prepared = prepare(session, source, options)?;
	let entry = prepared.entry;
	let options = &prepared.options;
	let inputs = Inputs {
		session,
		files: &prepared.files,
		entry,
		init_order: prepared.init_order,
		tests: prepared.tests,
		target,
		options,
		libraries,
		emulated_tls: false,
	};
	with_module(inputs, |built| {
		emit_and_link(session, entry, target, options, output, libraries, built)
	})
}

/// The files of a program and how they are compiled into one module.
pub(crate) struct Inputs<'a> {
	pub session: &'a Session,
	pub files: &'a [(FileId, String)],
	pub entry: FileId,
	pub init_order: Vec<(FileId, fa_analyzer::ids::NodeId)>,
	pub tests: Vec<(FileId, fa_analyzer::ids::NodeId)>,
	pub target: Target,
	pub options: &'a Options,
	pub libraries: &'a Libraries,
	/// Thread-local variables as emulated ones (`jit.rs`).
	pub emulated_tls: bool,
}

/// A lowered, verified and optimized module, ready for code generation.
pub(crate) struct Built<'a, 'ctx> {
	pub module: &'a Module<'ctx>,
	pub machine: &'a inkwell::targets::TargetMachine,
	pub timings: Timings,
	/// The functions a C library exports.
	pub c_exports: Vec<String>,
}

/// Lowers the files of a program into one module, verifies and optimizes
/// it, and gives it to `then` (the object and the link, or the JIT).
pub(crate) fn with_module<R>(
	inputs: Inputs<'_>,
	then: impl FnOnce(Built<'_, '_>) -> Result<R, Error>,
) -> Result<R, Error> {
	let Inputs {
		session,
		files,
		entry,
		init_order,
		tests,
		target,
		options,
		libraries,
		emulated_tls,
	} = inputs;
	let mut rendered = String::new();
	for (file, path) in files {
		if session
			.diagnostics(*file)
			.iter()
			.any(|diagnostic| diagnostic.severity == Severity::Error)
		{
			rendered.push_str(&session.render_diagnostics(*file, path));
		}
	}
	if !rendered.is_empty() {
		return Err(Error::Diagnostics(rendered));
	}
	let mut timings = Timings::default();
	let machine = target.machine(options)?;
	if emulated_tls {
		crate::target::use_emulated_tls(&machine);
	}
	let context = Context::create();
	let usize = context.ptr_sized_int_type(&machine.get_target_data(), None);

	// a C library runs in synchronous mode: its `async` functions are
	// exported as their blocking versions
	let asynchronous = libraries.c_library.is_none()
		&& options
			.asynchronous
			.unwrap_or_else(|| detect_asynchronous(session, entry));
	let start = Instant::now();
	let module = context.create_module("program");
	module.set_triple(&target.triple());
	module.set_data_layout(&machine.get_target_data().get_data_layout());
	let c_exports = Lowering::new(
		session, files, entry, init_order, &context, &module, target, usize,
	)
	.with_whole_program()
	.with_overflow_checks(options.overflow_checks)
	.with_leak_check(options.leak_check)
	.with_tests(tests)
	.with_asynchronous(asynchronous, options.dev())
	.with_c_library(libraries.c_library.clone())
	.lower()?;
	timings.build = start.elapsed();
	if libraries.c_library.is_some() {
		crate::lower::c_library::use_dynamic_tls(&module);
	}

	let start = Instant::now();
	module.verify()?;
	timings.verify = start.elapsed();

	let start = Instant::now();
	if !options.size {
		// The checked-in runtime is produced with clang -Os to keep its IR
		// compact. Its size attributes must not override a program's speed
		// pipeline (inlining, loop vectorization and unrolling).
		for function in module.get_functions() {
			for name in ["optsize", "minsize"] {
				function.remove_enum_attribute(
					AttributeLoc::Function,
					Attribute::get_named_enum_kind_id(name),
				);
			}
		}
	}
	keep_huge_functions_unoptimized(&module);
	crate::optimize::specialize_ascii_methods(&module, &machine, options)?;
	let pipeline = if asynchronous {
		options.coroutine_pipeline()
	} else {
		options.pipeline()
	};
	module.run_passes(pipeline, &machine, PassBuilderOptions::create())?;
	crate::optimize::expose_tail_recursion(&module, &machine, options)?;
	timings.optimize = start.elapsed();

	if target == Target::NativeFreestanding {
		for symbol in ["snprintf", "strtod", "pow", "floor"] {
			if module.get_function(symbol).is_some_and(|function| {
				function
					.as_global_value()
					.as_pointer_value()
					.get_first_use()
					.is_some()
			}) {
				return Err(Error::Unsupported {
					message: format!(
						"floats (`{symbol}` of the C library) on the freestanding target"
					),
					location: files
						.iter()
						.find(|(file, _)| *file == entry)
						.map(|(_, path)| path.clone())
						.unwrap_or_default(),
				});
			}
		}
	}

	then(Built {
		module: &module,
		machine: &machine,
		timings,
		c_exports,
	})
}

/// The object of a module, and its link into an executable, a `.wasm`
/// module or a C library.
fn emit_and_link(
	session: &Session,
	entry: FileId,
	target: Target,
	options: &Options,
	output: &Path,
	libraries: &Libraries,
	built: Built<'_, '_>,
) -> Result<Artifact, Error> {
	let Built {
		module,
		machine,
		mut timings,
		c_exports,
	} = built;
	let mut output = output.to_path_buf();
	if target == Target::Wasm
		&& output
			.extension()
			.is_none_or(|extension| extension != "wasm")
	{
		output = with_suffix(&output, ".wasm");
	}
	if let Some(directory) = output.parent()
		&& !directory.as_os_str().is_empty()
	{
		fs::create_dir_all(directory)?;
	}
	let ir = if options.emit_ir {
		let ir = with_suffix(&output, ".ll");
		module.print_to_file(&ir)?;
		Some(ir)
	} else {
		None
	};

	let object = with_suffix(&output, ".o");
	let start = Instant::now();
	machine.write_to_file(module, FileType::Object, &object)?;
	timings.emit = start.elapsed();

	let start = Instant::now();
	if let Some(library) = &libraries.c_library {
		// a C library: its shared and static libraries, and its header
		let directory = output.parent().unwrap_or(Path::new(".")).to_path_buf();
		let shared = directory.join(library.output_name(target));
		let archive = directory.join(library.static_name());
		let linked = link::link_c_library(
			target,
			options,
			None,
			std::slice::from_ref(&object),
			&c_exports,
			&shared,
			&archive,
			libraries,
		);
		let _ = fs::remove_file(&object);
		linked?;
		timings.link = start.elapsed();
		let header = write_header(session, entry, library, &directory)?;
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
			ir,
			c_library: Some(crate::artifact::CLibraryArtifacts {
				archive: (target == Target::Native).then_some(archive),
				header,
			}),
			timings,
		});
	}
	let linked = link::link(target, options, &object, &output, libraries);
	let _ = fs::remove_file(&object);
	linked?;
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
		ir,
		c_library: None,
		timings,
	})
}

/// Writes the header of a C library (`header.rs`) into `directory`, from
/// the interface of its API file.
pub(crate) fn write_header(
	session: &Session,
	api: FileId,
	library: &crate::options::CLibrary,
	directory: &Path,
) -> Result<PathBuf, Error> {
	let interface = &session.source_file(api).interface;
	let text = crate::header::c_header(session.types(), interface, library).map_err(|message| {
		Error::Unsupported {
			message: format!("the header of the C library: {message}"),
			location: session.display_path(api),
		}
	})?;
	let path = directory.join(&library.header);
	fs::write(&path, text)?;
	Ok(path)
}

/// The asynchronous mode of a standalone file (A6): its top level or its
/// exported function starts a task, or awaits something that may suspend.
fn detect_asynchronous(session: &Session, entry: FileId) -> bool {
	let interface = &session.source_file(entry).interface;
	// the key of the top level's summary (`check/tasks.rs`)
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

/// `path` followed by `suffix` (`with_extension` would replace what follows
/// a dot of the name).
pub(crate) fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
	let mut text = path.as_os_str().to_os_string();
	text.push(suffix);
	PathBuf::from(text)
}

/// Reads, analyzes and compiles a standalone file (`fa build file.fa`).
pub fn compile_file(
	path: &Path,
	target: Target,
	options: &Options,
	output: &Path,
) -> Result<Artifact, Error> {
	compile_file_linking(path, target, options, output, &Libraries::default())
}

/// [`compile_file`], linking C libraries. A file named `*.foreign.fa` is a
/// foreign file: it may declare and call C functions.
pub fn compile_file_linking(
	path: &Path,
	target: Target,
	options: &Options,
	output: &Path,
	libraries: &Libraries,
) -> Result<Artifact, Error> {
	let text = fs::read_to_string(path)?;
	let mut session = Session::new();
	// the analysis is for this target (`target`, compile-time arithmetic, D22)
	session.set_target(target.analysis_target());
	let foreign = path
		.file_name()
		.is_some_and(|name| name.to_string_lossy().ends_with(".foreign.fa"));
	let module = if foreign { "main.foreign" } else { "main" };
	let file = session.set_source(module, Arc::from(text.as_str()));
	session.analyze_file(file);
	compile_linking(
		&session,
		file,
		&path.display().to_string(),
		target,
		options,
		output,
		libraries,
	)
}

/// The IR of a standalone file as the lowering builds it, before any pass
/// (the golden tests of `tests/async_ir.rs`).
pub fn lowered_ir(path: &Path, target: Target, options: &Options) -> Result<String, Error> {
	let text = fs::read_to_string(path)?;
	let mut session = Session::new();
	session.set_target(target.analysis_target());
	let file = session.set_source("main", Arc::from(text.as_str()));
	session.analyze_file(file);
	if session
		.diagnostics(file)
		.iter()
		.any(|diagnostic| diagnostic.severity == Severity::Error)
	{
		let shown = path.display().to_string();
		return Err(Error::Diagnostics(session.render_diagnostics(file, &shown)));
	}
	let machine = target.machine(options)?;
	let context = Context::create();
	let usize = context.ptr_sized_int_type(&machine.get_target_data(), None);
	let module = context.create_module("program");
	module.set_triple(&target.triple());
	module.set_data_layout(&machine.get_target_data().get_data_layout());
	let asynchronous = options
		.asynchronous
		.unwrap_or_else(|| detect_asynchronous(&session, file));
	let files = [(file, "main.fa".to_string())];
	Lowering::new(
		&session,
		&files,
		file,
		Vec::new(),
		&context,
		&module,
		target,
		usize,
	)
	.with_whole_program()
	.with_overflow_checks(options.overflow_checks)
	.with_leak_check(options.leak_check)
	.with_asynchronous(asynchronous, options.dev())
	.lower()?;
	module.verify()?;
	Ok(module.print_to_string().to_string())
}

/// Runs a compiled program with arguments and returns its output: directly
/// on native, with Node (or Bun, or `$FA_JS_RUNTIME`) and the loader on wasm.
pub fn run(artifact: &Artifact, arguments: &[&str]) -> Result<Output, Error> {
	link::run(artifact, arguments)
}
