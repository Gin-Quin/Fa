//! Fa's LLVM backend: native executables and WebAssembly.
//!
//! [`compile`] lowers an analyzed file (`fa_analyzer`) to an LLVM module
//! (`lower`), links the runtime into it (`runtime`: a platform layer built as
//! IR, and the tier 1 runtime compiled from C to IR), optimizes it with the
//! new pass manager, emits an object for the host or for
//! `wasm32-unknown-unknown`, and links it (`cc`, `ld.lld` or `wasm-ld`). A
//! wasm module gets a JavaScript loader next to it.
//!
//! The LLVM code only exists when `LLVM_SYS_181_PREFIX` was set at build time
//! (see `scripts/setup-llvm.sh` and `build.rs`): check [`llvm_available`].
//! See `design/llvm-backend-plan.md`.

/// Whether this build of the crate links LLVM (`LLVM_SYS_181_PREFIX` was set).
pub const fn llvm_available() -> bool {
	cfg!(fa_llvm)
}

/// The message printed by the tests and examples when LLVM is not available.
pub const LLVM_MISSING: &str = "skipped: fa-codegen-llvm was built without LLVM; run scripts/setup-llvm.sh, export LLVM_SYS_181_PREFIX and rebuild";

mod artifact;
#[cfg(fa_llvm)]
mod compile;
mod error;
pub mod facts;
mod header;
mod host;
#[cfg(all(fa_llvm, unix))]
mod jit;
#[cfg(fa_llvm)]
mod link;
#[cfg(fa_llvm)]
mod lower;
#[cfg(any(fa_llvm, test))]
mod map_facts;
#[cfg(fa_llvm)]
mod optimize;
mod options;
#[cfg(fa_llvm)]
pub mod runtime;
mod split;
#[cfg(fa_llvm)]
mod target;
#[cfg(fa_llvm)]
mod units;

pub use artifact::{Artifact, CLibraryArtifacts, Timings};
#[cfg(fa_llvm)]
pub use compile::{
	compile, compile_file, compile_file_linking, compile_linking, compile_program,
	compile_program_linking, compile_tests_linking, lowered_ir, run,
};
pub use error::Error;
pub use header::c_header;
pub use host::javascript_runtime;
#[cfg(all(fa_llvm, unix))]
pub use jit::load;
pub use options::{CLibrary, Libraries, OptLevel, Options, Target, jit_available};
pub use split::{
	EntryInputs, GenericEntryRequest, GenericInstance, GenericProducer, GenericTables,
	GenericValueNeeds, PackageOutcome, SplitOptions, UnitOutput, UnitSymbols,
};
#[cfg(fa_llvm)]
pub use units::{compile_package, compile_unit, link_program};

/// Without LLVM: always [`Error::Unavailable`].
#[cfg(not(fa_llvm))]
pub fn compile(
	_session: &fa_analyzer::Session,
	_file: fa_analyzer::ids::FileId,
	_path: &str,
	_target: Target,
	_options: &Options,
	_output: &std::path::Path,
) -> Result<Artifact, Error> {
	Err(Error::Unavailable)
}

/// Without LLVM: always [`Error::Unavailable`].
#[cfg(not(fa_llvm))]
pub fn compile_file(
	_path: &std::path::Path,
	_target: Target,
	_options: &Options,
	_output: &std::path::Path,
) -> Result<Artifact, Error> {
	Err(Error::Unavailable)
}

#[cfg(all(test, not(fa_llvm)))]
#[test]
fn compiling_a_file_without_llvm_reports_unavailable() {
	let path = std::path::Path::new("unused.fa");
	assert!(matches!(
		compile_file(path, Target::Native, &Options::default(), path),
		Err(Error::Unavailable)
	));
}

/// Without LLVM: always [`Error::Unavailable`].
#[cfg(not(fa_llvm))]
pub fn run(_artifact: &Artifact, _arguments: &[&str]) -> Result<std::process::Output, Error> {
	Err(Error::Unavailable)
}

#[cfg(all(test, not(fa_llvm)))]
#[test]
fn running_an_artifact_without_llvm_reports_unavailable() {
	let artifact = Artifact {
		target: Target::Native,
		options: Options::default(),
		output: "unused".into(),
		loader: None,
		ir: None,
		c_library: None,
		output_size: 0,
		timings: Timings::default(),
	};
	assert!(matches!(run(&artifact, &[]), Err(Error::Unavailable)));
}

