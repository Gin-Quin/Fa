//! A package and its dependencies as one module system (advanced brief §17):
//! [`MountedResolver`] answers the lookups of the package's own files with
//! its [`CodebaseResolver`], then with its dependencies, and mounts the code
//! of each dependency (the `library/` of a sibling package or of a Git
//! repository, a remote file, a remote folder) under a module prefix of its
//! own, `$dependencies/<id>/…`, that no file of a package can have.
//!
//! Each mount has its own dependencies: the dependencies of a dependency are
//! visible in its files, never in the package's.

use std::collections::BTreeMap;
use std::sync::Arc;

use crate::modules::{CodebaseResolver, FileRole, ModuleEntry, ModuleResolver};

/// The first segment of the module paths of the dependencies.
pub const DEPENDENCY_PREFIX: &str = "$dependencies/";

/// The code of a dependency, mounted under a module prefix.
pub struct Mount {
	/// The module prefix of its files (`$dependencies/shared`).
	pub prefix: String,
	/// How the folder of its files is shown in messages (`../shared`,
	/// `github:farmers/farm@4.2.1`).
	pub shown: String,
	/// Its files.
	pub resolver: CodebaseResolver,
	/// Its own dependencies: key → entry (module paths with their prefix).
	pub dependencies: BTreeMap<String, ModuleEntry>,
}

/// A package (or a standalone program) and the code of its dependencies.
pub struct MountedResolver {
	/// The package's own files.
	pub main: CodebaseResolver,
	/// The dependencies of the package: key → entry (module paths with
	/// their prefix, or a folder of the package itself).
	pub dependencies: BTreeMap<String, ModuleEntry>,
	/// The code of every dependency, direct or not.
	pub mounts: Vec<Mount>,
}

impl MountedResolver {
	/// The mount of a module or folder path, and the path inside it.
	fn split<'p>(&self, path: &'p str) -> Option<(&Mount, &'p str)> {
		if !path.starts_with(DEPENDENCY_PREFIX) {
			return None;
		}
		self.mounts.iter().find_map(|mount| {
			let rest = path.strip_prefix(mount.prefix.as_str())?;
			if rest.is_empty() {
				Some((mount, ""))
			} else {
				rest.strip_prefix('/').map(|inner| (mount, inner))
			}
		})
	}

	/// An entry of a mount, with its prefix.
	fn prefixed(mount: &Mount, entry: ModuleEntry) -> ModuleEntry {
		let join = |path: String| {
			if path.is_empty() {
				mount.prefix.clone()
			} else {
				format!("{}/{path}", mount.prefix)
			}
		};
		match entry {
			ModuleEntry::File(module) => ModuleEntry::File(join(module)),
			ModuleEntry::Folder(folder) => ModuleEntry::Folder(join(folder)),
		}
	}
}

impl ModuleResolver for MountedResolver {
	fn role(&self, module: &str) -> FileRole {
		match self.split(module) {
			Some((mount, inner)) => match mount.resolver.role(inner) {
				// a dependency's files are modules, whatever their layout
				FileRole::Program => FileRole::Module,
				role => role,
			},
			None => self.main.role(module),
		}
	}

	fn name(&self, module: &str) -> Option<String> {
		match self.split(module) {
			Some((mount, inner)) => mount.resolver.name(inner),
			None => self.main.name(module),
		}
	}

	fn lookup(&self, from: &str, name: &str) -> Option<ModuleEntry> {
		match self.split(from) {
			Some((mount, inner)) => match mount.resolver.lookup(inner, name) {
				Some(entry) => Some(Self::prefixed(mount, entry)),
				None => mount.dependencies.get(name).cloned(),
			},
			None => self
				.main
				.lookup(from, name)
				.or_else(|| self.dependencies.get(name).cloned()),
		}
	}

	fn member(&self, folder: &str, name: &str) -> Option<ModuleEntry> {
		match self.split(folder) {
			Some((mount, inner)) => mount
				.resolver
				.member(inner, name)
				.map(|entry| Self::prefixed(mount, entry)),
			None => self.main.member(folder, name),
		}
	}

	fn source(&self, module: &str) -> Option<Arc<str>> {
		match self.split(module) {
			Some((mount, inner)) => mount.resolver.source(inner),
			None => self.main.source(module),
		}
	}

	fn path(&self, module: &str) -> String {
		match self.split(module) {
			Some((mount, inner)) => format!("{}/{}", mount.shown, mount.resolver.path(inner)),
			None => self.main.path(module),
		}
	}

	fn file(&self, module: &str) -> Option<std::path::PathBuf> {
		match self.split(module) {
			Some((mount, inner)) => mount.resolver.file(inner),
			None => self.main.file(module),
		}
	}

	fn modules(&self) -> Vec<String> {
		self.main.modules()
	}

