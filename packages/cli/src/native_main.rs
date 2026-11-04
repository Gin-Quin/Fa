//! The `fa-native` binary: the `fa` command with the LLVM backend
//! (`fa-codegen-llvm`) registered, run by `fa build` and `fa run` for the
//! `native`, `native-freestanding` and `wasm` targets (`src/native.rs`).
//! It is the only binary that links `libLLVM`.

use std::path::Path;

use fa::native::{Backend, Input};
use fa_analyzer::Session;
use fa_analyzer::ids::FileId;

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

	fn compile_unit(
		&self,
		session: &Session,
		files: &[FileId],
		entry: Option<FileId>,
		target: fa_codegen_llvm::Target,
		options: &fa_codegen_llvm::Options,
		split: &fa_codegen_llvm::SplitOptions,
		directory: &Path,
	) -> Result<fa_codegen_llvm::UnitOutput, fa_codegen_llvm::Error> {
		fa_codegen_llvm::compile_unit(session, files, entry, target, options, split, directory)
	}

	fn link_program(
		&self,
		target: fa_codegen_llvm::Target,
		options: &fa_codegen_llvm::Options,
		split: &fa_codegen_llvm::SplitOptions,
		inputs: &fa_codegen_llvm::EntryInputs,
		directory: &Path,
		output: &Path,
		libraries: &fa_codegen_llvm::Libraries,
	) -> Result<fa_codegen_llvm::Artifact, String> {
		fa_codegen_llvm::link_program(target, options, split, inputs, directory, output, libraries)
			.map_err(|error| format!("{error}\n"))
	}
}

fn main() {
	let arguments = std::env::args_os().skip(1).collect();
	let code = fa::native::main_with_backend(arguments, &Llvm);
	std::process::exit(code);
}
