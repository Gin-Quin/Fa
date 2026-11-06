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
		session_for("native")
	}

	fn session_for(target: &str) -> (Session, CLibrary) {
		session_with_resources(target, target == "wasm")
	}

	fn session_with_resources(target: &str, resources: bool) -> (Session, CLibrary) {
		let mut files = package_files();
		if resources {
			for (path, source) in &mut files {
				if path == "library/greetings.fa" {
					*source = source.replace(
						"export namespace = {",
						"export namespace = {\n\tlet lifetime = lifecycle.Lifetime { label = \"library\" }",
					);
				} else if path == "library/capi.foreign.fa" {
					*source = source.replace("export namespace = {", "export namespace = {\n\tfunction lifetimeLength = (): Int64 => greetings.lifetime.label.length\n\tasync function asynchronousDouble = (value: Int32): Int32 => {\n\t\tawait Task.yield()\n\t\tvalue * 2\n\t}\n");
				}
			}
			files.push(("library/lifecycle.foreign.fa".to_string(), "export namespace = {\n\ttype Lifetime = {\n\t\tlabel: String\n\t\tfunction destroy = (self) => { console.log(\"disposed {self.label}\") }\n\t}\n}\n".to_string()));
		}
		let mut session = Session::new();
		session.set_target(target);
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
				let extra = artifact.c_library.expect("a C library");
				assert_eq!(extra.archive, Some(directory.join("libfarm.a")));
				assert_eq!(extra.header, directory.join("farm.h"));
			}
			other => panic!("{other:?}"),
		}
		check_programs(&directory);
	}

	fn check_wasm_library(directory: &Path) {
		let script = directory.join("check.mjs");
		fs::write(&script, r#"import open from './libfarm.mjs';
const first = await open();
const second = await open();
const api = first.exports;
if (first.memory === second.memory) throw Error('instances share memory');
if ('$fa.initialize' in api) throw Error('private exports leaked');
if (api.farm_asynchronous_double(9) !== 18) throw Error('blocking async export');
const text = new TextEncoder().encode('A farm\0');
const name = first.allocate(text.length, 1);
new Uint8Array(first.memory.buffer, name, text.length).set(text);
const farm = api.farm_create_farm(name);
first.free(name, text.length, 1);
const result = first.allocate(16, 8);
function message() {
  const pointer = api.farm_last_error_message();
  const bytes = new Uint8Array(first.memory.buffer);
  let end = pointer;
  while (bytes[end] !== 0) end++;
  return new TextDecoder().decode(bytes.subarray(pointer, end));
}
console.log(`ready ${api.farm_sum_of_squares()} ${api.farm_lifetime_length()}`);
console.log(`empty ${api.farm_animal_count(farm, result)} ${message()}`);
console.log(`added ${api.farm_add_animals(farm, 7n)}`);
console.log(`animals ${api.farm_animal_count(farm, result)} ${new DataView(first.memory.buffer).getBigInt64(result, true)}`);
if (api.farm_divide(8, 0, result) !== 1 || message() !== 'MathError.DivisionByZero') throw Error('typed error');
if (api.farm_divide(9, 3, result) !== 0 || new DataView(first.memory.buffer).getInt32(result, true) !== 3) throw Error('out result');
if (second.exports.farm_last_error_message() === 0) throw Error('empty message pointer');
api.farm_destroy_farm(farm);
first.free(result, 16, 8);
first.dispose();
first.dispose();
second.dispose();
"#).unwrap();
		let output = Command::new(fa_codegen_llvm::javascript_runtime().expect("Node or Bun"))
			.arg(script)
			.output()
			.expect("the library's host");
		assert!(
			output.status.success(),
			"{}\n{}",
			String::from_utf8_lossy(&output.stdout),
			String::from_utf8_lossy(&output.stderr)
		);
		assert_eq!(
			String::from_utf8_lossy(&output.stdout),
			"ready 30 7\nempty 1 Error.Empty\nadded 0\nanimals 0 7\ndisposed library\ndisposed library\n"
		);
		assert!(directory.join("farm.h").is_file());
		assert!(!directory.join("libfarm.a").exists());
	}

	#[test]
	fn a_wasm_c_library_exports_its_api_and_disposes_each_instance() {
		let (mut session, library) = session_for("wasm");
		let mut program = session.analyze_package(None);
		assert!(!program.has_errors(&session), "the package has no errors");
		program.entry = session.loaded_module(&library.api_module());
		let directory = scratch("c-library-wasm-module");
		let artifact = fa_codegen_llvm::compile_program_linking(
			&session,
			&program,
			Target::Wasm,
			&options(),
			&directory.join("main"),
			&Libraries {
				c_library: Some(library),
				..Libraries::default()
			},
		)
		.unwrap_or_else(|error| panic!("{error}"));
		assert_eq!(artifact.output, directory.join("libfarm.wasm"));
		assert!(artifact.c_library.unwrap().archive.is_none());
		check_wasm_library(&directory);
	}

	#[test]
	fn a_wasm_c_library_compiled_per_unit_initializes_and_disposes() {
		let (mut session, library) = session_for("wasm");
		let directory = scratch("c-library-wasm-units");
		let run = fa_analyzer::pipeline::Options {
			entry: Some(library.api_module()),
			..fa_analyzer::pipeline::Options::default()
		};
		let split = fa_codegen_llvm::SplitOptions {
			lto: true,
			runtime_cache: Some(directory.join("runtime")),
			..fa_codegen_llvm::SplitOptions::default()
		};
		let (report, outcome) = fa_codegen_llvm::compile_package(
			&mut session,
			&run,
			Target::Wasm,
			&options(),
			&split,
			&directory.join("objects"),
			&directory.join("main"),
			&Libraries {
				c_library: Some(library),
				..Libraries::default()
			},
		);
		assert!(
			matches!(outcome, fa_codegen_llvm::PackageOutcome::Built(_)),
			"{outcome:?}; {:?}",
			report
				.modules
				.iter()
				.flat_map(|module| &module.diagnostics)
				.collect::<Vec<_>>()
		);
		check_wasm_library(&directory);
	}

	#[test]
	fn a_native_library_keeps_global_resource_ownership_until_unload() {
		let (mut session, library) = session_with_resources("native", true);
		let mut program = session.analyze_package(None);
		assert!(!program.has_errors(&session));
		program.entry = session.loaded_module(&library.api_module());
		let directory = scratch("c-library-resource-module");
		fa_codegen_llvm::compile_program_linking(
			&session,
			&program,
			Target::Native,
			&options(),
			&directory.join("main"),
			&Libraries {
				c_library: Some(library),
				..Libraries::default()
			},
		)
		.unwrap_or_else(|error| panic!("{error}"));
		let source = directory.join("resource.c");
		fs::write(&source, "#include <stdio.h>\n#include \"farm.h\"\nint main(void) { printf(\"length: %lld\\n\", (long long)farm_lifetime_length()); fflush(stdout); return 0; }\n").unwrap();
		for executable in link_c(&directory, &source) {
			let output = Command::new(executable).output().unwrap();
			assert!(
				output.status.success(),
				"{}",
				String::from_utf8_lossy(&output.stderr)
			);
			assert_eq!(
				String::from_utf8_lossy(&output.stdout),
				"length: 7\ndisposed library\n"
			);
		}
	}
}

#[cfg(not(fa_llvm))]
#[test]
fn c_library_tests_are_skipped_without_llvm() {
	eprintln!("{}", fa_codegen_llvm::LLVM_MISSING);
}
