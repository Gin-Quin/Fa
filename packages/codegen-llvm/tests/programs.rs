//! End-to-end tests: Fa programs compiled natively and to wasm, run, and
//! compared with their annotations (advanced brief, "Test notation"):
//!
//! ```fa
//! console.log(x) -- prints: 3        one line of the standard output, in order
//! -- exit: 101                       the exit code (0 by default)
//! -- arguments: one two              the arguments given to the program (or `-- args:`)
//! -- stderr: text                     a text the standard error contains
//! -- overflow-checks: false           compiled without overflow checks
//! -- skip-target: native-freestanding  a target the program doesn't run on
//! -- asynchronous: false              built in synchronous mode (design/async.md A5)
//! -- opt-level: 0                     built at `-O0` (the dev profile)
//! ```
//!
//! `FA_PROGRAMS=filter` runs the programs whose path contains `filter`.
//!
//! The programs are `tests/programs/*.fa`, which must all compile, the
//! programs of the `fa` command (`packages/cli/tests/programs`), and the
//! cases of the analyzer (`packages/analyzer/tests/cases`) that print
//! something and expect no diagnostic: those that use a construct this
//! backend doesn't compile yet are counted as skipped. Without LLVM
//! (`LLVM_SYS_181_PREFIX` unset at build time), a single test prints why the
//! others were not built.

#[cfg(not(fa_llvm))]
#[test]
fn llvm_tests_are_skipped_without_llvm() {
	assert!(!fa_codegen_llvm::llvm_available());
	eprintln!("{}", fa_codegen_llvm::LLVM_MISSING);
}

#[cfg(fa_llvm)]
mod with_llvm {
	use std::path::{Path, PathBuf};
	use std::sync::{Condvar, Mutex};

	use fa_codegen_llvm::{Error, Options, Target, compile_file};

	/// A program and what it must do.
	struct Program {
		path: PathBuf,
		prints: Vec<String>,
		exit: i32,
		arguments: Vec<String>,
		/// From `tests/programs`: it must compile.
		required: bool,
		/// `-- skip-target: native-freestanding`
		skip_targets: Vec<String>,
		/// `-- stderr: text`: texts the standard error contains.
		stderr: Vec<String>,
		/// `-- overflow-checks: false`
		overflow_checks: bool,
		/// `-- asynchronous: true|false`
		asynchronous: Option<bool>,
		/// `-- opt-level: 0`
		dev: bool,
		/// The files of a multi-file program (`-- file: path` sections).
		files: Vec<(String, String)>,
	}

