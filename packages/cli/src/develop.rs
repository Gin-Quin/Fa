//! `fa develop` and `fa test --watch` (advanced brief §19, §21.4): compile,
//! run, watch the files, and run again after a change.
//!
//! The watcher polls the files of the package (its configuration files,
//! `library/`, `application/`, `scripts/`), the packages of the workspace it
//! depends on, and the files of the workspace's root, every 150 ms
//! (`FA_WATCH_INTERVAL_MS`). An edited source file is analyzed again
//! incrementally ([`fa_analyzer::Session::update`]: the file, then its
//! readers when its interface changed); an added or removed file, a changed
//! configuration file or lockfile, or a change in another package reloads
//! the package.
//!
//! Hot reloading (decision D43, `hot.rs`): while the program runs, an edit
//! of the package's own sources that keeps the shape of the program swaps
//! the changed modules in the running Bun process (`develop.hotReload`,
//! on by default); any other change restarts the program, with the
//! reason. A change with errors keeps the running program until it is
//! fixed (design/decisions.md, "Development mode"). After an edit of its own
//! sources, the tests that cover it run again (`run_covering_tests`). `FA_DEVELOP_RUNS=<n>`
//! ends the command after `n` runs of the program or the tests (the tests
//! of the command).

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::time::{Duration, SystemTime};

use fa_codegen_js::{Options, Target};

use crate::workspace::{self, Implements, Selection};
use crate::{JsBuild, Loaded, USAGE, find_bun, hot, write_run_script};

/// The state of a watched file: its modification time and length.
type Stamp = (SystemTime, u64);

/// The files a command watches, polled.
pub(crate) struct Watcher {
	/// The folders watched with everything inside (`true`), or only their
	/// files (`false`).
	roots: Vec<(PathBuf, bool)>,
	/// The files and their stamps at the last poll.
	snapshot: BTreeMap<PathBuf, Stamp>,
}

/// What changed between two polls.
#[derive(Default, Debug)]
pub(crate) struct Changes {
	pub added: Vec<PathBuf>,
	pub removed: Vec<PathBuf>,
	pub modified: Vec<PathBuf>,
}

impl Changes {
	/// Whether nothing changed.
	pub(crate) fn is_empty(&self) -> bool {
		self.added.is_empty() && self.removed.is_empty() && self.modified.is_empty()
	}
}

/// Whether a file is watched: Fa sources and the lockfile.
fn is_watched(path: &Path) -> bool {
	path.extension().is_some_and(|extension| extension == "fa")
		|| path
			.file_name()
			.is_some_and(|name| name == "dependencies.lock")
}

impl Watcher {
	/// A watcher of folders, taking a first snapshot.
	pub(crate) fn new(roots: Vec<(PathBuf, bool)>) -> Self {
		let mut watcher = Self {
			roots,
			snapshot: BTreeMap::new(),
		};
		watcher.snapshot = watcher.scan();
		watcher
	}

	/// The watched files now.
	fn scan(&self) -> BTreeMap<PathBuf, Stamp> {
		let mut found = BTreeMap::new();
		for (root, recursive) in &self.roots {
			collect(root, *recursive, &mut found);
		}
		found
	}

	/// The changes since the last poll.
	pub(crate) fn poll(&mut self) -> Changes {
		let now = self.scan();
		let mut changes = Changes::default();
		for (path, stamp) in &now {
			match self.snapshot.get(path) {
				None => changes.added.push(path.clone()),
				Some(before) if before != stamp => changes.modified.push(path.clone()),
				Some(_) => {}
			}
		}
		for path in self.snapshot.keys() {
			if !now.contains_key(path) {
				changes.removed.push(path.clone());
			}
		}
		self.snapshot = now;
		changes
	}
}

/// Collects the watched files of a folder.
fn collect(directory: &Path, recursive: bool, found: &mut BTreeMap<PathBuf, Stamp>) {
	let Ok(entries) = fs::read_dir(directory) else {
		return;
	};
	for entry in entries.flatten() {
		let name = entry.file_name().to_string_lossy().to_string();
		if name.starts_with('.') {
			continue;
		}
		let Ok(kind) = entry.file_type() else {
			continue;
		};
		if kind.is_dir() {
			if recursive && !fa_codebase_parser::SKIPPED_FOLDERS.contains(&name.as_str()) {
				collect(&entry.path(), true, found);
			}
		} else if is_watched(&entry.path())
			&& let Ok(metadata) = entry.metadata()
		{
			let modified = metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH);
			found.insert(entry.path(), (modified, metadata.len()));
		}
	}
}

