//! What a command works on (advanced brief §16.1, §20): a standalone file
//! with the files next to it, or a package (`package.fa`, `library/`,
//! `application/`, `scripts/`), found from a path or from the current folder
//! upward. A script of a package (`scripts/x.fa`) is run in its package.
//!
//! [`load`] scans the files ([`fa_codebase_parser::Codebase`]), analyzes the
//! program and evaluates the package's configuration (D37).

use std::fs;
use std::path::{Path, PathBuf};

use fa_analyzer::configuration::{self, ConfigValue, PackageConfiguration};
use fa_analyzer::diagnostics::{SourceView, render_all};
use fa_analyzer::mounts::MountedResolver;
use fa_analyzer::{CodebaseResolver, ModuleResolver, Program, Session, Severity};
use fa_codebase_parser::Codebase;

use crate::report::Report;
use crate::{dependencies, timings, workspace};

/// What a command works on.
#[derive(Clone, Debug)]
pub(crate) struct Project {
	/// A package, or a standalone file.
	pub package: bool,
	/// The folder the codebase's relative paths start from: the package
	/// folder, or the folder of the standalone file.
	pub base: PathBuf,
	/// The module the program runs: the standalone file, a script, or the
	/// application's entry (`application/main`).
	pub entry: Option<String>,
	/// How `base` is shown in messages (as given on the command line).
	pub shown: PathBuf,
}

/// The program of a project, analyzed.
pub(crate) struct Loaded {
	/// The session holding every file.
	pub session: Session,
	/// The analyzed program.
	pub program: Program,
	/// The project.
	pub project: Project,
	/// The merged configuration of the package (for the target), if any.
	pub configuration: Option<PackageConfiguration>,
	/// The warnings of the resolution of the dependencies (`DEP004`).
	pub warnings: Vec<String>,
	/// The folders of the packages of the workspace it depends on.
	pub local_dependencies: Vec<PathBuf>,
}

/// The package folder containing `start` (itself, or a parent), if any.
/// The module of a file from its path relative to the package
/// (`library/math.fa` → `library/math`): `/` separators, and only the last
/// `.fa` removed (`x.fa.fa` → `x.fa`).
pub(crate) fn module_of(relative: &Path) -> String {
	without_fa(&relative.to_string_lossy().replace('\\', "/")).to_string()
}

/// A file name or path without its `.fa` extension (removed once).
pub(crate) fn without_fa(name: &str) -> &str {
	name.strip_suffix(".fa").unwrap_or(name)
}

fn find_package(start: &Path) -> Option<PathBuf> {
	fa_codebase_parser::discover(start).package
}

/// The project of a path given on the command line (a `.fa` file or a
/// folder), or of the current folder when none is given.
pub(crate) fn locate(path: Option<&Path>) -> Result<Project, String> {
	let given = path.map(Path::to_path_buf);
	let path = match &given {
		Some(path) => path.clone(),
		None => PathBuf::from("."),
	};
	if path.is_file() || path.extension().is_some_and(|extension| extension == "fa") {
		let absolute = fs::canonicalize(&path)
			.map_err(|error| format!("fa: can't read `{}`: {error}", path.display()))?;
		// a script of a package runs in its package
		if let Some(package) = absolute.parent().and_then(find_package)
			&& let Ok(relative) = absolute.strip_prefix(&package)
			&& relative.starts_with("scripts")
		{
			let module = module_of(relative);
			let shown = shown_base(&path, relative);
			return Ok(Project {
				package: true,
				base: package,
				entry: Some(module),
				shown,
			});
		}
		let base = absolute
			.parent()
			.map_or_else(|| PathBuf::from("."), Path::to_path_buf);
		let stem = absolute
			.file_name()
			.map(|name| without_fa(&name.to_string_lossy()).to_string())
			.unwrap_or_default();
		return Ok(Project {
			package: false,
			base,
			entry: Some(stem),
			shown: path.parent().map(Path::to_path_buf).unwrap_or_default(),
		});
	}
	if !path.is_dir() {
		return Err(format!(
			"fa: `{}` is neither a .fa file nor a folder",
			path.display()
		));
	}
	let absolute = fs::canonicalize(&path)
		.map_err(|error| format!("fa: can't read `{}`: {error}", path.display()))?;
	let Some(package) = find_package(&absolute) else {
		return Err(
			"error[CLI001]: no `package.fa` or `workspace.fa` found in this folder or its parents: run a file with `fa run file.fa`, or create a package with `fa new package <name>`"
				.to_string(),
		);
	};
	let current = std::env::current_dir()
		.ok()
		.and_then(|directory| fs::canonicalize(directory).ok());
	let shown = if given.is_some() && package == absolute {
		path
	} else if current.as_ref() == Some(&package) {
		PathBuf::new()
	} else {
		package.clone()
	};
	Ok(Project {
		package: true,
		base: package,
		entry: None,
		shown,
	})
}

