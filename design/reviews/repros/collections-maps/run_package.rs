use std::path::{Path, PathBuf};

use fa_analyzer::modules::CodebaseResolver;
use fa_codegen_llvm::{Libraries, OptLevel, Options, PackageOutcome, SplitOptions, Target};

fn main() {
	let root = PathBuf::from(std::env::args().nth(1).expect("review fixture directory"));
	let output = PathBuf::from(std::env::args().nth(2).expect("scratch output directory"));
	let package = root.join(std::env::var("FA_REVIEW_PACKAGE").unwrap_or("per-unit".into()));
	let mut files = Vec::new();
	collect(&package, &package, &mut files);
	let mut failures = 0;
	for target in [Target::Native, Target::Wasm, Target::NativeFreestanding] {
		for opt in [OptLevel::O0, OptLevel::O3] {
			let label = format!("{}-{opt:?}", target.name());
			let out = output.join(&label);
			std::fs::create_dir_all(&out).unwrap();
			let options = Options {
				opt_level: opt,
				leak_check: true,
				emit_ir: true,
				asynchronous: Some(false),
				..Options::default()
			};
			let mut session = fa_analyzer::session::Session::new();
			session.set_target(target.analysis_target());
			session.set_resolver(Box::new(CodebaseResolver::from_texts(
				&files,
				Some("application/main.fa"),
			)));
			let run = fa_analyzer::pipeline::Options {
				entry: Some("application/main".into()),
				..fa_analyzer::pipeline::Options::default()
			};
			let split = SplitOptions {
				lto: options.optimized(),
				..SplitOptions::default()
			};
			let (report, outcome) = fa_codegen_llvm::compile_package(
				&mut session,
				&run,
				target,
				&options,
				&split,
				&out.join("objects"),
				&out.join("main"),
				&Libraries::default(),
			);
			let PackageOutcome::Built(artifact) = outcome else {
				let diagnostics: Vec<_> = report
					.modules
					.iter()
					.flat_map(|module| module.diagnostics.iter().map(|d| d.message.clone()))
					.collect();
				eprintln!("{label} FAIL: {outcome:?}; {diagnostics:?}");
				failures += 1;
				continue;
			};
			let result = fa_codegen_llvm::run(&artifact, &[]).unwrap();
			if !result.status.success()
				|| result.stdout != b"per-unit ok\n"
				|| !result.stderr.is_empty()
			{
				eprintln!("{label} FAIL: {result:?}");
				failures += 1;
				continue;
			}
			println!("{label} PASS");
		}
	}
	assert_eq!(failures, 0, "per-unit failures (see the review report)");
}

fn collect(root: &Path, directory: &Path, files: &mut Vec<(String, String)>) {
	for entry in std::fs::read_dir(directory).unwrap() {
		let path = entry.unwrap().path();
		if path.is_dir() {
			collect(root, &path, files);
		} else if path.extension().is_some_and(|extension| extension == "fa") {
			files.push((
				path.strip_prefix(root)
					.unwrap()
					.to_string_lossy()
					.into_owned(),
				std::fs::read_to_string(path).unwrap(),
			));
		}
	}
	files.sort_by(|a, b| a.0.cmp(&b.0));
}