/// The polling interval.
fn interval() -> Duration {
	let milliseconds = std::env::var("FA_WATCH_INTERVAL_MS")
		.ok()
		.and_then(|value| value.parse().ok())
		.unwrap_or(150);
	Duration::from_millis(milliseconds)
}

/// The number of runs after which the command ends (`FA_DEVELOP_RUNS`).
fn run_limit() -> Option<usize> {
	std::env::var("FA_DEVELOP_RUNS")
		.ok()
		.and_then(|value| value.parse().ok())
}

/// The folders a loaded package is watched through: the package, the
/// packages of the workspace it uses, the root files of its workspace.
fn watched_roots(loaded: &Loaded) -> Vec<(PathBuf, bool)> {
	let project = &loaded.inner.project;
	if !project.package {
		return vec![(project.base.clone(), false)];
	}
	let mut roots = vec![(project.base.clone(), true)];
	for directory in &loaded.inner.local_dependencies {
		roots.push((directory.clone(), true));
	}
	if let Some(root) = crate::project::workspace_root(&project.base) {
		roots.push((root, false));
	}
	roots
}

/// Why a change needs the package to be loaded again rather than analyzed
/// incrementally; `None` for edits of the package's own sources.
fn reload_reason(loaded: &Loaded, changes: &Changes) -> Option<String> {
	let base = &loaded.inner.project.base;
	let shown = |path: &Path| {
		path.strip_prefix(base).map_or_else(
			|_| crate::dependencies::shown_path(path),
			|relative| relative.display().to_string(),
		)
	};
	if let Some(path) = changes.added.first() {
		return Some(format!("`{}` was added", shown(path)));
	}
	if let Some(path) = changes.removed.first() {
		return Some(format!("`{}` was removed", shown(path)));
	}
	for path in &changes.modified {
		let name = path
			.file_name()
			.map(|name| name.to_string_lossy().to_string())
			.unwrap_or_default();
		if matches!(
			name.as_str(),
			"package.fa" | "workspace.fa" | "dependencies.fa" | "dependencies.lock"
		) {
			return Some(format!("the configuration `{}` changed", shown(path)));
		}
		if !path.starts_with(base) || !loaded.inner.project.package {
			return Some(format!("`{}` changed", shown(path)));
		}
	}
	None
}

/// Applies edits of the package's own sources to its session: each file
/// analyzed again with its readers ([`fa_analyzer::Session::update`]),
/// then the program assembled again. Returns the edited files as shown.
fn apply_edits(loaded: &mut Loaded, modified: &[PathBuf]) -> Vec<String> {
	let base = loaded.inner.project.base.clone();
	let mut shown = Vec::new();
	for path in modified {
		let Ok(relative) = path.strip_prefix(&base) else {
			continue;
		};
		let relative = relative.to_string_lossy().replace('\\', "/");
		let module = relative.trim_end_matches(".fa").to_string();
		let Ok(text) = fs::read_to_string(path) else {
			continue;
		};
		loaded.inner.session.update(&module, text.into());
		shown.push(relative);
	}
	let entry = loaded
		.inner
		.program
		.entry
		.map(|file| loaded.inner.session.module(file).to_string());
	let session = &mut loaded.inner.session;
	loaded.inner.program = match &loaded.inner.project.entry {
		Some(script) => session
			.analyze_program(script)
			.unwrap_or_else(|| loaded.inner.program.clone()),
		None => session.analyze_package(entry.as_deref()),
	};
	if let Some(file) = loaded
		.inner
		.program
		.entry
		.or_else(|| loaded.inner.program.files.first().copied())
	{
		loaded.file = file;
	}
	shown
}

/// The modules of the edited files of the package's own sources, with
/// their texts before the edit.
fn texts_before(loaded: &Loaded, modified: &[PathBuf]) -> Vec<(String, String)> {
	let base = &loaded.inner.project.base;
	modified
		.iter()
		.filter_map(|path| {
			let relative = path.strip_prefix(base).ok()?;
			let module = relative
				.to_string_lossy()
				.replace('\\', "/")
				.trim_end_matches(".fa")
				.to_string();
			let file = loaded.inner.session.loaded_module(&module)?;
			let text = loaded.inner.session.text(file).to_string();
			Some((module, text))
		})
		.collect()
}

