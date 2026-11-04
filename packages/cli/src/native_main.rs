//! The `fa-native` binary: the `fa` command with the LLVM backend
//! (`fa-codegen-llvm`) registered, run by `fa build` and `fa run` for the
//! `native`, `native-freestanding` and `wasm` targets (`src/native.rs`).
//! It is the only binary that links `libLLVM`.

use std::path::Path;

use fa::native::{Backend, Input};
use fa_analyzer::Session;

/// Counts the heap in use, like `fa` (`fa_interpreter::memory`).
#[global_allocator]
static ALLOCATOR: fa_interpreter::memory::CountingAllocator =
	fa_interpreter::memory::CountingAllocator;

/// The LLVM backend.
struct Llvm;

impl Backend for Llvm {
	fn compile(
		&self,
		session: &Session,
		input: Input<'_>,
		target: fa_codegen_llvm::Target,
		options: &fa_codegen_llvm::Options,
		output: &Path,
		libraries: &fa_codegen_llvm::Libraries,
	) -> Result<fa_codegen_llvm::Artifact, String> {
		let compiled = match input {
			Input::File { file, shown } => fa_codegen_llvm::compile_linking(
				session, file, shown, target, options, output, libraries,
			),
			Input::Program(program) => fa_codegen_llvm::compile_program_linking(
				session, program, target, options, output, libraries,
			),
			Input::Tests(program) => fa_codegen_llvm::compile_tests_linking(
				session, program, target, options, output, libraries,
			),
		};
		compiled.map_err(|error| format!("{error}\n"))
	}

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
	) -> (fa_analyzer::pipeline::Report, fa_codegen_llvm::PackageOutcome) {
		fa_codegen_llvm::compile_package(
			root, run, target, options, split, objects, output, libraries,
		)
	}
}

fn main() {
	let arguments = std::env::args_os().skip(1).collect();
	let code = fa::native::main_with_backend(arguments, &Llvm);
	std::process::exit(code);
}