	/// Reads the annotations of a program; `None` for an analyzer case that
	/// isn't a runnable program (it expects diagnostics, or isn't standalone).
	fn read_program(path: &Path, required: bool) -> Option<Program> {
		let text = std::fs::read_to_string(path).expect("a readable program");
		let mut program = Program {
			path: path.to_path_buf(),
			prints: Vec::new(),
			exit: 0,
			arguments: Vec::new(),
			required,
			skip_targets: Vec::new(),
			stderr: Vec::new(),
			overflow_checks: true,
			asynchronous: None,
			dev: false,
			files: Vec::new(),
		};
		for line in text.lines() {
			if let Some(path) = line.trim_start().strip_prefix("-- file:") {
				program.files.push((path.trim().to_string(), String::new()));
				continue;
			}
			if let Some((_, file)) = program.files.last_mut() {
				file.push_str(line);
				file.push('\n');
			}
			if let Some(index) = line.find("-- prints:") {
				let printed = &line[index + "-- prints:".len()..];
				let printed = printed.strip_prefix(' ').unwrap_or(printed).trim_end();
				let printed = printed.replace("<TAB>", "\t");
				// `"line" ×3`: the line, three times
				match printed.rsplit_once(" ×").and_then(|(line, count)| {
					let count: usize = count.parse().ok()?;
					let line = line.strip_prefix('"')?.strip_suffix('"')?;
					Some((line.to_string(), count))
				}) {
					Some((line, count)) => program.prints.extend(std::iter::repeat_n(line, count)),
					// `"text"`: the brief's notation of a printed string
					None => match printed
						.strip_prefix('"')
						.and_then(|line| line.strip_suffix('"'))
					{
						Some(line) if !line.contains('"') => program.prints.push(line.to_string()),
						_ => program.prints.push(printed),
					},
				}
			}
			if let Some(index) = line.find("-- exit:") {
				program.exit = line[index + "-- exit:".len()..]
					.trim()
					.parse()
					.expect("an exit code");
			}
			if let Some(index) = line.find("-- skip-target:") {
				program.skip_targets.extend(
					line[index + "-- skip-target:".len()..]
						.split_whitespace()
						.map(str::to_string),
				);
			}
			if line.contains("-- overflow-checks: false") {
				program.overflow_checks = false;
			}
			if line.contains("-- asynchronous: false") {
				program.asynchronous = Some(false);
			}
			if line.contains("-- asynchronous: true") {
				program.asynchronous = Some(true);
			}
			if line.contains("-- opt-level: 0") {
				program.dev = true;
			}
			if let Some(index) = line.find("-- stderr:") {
				program
					.stderr
					.push(line[index + "-- stderr:".len()..].trim().to_string());
			}
			if let Some(index) = line.find("-- args:") {
				program.arguments = line[index + "-- args:".len()..]
					.split_whitespace()
					.map(str::to_string)
					.collect();
			}
			if !required && line.contains("-- pending") {
				return None;
			}
			if let Some(index) = line.find("-- arguments:") {
				program.arguments = line[index + "-- arguments:".len()..]
					.split_whitespace()
					.map(str::to_string)
					.collect();
			}
			if !required {
				// `-- error: E-NUM-6`, `-- kind: test`…: an expected diagnostic or a
				// case that isn't a program (a comment that mentions an error
				// is not one: `-- stderr: error: IndexError`)
				let comment = line
					.find("--")
					.map(|index| &line[index + 2..])
					.unwrap_or("");
				let expects_diagnostic = ["error", "warning", "pending", "kind:", "target:"]
					.iter()
					.any(|word| comment.trim_start().starts_with(word));
				if expects_diagnostic {
					return None;
				}
			}
		}
		if !required && program.prints.is_empty() {
			return None;
		}
		Some(program)
	}

	fn programs() -> Vec<Program> {
		let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
		let mut programs = Vec::new();
		let mut own: Vec<PathBuf> = std::fs::read_dir(manifest.join("tests/programs"))
			.expect("tests/programs")
			.map(|entry| entry.expect("an entry").path())
			.filter(|path| path.extension().is_some_and(|extension| extension == "fa"))
			.collect();
		own.sort();
		programs.extend(own.iter().filter_map(|path| read_program(path, true)));
		let mut cases = Vec::new();
		// the programs of the `fa` command (the JavaScript backend's)
		collect_cases(&manifest.join("../cli/tests/programs"), &mut cases);
		collect_cases(&manifest.join("../analyzer/tests/cases"), &mut cases);
		cases.sort();
		// the JavaScript-only programs of the `fa` command (±(2^53 − 1), names)
		cases.retain(|path| !path.to_string_lossy().contains("javascript"));
		// an analyzer gap: the methods of two structurally equal types resolve
		// to one declaration
		cases.retain(|path| {
			!path.ends_with("3.2-object-types/obj-1.fa")
				&& !path.ends_with("10-structural-typing/adv-10.8-02.fa")
		});
		// run with arguments the annotations don't give
		cases.retain(|path| {
			!path.ends_with("15-modules/adv-15.9-18.fa")
				&& !path.ends_with("15-modules/adv-15.9-16.fa")
		});
		programs.extend(cases.iter().filter_map(|path| read_program(path, false)));
		// `FA_PROGRAMS=async` runs the programs whose path contains it
		if let Some(filter) = std::env::var_os("FA_PROGRAMS") {
			let filter = filter.to_string_lossy().into_owned();
			programs.retain(|program| program.path.to_string_lossy().contains(&filter));
		}
		programs
	}