/// The edited files of the package's own sources (before they are
/// applied), with their texts: none when the change reloads the package.
fn own_edits(loaded: &Loaded, changes: &Changes) -> Vec<(String, String)> {
	if reload_reason(loaded, changes).is_some() {
		return Vec::new();
	}
	texts_before(loaded, &changes.modified)
}

/// Loads a project again after a change (for `target`), or applies the
/// edits of its own sources. `Err` when it can't be read.
fn refresh(
	loaded: &mut Loaded,
	path: Option<&Path>,
	changes: &Changes,
	target: &str,
) -> Result<String, String> {
	match reload_reason(loaded, changes) {
		Some(reason) => {
			*loaded = Loaded::load(path, target)?;
			Ok(reason)
		}
		None => {
			let edited = apply_edits(loaded, &changes.modified);
			Ok(match edited.as_slice() {
				[one] => format!("`{one}` changed"),
				_ => format!("{} files changed", edited.len()),
			})
		}
	}
}

/// What `fa develop` runs the program on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DevelopTarget {
	/// Bun, its modules swapped in the process.
	Bun,
	/// A browser: the web build served by a development server
	/// (`develop_server.js`, on `develop.port`), its modules swapped in the
	/// pages, a page reloaded when a swap isn't possible.
	Web,
	/// An executable, rebuilt per unit (the objects of the unchanged units
	/// cached) and restarted after each change.
	Native,
}

impl DevelopTarget {
	fn name(self) -> &'static str {
		match self {
			DevelopTarget::Bun => "bun",
			DevelopTarget::Web => "web",
			DevelopTarget::Native => "native",
		}
	}
}

/// Opens a page in the user's browser (`develop.open`): `xdg-open` (Linux),
/// `open` (macOS), `start` (Windows), or the command `FA_OPEN_COMMAND`
/// names (given the URL as its argument); says where to go when it can't.
fn open_browser(url: &str) {
	let mut command = match std::env::var_os("FA_OPEN_COMMAND") {
		Some(program) => Command::new(program),
		None if cfg!(windows) => {
			let mut command = Command::new("cmd");
			command.args(["/C", "start", ""]);
			command
		}
		None if cfg!(target_os = "macos") => Command::new("open"),
		None => Command::new("xdg-open"),
	};
	let opened = command
		.arg(url)
		.stdin(std::process::Stdio::null())
		.stdout(std::process::Stdio::null())
		.stderr(std::process::Stdio::null())
		.spawn();
	if opened.is_err() {
		eprintln!("fa develop: can't open a browser: open {url}");
	}
}

/// The development server of `--target=web`, its root and port filled in.
fn server_script(root: &Path, port: u16) -> String {
	let quoted = format!("{:?}", root.display().to_string());
	include_str!("develop_server.js")
		.replace("const root = __ROOT__;", &format!("const root = {quoted};"))
		.replace("const port = __PORT__;", &format!("const port = {port};"))
}

/// A package being developed.
struct Developed {
	/// What the program runs on.
	target: DevelopTarget,
	/// The path given to [`Loaded::load`].
	path: Option<PathBuf>,
	/// Its name, in messages.
	name: String,
	/// The program, analyzed.
	loaded: Loaded,
	watcher: Watcher,
	/// The running program.
	child: Option<Child>,
	/// The running program as generated, when its modules can be swapped.
	hot: Option<hot::Running>,
	/// The channel with the running program (hot reloading).
	channel: Option<hot::Channel>,
}

/// What an edit did to the running program.
enum Reload {
	/// The changed modules were swapped.
	Swapped(Vec<String>),
	/// The program has errors: it keeps running.
	Errors,
	/// It must restart, for this reason.
	Restart(String),
}

impl Developed {
	/// Whether the program is generated for hot reloading.
	fn hot_enabled(&self) -> bool {
		self.target != DevelopTarget::Native
			&& self
				.loaded
				.inner
				.configuration
				.as_ref()
				.is_none_or(|configuration| configuration.develop.hot_reload)
	}

