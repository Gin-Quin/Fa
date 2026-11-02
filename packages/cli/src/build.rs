//! `fa build`: the options, the JavaScript targets (on the streaming
//! pipeline for a package, minified with Bun) and the LLVM ones.

use super::*;

/// How a program runs (design/async.md A5): `--asynchronous`, and `-v`.
#[derive(Copy, Clone)]
pub(crate) struct Mode {
	/// `--asynchronous=true|false`; `None`: `build.asynchronous`, else detected.
	pub(crate) asynchronous: Option<bool>,
	/// Say why the program is asynchronous or not.
	pub(crate) verbose: bool,
}

/// The options of `fa build`.
#[derive(Clone)]
pub(crate) struct BuildOptions {
	/// The file or the package to build.
	path: PathBuf,
	/// Whether the path was given (else: the current folder's package, or
	/// the packages of its workspace).
	given: bool,
	/// The target, as named on the command line.
	target: String,
	/// Whether `--target` was given (else: `bun`).
	target_given: bool,
	/// `--output=<directory>`.
	output: Option<PathBuf>,
	/// `--declarations`: the TypeScript declarations of the exports.
	declarations: bool,
	/// `--minify=true|false`; `None`: the package's `build.minify` (`true`
	/// by default, also for a standalone file).
	minify: Option<bool>,
	/// The options of the LLVM backend, and `--overflow-checks` for every
	/// target: a release build doesn't check overflows (they wrap, like
	/// Rust's; design/decisions.md, "Overflow checks").
	native: fa_codegen_llvm::Options,
	/// `--asynchronous=true|false`; `None`: `build.asynchronous`, else detected.
	asynchronous: Option<bool>,
	/// `-v`: say why the program is asynchronous or not.
	verbose: bool,
	/// The options given to `fa-native` for the LLVM targets: those of the
	/// backend, `--asynchronous`, `-v` and `--output`, as written.
	forwarded: Vec<OsString>,
}

/// Reads the command line of `fa build`; on a usage error, its exit code.
pub(crate) fn build_options(arguments: &[OsString]) -> Result<BuildOptions, i32> {
	let mut options = BuildOptions {
		path: PathBuf::from("."),
		given: false,
		target: "bun".to_string(),
		target_given: false,
		output: None,
		declarations: false,
		minify: None,
		native: fa_codegen_llvm::Options {
			overflow_checks: false,
			..fa_codegen_llvm::Options::default()
		},
		asynchronous: None,
		verbose: false,
		forwarded: Vec::new(),
	};
	let mut files = Vec::new();
	for argument in arguments {
		let text = argument.to_string_lossy();
		if native_option(&text, &mut options.native) {
			options.forwarded.push(argument.clone());
			continue;
		}
		if asynchronous_option(&text).is_some()
			|| text == "-v"
			|| text == "--verbose"
			|| text.starts_with("--output=")
		{
			options.forwarded.push(argument.clone());
		}
		if let Some(mode) = asynchronous_option(&text) {
			let Ok(mode) = mode else {
				eprintln!(
					"fa build: `{text}`: write `--asynchronous=true` or `--asynchronous=false`"
				);
				return Err(USAGE);
			};
			options.asynchronous = Some(mode);
		} else if text == "-v" || text == "--verbose" {
			options.verbose = true;
		} else if let Some(value) = text.strip_prefix("--target=") {
			options.target = value.to_string();
			options.target_given = true;
		} else if let Some(value) = text.strip_prefix("--output=") {
			options.output = Some(PathBuf::from(value));
		} else if text == "--declarations" {
			options.declarations = true;
		} else if let Some(value) = text.strip_prefix("--minify") {
			options.minify = match value {
				"=true" => Some(true),
				"=false" => Some(false),
				_ => {
					eprintln!("fa build: `{text}`: write `--minify=true` or `--minify=false`");
					return Err(USAGE);
				}
			};
		} else if text.starts_with("--") {
			eprintln!("fa build: unknown option `{text}`");
			return Err(USAGE);
		} else {
			files.push(argument.clone());
		}
	}
	if let Some(path) = optional_path(&files, "build")? {
		options.path = path;
		options.given = true;
	}
	Ok(options)
}