	fn collect_cases(directory: &Path, cases: &mut Vec<PathBuf>) {
		let Ok(entries) = std::fs::read_dir(directory) else {
			return;
		};
		for entry in entries {
			let path = entry.expect("an entry").path();
			if path.is_dir() {
				// a directory with a `main.fa` is a multi-file program (M2)
				if !path.join("main.fa").exists() {
					collect_cases(&path, cases);
				}
			} else if path.extension().is_some_and(|extension| extension == "fa") {
				cases.push(path);
			}
		}
	}

	enum Outcome {
		Passed,
		Skipped(String),
		Failed(String),
	}

	fn check(program: &Program, target: Target, directory: &Path) -> Outcome {
		let name = program
			.path
			.file_stem()
			.map(|stem| stem.to_string_lossy().into_owned())
			.unwrap_or_default();
		let parent = program
			.path
			.parent()
			.and_then(Path::file_name)
			.map(|name| name.to_string_lossy().into_owned())
			.unwrap_or_default();
		let output = directory.join(format!("{parent}-{name}"));
		// no signal handler catches a stack overflow when freestanding
		let stack_overflow = program.path.to_string_lossy().contains("stack-overflow");
		if program
			.skip_targets
			.iter()
			.any(|skipped| skipped == target.name())
			|| (stack_overflow && target == Target::NativeFreestanding)
		{
			return Outcome::Skipped(format!("not run on {}", target.name()));
		}
		let options = Options {
			overflow_checks: program.overflow_checks,
			// every block a program allocates is freed by its end
			leak_check: true,
			asynchronous: program.asynchronous,
			opt_level: if program.dev {
				fa_codegen_llvm::OptLevel::O0
			} else {
				fa_codegen_llvm::OptLevel::default()
			},
			..Options::default()
		};
		let compiled = if program.files.is_empty() {
			compile_file(&program.path, target, &options, &output)
		} else {
			compile_sources(&program.files, target, &options, &output)
		};
		let artifact = match compiled {
			Ok(artifact) => artifact,
			Err(error @ (Error::Unsupported { .. } | Error::Diagnostics(_)))
				if !program.required =>
			{
				return Outcome::Skipped(error.to_string());
			}
			// floats need the C library
			Err(error @ Error::Unsupported { .. }) if target == Target::NativeFreestanding => {
				return Outcome::Skipped(error.to_string());
			}
			Err(error) => return Outcome::Failed(format!("does not compile: {error}")),
		};
		if let Some(failure) = expect(program, &artifact) {
			return Outcome::Failed(failure);
		}
		// a package (the command line compiles them per unit), compiled per
		// unit too (design/parallel-pipeline.md §3.3)
		if !program
			.files
			.iter()
			.any(|(path, _)| path == "application/main.fa")
		{
			return Outcome::Passed;
		}
		let split_output = directory.join(format!("{parent}-{name}-split"));
		match compile_split(&program.files, target, &options, &split_output) {
			Ok(Some(artifact)) => match expect(program, &artifact) {
				Some(failure) => Outcome::Failed(format!("compiled per unit: {failure}")),
				None => Outcome::Passed,
			},
			// compiled as one module by the command line (never a package
			// of `tests/programs`)
			Ok(None) if program.required => {
				Outcome::Failed("compiled as one module, not per unit".to_string())
			}
			Ok(None) => Outcome::Passed,
			Err(error) => Outcome::Failed(format!("does not compile per unit: {error}")),
		}
	}

