//! `fa run`: a program compiled into the cache and run with Bun, or
//! natively; finding Bun, and forwarding the exit code.

use super::*;

/// `fa run [--target=…] <file.fa> [--] [arguments...]`.
pub(crate) fn run(arguments: &[OsString]) -> i32 {
	let all = arguments;
	let mut native = fa_codegen_llvm::Options::default();
	let mut arguments = arguments;
	let mut native_target = None;
	let mut asynchronous = None;
	while let Some((first, rest)) = arguments.split_first() {
		let text = first.to_string_lossy();
		if native_option(&text, &mut native) {
			arguments = rest;
		} else if let Some(mode) = asynchronous_option(&text) {
			let Ok(mode) = mode else {
				eprintln!(
					"fa run: `{text}`: write `--asynchronous=true` or `--asynchronous=false`"
				);
				return USAGE;
			};
			asynchronous = Some(mode);
			arguments = rest;
		} else if let Some(value) = text.strip_prefix("--target=") {
			if value != "bun" {
				match fa_codegen_llvm::Target::from_name(value) {
					Some(target) => native_target = Some(target),
					None => {
						eprintln!(
							"error[CLI002]: unknown target `{value}`: `fa run` runs on `bun`, `native`, `native-freestanding` or `wasm`"
						);
						return USAGE;
					}
				}
			}
			arguments = rest;
		} else {
			break;
		}
	}
	let mode = Mode {
		asynchronous,
		verbose: false,
	};
	if let Some(target) = native_target {
		// the LLVM backend is in `fa-native` (`native.rs`)
		return match crate::native::backend() {
			Some(backend) => run_native(backend, arguments, target, &native, mode),
			None => crate::native::delegate("run", all, target.name()),
		};
	}
	let Some((file, rest)) = arguments.split_first() else {
		eprintln!("fa run: which file? ex: `fa run main.fa`");
		return USAGE;
	};
	let program_arguments = match rest.split_first() {
		Some((separator, rest)) if separator == "--" => rest,
		_ => rest,
	};
	let mut path = PathBuf::from(file);
	// the name of a package of the workspace
	if !path.exists() && path.extension().is_none_or(|extension| extension != "fa") {
		match workspace::select(Some(&path), Implements::Application) {
			Ok(Selection::One(Some(directory))) => path = directory,
			Ok(_) => {}
			Err(message) => {
				crate::color::eprintln_message(&message);
				return 1;
			}
		}
	}
	// a package (not one of its scripts): on the parallel and streaming
	// pipeline, its modules written into the cache by the threads that
	// analyze them (an asynchronous mode configured: sequential)
	if pipeline_enabled()
		&& mode.asynchronous.is_none()
		&& let Some(project) = streaming::locate(Some(&path))
		&& !configures_asynchronous(&project.base)
	{
		let directory = run_directory(&path);
		let purpose = streaming::Purpose {
			tests: false,
			quiet: true,
			replace: true,
		};
		let mut options = Options::new(Target::Bun, String::new());
		options.overflow_checks = native.overflow_checks;
		let main = match streaming::build(project, Target::Bun, &options, purpose, |_| directory) {
			streaming::Built::Written(modules) => modules[0].clone(),
			streaming::Built::Failed(code) => return code,
		};
		return run_with_bun(&main, program_arguments);
	}
	let mut loaded = match Loaded::new(&path) {
		Ok(loaded) => loaded,
		Err(message) => {
			crate::color::eprintln_message(&message);
			return 1;
		}
	};
	loaded.configure_asynchronous(mode.asynchronous, false);
	if loaded.has_errors() {
		crate::color::eprint_diagnostics(&loaded.rendered());
		return 1;
	}
	let mut options = Options::new(Target::Bun, loaded.shown.clone());
	options.asynchronous = Some(loaded.inner.program.asynchronous);
	options.libraries = loaded.c_libraries();
	options.overflow_checks = native.overflow_checks;
	let generated = match loaded.generate(&options) {
		Ok(generated) => generated,
		Err(rendered) => {
			crate::color::eprint_diagnostics(&rendered);
			return 1;
		}
	};
	let script = match write_run_script(&path, &generated) {
		Ok(script) => script,
		Err(error) => {
			eprintln!("fa run: can't write the compiled program: {error}");
			return 1;
		}
	};
	run_with_bun(&script, program_arguments)
}

/// The message of `CLI005`, when Bun can't be found.
pub(crate) const BUN_MISSING: &str = "error[CLI005]: Bun is needed to run Fa code on the bun target, and it is neither on the PATH nor in ~/.bun/bin: install it from https://bun.sh";

/// Runs a compiled program (or the compiled tests) with Bun, forwarding
/// the arguments and the exit code.
pub(crate) fn run_with_bun(script: &Path, arguments: &[OsString]) -> i32 {
	let Some(bun) = find_bun() else {
		eprintln!("{BUN_MISSING}");
		return 1;
	};
	let _ = std::io::stdout().flush();
	match Command::new(&bun).arg(script).args(arguments).status() {
		Ok(status) => exit_code(status),
		Err(error) => {
			eprintln!("fa: can't start `{}`: {error}", bun.display());
			1
		}
	}
}

