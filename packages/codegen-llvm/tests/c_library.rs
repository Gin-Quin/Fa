//! C libraries (`build.cLibrary`, `lower/c_library.rs`) built with the
//! leak check of the runtime (`Options::leak_check`): the fixture package
//! of the `fa` command (`packages/cli/tests/c-library/farm/`), compiled as
//! one module and per unit, used by its C program linked statically and
//! dynamically. The module values are dropped when the library is
//! unloaded, and a block left allocated is reported then: a C program
//! that doesn't destroy a farm exits with `leak: 2 blocks` (code 102).

#[cfg(fa_llvm)]
mod llvm {
	use std::fs;
	use std::path::{Path, PathBuf};
	use std::process::Command;

	use fa_analyzer::configuration::{self, ConfigurationKind};
	use fa_analyzer::modules::CodebaseResolver;
	use fa_analyzer::session::Session;
	use fa_codegen_llvm::{CLibrary, Libraries, Options, Target};

	/// The fixture of the `fa` command: the package and the C program.
	fn fixture() -> PathBuf {
		Path::new(env!("CARGO_MANIFEST_DIR")).join("../cli/tests/c-library")
	}

	/// The files of the package, by path.
	fn package_files() -> Vec<(String, String)> {
		let root = fixture().join("farm");
		let mut files = vec![(
			"package.fa".to_string(),
			fs::read_to_string(root.join("package.fa")).unwrap(),
		)];
		let mut names: Vec<String> = fs::read_dir(root.join("library"))
			.unwrap()
			.map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
			.collect();
		names.sort();
		for name in names {
			let text = fs::read_to_string(root.join("library").join(&name)).unwrap();
			files.push((format!("library/{name}"), text));
		}
		files
	}

	/// A session over the package, its configuration evaluated (the API
	/// file's checks read it), and the library it builds.
	fn session() -> (Session, CLibrary) {
		let files = package_files();
		let mut session = Session::new();
		session.set_target("native");
		let written = configuration::evaluate_source(&files[0].1, ConfigurationKind::Package)
			.0
			.expect("a configuration");
		let full = configuration::merge_values(&configuration::defaults(), &written);
		let options = configuration::PackageConfiguration::from_value(&full)
			.build
			.c_library
			.expect("`build.cLibrary`");
		session.set_configuration(configuration::to_value(&full));
		session.set_resolver(Box::new(CodebaseResolver::from_texts(&files, None)));
		let library = CLibrary {
			header: options.header,
			prefix: options.prefix,
			api: options.api,
		};
		(session, library)
	}

	fn scratch(name: &str) -> PathBuf {
		let directory = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
		let _ = fs::remove_dir_all(&directory);
		fs::create_dir_all(&directory).unwrap();
		directory
	}

	fn options() -> Options {
		Options {
			leak_check: true,
			opt_level: fa_codegen_llvm::OptLevel::O1,
			..Options::default()
		}
	}

	/// Compiles a C program against the library of `directory`, statically
	/// and dynamically: the two executables.
	fn link_c(directory: &Path, source: &Path) -> [PathBuf; 2] {
		// the clang of the LLVM install (`FA_LLVM_WITH_CLANG=1`), else `cc`
		let clang = Some(Path::new(env!("FA_LLVM_PREFIX")).join("bin/clang"))
			.filter(|clang| clang.is_file())
			.unwrap_or_else(|| PathBuf::from("cc"));
		let name = source.file_stem().unwrap().to_string_lossy().into_owned();
		let statically = directory.join(format!("{name}-static"));
		let dynamically = directory.join(format!("{name}-dynamic"));
		let status = Command::new(&clang)
			.args(["-std=c11", "-Wall", "-Werror", "-I"])
			.arg(directory)
			.arg(source)
			.arg(directory.join("libfarm.a"))
			.args(["-lm", "-pthread", "-o"])
			.arg(&statically)
			.status()
			.unwrap();
		assert!(status.success(), "{name} links with libfarm.a");
		let status = Command::new(&clang)
			.args(["-std=c11", "-Wall", "-Werror", "-I"])
			.arg(directory)
			.arg(source)
			.arg(format!("-L{}", directory.display()))
			.args(["-lfarm", "-pthread"])
			.arg(format!("-Wl,-rpath,{}", directory.display()))
			.arg("-o")
			.arg(&dynamically)
			.status()
			.unwrap();
		assert!(status.success(), "{name} links with libfarm.so");
		[statically, dynamically]
	}