	/// Runs a compiled program: what it does that its annotations don't say.
	fn expect(program: &Program, artifact: &fa_codegen_llvm::Artifact) -> Option<String> {
		let arguments: Vec<&str> = program.arguments.iter().map(String::as_str).collect();
		let result = match run_within(artifact, &arguments, None) {
			Ok(result) => result,
			Err(reason) => return Some(reason),
		};
		let stdout = String::from_utf8_lossy(&result.stdout);
		let stderr = String::from_utf8_lossy(&result.stderr);
		let printed: Vec<&str> = stdout.lines().collect();
		let expected: Vec<&str> = program.prints.iter().map(String::as_str).collect();
		let code = match result.status.code() {
			Some(code) => code,
			None => {
				return Some(format!(
					"ended by {}, after printing {printed:?}\n{stderr}",
					signal_of(&result.status)
				));
			}
		};
		let missing_stderr = program
			.stderr
			.iter()
			.any(|text| !stderr.contains(text.as_str()));
		if printed != expected || code != program.exit || missing_stderr {
			return Some(format!(
				"expected {expected:?} and exit {}, got {printed:?} and exit {code}\n{stderr}",
				program.exit
			));
		}
		None
	}

	/// The time a program may run: `FA_PROGRAM_TIMEOUT` seconds, else two
	/// minutes, times the load of the machine per core when it is above
	/// one (a full workspace run on a busy machine).
	fn time_limit() -> std::time::Duration {
		let base = std::env::var("FA_PROGRAM_TIMEOUT")
			.ok()
			.and_then(|value| value.parse().ok())
			.unwrap_or(120.0);
		let cores = std::thread::available_parallelism().map_or(1, usize::from) as f64;
		let load = std::fs::read_to_string("/proc/loadavg")
			.ok()
			.and_then(|text| text.split_whitespace().next()?.parse::<f64>().ok())
			.unwrap_or(0.0);
		std::time::Duration::from_secs_f64(base * (load / cores).max(1.0))
	}

	/// Runs a compiled program like `fa_codegen_llvm::run`, killed when it
	/// runs longer than [`time_limit`]; the error says why it didn't run.
	fn run_within(
		artifact: &fa_codegen_llvm::Artifact,
		arguments: &[&str],
		limit: Option<std::time::Duration>,
	) -> Result<std::process::Output, String> {
		use std::io::Read;
		use std::process::{Command, Stdio};
		let mut command = match (&artifact.loader, artifact.target) {
			(Some(loader), Target::Wasm) => {
				let runtime = fa_codegen_llvm::javascript_runtime()
					.map_err(|error| format!("does not run: {error}"))?;
				let mut command = Command::new(runtime);
				command.arg(loader);
				command
			}
			_ => Command::new(&artifact.output),
		};
		let mut child = command
			.args(arguments)
			.stdout(Stdio::piped())
			.stderr(Stdio::piped())
			.spawn()
			.map_err(|error| format!("does not run: {error}"))?;
		// the pipes are read while it runs (a full pipe would block it)
		let mut stdout = child.stdout.take().expect("a pipe");
		let mut stderr = child.stderr.take().expect("a pipe");
		let out = std::thread::spawn(move || {
			let mut bytes = Vec::new();
			let _ = stdout.read_to_end(&mut bytes);
			bytes
		});
		let err = std::thread::spawn(move || {
			let mut bytes = Vec::new();
			let _ = stderr.read_to_end(&mut bytes);
			bytes
		});
		let limit = limit.unwrap_or_else(time_limit);
		let started = std::time::Instant::now();
		let status = loop {
			match child.try_wait() {
				Ok(Some(status)) => break status,
				Ok(None) if started.elapsed() > limit => {
					let _ = child.kill();
					let _ = child.wait();
					let printed =
						String::from_utf8_lossy(&out.join().unwrap_or_default()).into_owned();
					return Err(format!(
						"timed out after {:.0} s (the limit scales with the load; FA_PROGRAM_TIMEOUT), after printing {:?}",
						limit.as_secs_f64(),
						printed.lines().collect::<Vec<_>>()
					));
				}
				Ok(None) => std::thread::sleep(std::time::Duration::from_millis(5)),
				Err(error) => return Err(format!("does not run: {error}")),
			}
		};
		Ok(std::process::Output {
			status,
			stdout: out.join().unwrap_or_default(),
			stderr: err.join().unwrap_or_default(),
		})
	}