	/// Runs the tests that cover edited modules (architecture/hot-reloading:
	/// "re-run the tests that cover what changed"): the test files that
	/// read one of them, directly or through other files, each in a Bun
	/// process of its own, while the program goes on.
	fn run_covering_tests(&mut self, edited: &[(String, String)]) {
		let session = &self.loaded.inner.session;
		let tests: Vec<String> = self
			.loaded
			.inner
			.program
			.tests
			.iter()
			.map(|&file| session.module(file).to_string())
			.collect();
		if tests.is_empty() || edited.is_empty() || self.loaded.has_errors() {
			return;
		}
		let mut reached: std::collections::BTreeSet<String> =
			edited.iter().map(|(module, _)| module.clone()).collect();
		let mut stack: Vec<String> = reached.iter().cloned().collect();
		while let Some(module) = stack.pop() {
			for reader in session.readers(&module) {
				if reached.insert(reader.to_string()) {
					stack.push(reader.to_string());
				}
			}
		}
		let covering: Vec<String> = tests
			.into_iter()
			.filter(|test| reached.contains(test))
			.collect();
		if covering.is_empty() {
			return;
		}
		let shown: Vec<String> = covering
			.iter()
			.map(|module| format!("`{module}.fa`"))
			.collect();
		eprintln!(
			"fa develop: {}running the tests of {}",
			self.prefix(),
			shown.join(", ")
		);
		let options = Options::new(Target::Bun, self.loaded.shown.clone());
		let entry = fa_codegen_js::EntryOptions { tests: true };
		let generated = match self.loaded.generate_program(&options, &entry) {
			Ok(generated) => generated,
			Err(rendered) => {
				crate::color::eprint_diagnostics(&rendered);
				return;
			}
		};
		let marker = self.loaded.inner.project.base.join(".fa-develop-tests");
		match write_run_script(&marker, &generated) {
			Ok(main) => {
				crate::run_test_processes(&main, &covering);
			}
			Err(error) => eprintln!("fa develop: can't write the compiled tests: {error}"),
		}
	}

	/// After edits of the package's own sources (`edited`: the modules and
	/// their texts before), swaps the changed modules in the running
	/// program, or says why it restarts.
	fn reload(&mut self, edited: &[(String, String)]) -> Reload {
		if self.loaded.has_errors() {
			crate::color::eprint_diagnostics(&self.loaded.rendered());
			eprintln!(
				"fa develop: {}fix the errors to reload the program",
				self.prefix()
			);
			return Reload::Errors;
		}
		let mut options = Options::new(self.javascript_target(), self.loaded.shown.clone());
		options.hot = true;
		let output = match self.loaded.generate(&options) {
			Ok(JsBuild::Program(output)) => output,
			Ok(JsBuild::Single(_)) => return Reload::Restart("one module".to_string()),
			Err(rendered) => {
				crate::color::eprint_diagnostics(&rendered);
				return Reload::Errors;
			}
		};
		crate::color::eprint_diagnostics(&self.loaded.rendered());
		let (Some(running), Some(channel), Some(_)) = (
			self.hot.as_mut(),
			self.channel.as_mut(),
			self.child.as_ref(),
		) else {
			return Reload::Restart("the program isn't running".to_string());
		};
		let plan = match running.plan(&self.loaded, &output, edited) {
			Ok(plan) => plan,
			Err(reason) => return Reload::Restart(reason),
		};
		let changed = plan.changed.clone();
		match running.swap(plan, channel) {
			Ok(()) => Reload::Swapped(changed),
			Err(reason) => Reload::Restart(reason),
		}
	}

