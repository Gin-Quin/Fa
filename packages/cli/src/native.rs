//! The LLVM backend, outside the `fa` binary (design/decisions.md, "CLI
//! (lead, after the tooling review)"): `fa` doesn't link `libLLVM`, which
//! takes ~30 ms to load, so that `fa check`, `fa format`, `fa version` and
//! the language server start without it. The `native`, `native-freestanding`
//! and `wasm` targets of `fa build` and `fa run` are compiled by the helper
//! binary `fa-native` (`src/native_main.rs`): the same command line, run by
//! [`main_with_backend`] with the LLVM backend registered. `fa` looks for
//! `fa-native` next to its own executable, then on the `PATH`.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use fa_analyzer::Session;
use fa_analyzer::ids::FileId;
use fa_analyzer::modules::Program;

/// What the backend compiles: one file, or a program of several files.
pub enum Input<'a> {
	/// A standalone file using no other file, and its path as shown.
	File { file: FileId, shown: &'a str },
	/// A program of several files (one LLVM module).
	Program(&'a Program),
	/// The tests of a program (`fa test --target=native`): an executable
	/// running the test whose index is its first argument.
	Tests(&'a Program),
}

/// The LLVM backend, registered by `fa-native`.
pub trait Backend: Sync {
	/// Compiles and links an analyzed program into `output` (`main`, or
	/// `main.wasm` and its loader `main.mjs`); on failure, the message.
	fn compile(
		&self,
		session: &Session,
		input: Input<'_>,
		target: fa_codegen_llvm::Target,
		options: &fa_codegen_llvm::Options,
		output: &Path,
		libraries: &fa_codegen_llvm::Libraries,
	) -> Result<fa_codegen_llvm::Artifact, String>;

	/// Analyzes a package on the pipeline and compiles each unit into an
	/// object of its own (design/parallel-pipeline.md §3.3), on the thread
	/// that analyzed it, then links the program
	/// (`fa_codegen_llvm::compile_package`).
	#[allow(clippy::too_many_arguments)]
	fn compile_package(
		&self,
		root: &mut Session,
		run: &fa_analyzer::pipeline::Options,
		target: fa_codegen_llvm::Target,
		options: &fa_codegen_llvm::Options,
		split: &fa_codegen_llvm::SplitOptions,
		objects: &Path,
		output: &Path,
		libraries: &fa_codegen_llvm::Libraries,
	) -> (
		fa_analyzer::pipeline::Report,
		fa_codegen_llvm::PackageOutcome,
	);
}

static BACKEND: OnceLock<&'static dyn Backend> = OnceLock::new();

/// The backend of this process: `Some` in `fa-native` only.
pub(crate) fn backend() -> Option<&'static dyn Backend> {
	BACKEND.get().copied()
}

/// Runs a command line like [`crate::main`], the LLVM backend registered:
/// the `main` of `fa-native`.
pub fn main_with_backend(arguments: Vec<OsString>, backend: &'static dyn Backend) -> i32 {
	let _ = BACKEND.set(backend);
	crate::main(arguments)
}

/// The name of the helper binary.
const HELPER: &str = if cfg!(windows) {
	"fa-native.exe"
} else {
	"fa-native"
};

/// The `fa-native` helper: next to the running `fa`, else on the `PATH`.
fn helper() -> Option<PathBuf> {
	let beside = std::env::current_exe()
		.ok()
		.and_then(|executable| Some(executable.parent()?.join(HELPER)))
		.filter(|candidate| candidate.is_file());
	beside.or_else(|| {
		let path = std::env::var_os("PATH")?;
		std::env::split_paths(&path)
			.map(|directory| directory.join(HELPER))
			.find(|candidate| candidate.is_file())
	})
}

/// Runs a command line on `fa-native` (its output and its exit code are
/// the command's); `1` when it can't be found or started.
pub(crate) fn delegate(command: &str, arguments: &[OsString], target: &str) -> i32 {
	let Some(helper) = helper() else {
		eprintln!(
			"error[CLI009]: the `{target}` target needs `fa-native`, the LLVM backend of `fa`, and it is neither next to `fa` nor on the PATH: build it with `cargo build -p fa` (LLVM_SYS_181_PREFIX set), or install it next to `fa`"
		);
		return 1;
	};
	let mut all = vec![OsString::from(command)];
	if command == "build" && crate::timings::enabled() {
		all.push(OsString::from("--timings"));
	}
	all.extend(arguments.iter().cloned());
	let _ = std::io::Write::flush(&mut std::io::stdout());
	match Command::new(&helper).args(&all).status() {
		Ok(status) => crate::run::exit_code(status),
		Err(error) => {
			eprintln!("fa: can't start `{}`: {error}", helper.display());
			1
		}
	}
}
