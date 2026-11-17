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

	/// Watches other folders (after the package was loaded again), keeping
	/// the snapshot of the last poll for the files watched before: a file
	/// changed, added or removed while the package was loading is reported
	/// by the next poll instead of being absorbed by a new snapshot. The
	/// files of the folders watched only now are taken as they are.
	pub(crate) fn watch(&mut self, roots: Vec<(PathBuf, bool)>) {
		let fresh = Watcher {
			roots,
			snapshot: BTreeMap::new(),
		};
		let now = fresh.scan();
		let mut snapshot: BTreeMap<PathBuf, Stamp> = std::mem::take(&mut self.snapshot)
			.into_iter()
			.filter(|(path, _)| fresh.covers(path))
			.collect();
		for (path, stamp) in now {
			if !self.covers(&path) {
				snapshot.insert(path, stamp);
			}
		}
		*self = Watcher {
			roots: fresh.roots,
			snapshot,
		};
	}

	/// Whether a file is in one of the watched folders.
	fn covers(&self, path: &Path) -> bool {
		self.roots.iter().any(|(root, recursive)| {
			if *recursive {
				path.starts_with(root)
			} else {
				path.parent() == Some(root.as_path())
			}
		})
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

/// Loads a project for a command that watches it, with its watcher: the
/// files of the project are listed before the load, so that a change made
/// while it loads is reported by the first poll.
fn load_watched(path: Option<&Path>, target: &str) -> Result<(Loaded, Watcher), String> {
	let before = crate::project::locate(path).map_or_else(
		|_| Vec::new(),
		|project| vec![(project.base, project.package)],
	);
	let mut watcher = Watcher::new(before);
	let loaded = Loaded::load(path, target)?;
	watcher.watch(watched_roots(&loaded));
	Ok((loaded, watcher))
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
		let module = crate::project::module_of(relative);
		let text = match fs::read_to_string(path) {
			Ok(text) => text,
			Err(error) if error.kind() == std::io::ErrorKind::InvalidData => {
				eprintln!(
					"fa: `{}` is not UTF-8 text: save it as UTF-8",
					relative.display()
				);
				continue;
			}
			Err(error) => {
				eprintln!("fa: can't read `{}`: {error}", relative.display());
				continue;
			}
		};
		loaded.inner.session.update(&module, text.into());
		shown.push(relative.to_string_lossy().replace('\\', "/"));
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
			let module = crate::project::module_of(relative);
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
			command.env("FA_HOT_CHANNEL", channel.variable());
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
		let (loaded, watcher) = match load_watched(path.as_deref(), target.name()) {
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
						let message = format!("fa develop: {}{reason}: {what}", package.prefix());
						eprintln!("{message}");
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
						let message =
							format!("fa develop: {}{reason}: {verb} ({why})", package.prefix());
						eprintln!("{message}");
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
					let message = format!("fa develop: {}{reason}: {verb}", package.prefix());
					eprintln!("{message}");
					// the changes made while it loaded are seen by the next poll
					package.watcher.watch(watched_roots(&package.loaded));
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

/// The changes of two polls together: a file removed then added again
/// (an editor saving through a new file) is modified, a file added then
/// removed is no change.
fn merge(mut first: Changes, second: Changes) -> Changes {
	for path in second.added {
		if let Some(index) = first.removed.iter().position(|other| *other == path) {
			first.removed.remove(index);
			if !first.modified.contains(&path) {
				first.modified.push(path);
			}
		} else if !first.added.contains(&path) {
			first.added.push(path);
		}
	}
	for path in second.removed {
		if let Some(index) = first.added.iter().position(|other| *other == path) {
			first.added.remove(index);
			continue;
		}
		first.modified.retain(|other| *other != path);
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
pub(crate) fn watch_tests(
	selection: Selection,
	target: Option<fa_codegen_llvm::Target>,
	asynchronous: Option<bool>,
) -> i32 {
	let target_name = target.map_or("bun", |target| target.analysis_target());
	let run = |loaded: &mut Loaded| {
		loaded.configure_asynchronous(asynchronous, true);
		match target {
			Some(target) => crate::testing::run_llvm_tests(
				crate::native::backend().expect("the LLVM test command was delegated"),
				loaded,
				target,
			),
			None => crate::run_tests(loaded),
		}
	};
	let paths = paths(selection);
	if paths.is_empty() {
		eprintln!("fa test: no package of the workspace has test files (`*.test.fa`)");
		return 0;
	}
	let limit = run_limit();
	let mut runs = 0;
	let mut watched: Vec<(Option<PathBuf>, Loaded, Watcher)> = Vec::new();
	for (_, path) in paths {
		let (loaded, watcher) = match load_watched(path.as_deref(), target_name) {
			Ok(loaded) => loaded,
			Err(message) => {
				crate::color::eprintln_message(&message);
				return 1;
			}
		};
		watched.push((path, loaded, watcher));
	}
	for (_, loaded, _) in &mut watched {
		run(loaded);
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
			match refresh(loaded, path.as_deref(), &changes, target_name) {
				Ok(reason) => {
					eprintln!("fa test: {reason}: running the tests again");
					// the changes made while it loaded are seen by the next poll
					watcher.watch(watched_roots(loaded));
					run(loaded);
					runs += 1;
				}
				Err(message) => eprintln!("{message}"),
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	/// An empty scratch folder of a test.
	fn scratch(name: &str) -> PathBuf {
		let directory =
			std::env::temp_dir().join(format!("fa-develop-{name}-{}", std::process::id()));
		let _ = fs::remove_dir_all(&directory);
		fs::create_dir_all(&directory).unwrap();
		fs::canonicalize(&directory).unwrap()
	}

	/// Writes files into a folder.
	fn write(folder: &Path, files: &[(&str, &str)]) {
		for (path, text) in files {
			let path = folder.join(path);
			fs::create_dir_all(path.parent().unwrap()).unwrap();
			fs::write(path, text).unwrap();
		}
	}

	fn names(paths: &[PathBuf], base: &Path) -> Vec<String> {
		paths
			.iter()
			.map(|path| path.strip_prefix(base).unwrap().display().to_string())
			.collect()
	}

	#[test]
	fn the_watcher_reports_the_files_added_removed_and_modified() {
		let base = scratch("poll");
		write(
			&base,
			&[
				("package.fa", "export = {}\n"),
				("library/a.fa", "export = 1\n"),
				("library/b.fa", "export = 2\n"),
				("library/notes.txt", "not watched\n"),
				(".hidden/c.fa", "export = 3\n"),
				("node_modules/d.fa", "export = 4\n"),
			],
		);
		let mut watcher = Watcher::new(vec![(base.clone(), true)]);
		assert!(watcher.poll().is_empty());
		write(
			&base,
			&[
				("library/a.fa", "export = 100\n"),
				("library/new.fa", "export = 5\n"),
				("library/notes.txt", "still not watched\n"),
				(".hidden/c.fa", "export = 30\n"),
				("node_modules/d.fa", "export = 40\n"),
			],
		);
		fs::remove_file(base.join("library/b.fa")).unwrap();
		let changes = watcher.poll();
		assert_eq!(names(&changes.added, &base), ["library/new.fa"]);
		assert_eq!(names(&changes.removed, &base), ["library/b.fa"]);
		assert_eq!(names(&changes.modified, &base), ["library/a.fa"]);
		assert!(watcher.poll().is_empty());
		// a folder watched without its subfolders
		let mut files = Watcher::new(vec![(base.clone(), false)]);
		write(&base, &[("library/a.fa", "export = 1000\n")]);
		assert!(files.poll().is_empty());
		write(&base, &[("dependencies.lock", "changed\n")]);
		assert_eq!(names(&files.poll().added, &base), ["dependencies.lock"]);
		let _ = fs::remove_dir_all(base);
	}

	/// Item 4 of the tooling review: the changes made while the package
	/// loads again are reported by the next poll, not absorbed by a new
	/// snapshot.
	#[test]
	fn the_changes_made_while_loading_are_seen_by_the_next_poll() {
		let base = scratch("during-load");
		let other = scratch("during-load-other");
		let gone = scratch("during-load-gone");
		write(
			&base,
			&[
				("library/a.fa", "export = 1\n"),
				("library/b.fa", "export = 2\n"),
			],
		);
		write(&other, &[("library/x.fa", "export = 1\n")]);
		write(&gone, &[("library/y.fa", "export = 1\n")]);
		let mut watcher = Watcher::new(vec![(base.clone(), true), (gone.clone(), true)]);
		write(&base, &[("library/zz.fa", "export = 1\n")]);
		let changes = watcher.poll();
		assert_eq!(names(&changes.added, &base), ["library/zz.fa"]);
		// the package loads again; meanwhile, an edit, a new file, a
		// removal
		write(
			&base,
			&[
				("library/a.fa", "export = 100\n"),
				("library/c.fa", "export = 3\n"),
			],
		);
		fs::remove_file(base.join("library/b.fa")).unwrap();
		// loaded: it now uses `other`, and no longer `gone`
		watcher.watch(vec![(base.clone(), true), (other.clone(), true)]);
		let changes = watcher.poll();
		assert_eq!(names(&changes.modified, &base), ["library/a.fa"]);
		assert_eq!(names(&changes.added, &base), ["library/c.fa"]);
		assert_eq!(names(&changes.removed, &base), ["library/b.fa"]);
		assert!(watcher.poll().is_empty());
		// the files of a folder watched from now on are taken as they are
		write(&other, &[("library/x.fa", "export = 100\n")]);
		assert_eq!(names(&watcher.poll().modified, &other), ["library/x.fa"]);
		for directory in [base, other, gone] {
			let _ = fs::remove_dir_all(directory);
		}
	}

	#[test]
	fn merged_polls_see_a_file_saved_through_a_new_file_as_modified() {
		let path = |name: &str| PathBuf::from(format!("library/{name}.fa"));
		// vim: the file removed, then written again
		let first = Changes {
			removed: vec![path("a")],
			..Changes::default()
		};
		let second = Changes {
			added: vec![path("a")],
			..Changes::default()
		};
		let merged = merge(first, second);
		assert!(merged.added.is_empty() && merged.removed.is_empty());
		assert_eq!(merged.modified, [path("a")]);
		// a file added then removed: no change
		let first = Changes {
			added: vec![path("b")],
			modified: vec![path("c")],
			..Changes::default()
		};
		let second = Changes {
			removed: vec![path("b"), path("c")],
			modified: vec![path("d")],
			..Changes::default()
		};
		let merged = merge(first, second);
		assert!(merged.added.is_empty());
		assert_eq!(merged.removed, [path("c")]);
		assert_eq!(merged.modified, [path("d")]);
		// an added file modified again is still added
		let first = Changes {
			added: vec![path("e")],
			..Changes::default()
		};
		let second = Changes {
			modified: vec![path("e")],
			..Changes::default()
		};
		let merged = merge(first, second);
		assert_eq!(merged.added, [path("e")]);
		assert!(merged.modified.is_empty());
	}

	#[test]
	fn only_the_edits_of_the_own_sources_are_applied_without_a_reload() {
		let workspace = scratch("reload-reason");
		write(
			&workspace,
			&[
				("workspace.fa", "export = { name = \"farm\" }\n"),
				("packages/farm/package.fa", "export = { name = \"farm\" }\n"),
				(
					"packages/farm/library/greet.fa",
					"export function = (): String => \"hi\"\n",
				),
				(
					"packages/farm/application/main.fa",
					"export function = () => {\n\tconsole.log(library.greet())\n}\n",
				),
			],
		);
		let package = workspace.join("packages/farm");
		let (loaded, mut watcher) = load_watched(Some(&package), "bun").expect("it loads");
		assert!(!loaded.has_errors(), "{}", loaded.rendered());
		assert!(watcher.poll().is_empty());
		let modified = |path: PathBuf| Changes {
			modified: vec![path],
			..Changes::default()
		};
		let reason = |changes: Changes| reload_reason(&loaded, &changes);
		assert_eq!(reason(modified(package.join("library/greet.fa"))), None);
		assert_eq!(
			reason(Changes {
				added: vec![package.join("library/new.fa")],
				..Changes::default()
			}),
			Some("`library/new.fa` was added".to_string())
		);
		assert_eq!(
			reason(Changes {
				removed: vec![package.join("library/greet.fa")],
				..Changes::default()
			}),
			Some("`library/greet.fa` was removed".to_string())
		);
		assert_eq!(
			reason(modified(package.join("package.fa"))),
			Some("the configuration `package.fa` changed".to_string())
		);
		assert_eq!(
			reason(modified(package.join("dependencies.lock"))),
			Some("the configuration `dependencies.lock` changed".to_string())
		);
		assert!(
			reason(modified(workspace.join("workspace.fa")))
				.is_some_and(|reason| reason.starts_with("the configuration `")),
		);
		assert!(
			reason(modified(workspace.join("packages/other/library/x.fa")))
				.is_some_and(|reason| reason.ends_with("x.fa` changed")),
		);
		// the root files of the workspace are watched, not its other folders
		write(
			&workspace,
			&[
				("workspace.fa", "export = { name = \"farms\" }\n"),
				("scratch/notes.fa", "export = 1\n"),
			],
		);
		let changes = watcher.poll();
		assert_eq!(changes.modified, [workspace.join("workspace.fa")]);
		assert!(changes.added.is_empty());
		let _ = fs::remove_dir_all(workspace);
	}

	#[test]
	fn an_unreadable_edit_is_reported_and_skipped() {
		let package = scratch("unreadable");
		write(
			&package,
			&[
				("package.fa", "export = { name = \"farm\" }\n"),
				(
					"library/greet.fa",
					"export function = (): String => \"hi\"\n",
				),
				(
					"application/main.fa",
					"export function = () => {\n\tconsole.log(library.greet())\n}\n",
				),
			],
		);
		let (mut loaded, _) = load_watched(Some(&package), "bun").expect("it loads");
		let greet = package.join("library/greet.fa");
		fs::write(&greet, b"export = \"\xff\"\n").unwrap();
		assert!(apply_edits(&mut loaded, &[greet]).is_empty());
		let main = package.join("application/main.fa");
		assert_eq!(apply_edits(&mut loaded, &[main]), ["application/main.fa"]);
		let _ = fs::remove_dir_all(package);
	}
}