	/// Compiles the program and starts it; `false` when it has errors.
	fn start(&mut self, bun: &Path) -> bool {
		crate::color::eprint_diagnostics(&self.loaded.rendered());
		if self.loaded.has_errors() {
			eprintln!(
				"fa develop: {}fix the errors to run the program again",
				self.prefix()
			);
			return false;
		}
		if self.target == DevelopTarget::Native {
			return self.start_native();
		}
		// the server of the pages runs on: the new build replaces the old one
		let serving = self.target == DevelopTarget::Web && self.child.is_some();
		let mut options = Options::new(self.javascript_target(), self.loaded.shown.clone());
		options.hot = self.hot_enabled();
		let generated = match self.loaded.generate(&options) {
			Ok(generated) => generated,
			Err(rendered) => {
				crate::color::eprint_diagnostics(&rendered);
				return false;
			}
		};
		let marker = self.loaded.inner.project.base.join(".fa-develop");
		let script = match write_run_script(&marker, &generated) {
			Ok(script) => script,
			Err(error) => {
				eprintln!("fa develop: can't write the compiled program: {error}");
				return false;
			}
		};
		self.hot = match &generated {
			JsBuild::Program(output) if options.hot => {
				Some(hot::Running::new(&script, output, &self.loaded))
			}
			_ => None,
		};
		if serving {
			// the pages load the new build again
			if let Some(channel) = self.channel.as_mut()
				&& let Err(reason) = channel.reload()
			{
				eprintln!(
					"fa develop: {}can't reload the pages: {reason}",
					self.prefix()
				);
			}
			return true;
		}
		self.channel = if self.hot.is_some() || self.target == DevelopTarget::Web {
			hot::Channel::open()
		} else {
			None
		};
		if self.channel.is_none() {
			self.hot = None;
		}
		let mut command = Command::new(bun);
		if self.target == DevelopTarget::Web {
			let root = script.parent().map(Path::to_path_buf).unwrap_or_default();
			let port = self
				.loaded
				.inner
				.configuration
				.as_ref()
				.map_or(3000, |configuration| configuration.develop.port);
			let server = root.with_file_name("develop-server.mjs");
			if let Err(error) = fs::write(&server, server_script(&root, port)) {
				eprintln!("fa develop: can't write the development server: {error}");
				return false;
			}
			command.arg(&server);
		} else {
			command.arg(&script);
		}
		if let Some(channel) = &self.channel {
			command.env("FA_HOT_CHANNEL", channel.port().to_string());
		}
		let _ = std::io::stdout().flush();
		match command.spawn() {
			Ok(child) => {
				self.child = Some(child);
				// `develop.open`: a browser on the served pages
				let configuration = self.loaded.inner.configuration.as_ref();
				if self.target == DevelopTarget::Web
					&& configuration.is_some_and(|configuration| configuration.develop.open)
				{
					let port =
						configuration.map_or(3000, |configuration| configuration.develop.port);
					open_browser(&format!("http://localhost:{port}/"));
				}
				true
			}
			Err(error) => {
				eprintln!("fa develop: can't start `{}`: {error}", bun.display());
				false
			}
		}
	}

	/// The JavaScript target of the program.
	fn javascript_target(&self) -> Target {
		match self.target {
			DevelopTarget::Web => Target::Web,
			_ => Target::Bun,
		}
	}

	/// `--target=native`: builds the executable (per unit, the objects of
	/// the unchanged units from the cache) and starts it, saying how long
	/// the build took.
	fn start_native(&mut self) -> bool {
		let Some(backend) = crate::native::backend() else {
			eprintln!("fa develop: the `native` target needs `fa-native`");
			return false;
		};
		let base = self.loaded.inner.project.base.clone();
		let directory = crate::cache_directory().join(format!(
			"develop-native-{:016x}",
			crate::fnv1a(base.to_string_lossy().as_bytes())
		));
		let path = match &self.path {
			Some(path) => path.clone(),
			None => base,
		};
		let start = std::time::Instant::now();
		let mode = crate::Mode {
			asynchronous: None,
			verbose: false,
		};
		let artifact = match crate::build_native(
			backend,
			&path,
			fa_codegen_llvm::Target::Native,
			&fa_codegen_llvm::Options::default(),
			mode,
			Some(&directory),
			true,
		) {
			Ok(artifact) => artifact,
			Err(message) => {
				crate::color::eprint_diagnostics(&message);
				eprintln!(
					"fa develop: {}fix the errors to run the program again",
					self.prefix()
				);
				return false;
			}
		};
		eprintln!(
			"fa develop: {}built for native in {} ms",
			self.prefix(),
			start.elapsed().as_millis()
		);
		let _ = std::io::stdout().flush();
		match Command::new(&artifact.output).spawn() {
			Ok(child) => {
				self.child = Some(child);
				true
			}
			Err(error) => {
				eprintln!("fa develop: can't start the program: {error}");
				false
			}
		}
	}

	/// Runs the program again after a change: a new build in the pages of
	/// `--target=web` (the server runs on), else a restart.
	fn restart(&mut self, bun: &Path) -> bool {
		if self.target != DevelopTarget::Web {
			self.stop();
		}
		self.start(bun)
	}