/// What the backend compiles into one module.
#[derive(Clone, Copy)]
pub enum Source<'a> {
	/// A standalone file, and its path in the locations of stops.
	File {
		file: fa_analyzer::ids::FileId,
		path: &'a str,
	},
	/// A program of several files.
	Program(&'a fa_analyzer::modules::Program),
	/// The tests of a program: run with the index of a test as its first
	/// argument, it runs that test.
	Tests(&'a fa_analyzer::modules::Program),
}

/// How a program run in a child ended ([`JitProgram::run_in_child`]).
#[derive(Clone, Debug)]
pub struct ChildOutput {
	/// The exit code (128 + the signal when a signal ended it).
	pub code: i32,
	/// What it wrote on the standard error.
	pub stderr: Vec<u8>,
}

/// A program loaded in this process by the JIT (`jit.rs`): its `main`. A
/// trait object, so that the `fa` command, which only passes it on to
/// `fa-native`, references no code of LLVM and doesn't load `libLLVM`.
pub trait LoadedProgram: std::fmt::Debug {
	/// Runs `main` in this process, with the signals of a new program
	/// (`SIGPIPE` ends it, where the compiler ignores it). Returns its exit
	/// code, when `main` returns: a program that stops or exits ends the
	/// process itself.
	fn run(&self, name: &str, arguments: &[&std::ffi::OsStr]) -> i32;

	/// Runs `main` in a child of this process (a copy made by `fork`: the
	/// program is not compiled again), its standard input closed, and
	/// returns its exit code and what it wrote on the standard error.
	/// `None` when it didn't end before `timeout`, and was killed.
	fn run_in_child(
		&self,
		name: &str,
		arguments: &[&std::ffi::OsStr],
		timeout: std::time::Duration,
	) -> std::io::Result<Option<ChildOutput>>;
}

/// Without the JIT (no LLVM, or no `fork`): always [`Error::Unavailable`].
#[cfg(not(all(fa_llvm, unix)))]
pub fn load(
	_session: &fa_analyzer::Session,
	_source: Source<'_>,
	_options: &Options,
) -> Result<Box<dyn LoadedProgram>, Error> {
	Err(Error::Unavailable)
}

/// The tests of a program, in the order `fa test` numbers them (the index
/// the executable of [`compile_tests_linking`] takes): the test files of
/// `program.tests` in order, their `test` blocks (not `@test`) in order of
/// appearance.
pub fn program_tests(
	session: &fa_analyzer::Session,
	program: &fa_analyzer::modules::Program,
) -> Vec<(fa_analyzer::ids::FileId, fa_analyzer::ids::NodeId)> {
	use fa_analyzer::syntax::{self, View};
	let mut tests = Vec::new();
	for &file in &program.tests {
		let tree = session.tree(file);
		let root = fa_analyzer::ids::NodeId::new(tree.root);
		let View::Module { statements } = syntax::view(tree, root) else {
			continue;
		};
		for statement in statements.iter() {
			if let View::Test {
				compile_time: false,
				..
			} = syntax::view(tree, statement)
			{
				tests.push((file, statement));
			}
		}
	}
	tests
}

/// Without LLVM: always [`Error::Unavailable`].
#[cfg(not(fa_llvm))]
pub fn compile_tests_linking(
	_session: &fa_analyzer::Session,
	_program: &fa_analyzer::modules::Program,
	_target: Target,
	_options: &Options,
	_output: &std::path::Path,
	_libraries: &Libraries,
) -> Result<Artifact, Error> {
	Err(Error::Unavailable)
}

/// Without LLVM: always [`Error::Unavailable`].
#[cfg(not(fa_llvm))]
pub fn compile_program(
	_session: &fa_analyzer::Session,
	_program: &fa_analyzer::modules::Program,
	_target: Target,
	_options: &Options,
	_output: &std::path::Path,
) -> Result<Artifact, Error> {
	Err(Error::Unavailable)
}

/// Without LLVM: always [`Error::Unavailable`].
#[cfg(not(fa_llvm))]
pub fn compile_program_linking(
	_session: &fa_analyzer::Session,
	_program: &fa_analyzer::modules::Program,
	_target: Target,
	_options: &Options,
	_output: &std::path::Path,
	_libraries: &Libraries,
) -> Result<Artifact, Error> {
	Err(Error::Unavailable)
}

/// Without LLVM: always [`Error::Unavailable`].
#[cfg(not(fa_llvm))]
pub fn compile_linking(
	_session: &fa_analyzer::Session,
	_file: fa_analyzer::ids::FileId,
	_path: &str,
	_target: Target,
	_options: &Options,
	_output: &std::path::Path,
	_libraries: &Libraries,
) -> Result<Artifact, Error> {
	Err(Error::Unavailable)
}

/// Without LLVM: the analysis, and [`Error::Unavailable`].
#[cfg(not(fa_llvm))]
#[allow(clippy::too_many_arguments)]
pub fn compile_package(
	root: &mut fa_analyzer::Session,
	run: &fa_analyzer::pipeline::Options,
	_target: Target,
	_options: &Options,
	_split: &SplitOptions,
	_objects: &std::path::Path,
	_output: &std::path::Path,
	_libraries: &Libraries,
) -> (fa_analyzer::pipeline::Report, PackageOutcome) {
	let report =
		fa_analyzer::pipeline::analyze_package(root, run, None, &fa_analyzer::pipeline::NoSink);
	(report, PackageOutcome::Failed(Error::Unavailable))
}

/// Without LLVM: always [`Error::Unavailable`].
#[cfg(not(fa_llvm))]
pub fn compile_file(
	_path: &std::path::Path,
	_target: Target,
	_options: &Options,
	_output: &std::path::Path,
) -> Result<Artifact, Error> {
	Err(Error::Unavailable)
}

/// Without LLVM: always [`Error::Unavailable`].
#[cfg(not(fa_llvm))]
pub fn compile_unit(
	_session: &fa_analyzer::Session,
	_files: &[fa_analyzer::ids::FileId],
	_entry: Option<fa_analyzer::ids::FileId>,
	_target: Target,
	_options: &Options,
	_split: &SplitOptions,
	_directory: &std::path::Path,
) -> Result<UnitOutput, Error> {
	Err(Error::Unavailable)
}

/// Without LLVM: always [`Error::Unavailable`].
#[cfg(not(fa_llvm))]
pub fn link_program(
	_target: Target,
	_options: &Options,
	_split: &SplitOptions,
	_inputs: &EntryInputs,
	_directory: &std::path::Path,
	_output: &std::path::Path,
	_libraries: &Libraries,
) -> Result<Artifact, Error> {
	Err(Error::Unavailable)
}