/// `fa build [<file.fa> | <package>] [--target=…] [--output=…]
/// [--declarations] [--minify=true|false] [native options]`.
pub(crate) fn build(arguments: &[OsString]) -> i32 {
	let options = match build_options(arguments) {
		Ok(options) => options,
		Err(code) => return code,
	};
	let argument = options.given.then(|| options.path.clone());
	on_selection(
		argument.as_deref(),
		Implements::Application,
		"build",
		|path| {
			let mut options = options.clone();
			options.path = path.map_or_else(|| PathBuf::from("."), Path::to_path_buf);
			build_one(&options)
		},
	)
}

/// `fa build` of one file or package: for the target given, else for
/// every target of the package's configuration (`targets`; a standalone
/// file or a script: `bun`), each target's outcome reported, `1` when one
/// fails.
pub(crate) fn build_one(options: &BuildOptions) -> i32 {
	if options.target_given {
		// (`fa-native` runs for `fa`, which warned)
		if native::backend().is_none() {
			warn_unconfigured_target(&options.path, &options.target);
		}
		return build_target(options);
	}
	let Some(targets) = configured_targets(&options.path) else {
		return build_target(options);
	};
	let mut outcomes = Vec::new();
	for target in &targets {
		let mut options = options.clone();
		options.target = target.clone();
		if targets.len() > 1 {
			eprintln!("fa build: target `{target}`");
		}
		outcomes.push((target.as_str(), build_target(&options)));
	}
	if targets.len() > 1 {
		let report: Vec<String> = outcomes
			.iter()
			.map(|(target, code)| {
				let outcome = if *code == 0 { "built" } else { "failed" };
				format!("`{target}` {outcome}")
			})
			.collect();
		eprintln!("fa build: {}", report.join(", "));
	}
	i32::from(outcomes.iter().any(|(_, code)| *code != 0))
}

/// The targets of the package of a path (its configuration's `targets`),
/// or `None` for a standalone file or a script.
fn configured_targets(path: &Path) -> Option<Vec<String>> {
	let project = project::locate(Some(path)).ok()?;
	if !project.package || project.entry.is_some() {
		return None;
	}
	Some(configuration_for(&project.base, "bun").targets)
}

/// `fa build` of one file or package for `options.target`.
fn build_target(options: &BuildOptions) -> i32 {
	if let Some(native_target) = fa_codegen_llvm::Target::from_name(&options.target) {
		return build_native_target(options, native_target);
	}
	let Some(target) = Target::from_name(&options.target) else {
		if options.target == "deno" {
			eprintln!(
				"fa build: `deno` is not a target: Fa doesn't support Deno; use `bun` or `node` to run on a server JavaScript runtime"
			);
			return USAGE;
		}
		eprintln!(
			"error[CLI002]: unknown target `{}`: the targets are `bun`, `node`, `web`, `native` and `wasm`",
			options.target
		);
		return USAGE;
	};
	build_javascript(options, target)
}

/// `CLI003`: a package built for a target that its `targets` don't list
/// (built anyway).
pub(crate) fn warn_unconfigured_target(path: &Path, target: &str) {
	let Ok(project) = project::locate(Some(path)) else {
		return;
	};
	if !project.package || project.entry.is_some() {
		return;
	}
	let name = target.strip_suffix("-freestanding").unwrap_or(target);
	if !["web", "bun", "node", "native", "wasm"].contains(&name) {
		return;
	}
	let targets = configuration_for(&project.base, name).targets;
	if !targets.iter().any(|configured| configured == name) {
		eprintln!(
			"warning[CLI003]: `{name}` is not one of the `targets` of the package ({}): it is built anyway; add it to `targets` in `package.fa` to build it by default",
			targets
				.iter()
				.map(|target| format!("`{target}`"))
				.collect::<Vec<_>>()
				.join(", ")
		);
	}
}

/// The configuration of a package for a target, read without analyzing
/// the package: its `package.fa` merged with its workspace's `workspace.fa`
/// (the defaults when it can't be read).
pub(crate) fn configuration_for(
	package: &Path,
	target: &str,
) -> fa_analyzer::configuration::PackageConfiguration {
	let written = workspace::package_configuration(package);
	let workspace_written =
		project::workspace_root(package).and_then(|root| workspace::workspace_configuration(&root));
	let merged = written.as_ref().map(|written| {
		fa_analyzer::configuration::merge(workspace_written.as_ref(), written, target)
	});
	fa_analyzer::configuration::PackageConfiguration::from_value(
		&merged.unwrap_or(fa_analyzer::configuration::ConfigValue::Object(Vec::new())),
	)
}