/// The folder of the cache a program of several modules is compiled into
/// to be run (`fa run`, `fa test`): one per source path.
pub(crate) fn run_directory(source: &Path) -> PathBuf {
	let absolute = fs::canonicalize(source).unwrap_or_else(|_| source.to_path_buf());
	let key = fnv1a(absolute.to_string_lossy().as_bytes());
	cache_directory()
		.join(format!("{key:016x}"))
		.join("program")
}

/// `fa run --target=native|wasm`: compiles into the cache and runs.
pub(crate) fn run_native(
	backend: &dyn crate::native::Backend,
	arguments: &[OsString],
	target: fa_codegen_llvm::Target,
	options: &fa_codegen_llvm::Options,
	mode: Mode,
) -> i32 {
	let Some((file, rest)) = arguments.split_first() else {
		eprintln!("fa run: which file? ex: `fa run main.fa`");
		return USAGE;
	};
	let program_arguments = match rest.split_first() {
		Some((separator, rest)) if separator == "--" => rest,
		_ => rest,
	};
	let path = PathBuf::from(file);
	let source = fs::read(&path).unwrap_or_default();
	let directory = cache_directory().join(format!(
		"{}-{:016x}",
		target.name(),
		fnv1a(&[path.display().to_string().as_bytes(), &source].concat())
	));
	let artifact = match build_native(backend, &path, target, options, mode, Some(&directory), true) {
		Ok(artifact) => artifact,
		Err(message) => {
			crate::color::eprint_diagnostics(&message);
			return 1;
		}
	};
	let mut command = match &artifact.loader {
		Some(loader) => match fa_codegen_llvm::javascript_runtime() {
			Ok(runtime) => {
				let mut command = Command::new(runtime);
				command.arg(loader);
				command
			}
			Err(error) => {
				eprintln!("fa run: {error}");
				return 1;
			}
		},
		None => Command::new(&artifact.output),
	};
	let _ = std::io::stdout().flush();
	match command.args(program_arguments).status() {
		Ok(status) => exit_code(status),
		Err(error) => {
			eprintln!("fa run: can't start the program: {error}");
			1
		}
	}
}

/// The exit code of a finished process (128 + the signal when killed).
pub(crate) fn exit_code(status: std::process::ExitStatus) -> i32 {
	if let Some(code) = status.code() {
		return code;
	}
	#[cfg(unix)]
	{
		use std::os::unix::process::ExitStatusExt as _;
		if let Some(signal) = status.signal() {
			return 128 + signal;
		}
	}
	1
}

/// The cache directory of `fa run` and `fa test`: `run/` in the cache of
/// `fa` (`$FA_CACHE`, else `$XDG_CACHE_HOME/fa` or `~/.cache/fa`; see
/// [`dependencies::cache_root`]).
pub(crate) fn cache_directory() -> PathBuf {
	dependencies::cache_root().join("run")
}

/// Writes the compiled program of a source file into the cache: one
/// directory per source path, so that concurrent runs of different files
/// don't collide (and a file is written to a new name, then renamed).
pub(crate) fn write_run_script(source: &Path, code: &JsBuild) -> std::io::Result<PathBuf> {
	let absolute = fs::canonicalize(source).unwrap_or_else(|_| source.to_path_buf());
	let key = fnv1a(absolute.to_string_lossy().as_bytes());
	let directory = cache_directory().join(format!("{key:016x}"));
	fs::create_dir_all(&directory)?;
	let code = match code {
		JsBuild::Single(output) => &output.code,
		JsBuild::Program(output) => {
			// one module per file, in a folder of its own (a new one per run, renamed into place)
			let modules = directory.join("program");
			let partial = directory.join(format!("program.{}.tmp", std::process::id()));
			let _ = fs::remove_dir_all(&partial);
			project::write_modules(&partial, &output.modules)?;
			let _ = fs::remove_dir_all(&modules);
			fs::rename(&partial, &modules)?;
			return Ok(modules.join("main.js"));
		}
	};
	let stem = source.file_stem().map_or("main".to_string(), |stem| {
		stem.to_string_lossy().to_string()
	});
	let script = directory.join(format!("{stem}.mjs"));
	let partial = directory.join(format!("{stem}.{}.tmp", std::process::id()));
	fs::write(&partial, code)?;
	fs::rename(&partial, &script)?;
	Ok(script)
}

/// The FNV-1a hash of bytes.
pub(crate) fn fnv1a(bytes: &[u8]) -> u64 {
	let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
	for &byte in bytes {
		hash ^= u64::from(byte);
		hash = hash.wrapping_mul(0x0100_0000_01b3);
	}
	hash
}

/// The `bun` executable: on the `PATH`, or in `~/.bun/bin`.
pub fn find_bun() -> Option<PathBuf> {
	let name = if cfg!(windows) { "bun.exe" } else { "bun" };
	if let Some(path) = std::env::var_os("PATH") {
		for directory in std::env::split_paths(&path) {
			let candidate = directory.join(name);
			if candidate.is_file() {
				return Some(candidate);
			}
		}
	}
	let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))?;
	let candidate = PathBuf::from(home).join(".bun").join("bin").join(name);
	candidate.is_file().then_some(candidate)
}