	/// Stops the running program.
	fn stop(&mut self) {
		if let Some(mut child) = self.child.take() {
			let _ = child.kill();
			let _ = child.wait();
		}
	}

	/// `api: ` when several packages are developed.
	fn prefix(&self) -> String {
		if self.name.is_empty() {
			String::new()
		} else {
			format!("{}: ", self.name)
		}
	}
}

/// The paths a command works on, from a selection.
fn paths(selection: Selection) -> Vec<(String, Option<PathBuf>)> {
	match selection {
		Selection::One(path) => vec![(String::new(), path)],
		Selection::Packages(packages) => packages
			.into_iter()
			.map(|(name, directory)| (name, Some(directory)))
			.collect(),
	}
}

/// `fa develop [<package>] [--target=bun]`.
pub(crate) fn develop(arguments: &[OsString]) -> i32 {
	let mut given = Vec::new();
	let mut target = DevelopTarget::Bun;
	for argument in arguments {
		let text = argument.to_string_lossy();
		if let Some(value) = text.strip_prefix("--target=") {
			target = match value {
				"bun" => DevelopTarget::Bun,
				"web" => DevelopTarget::Web,
				"native" => DevelopTarget::Native,
				_ => {
					eprintln!(
						"fa develop: `fa develop` runs on `bun` (the default), `web` or `native`: build and run the `{value}` target with `fa build` and `fa preview`"
					);
					return USAGE;
				}
			};
		} else if text.starts_with("--") {
			eprintln!("fa develop: unknown option `{text}`: see `fa help develop`");
			return USAGE;
		} else {
			given.push(PathBuf::from(argument));
		}
	}
	if given.len() > 1 {
		eprintln!("fa develop: one package at most, ex: `fa develop api`");
		return USAGE;
	}
	let selection =
		match workspace::select(given.first().map(PathBuf::as_path), Implements::Application) {
			Ok(selection) => selection,
			Err(message) => {
				crate::color::eprintln_message(&message);
				return 1;
			}
		};
	let paths = paths(selection);
	if paths.is_empty() {
		eprintln!(
			"fa develop: no package of the workspace has an application (`application/main.fa`)"
		);
		return 1;
	}
	// the LLVM backend is in `fa-native` (`native.rs`)
	if target == DevelopTarget::Native && crate::native::backend().is_none() {
		return crate::native::delegate("develop", arguments, "native");
	}
	let bun = match find_bun() {
		Some(bun) => bun,
		None if target == DevelopTarget::Native => PathBuf::new(),
		None => {
			eprintln!("{}", crate::BUN_MISSING);
			return 1;
		}
	};
	let mut developed = Vec::new();
	for (name, path) in paths {
		let loaded = match Loaded::load(path.as_deref(), target.name()) {
			Ok(loaded) => loaded,
			Err(message) => {
				crate::color::eprintln_message(&message);
				return 1;
			}
		};
		if loaded.inner.project.package && loaded.inner.program.entry.is_none() {
			eprintln!("fa develop: the package has no application to run (`application/main.fa`)");
			return 1;
		}
		let watcher = Watcher::new(watched_roots(&loaded));
		developed.push(Developed {
			target,
			path,
			name,
			loaded,
			watcher,
			child: None,
			hot: None,
			channel: None,
		});
	}
	let limit = run_limit();
	let mut runs = 0;
	for package in &mut developed {
		eprintln!(
			"fa develop: {}running on {}, watching for changes",
			package.prefix(),
			package.target.name()
		);
		package.start(&bun);
	}
	loop {
		std::thread::sleep(interval());
		for package in &mut developed {
			// the program ended by itself
			if let Some(child) = &mut package.child
				&& let Ok(Some(status)) = child.try_wait()
			{
				package.child = None;
				runs += 1;
				eprintln!(
					"fa develop: {}the program exited with code {}; watching for changes",
					package.prefix(),
					crate::exit_code(status)
				);
			}
			let changes = package.watcher.poll();
			if changes.is_empty() {
				continue;
			}
			// let a burst of writes settle
			std::thread::sleep(Duration::from_millis(30));
			let more = package.watcher.poll();
			let changes = merge(changes, more);
			// edits of its own sources while it runs: swapped (D43)
			if package.child.is_some()
				&& package.hot.is_some()
				&& reload_reason(&package.loaded, &changes).is_none()
			{
				let before = texts_before(&package.loaded, &changes.modified);
				let edited = apply_edits(&mut package.loaded, &changes.modified);
				let reason = match edited.as_slice() {
					[one] => format!("`{one}` changed"),
					_ => format!("{} files changed", edited.len()),
				};
				match package.reload(&before) {
					Reload::Swapped(modules) => {
						let shown: Vec<String> =
							modules.iter().map(|module| format!("`{module}`")).collect();
						let what = if shown.is_empty() {
							"no code changed".to_string()
						} else {
							format!("reloaded {}", shown.join(", "))
						};
						eprintln!("fa develop: {}{reason}: {what}", package.prefix());
						package.run_covering_tests(&before);
					}
					Reload::Errors => {}
					Reload::Restart(why) => {
						runs += 1;
						let verb = if package.target == DevelopTarget::Web {
							"reloading the pages"
						} else {
							"restarting"
						};
						eprintln!("fa develop: {}{reason}: {verb} ({why})", package.prefix());
						package.watcher = Watcher::new(watched_roots(&package.loaded));
						if package.restart(&bun) {
							package.run_covering_tests(&before);
						}
					}
				}
				continue;
			}
			let edited = own_edits(&package.loaded, &changes);
			let path = package.path.clone();
			match refresh(
				&mut package.loaded,
				path.as_deref(),
				&changes,
				package.target.name(),
			) {
				Ok(reason) => {
					if package.child.is_some() {
						runs += 1;
					}
					let verb = if package.child.is_some() && package.target == DevelopTarget::Web {
						"reloading the pages"
					} else if package.child.is_some() {
						"restarting"
					} else {
						"running again"
					};
					eprintln!("fa develop: {}{reason}: {verb}", package.prefix());
					package.watcher = Watcher::new(watched_roots(&package.loaded));
					if package.restart(&bun) {
						package.run_covering_tests(&edited);
					}
				}
				Err(message) => eprintln!("{message}"),
			}
		}
		if limit.is_some_and(|limit| runs >= limit)
			&& developed.iter().all(|package| package.child.is_none())
		{
			return 0;
		}
	}
}