/// `fa build` for `native`, `native-freestanding` or `wasm`: on `fa-native`
/// (`native.rs`).
pub(crate) fn build_native_target(options: &BuildOptions, target: fa_codegen_llvm::Target) -> i32 {
	let Some(backend) = native::backend() else {
		let mut arguments = vec![OsString::from(format!("--target={}", target.name()))];
		arguments.extend(options.forwarded.iter().cloned());
		arguments.push(options.path.clone().into_os_string());
		return native::delegate("build", &arguments, target.name());
	};
	let mode = Mode {
		asynchronous: options.asynchronous,
		verbose: options.verbose,
	};
	// `--output=<directory>`: into `<directory>/<target>/`
	let output = options
		.output
		.as_ref()
		.map(|output| output.join(target.name()));
	let built = build_native(
		backend,
		&options.path,
		target,
		&options.native,
		mode,
		output.as_deref(),
	);
	match built {
		Ok(artifact) => {
			let written = artifact.loader.as_ref().unwrap_or(&artifact.output);
			eprintln!("fa build: wrote {}", dependencies::shown_path(written));
			0
		}
		Err(message) => {
			crate::color::eprint_diagnostics(&message);
			1
		}
	}
}

/// `fa build` for a JavaScript target: `main.js` (and the other modules of
/// a program), minified unless `build.minify` or `--minify=false` says
/// otherwise (the generated code is readable before minification).
pub(crate) fn build_javascript(options: &BuildOptions, target: Target) -> i32 {
	let path = &options.path;
	// `--output=<directory>`: into `<directory>/<target>/`
	let directory = match &options.output {
		Some(output) => output.join(target.name()),
		None => {
			let folder = if path.is_dir() {
				path.as_path()
			} else {
				path.parent().unwrap_or(Path::new("."))
			};
			folder.join("build").join(target.name())
		}
	};
	// a package: on the parallel and streaming pipeline (`streaming.rs`)
	if pipeline_enabled()
		&& !options.declarations
		&& let Some(project) = streaming::locate(Some(path))
	{
		let mut js_options = Options::new(target, String::new());
		js_options.overflow_checks = options.native.overflow_checks;
		let mut minify = false;
		let purpose = streaming::Purpose::default();
		let built = streaming::build(project, target, &js_options, purpose, |prepared| {
			let configuration = prepared.configuration.as_ref();
			minify = options.minify.unwrap_or_else(|| {
				configuration.is_none_or(|configuration| configuration.build.minify)
			});
			// a package builds into `<package>/<build.output>/<target>/` (D36)
			match (&options.output, configuration) {
				(None, Some(configuration)) => prepared
					.project
					.base
					.join(&configuration.build.output)
					.join(target.name()),
				_ => directory.clone(),
			}
		});
		let modules = match built {
			streaming::Built::Written(modules) => modules,
			streaming::Built::Failed(code) => return code,
		};
		return finish_build(&modules, minify, options.minify.is_none(), target);
	}
	let mut loaded = match Loaded::load(Some(path), target.name()) {
		Ok(loaded) => loaded,
		Err(message) => {
			crate::color::eprintln_message(&message);
			return 1;
		}
	};
	loaded.configure_asynchronous(options.asynchronous, false);
	if options.verbose {
		eprintln!(
			"fa build: asynchronous mode {}",
			loaded.inner.program.asynchronous_reason
		);
	}
	crate::color::eprint_diagnostics(&timings::phase("render", || loaded.rendered()));
	if loaded.has_errors() {
		return 1;
	}
	let mut js_options = Options::new(target, loaded.shown.clone());
	js_options.libraries = loaded.c_libraries();
	js_options.declarations = options.declarations;
	js_options.overflow_checks = options.native.overflow_checks;
	let configuration = loaded
		.inner
		.configuration
		.as_ref()
		.filter(|_| loaded.inner.project.package);
	// a package builds into `<package>/<build.output>/<target>/` (D36)
	let directory = match (&options.output, configuration) {
		(None, Some(configuration)) => loaded
			.inner
			.project
			.base
			.join(&configuration.build.output)
			.join(target.name()),
		_ => directory,
	};
	let minify = options
		.minify
		.unwrap_or_else(|| configuration.is_none_or(|configuration| configuration.build.minify));
	let written = if loaded.inner.is_single_file() {
		let generated = match timings::phase("generation", || loaded.generate(&js_options)) {
			Ok(generated) => generated,
			Err(rendered) => {
				crate::color::eprint_diagnostics(&rendered);
				return 1;
			}
		};
		timings::phase("write", || write_build(&directory, target, &generated))
			.map(|main| vec![main])
	} else {
		// the modules are generated in parallel, each written by its thread
		match timings::phase("generation", || {
			stream_build(&mut loaded, &js_options, &directory, target)
		}) {
			Ok(Ok(written)) => Ok(written),
			Ok(Err(rendered)) => {
				crate::color::eprint_diagnostics(&rendered);
				return 1;
			}
			Err(error) => Err(error),
		}
	};
	let modules = match written {
		Ok(modules) => modules,
		Err(error) => {
			eprintln!(
				"fa build: can't write into `{}`: {error}",
				directory.display()
			);
			return 1;
		}
	};
	finish_build(&modules, minify, options.minify.is_none(), target)
}