/// How the base folder is shown, from a path given for a file inside it.
fn shown_base(given: &Path, relative: &Path) -> PathBuf {
	let mut shown = given.to_path_buf();
	for _ in relative.components() {
		if !shown.pop() {
			return PathBuf::new();
		}
	}
	shown
}

/// The language's target of an analysis target (`native-freestanding` is
/// a variant of `native`, `Session::set_target`).
pub(crate) fn language_target(target: &str) -> &str {
	if target == "native-freestanding" {
		"native"
	} else {
		target
	}
}

/// Scans, analyzes and configures a project for a target (`bun` when
/// none is named).
pub(crate) fn load(project: Project, target: &str) -> Result<Loaded, String> {
	let loaded = load_session(project, target)?;
	timings::record_session(&loaded.session, &loaded.all_files());
	Ok(loaded)
}

/// [`load`], without recording the sizes.
fn load_session(project: Project, target: &str) -> Result<Loaded, String> {
	let mut session = Session::new();
	// the analysis is for this target (D22): `target`, compile-time arithmetic
	session.set_target(target);
	// the configuration and the dependencies of `native-freestanding` are native's
	let target = language_target(target);
	if !project.package {
		let entry = project.entry.clone().unwrap_or_default();
		let file = project.base.join(format!("{entry}.fa"));
		let (codebase, _) = timings::phase("scan", || Codebase::scan_standalone(&file))
			.map_err(|error| format!("fa: can't read `{}`: {error}", file.display()))?;
		session.set_resolver(Box::new(CodebaseResolver::new(
			codebase,
			Some(project.base.clone()),
		)));
		// the files are parsed when the analysis reaches them
		let program = timings::phase("analysis", || session.analyze_program(&entry))
			.ok_or_else(|| format!("fa: can't read `{}`", file.display()))?;
		return Ok(Loaded {
			session,
			program,
			project,
			configuration: None,
			warnings: Vec::new(),
			local_dependencies: Vec::new(),
		});
	}
	let Prepared {
		mut session,
		project,
		configuration,
		warnings,
		local_dependencies,
		..
	} = prepare_package(project, target, session)?;
	let program = match &project.entry {
		Some(script) => match timings::phase("analysis", || session.analyze_program(script)) {
			Some(program) => program,
			None => return Err(format!("fa: can't read the script `{script}.fa`")),
		},
		None => {
			let entry = without_fa(
				configuration
					.as_ref()
					.map_or("application/main.fa", |configuration| {
						configuration.entry.as_str()
					}),
			)
			.to_string();
			// every file is read and parsed first (in parallel), in the
			// order of `analyze_package`, so that the analysis phase is
			// measured alone
			let entry = timings::phase("parse", || {
				let mut modules = vec![entry.clone()];
				modules.extend(
					session
						.resolver()
						.map(|resolver| resolver.modules())
						.unwrap_or_default(),
				);
				session.load_modules(&modules);
				session.loaded_module(&entry).map(|_| entry)
			});
			timings::phase("analysis", || session.analyze_package(entry.as_deref()))
		}
	};
	Ok(Loaded {
		session,
		program,
		project,
		configuration,
		warnings,
		local_dependencies,
	})
}