	/// A program that never ends is killed and reported as a timeout.
	#[test]
	fn a_program_running_too_long_is_a_timeout() {
		let directory = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("fa-timeout");
		std::fs::create_dir_all(&directory).expect("a directory");
		let source = directory.join("forever.fa");
		std::fs::write(&source, "while true {\n\tconsole.log(1)\n}\n").expect("a file");
		let artifact = compile_file(
			&source,
			Target::Native,
			&Options::default(),
			&directory.join("forever"),
		)
		.expect("it compiles");
		let limit = std::time::Duration::from_millis(300);
		match run_within(&artifact, &[], Some(limit)) {
			Err(reason) => assert!(reason.starts_with("timed out after"), "{reason}"),
			Ok(output) => panic!("it ended: {:?}", output.status),
		}
	}

	/// How a process ended without an exit code.
	fn signal_of(status: &std::process::ExitStatus) -> String {
		#[cfg(unix)]
		{
			use std::os::unix::process::ExitStatusExt as _;
			if let Some(signal) = status.signal() {
				return format!("signal {signal}");
			}
		}
		format!("{status}")
	}

	/// How many programs compile at the same time, all targets together (the
	/// tests of the targets run in parallel; each compilation holds an LLVM
	/// module in this process).
	const COMPILATIONS: usize = 4;

	/// The number of compilations running, and its change.
	static RUNNING: (Mutex<usize>, Condvar) = (Mutex::new(0), Condvar::new());

	/// Runs `f` once fewer than [`COMPILATIONS`] others are running.
	fn limited<R>(f: impl FnOnce() -> R) -> R {
		let (running, changed) = &RUNNING;
		let mut count = running.lock().unwrap_or_else(|poison| poison.into_inner());
		while *count >= COMPILATIONS {
			count = changed
				.wait(count)
				.unwrap_or_else(|poison| poison.into_inner());
		}
		*count += 1;
		drop(count);
		let result = f();
		*running.lock().unwrap_or_else(|poison| poison.into_inner()) -= 1;
		changed.notify_one();
		result
	}

	/// Analyzes and compiles a multi-file program: a package when a path
	/// starts with `library/`, `application/` or `scripts/`, else the
	/// program of `main.fa` (or of the first file).
	fn compile_sources(
		files: &[(String, String)],
		target: Target,
		options: &Options,
		output: &Path,
	) -> Result<fa_codegen_llvm::Artifact, Error> {
		use fa_analyzer::modules::CodebaseResolver;
		let entry = files
			.iter()
			.find(|(path, _)| path == "main.fa")
			.or_else(|| files.iter().find(|(path, _)| path.ends_with(".fa")))
			.map(|(path, _)| path.clone())
			.unwrap_or_default();
		let is_package = files.iter().any(|(path, _)| {
			path.starts_with("library/")
				|| path.starts_with("application/")
				|| path.starts_with("scripts/")
		});
		let mut session = fa_analyzer::session::Session::new();
		session.set_target(target.analysis_target());
		session.set_resolver(Box::new(CodebaseResolver::from_texts(files, Some(&entry))));
		let program = if is_package {
			let entry = files
				.iter()
				.any(|(path, _)| path == "application/main.fa")
				.then_some("application/main");
			session.analyze_package(entry)
		} else {
			let module = entry.strip_suffix(".fa").unwrap_or(&entry);
			match session.analyze_program(module) {
				Some(program) => program,
				None => return Err(Error::Diagnostics(format!("{module} is not found"))),
			}
		};
		fa_codegen_llvm::compile_program(&session, &program, target, options, output)
	}

