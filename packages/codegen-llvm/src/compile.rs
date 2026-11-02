use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;
use std::sync::Arc;
use std::time::Instant;

use fa_analyzer::Severity;
use fa_analyzer::ids::FileId;
use fa_analyzer::session::Session;
use inkwell::context::Context;
use inkwell::passes::PassBuilderOptions;
use inkwell::targets::FileType;
use inkwell::values::BasicValue;

pub use crate::artifact::{Artifact, Timings};
use crate::error::Error;
use crate::link;
use crate::lower::Lowering;
use crate::target::{Options, Target};

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
	if session
		.diagnostics(file)
		.iter()
		.any(|diagnostic| diagnostic.severity == Severity::Error)
	{
		return Err(Error::Diagnostics(session.render_diagnostics(file, path)));
	}
	let mut timings = Timings::default();
	let machine = target.machine(options)?;
	let context = Context::create();
	let usize = context.ptr_sized_int_type(&machine.get_target_data(), None);

	let start = Instant::now();
	let module = context.create_module("program");
	module.set_triple(&target.triple());
	module.set_data_layout(&machine.get_target_data().get_data_layout());
	Lowering::new(session, file, path, &context, &module, target, usize)
		.with_overflow_checks(options.overflow_checks)
		.lower()?;
	timings.build = start.elapsed();

	let start = Instant::now();
	module.verify()?;
	timings.verify = start.elapsed();

	let start = Instant::now();
	module.run_passes(options.pipeline(), &machine, PassBuilderOptions::create())?;
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
					location: path.to_string(),
				});
			}
		}
	}

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
	machine.write_to_file(&module, FileType::Object, &object)?;
	timings.emit = start.elapsed();

	let start = Instant::now();
	let linked = link::link(target, options, &object, &output);
	let _ = fs::remove_file(&object);
	linked?;
	timings.link = start.elapsed();

	let loader = if target == Target::Wasm {
		Some(link::write_loader(&output)?)
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
		timings,
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
	let text = fs::read_to_string(path)?;
	let mut session = Session::new();
	let file = session.set_source("main", Arc::from(text.as_str()));
	session.analyze_file(file);
	compile(
		&session,
		file,
		&path.display().to_string(),
		target,
		options,
		output,
	)
}

/// Runs a compiled program with arguments and returns its output: directly
/// on native, with Node (or Bun, or `$FA_JS_RUNTIME`) and the loader on wasm.
pub fn run(artifact: &Artifact, arguments: &[&str]) -> Result<Output, Error> {
	link::run(artifact, arguments)
}