/// A package ready to be analyzed: its session (the module system, the
/// target, the configuration evaluated), not analyzed yet.
pub(crate) struct Prepared {
	pub session: Session,
	pub project: Project,
	pub configuration: Option<PackageConfiguration>,
	pub warnings: Vec<String>,
	pub local_dependencies: Vec<PathBuf>,
	/// The module of the application's entry (`application/main`).
	pub entry: String,
}

/// Scans a package, resolves its dependencies and evaluates its
/// configuration for a target, in `session` (whose target is set).
pub(crate) fn prepare_package(
	project: Project,
	target: &str,
	session: Session,
) -> Result<Prepared, String> {
	prepare_package_with(project, target, session, true, |resolver| resolver)
}

/// [`prepare_package`], writing the lockfile or not, the module system
/// given to `wrap` (an editor replaces the texts of its open documents).
fn prepare_package_with(
	project: Project,
	target: &str,
	mut session: Session,
	write_lockfile: bool,
	wrap: impl FnOnce(Box<dyn ModuleResolver>) -> Box<dyn ModuleResolver>,
) -> Result<Prepared, String> {
	let target = language_target(target);
	let codebase = timings::phase("scan", || Codebase::scan_package(&project.base))
		.map_err(|error| format!("fa: can't read `{}`: {error}", project.base.display()))?;
	let main = CodebaseResolver::new(codebase, Some(project.base.clone()));
	let (resolver, warnings) = with_dependencies(main, &project.base, target, write_lockfile)?;
	// the folders of the packages of the workspace it uses (`fa develop` watches them)
	let local_dependencies = resolver
		.mounts
		.iter()
		.filter_map(|mount| mount.resolver.base.clone())
		.filter(|base| !base.starts_with(dependencies::cache_root()))
		.collect();
	session.set_resolver(wrap(Box::new(resolver)));
	// the configuration first: it names the entry (D37), and is the value of
	// the static `configuration` (merged for the target, defaults applied)
	let configuration = timings::phase("configuration", || {
		let configuration = package_configuration(&mut session, &project.base, target);
		if let Some(written) = merged_configuration(&mut session, &project.base, target) {
			let full = configuration::merge_values(&configuration::defaults(), &written);
			session.set_configuration(configuration::to_value(&full));
		}
		configuration
	});
	let entry = without_fa(
		configuration
			.as_ref()
			.map_or("application/main.fa", |configuration| {
				configuration.entry.as_str()
			}),
	)
	.to_string();
	Ok(Prepared {
		session,
		project,
		configuration,
		warnings,
		local_dependencies,
		entry,
	})
}

/// A session for an editor (the language server) over the package of a
/// folder: its files scanned, its dependencies resolved like the command
/// line does (the lockfile is read, never written), its configuration
/// evaluated for the target `bun`, its module system given to `wrap` (the
/// texts of the open documents). Also gives the module of the application's
/// entry (`application/main`) and the warnings of the resolution
/// (`DEP004`); `Err` holds the error of the resolution, as `fa check`
/// prints it.
pub fn editor_session(
	package: &Path,
	wrap: impl FnOnce(Box<dyn ModuleResolver>) -> Box<dyn ModuleResolver>,
) -> Result<EditorSession, String> {
	let project = Project {
		package: true,
		base: package.to_path_buf(),
		entry: None,
		shown: package.to_path_buf(),
	};
	let prepared = prepare_package_with(project, "bun", Session::new(), false, wrap)?;
	Ok(EditorSession {
		session: prepared.session,
		entry: prepared.entry,
		warnings: prepared.warnings,
	})
}