/// The end of `fa build` for a JavaScript target: the modules written
/// (`main.js` first) are minified when asked.
pub(crate) fn finish_build(
	modules: &[PathBuf],
	minify: bool,
	by_default: bool,
	target: Target,
) -> i32 {
	if minify {
		for module in modules {
			match timings::phase("minify", || minify_with_bun(module, target)) {
				Ok(()) => {}
				// the default without Bun: the readable code stays
				Err(MinifyError::NoBun) if by_default => {
					eprintln!(
						"fa build: Bun isn't installed: the JavaScript is not minified (`--minify=false` says so explicitly)"
					);
					break;
				}
				Err(error) => {
					eprintln!("fa build: {error}");
					return 1;
				}
			}
		}
	}
	if let Some(main) = modules.first() {
		eprintln!("fa build: wrote {}", dependencies::shown_path(main));
	}
	0
}

/// Why a module couldn't be minified.
pub(crate) enum MinifyError {
	/// Bun is neither on the `PATH` nor in `~/.bun/bin`.
	NoBun,
	/// Bun failed: the message.
	Failed(String),
}

impl std::fmt::Display for MinifyError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			MinifyError::NoBun => write!(
				f,
				"--minify=true needs Bun, which is neither on the PATH nor in ~/.bun/bin: install it from https://bun.sh, or build with --minify=false"
			),
			MinifyError::Failed(message) => f.write_str(message),
		}
	}
}

/// Minifies a generated module in place with `bun build --minify`: the
/// identifiers, whitespace and syntax, its imports kept external and its
/// exports kept (`build.minify`, documentation/architecture/configuration.md).
pub(crate) fn minify_with_bun(main: &Path, target: Target) -> Result<(), MinifyError> {
	let Some(bun) = find_bun() else {
		return Err(MinifyError::NoBun);
	};
	let minified = main.with_extension("min.js");
	let bun_target = match target {
		Target::Bun => "bun",
		Target::Node => "node",
		Target::Web => "browser",
	};
	let result = Command::new(&bun)
		.arg("build")
		.arg(main)
		.args(["--minify", "--format=esm", "--external", "*"])
		.arg(format!("--target={bun_target}"))
		.arg("--outfile")
		.arg(&minified)
		.output()
		.map_err(|error| {
			MinifyError::Failed(format!("can't start `{}`: {error}", bun.display()))
		})?;
	if !result.status.success() {
		let _ = fs::remove_file(&minified);
		return Err(MinifyError::Failed(format!(
			"`bun build --minify` failed: {}",
			String::from_utf8_lossy(&result.stderr).trim_end()
		)));
	}
	fs::rename(&minified, main)
		.map_err(|error| MinifyError::Failed(format!("can't write `{}`: {error}", main.display())))
}

/// Generates the modules of a program of several files into a directory:
/// each module is written by the thread that generated it into
/// `<directory>.partial/`, whose files are moved into the directory once
/// every module is written (and which is deleted on an error). Returns the
/// paths of the modules, or the errors rendered.
pub(crate) fn stream_build(
	loaded: &mut Loaded,
	options: &Options,
	directory: &Path,
	target: Target,
) -> std::io::Result<Result<Vec<PathBuf>, String>> {
	let mut partial = directory.as_os_str().to_owned();
	partial.push(".partial");
	let partial = PathBuf::from(partial);
	let _ = fs::remove_dir_all(&partial);
	fs::create_dir_all(&partial)?;
	let sink = project::DirectorySink::new(&partial);
	let mut options = options.clone();
	options.path = loaded.inner.project.shown.display().to_string();
	let program = loaded.inner.program.clone();
	let entry = fa_codegen_js::EntryOptions::default();
	let result = fa_codegen_js::generate_program_into(
		&mut loaded.inner.session,
		&program,
		&options,
		&entry,
		&sink,
	);
	let written = match result {
		Ok(written) => written,
		Err(error) => {
			let _ = fs::remove_dir_all(&partial);
			return match error {
				fa_codegen_js::WriteError::Generate(GenerateError { diagnostics }) => {
					Ok(Err(loaded.inner.render_errors(&diagnostics)))
				}
				fa_codegen_js::WriteError::Io(error) => Err(error),
			};
		}
	};
	if target == Target::Node {
		fs::write(partial.join("package.json"), "{ \"type\": \"module\" }\n")?;
	}
	timings::phase("write", || {
		project::move_into(&partial, directory, &written.paths)?;
		if target == Target::Node {
			fs::rename(partial.join("package.json"), directory.join("package.json"))?;
		}
		fs::remove_dir_all(&partial)
	})?;
	Ok(Ok(written
		.paths
		.iter()
		.map(|path| directory.join(path))
		.collect()))
}

