//! Standalone review driver; uses the warm workspace rlibs (see README).
//! Requires actual per-unit output; fallback is a failure. Leak checks enabled.
use std::io::Write;
use std::path::{Path, PathBuf};

use fa_analyzer::modules::CodebaseResolver;
use fa_codegen_llvm::{Libraries, OptLevel, Options, PackageOutcome, SplitOptions, Target};

fn files(root: &Path, directory: &Path, output: &mut Vec<(String, String)>) {
	for entry in std::fs::read_dir(directory).unwrap() {
		let path = entry.unwrap().path();
		if path.is_dir() {
			files(root, &path, output);
		} else if path.extension().is_some_and(|extension| extension == "fa") {
			output.push((
				path.strip_prefix(root)
					.unwrap()
					.to_string_lossy()
					.to_string(),
				std::fs::read_to_string(path).unwrap(),
			));
		}
	}
}

fn main() {
	let args: Vec<_> = std::env::args().collect();
	let root = Path::new(&args[1]);
	let target = Target::from_name(&args[2]).unwrap();
	let output = PathBuf::from(&args[4]);
	let options = Options {
		opt_level: if args[3] == "0" {
			OptLevel::O0
		} else {
			OptLevel::O3
		},
		leak_check: true,
		..Options::default()
	};
	let mut sources = Vec::new();
	files(root, root, &mut sources);
	let mut session = fa_analyzer::Session::new();
	session.set_target(target.analysis_target());
	session.set_resolver(Box::new(CodebaseResolver::from_texts(
		&sources,
		Some("application/main.fa"),
	)));
	let run = fa_analyzer::pipeline::Options {
		entry: Some("application/main".into()),
		..Default::default()
	};
	let split = SplitOptions {
		lto: options.optimized(),
		..Default::default()
	};
	let (report, outcome) = fa_codegen_llvm::compile_package(
		&mut session,
		&run,
		target,
		&options,
		&split,
		&output.with_extension("objects"),
		&output,
		&Libraries::default(),
	);
	let artifact = match outcome {
		PackageOutcome::Built(artifact) => artifact,
		_ => {
			for module in report.modules {
				for diagnostic in module.diagnostics {
					eprintln!("{}", diagnostic.message);
				}
			}
			panic!("per-unit compilation failed: {outcome:?}");
		}
	};
	let result = fa_codegen_llvm::run(&artifact, &[]).unwrap();
	std::io::stdout().write_all(&result.stdout).unwrap();
	std::io::stderr().write_all(&result.stderr).unwrap();
	std::process::exit(result.status.code().unwrap_or(1));
}