/// [`editor_session`] without the network: the dependencies come from the
/// lockfile and the cache only, so that it answers at once. `None` when a
/// source must be fetched (cloned, fetched again, downloaded): the editor
/// then resolves with [`editor_session`] on another thread.
pub fn offline_editor_session(
	package: &Path,
	wrap: impl FnOnce(Box<dyn ModuleResolver>) -> Box<dyn ModuleResolver>,
) -> Option<Result<EditorSession, String>> {
	let (found, fetched) = crate::offline::offline(|| editor_session(package, wrap));
	(!fetched).then_some(found)
}

/// The session of a package for an editor ([`editor_session`]).
pub struct EditorSession {
	pub session: Session,
	/// The module of the application's entry (`application/main`).
	pub entry: String,
	/// The warnings of the resolution of the dependencies (`DEP004`), as
	/// `fa check` prints them.
	pub warnings: Vec<String>,
}

/// What resolves the dependencies of a package: its workspace (the
/// lockfile at its root, the versions shared by all its packages), or the
/// package alone.
pub(crate) fn scope_of(package: &Path) -> Result<dependencies::Scope, String> {
	if let Some(root) = workspace_root(package) {
		return scope_of_workspace(&root);
	}
	let name = workspace::package_configuration(package)
		.and_then(|value| {
			value
				.field("name")
				.and_then(ConfigValue::as_str)
				.map(str::to_string)
		})
		.unwrap_or_default();
	Ok(dependencies::Scope {
		root: package.to_path_buf(),
		packages: vec![(name, package.to_path_buf())],
		workspace: false,
	})
}

/// The scope of a workspace: all its packages.
pub(crate) fn scope_of_workspace(root: &Path) -> Result<dependencies::Scope, String> {
	let workspace = workspace::load(root)?;
	Ok(dependencies::Scope {
		root: workspace.root,
		packages: workspace
			.members
			.into_iter()
			.map(|member| (member.name, member.directory))
			.collect(),
		workspace: true,
	})
}

/// The folder of the workspace a package belongs to (a `workspace.fa`
/// above it, the package being inside its `packages/`).
pub(crate) fn workspace_root(package: &Path) -> Option<PathBuf> {
	let root = fa_codebase_parser::find_workspace(package.parent()?)?;
	package.starts_with(root.join("packages")).then_some(root)
}

/// The module system of a package with its dependencies resolved (and
/// the lockfile written); the warnings of the resolution.
fn with_dependencies(
	main: CodebaseResolver,
	package: &Path,
	target: &str,
	write_lockfile: bool,
) -> Result<(MountedResolver, Vec<String>), String> {
	let scope = scope_of(package)?;
	let resolution = dependencies::resolve(&scope, &dependencies::Update::None, write_lockfile)?;
	let resolver = dependencies::mount(main, package, &resolution, target)?;
	Ok((resolver, resolution.warnings))
}

/// The configuration of the package for a target: `package.fa` merged with
/// the `workspace.fa` of its workspace, if any (§16.4).
fn package_configuration(
	session: &mut Session,
	base: &Path,
	target: &str,
) -> Option<PackageConfiguration> {
	let file = session.file_of_module("package")?;
	session.analyze_file(file);
	let package = session.source_file(file).configuration.clone()?;
	let workspace = workspace_configuration(base);
	let merged = configuration::merge(workspace.as_ref(), &package, target);
	Some(PackageConfiguration::from_value(&merged))
}

/// The written configuration merged for a target (without defaults).
fn merged_configuration(session: &mut Session, base: &Path, target: &str) -> Option<ConfigValue> {
	let file = session.file_of_module("package")?;
	let package = session.source_file(file).configuration.clone()?;
	let workspace = workspace_configuration(base);
	Some(configuration::merge(workspace.as_ref(), &package, target))
}

/// The written configuration of the workspace a package belongs to, if any.
fn workspace_configuration(package: &Path) -> Option<ConfigValue> {
	workspace::workspace_configuration(&workspace_root(package)?)
}