	/// Runs the C programs of the fixture against the library built in
	/// `directory`: the complete one ends without leaks, the leaky one
	/// reports the farm it didn't destroy.
	fn check_programs(directory: &Path) {
		let expected = fs::read_to_string(fixture().join("expected.txt")).unwrap();
		for program in link_c(directory, &fixture().join("main.c")) {
			let output = Command::new(&program).output().unwrap();
			assert_eq!(
				(
					output.status.code(),
					String::from_utf8_lossy(&output.stdout).to_string()
				),
				(Some(0), expected.clone()),
				"{}: {}",
				program.display(),
				String::from_utf8_lossy(&output.stderr)
			);
		}
		let leaky = directory.join("leaky.c");
		fs::write(
			&leaky,
			"#include <stdio.h>\n#include \"farm.h\"\n\nint main(void) {\n\tfarm_Farm *farm = farm_create_farm(\"Forgotten\");\n\tprintf(\"%lld\\n\", (long long)farm_name_length(farm));\n\treturn 0;\n}\n",
		)
		.unwrap();
		for program in link_c(directory, &leaky) {
			let output = Command::new(&program).output().unwrap();
			assert_eq!(String::from_utf8_lossy(&output.stdout), "9\n");
			assert_eq!(output.status.code(), Some(102), "{}", program.display());
			assert!(
				String::from_utf8_lossy(&output.stderr).contains("leak: 2 blocks"),
				"{}",
				String::from_utf8_lossy(&output.stderr)
			);
		}
	}

	#[test]
	fn a_c_library_compiled_as_one_module_frees_its_values_at_unload() {
		let (mut session, library) = session();
		let mut program = session.analyze_package(None);
		assert!(!program.has_errors(&session), "the package has no errors");
		program.entry = session.loaded_module(&library.api_module());
		let directory = scratch("c-library-module");
		let libraries = Libraries {
			c_library: Some(library),
			..Libraries::default()
		};
		let artifact = fa_codegen_llvm::compile_program_linking(
			&session,
			&program,
			Target::Native,
			&options(),
			&directory.join("main"),
			&libraries,
		)
		.unwrap_or_else(|error| panic!("{error}"));
		assert_eq!(artifact.output, directory.join("libfarm.so"));
		check_programs(&directory);
	}

	#[test]
	fn a_c_library_compiled_per_unit_frees_its_values_at_unload() {
		let (mut session, library) = session();
		let directory = scratch("c-library-units");
		let run = fa_analyzer::pipeline::Options {
			entry: Some(library.api_module()),
			..fa_analyzer::pipeline::Options::default()
		};
		let split = fa_codegen_llvm::SplitOptions {
			lto: true,
			runtime_cache: Some(directory.join("runtime")),
			..fa_codegen_llvm::SplitOptions::default()
		};
		let libraries = Libraries {
			c_library: Some(library),
			..Libraries::default()
		};
		let (_, outcome) = fa_codegen_llvm::compile_package(
			&mut session,
			&run,
			Target::Native,
			&options(),
			&split,
			&directory.join("objects"),
			&directory.join("main"),
			&libraries,
		);
		match outcome {
			fa_codegen_llvm::PackageOutcome::Built(artifact) => {
				let (archive, header) = artifact.c_library.expect("a C library");
				assert_eq!(archive, directory.join("libfarm.a"));
				assert_eq!(header, directory.join("farm.h"));
			}
			other => panic!("{other:?}"),
		}
		check_programs(&directory);
	}
}

#[cfg(not(fa_llvm))]
#[test]
fn c_library_tests_are_skipped_without_llvm() {
	eprintln!("{}", fa_codegen_llvm::LLVM_MISSING);
}