	fn modules_in(&self, folder: &str) -> Vec<String> {
		let Some((mount, inner)) = self.split(folder) else {
			return self.main.modules_in(folder);
		};
		let modules = if inner.is_empty() {
			mount.resolver.modules()
		} else {
			mount.resolver.modules_in(inner)
		};
		modules
			.into_iter()
			.map(|module| format!("{}/{module}", mount.prefix))
			.collect()
	}

	fn configurations(&self) -> Vec<String> {
		self.main.configurations()
	}

	fn scan_diagnostics(&self) -> Vec<(String, &'static str, String)> {
		self.main.scan_diagnostics()
	}

	fn is_test_name(&self, from: &str, name: &str) -> bool {
		match self.split(from) {
			Some((mount, inner)) => mount.resolver.is_test_name(inner, name),
			None => self.main.is_test_name(from, name),
		}
	}

	fn visible_names(&self, from: &str) -> Vec<String> {
		let (mut names, dependencies) = match self.split(from) {
			Some((mount, inner)) => (mount.resolver.visible_names(inner), &mount.dependencies),
			None => (self.main.visible_names(from), &self.dependencies),
		};
		names.extend(dependencies.keys().cloned());
		names.sort();
		names.dedup();
		names
	}

	fn read_input(&self, path: &str) -> Result<String, crate::comptime::inputs::InputError> {
		self.main.read_input(path)
	}

	fn list_input(
		&self,
		path: &str,
	) -> Result<Vec<crate::comptime::inputs::DirectoryEntry>, crate::comptime::inputs::InputError>
	{
		self.main.list_input(path)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::Session;

	fn files(list: &[(&str, &str)]) -> Vec<(String, String)> {
		list.iter()
			.map(|(path, text)| (path.to_string(), text.to_string()))
			.collect()
	}

	/// A package using a dependency `shared` whose own dependency `farm` it
	/// can't see.
	fn resolver(main: &str) -> MountedResolver {
		let farm = Mount {
			prefix: "$dependencies/farm@4.2.1".to_string(),
			shown: "github:farmers/farm@4.2.1".to_string(),
			resolver: CodebaseResolver::from_texts(
				&files(&[("library/cows.fa", "export function = (): Integer => 3\n")]),
				None,
			),
			dependencies: BTreeMap::new(),
		};
		let shared = Mount {
			prefix: "$dependencies/shared".to_string(),
			shown: "../shared".to_string(),
			resolver: CodebaseResolver::from_texts(
				&files(&[
					(
						"library/models.fa",
						"export type = { name: String, cows: Integer }\n",
					),
					(
						"library/make.fa",
						"export function = (name: String): models => models { name = name, cows = farm.cows() }\n",
					),
				]),
				None,
			),
			dependencies: BTreeMap::from([(
				"farm".to_string(),
				ModuleEntry::Folder("$dependencies/farm@4.2.1/library".to_string()),
			)]),
		};
		MountedResolver {
			main: CodebaseResolver::from_texts(
				&files(&[
					("package.fa", "export = { name = \"api\" }\n"),
					("application/main.fa", main),
				]),
				None,
			),
			dependencies: BTreeMap::from([(
				"shared".to_string(),
				ModuleEntry::Folder("$dependencies/shared/library".to_string()),
			)]),
			mounts: vec![farm, shared],
		}
	}

	fn errors(main: &str) -> Vec<String> {
		let mut session = Session::new();
		session.set_resolver(Box::new(resolver(main)));
		let program = session.analyze_package(Some("application/main"));
		let mut found = Vec::new();
		for &file in program.files.iter().chain(&program.tests) {
			for diagnostic in session.diagnostics(file) {
				// the unused-value warning is tested by its own cases
				if diagnostic.code.id == "W-DECL-2" {
					continue;
				}
				found.push(format!(
					"{}: {} {}",
					session.display_path(file),
					diagnostic.code.id,
					diagnostic.message
				));
			}
		}
		found
	}

	#[test]
	fn a_dependency_is_a_namespace_and_its_dependencies_are_not_visible() {
		assert_eq!(
			errors(
				"export function = () => {\n\tlet user: shared.models = shared.make(\"Ada\")\n\tconsole.log(user.name, user.cows)\n}\n"
			),
			Vec::<String>::new()
		);
		let found = errors("export function = () => {\n\tconsole.log(farm.cows())\n}\n");
		assert!(
			found
				.iter()
				.any(|error| error.starts_with("application/main.fa")),
			"{found:?}"
		);
	}

	#[test]
	fn the_files_of_a_dependency_are_shown_with_its_source() {
		let resolver = resolver("export function = () => {}\n");
		assert_eq!(
			resolver.path("$dependencies/shared/library/models"),
			"../shared/library/models.fa"
		);
		assert_eq!(
			resolver.role("$dependencies/shared/library/models"),
			FileRole::Module
		);
		assert_eq!(
			resolver.lookup("$dependencies/shared/library/make", "farm"),
			Some(ModuleEntry::Folder(
				"$dependencies/farm@4.2.1/library".to_string()
			))
		);
		assert_eq!(resolver.lookup("application/main", "farm"), None);
	}
}
