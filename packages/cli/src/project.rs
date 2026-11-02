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
use fa_analyzer::{CodebaseResolver, Program, Session, Severity};
use fa_codebase_parser::Codebase;

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
}

/// The package folder containing `start` (itself, or a parent), if any.
fn find_package(start: &Path) -> Option<PathBuf> {
	let mut current = Some(start);
	while let Some(directory) = current {
		if directory.join("package.fa").is_file() {
			return Some(directory.to_path_buf());
		}
		current = directory.parent();
	}
	None
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
			let module = relative
				.to_string_lossy()
				.replace('\\', "/")
				.trim_end_matches(".fa")
				.to_string();
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
			.map(|name| name.to_string_lossy().trim_end_matches(".fa").to_string())
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

/// Scans, analyzes and configures a project for a target (`bun` when
/// none is named).
pub(crate) fn load(project: Project, target: &str) -> Result<Loaded, String> {
	let mut session = Session::new();
	if !project.package {
		let entry = project.entry.clone().unwrap_or_default();
		let file = project.base.join(format!("{entry}.fa"));
		let (codebase, _) = Codebase::scan_standalone(&file)
			.map_err(|error| format!("fa: can't read `{}`: {error}", file.display()))?;
		session.set_resolver(Box::new(CodebaseResolver::new(
			codebase,
			Some(project.base.clone()),
		)));
		let program = session
			.analyze_program(&entry)
			.ok_or_else(|| format!("fa: can't read `{}`", file.display()))?;
		return Ok(Loaded {
			session,
			program,
			project,
			configuration: None,
		});
	}
	let codebase = Codebase::scan_package(&project.base)
		.map_err(|error| format!("fa: can't read `{}`: {error}", project.base.display()))?;
	session.set_resolver(Box::new(CodebaseResolver::new(
		codebase,
		Some(project.base.clone()),
	)));
	// the configuration first: it names the entry (D37)
	let configuration = package_configuration(&mut session, &project.base, target);
	let program = match &project.entry {
		Some(script) => match session.analyze_program(script) {
			Some(program) => program,
			None => return Err(format!("fa: can't read the script `{script}.fa`")),
		},
		None => {
			let entry = configuration
				.as_ref()
				.map_or("application/main.fa", |configuration| {
					configuration.entry.as_str()
				})
				.trim_end_matches(".fa")
				.to_string();
			let entry = session.file_of_module(&entry).map(|_| entry);
			session.analyze_package(entry.as_deref())
		}
	};
	Ok(Loaded {
		session,
		program,
		project,
		configuration,
	})
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

/// The written configuration of the workspace a package belongs to (a
/// `workspace.fa` above it, the package being inside its `packages/`).
fn workspace_configuration(package: &Path) -> Option<ConfigValue> {
	let mut current = package.parent();
	while let Some(directory) = current {
		let path = directory.join("workspace.fa");
		if path.is_file() {
			if !package.starts_with(directory.join("packages")) {
				return None;
			}
			let text = fs::read_to_string(path).ok()?;
			return configuration::evaluate_source(
				&text,
				configuration::ConfigurationKind::Workspace,
			)
			.0;
		}
		current = directory.parent();
	}
	None
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
		if self.project.shown.as_os_str().is_empty() {
			relative
		} else {
			self.project.shown.join(relative).display().to_string()
		}
	}

	/// The diagnostics of every file, rendered.
	pub(crate) fn rendered(&self) -> String {
		let mut out = String::new();
		for file in self.all_files() {
			let shown = self.shown(file);
			let source = SourceView::new(&shown, self.session.text(file));
			out.push_str(&render_all(self.session.diagnostics(file), &source));
		}
		out
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

/// `fa new package <name>` (decision D46): the package of the hello-world
/// guide, with a test. `CLI007` when the folder exists and isn't empty.
pub(crate) fn new_package(name: &str, folder: &Path) -> Result<(), String> {
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
	let package_name = folder
		.file_name()
		.map_or(name.to_string(), |name| name.to_string_lossy().to_string());
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
			"export function = (arguments: Array(String)) => {\n\tlet name = arguments.get(0) ?? \"World\"\n\tconsole.log(library.greet(name))\n}\n"
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