	/// Compiles a multi-file program per unit (`compile_package`): `None`
	/// when it can't be (the command line compiles it as one module).
	fn compile_split(
		files: &[(String, String)],
		target: Target,
		options: &Options,
		output: &Path,
	) -> Result<Option<fa_codegen_llvm::Artifact>, Error> {
		use fa_analyzer::modules::CodebaseResolver;
		let entry = files
			.iter()
			.find(|(path, _)| path == "main.fa")
			.or_else(|| files.iter().find(|(path, _)| path.ends_with(".fa")))
			.map(|(path, _)| path.clone())
			.unwrap_or_default();
		let module = "application/main".to_string();
		let mut session = fa_analyzer::session::Session::new();
		session.set_target(target.analysis_target());
		session.set_resolver(Box::new(CodebaseResolver::from_texts(files, Some(&entry))));
		let run = fa_analyzer::pipeline::Options {
			entry: Some(module),
			..fa_analyzer::pipeline::Options::default()
		};
		let split = fa_codegen_llvm::SplitOptions {
			lto: options.optimized(),
			runtime_cache: Some(PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("fa-runtime")),
			..fa_codegen_llvm::SplitOptions::default()
		};
		let objects = PathBuf::from(format!("{}.objects", output.display()));
		let (report, outcome) = fa_codegen_llvm::compile_package(
			&mut session,
			&run,
			target,
			options,
			&split,
			&objects,
			output,
			&fa_codegen_llvm::Libraries::default(),
		);
		match outcome {
			fa_codegen_llvm::PackageOutcome::Built(artifact) => Ok(Some(artifact)),
			fa_codegen_llvm::PackageOutcome::Unsupported(reason) => {
				if std::env::var_os("FA_SPLIT_TRACE").is_some() {
					eprintln!("split fallback: {}: {reason}", output.display());
				}
				Ok(None)
			}
			fa_codegen_llvm::PackageOutcome::Errors => Err(Error::Diagnostics(format!(
				"{:?}",
				report
					.modules
					.iter()
					.flat_map(|module| module.diagnostics.iter().map(|d| d.message.clone()))
					.collect::<Vec<_>>()
			))),
			fa_codegen_llvm::PackageOutcome::Failed(error) => Err(error),
		}
	}

	/// Runs every program on a target, with a few threads.
	fn run_all(target: Target) {
		let directory = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
			.join("fa-programs")
			.join(target.name());
		std::fs::create_dir_all(&directory).expect("a directory");
		let programs = programs();
		let next = Mutex::new(0usize);
		let results = Mutex::new(Vec::new());
		std::thread::scope(|scope| {
			for _ in 0..3 {
				scope.spawn(|| {
					loop {
						let index = {
							let mut next = next.lock().unwrap();
							*next += 1;
							*next - 1
						};
						let Some(program) = programs.get(index) else {
							break;
						};
						let outcome = limited(|| check(program, target, &directory));
						results.lock().unwrap().push((index, outcome));
					}
				});
			}
		});
		let mut results = results.into_inner().unwrap();
		results.sort_by_key(|(index, _)| *index);
		let (mut passed, mut skipped, mut failures) = (0, Vec::new(), Vec::new());
		for (index, outcome) in results {
			let path = programs[index].path.display().to_string();
			match outcome {
				Outcome::Passed => passed += 1,
				Outcome::Skipped(reason) => skipped.push(format!("{path}: {reason}")),
				Outcome::Failed(reason) => failures.push(format!("{path}: {reason}")),
			}
		}
		eprintln!(
			"{}: {passed} passed, {} skipped, {} failed",
			target.name(),
			skipped.len(),
			failures.len()
		);
		if std::env::var_os("FA_SHOW_SKIPPED").is_some() {
			for reason in &skipped {
				eprintln!("skipped {reason}");
			}
		}
		assert!(
			failures.is_empty(),
			"{} programs failed on {}:\n{}",
			failures.len(),
			target.name(),
			failures.join("\n\n")
		);
	}

	#[test]
	fn native_programs() {
		run_all(Target::Native);
	}

	#[test]
	#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
	fn freestanding_programs() {
		run_all(Target::NativeFreestanding);
	}

	#[test]
	fn wasm_programs() {
		if fa_codegen_llvm::javascript_runtime().is_err() {
			eprintln!("skipped: neither node nor bun was found");
			return;
		}
		run_all(Target::Wasm);
	}
}