impl Loaded {
	/// Every file of the program, its configuration files and tests included.
	pub(crate) fn all_files(&self) -> Vec<fa_analyzer::ids::FileId> {
		let mut files: Vec<_> = self
			.program
			.configurations
			.iter()
			.chain(&self.program.files)
			.chain(&self.program.tests)
			.copied()
			.collect();
		files.sort_by(|&a, &b| self.session.module(a).cmp(self.session.module(b)));
		files.dedup();
		files
	}

	/// The path of a file as shown in messages.
	pub(crate) fn shown(&self, file: fa_analyzer::ids::FileId) -> String {
		let relative = self.session.display_path(file);
		let dependency = self
			.session
			.module(file)
			.starts_with(fa_analyzer::mounts::DEPENDENCY_PREFIX);
		if self.project.shown.as_os_str().is_empty() || dependency {
			relative
		} else {
			self.project.shown.join(relative).display().to_string()
		}
	}

	/// The diagnostics of every file.
	pub(crate) fn report(&self) -> Report {
		let mut report = Report::default();
		for warning in &self.warnings {
			report.warning(warning);
		}
		for file in self.all_files() {
			report.file(
				&self.shown(file),
				self.session.text(file),
				self.session.diagnostics(file),
			);
		}
		report
	}

	/// The diagnostics of every file, rendered.
	pub(crate) fn rendered(&self) -> String {
		self.report().text()
	}

	/// Whether a file of the program has an error.
	pub(crate) fn has_errors(&self) -> bool {
		self.all_files().iter().any(|&file| {
			self.session
				.diagnostics(file)
				.iter()
				.any(|diagnostic| diagnostic.severity == Severity::Error)
		})
	}

	/// Whether the program is one standalone file that uses no other file
	/// and exports nothing: it is generated as one JavaScript module.
	pub(crate) fn is_single_file(&self) -> bool {
		let Some(entry) = self.program.entry else {
			return false;
		};
		!self.project.package
			&& self.program.files == [entry]
			&& self.session.source_file(entry).interface.export.is_none()
	}

	/// The errors of the JavaScript backend, rendered with their files.
	pub(crate) fn render_errors(&self, diagnostics: &[fa_analyzer::Diagnostic]) -> String {
		let mut out = String::new();
		for file in self.all_files() {
			let of_file: Vec<_> = diagnostics
				.iter()
				.filter(|diagnostic| diagnostic.location.file == file)
				.cloned()
				.collect();
			if of_file.is_empty() {
				continue;
			}
			let shown = self.shown(file);
			let source = SourceView::new(&shown, self.session.text(file));
			out.push_str(&render_all(&of_file, &source));
		}
		out
	}
}

/// Writes the modules of a program into a folder (creating its subfolders).
pub(crate) fn write_modules(directory: &Path, modules: &[(String, String)]) -> std::io::Result<()> {
	for (path, code) in modules {
		let path = directory.join(path);
		if let Some(parent) = path.parent() {
			fs::create_dir_all(parent)?;
		}
		fs::write(path, code)?;
	}
	Ok(())
}

/// Writes generated modules into a folder as they come, from any thread
/// (creating each subfolder once).
pub(crate) struct DirectorySink {
	root: PathBuf,
	folders: std::sync::Mutex<std::collections::HashSet<PathBuf>>,
}

impl DirectorySink {
	pub(crate) fn new(root: &Path) -> Self {
		Self {
			root: root.to_path_buf(),
			folders: std::sync::Mutex::new(std::collections::HashSet::new()),
		}
	}
}

impl fa_codegen_js::ModuleSink for DirectorySink {
	fn write(&self, path: &str, code: &str) -> std::io::Result<()> {
		let path = self.root.join(path);
		if let Some(parent) = path.parent() {
			let created = self
				.folders
				.lock()
				.unwrap_or_else(|error| error.into_inner())
				.contains(parent);
			if !created {
				fs::create_dir_all(parent)?;
				self.folders
					.lock()
					.unwrap_or_else(|error| error.into_inner())
					.insert(parent.to_path_buf());
			}
		}
		fs::write(path, code)
	}
}