/// The changes of two polls together.
fn merge(mut first: Changes, second: Changes) -> Changes {
	for path in second.added {
		if !first.added.contains(&path) {
			first.added.push(path);
		}
	}
	for path in second.removed {
		first.added.retain(|other| *other != path);
		if !first.removed.contains(&path) {
			first.removed.push(path);
		}
	}
	for path in second.modified {
		if !first.modified.contains(&path) && !first.added.contains(&path) {
			first.modified.push(path);
		}
	}
	first
}

/// `fa test --watch`: runs the tests, then again after each change.
pub(crate) fn watch_tests(selection: Selection) -> i32 {
	let paths = paths(selection);
	if paths.is_empty() {
		eprintln!("fa test: no package of the workspace has test files (`*.test.fa`)");
		return 0;
	}
	let limit = run_limit();
	let mut runs = 0;
	let mut watched: Vec<(Option<PathBuf>, Loaded, Watcher)> = Vec::new();
	for (_, path) in paths {
		let loaded = match Loaded::load(path.as_deref(), "bun") {
			Ok(loaded) => loaded,
			Err(message) => {
				crate::color::eprintln_message(&message);
				return 1;
			}
		};
		let watcher = Watcher::new(watched_roots(&loaded));
		watched.push((path, loaded, watcher));
	}
	for (_, loaded, _) in &mut watched {
		crate::run_tests(loaded);
		runs += 1;
	}
	eprintln!("fa test: watching for changes");
	loop {
		if limit.is_some_and(|limit| runs >= limit) {
			return 0;
		}
		std::thread::sleep(interval());
		for (path, loaded, watcher) in &mut watched {
			let changes = watcher.poll();
			if changes.is_empty() {
				continue;
			}
			std::thread::sleep(Duration::from_millis(30));
			let changes = merge(changes, watcher.poll());
			match refresh(loaded, path.as_deref(), &changes, "bun") {
				Ok(reason) => {
					eprintln!("fa test: {reason}: running the tests again");
					*watcher = Watcher::new(watched_roots(loaded));
					crate::run_tests(loaded);
					runs += 1;
				}
				Err(message) => eprintln!("{message}"),
			}
		}
	}
}