/// Writes `main.js` (and `main.d.ts`, and a `package.json` declaring an ES
/// module for Node) into a directory, or the modules of a program; returns
/// the path of `main.js`.
pub(crate) fn write_build(
	directory: &Path,
	target: Target,
	output: &JsBuild,
) -> std::io::Result<PathBuf> {
	fs::create_dir_all(directory)?;
	let main = directory.join("main.js");
	match output {
		JsBuild::Single(output) => {
			fs::write(&main, &output.code)?;
			if let Some(declarations) = &output.declarations {
				fs::write(directory.join("main.d.ts"), declarations)?;
			}
		}
		JsBuild::Program(output) => project::write_modules(directory, &output.modules)?,
	}
	if target == Target::Node {
		fs::write(directory.join("package.json"), "{ \"type\": \"module\" }\n")?;
	}
	Ok(main)
}

/// Compiles a file or a package with the LLVM backend (`main`, or
/// `main.wasm` and `main.mjs`) into `output`, else into
/// `<package>/<build.output>/<target>/` for a package (D36) and
/// `build/<target>/` next to a standalone file; on failure, the message to
/// print.
pub(crate) fn build_native(
	backend: &dyn native::Backend,
	path: &Path,
	target: fa_codegen_llvm::Target,
	options: &fa_codegen_llvm::Options,
	mode: Mode,
	output: Option<&Path>,
) -> Result<fa_codegen_llvm::Artifact, String> {
	if !fa_codegen_llvm::llvm_available() {
		return Err(format!(
			"fa: the `{}` target needs the LLVM backend, and this `fa` was built without LLVM: run scripts/setup-llvm.sh, export LLVM_SYS_181_PREFIX and rebuild\n",
			target.name()
		));
	}
	let mut loaded = Loaded::load(Some(path), target.language_target())?;
	loaded.configure_asynchronous(mode.asynchronous, false);
	if mode.verbose {
		eprintln!(
			"fa build: asynchronous mode {}",
			loaded.inner.program.asynchronous_reason
		);
	}
	let rendered = loaded.rendered();
	if loaded.has_errors() {
		return Err(rendered);
	}
	crate::color::eprint_diagnostics(&rendered);
	let configuration = loaded
		.inner
		.configuration
		.as_ref()
		.filter(|_| loaded.inner.project.package);
	let directory = match (output, configuration) {
		(Some(output), _) => output.to_path_buf(),
		(None, Some(configuration)) => loaded
			.inner
			.project
			.base
			.join(&configuration.build.output)
			.join(target.name()),
		(None, None) => path
			.parent()
			.unwrap_or(Path::new("."))
			.join("build")
			.join(target.name()),
	};
	let directory = directory.as_path();
	// the C libraries of foreign code (`build.link`, `build.libraryPaths`)
	let libraries = loaded
		.inner
		.configuration
		.as_ref()
		.map(|configuration| fa_codegen_llvm::Libraries {
			link: configuration.build.link.clone(),
			paths: configuration
				.build
				.library_paths
				.iter()
				.map(|path| loaded.inner.project.base.join(path))
				.collect(),
		})
		.unwrap_or_default();
	// a program of several files: one LLVM module (advanced brief §15.7)
	let input = if loaded.inner.is_single_file() {
		native::Input::File {
			file: loaded.file,
			shown: &loaded.shown,
		}
	} else {
		native::Input::Program(&loaded.inner.program)
	};
	// generation, optimization and link: one call for now
	timings::phase("llvm", || {
		backend.compile(
			loaded.session(),
			input,
			target,
			options,
			&directory.join("main"),
			&libraries,
		)
	})
}