/// Moves files written into `from` to the same paths in `to` (each file is
/// replaced at once; the other files of `to` stay).
pub(crate) fn move_into(from: &Path, to: &Path, paths: &[String]) -> std::io::Result<()> {
	let mut folders = std::collections::HashSet::new();
	for path in paths {
		let target = to.join(path);
		if let Some(parent) = target.parent()
			&& folders.insert(parent.to_path_buf())
		{
			fs::create_dir_all(parent)?;
		}
		fs::rename(from.join(path), target)?;
	}
	Ok(())
}

/// `fa new package <name>` (decision D46): the package of the hello-world
/// guide, with a test. `CLI007` when the folder exists and isn't empty,
/// `CLI008` when its name isn't an identifier (the name of a package is the
/// name of a dependency, and is written into `package.fa`).
pub(crate) fn new_package(name: &str, folder: &Path) -> Result<(), String> {
	let package_name = folder
		.file_name()
		.map_or(name.to_string(), |name| name.to_string_lossy().to_string());
	if !fa_codebase_parser::codebase::is_identifier(&package_name) {
		return Err(format!(
			"error[CLI008]: `{package_name}` is not a valid package name: a name starts with a letter or `_`, followed by letters, digits or `_` (ex: `myPackage`)"
		));
	}
	if folder.exists()
		&& fs::read_dir(folder)
			.map(|mut entries| entries.next().is_some())
			.unwrap_or(true)
	{
		return Err(format!(
			"error[CLI007]: `{}` already exists and isn't empty: choose another name, or empty it",
			folder.display()
		));
	}
	let files = [
		(
			"package.fa",
			format!("export = {{\n\tname = \"{package_name}\"\n\tversion = \"0.1.0\"\n}}\n"),
		),
		("dependencies.fa", "export = {}\n".to_string()),
		(
			"library/greet.fa",
			"export function = (name: String): String => \"Hello, {name}!\"\n".to_string(),
		),
		(
			"library/greet.test.fa",
			"test \"greets by name\" {\n\tassert greet(\"Ada\") == \"Hello, Ada!\"\n}\n".to_string(),
		),
		(
			"application/main.fa",
			"export function = (arguments: Array<String>) => {\n\tlet name = arguments.get(0) ?? \"World\"\n\tconsole.log(library.greet(name))\n}\n"
				.to_string(),
		),
	];
	for (path, text) in files {
		let path = folder.join(path);
		if let Some(parent) = path.parent() {
			fs::create_dir_all(parent)
				.map_err(|error| format!("fa new: can't create `{}`: {error}", parent.display()))?;
		}
		fs::write(&path, text)
			.map_err(|error| format!("fa new: can't write `{}`: {error}", path.display()))?;
	}
	Ok(())
}

/// Whether a focused test (`test only`) must fail the run (`TEST004`): on
/// CI (the `CI` variable), or on the default branch of the Git repository
/// (decision D34: `origin/HEAD`, else `init.defaultBranch`, else `main`).
pub(crate) fn focus_forbidden(base: &Path) -> Option<String> {
	if std::env::var_os("CI").is_some_and(|value| !value.is_empty()) {
		return Some("the `CI` variable is set".to_string());
	}
	let git = |arguments: &[&str]| -> Option<String> {
		let output = std::process::Command::new("git")
			.args(arguments)
			.current_dir(base)
			.output()
			.ok()?;
		output
			.status
			.success()
			.then(|| String::from_utf8_lossy(&output.stdout).trim().to_string())
	};
	let branch = git(&["rev-parse", "--abbrev-ref", "HEAD"])?;
	let default = git(&["symbolic-ref", "--short", "refs/remotes/origin/HEAD"])
		.map(|reference| {
			reference
				.rsplit('/')
				.next()
				.unwrap_or(&reference)
				.to_string()
		})
		.or_else(|| git(&["config", "init.defaultBranch"]))
		.unwrap_or_else(|| "main".to_string());
	(branch == default).then(|| format!("`{branch}` is the default branch"))
}
