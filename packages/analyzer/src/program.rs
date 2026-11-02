//! The analysis of a whole program (advanced brief §15, §21): the files
//! analyzed, the rounds that settle cycles between files, the module-level
//! graph (initialization order, `MOD007`), the entry function (`MOD015`),
//! and the incremental [`Session::update`].

use std::sync::Arc;

use rustc_hash::{FxHashMap, FxHashSet};

use crate::analysis::Callee;
use crate::diagnostics::codes::advanced::{MOD001, MOD002, MOD003, MOD007, MOD015};
use crate::diagnostics::{Diagnostic, Location, SourceSpan};
use crate::ids::{FileId, NodeId, TypeId};
use crate::modules::{FileRole, Program};
use crate::session::Session;
use crate::symbols::{Resolution, ScopeKind, SymbolKind};
use crate::syntax;
use crate::types::{CollKind, TypeKind};

/// The most rounds the program analysis runs to settle cycles between files.
const MAX_ROUNDS: usize = 4;

/// A module-level declaration: a file and its declaration node.
type Decl = (FileId, NodeId);

impl Session {
	/// Analyzes a program run from one file: a standalone file or a script
	/// (`module`), and every file it uses. Returns `None` when the module
	/// can't be read.
	pub fn analyze_program(&mut self, module: &str) -> Option<Program> {
		let entry = self.file_of_module(module)?;
		self.settle(&[entry]);
		let files = self.analyzed_files();
		self.add_scan_diagnostics(&files, false);
		let mut program = self.assemble(files, Some(entry), Vec::new());
		self.check_entry_function(entry, false);
		program.entry = Some(entry);
		Some(program)
	}

	/// Analyzes every file of a package (decision D42): its library, its
	/// application, its scripts and its test files. `entry` is the module of
	/// the application's entry (`application/main`), checked when it exists.
	pub fn analyze_package(&mut self, entry: Option<&str>) -> Program {
		let modules = self
			.resolver()
			.map(|resolver| resolver.modules())
			.unwrap_or_default();
		let mut roots = Vec::new();
		let mut entry_file = None;
		if let Some(entry) = entry
			&& let Some(file) = self.file_of_module(entry)
		{
			roots.push(file);
			entry_file = Some(file);
		}
		for module in &modules {
			if let Some(file) = self.file_of_module(module)
				&& !roots.contains(&file)
			{
				roots.push(file);
			}
		}
		self.settle(&roots);
		let files = self.analyzed_files();
		self.add_scan_diagnostics(&files, true);
		let (tests, files): (Vec<FileId>, Vec<FileId>) = files
			.into_iter()
			.partition(|&file| self.role(file) == FileRole::Test);
		let mut program = self.assemble(files, entry_file, tests);
		if let Some(entry) = entry_file {
			self.check_entry_function(entry, true);
		}
		program.entry = entry_file;
		// the configuration files are evaluated (D37)
		let configurations = self
			.resolver()
			.map(|resolver| resolver.configurations())
			.unwrap_or_default();
		for module in configurations {
			if let Some(file) = self.file_of_module(&module) {
				self.analyze_file(file);
				program.configurations.push(file);
			}
		}
		program
	}

	/// Analyzes the roots, then walks again every analyzed file as long as
	/// one of them read a declaration of a file whose analysis was in
	/// progress (a cycle between files): each round reads the interfaces the
	/// previous one published.
	fn settle(&mut self, roots: &[FileId]) {
		for &root in roots {
			self.analyze_file(root);
		}
		for _ in 1..MAX_ROUNDS {
			let files = self.analyzed_files();
			if !files.iter().any(|&file| self.source_file(file).deferred) {
				break;
			}
			for &file in &files {
				self.reset_analysis(file);
			}
			for &root in roots {
				self.analyze_file(root);
			}
			for &file in &files {
				self.analyze_file(file);
			}
		}
	}

	/// The current files whose analysis ran, by path.
	fn analyzed_files(&self) -> Vec<FileId> {
		let mut files: Vec<FileId> = (0..self.file_count())
			.map(FileId::new)
			.filter(|&file| {
				self.source_file(file).analyzed
					&& self.loaded_module(self.module(file)) == Some(file)
					&& !matches!(self.role(file), FileRole::Configuration(_))
			})
			.collect();
		files.sort_by(|&a, &b| self.module(a).cmp(self.module(b)));
		files
	}

	/// The problems of the scan (`MOD001`–`MOD003`) on the files they
	/// concern: all of them in a package, only the analyzed files' for a
	/// standalone program.
	fn add_scan_diagnostics(&mut self, files: &[FileId], all: bool) {
		let Some(resolver) = self.resolver() else {
			return;
		};
		let found = resolver.scan_diagnostics();
		for (module, code, message) in found {
			// a program's own name is never used: only its siblings' matter
			if !all
				&& self
					.loaded_module(&module)
					.is_some_and(|file| self.role(file) == FileRole::Program)
			{
				continue;
			}
			let file = match self.loaded_module(&module) {
				Some(file) if files.contains(&file) => file,
				_ if all => match self.file_of_module(&module) {
					Some(file) => file,
					None => continue,
				},
				_ => continue,
			};
			let code = match code {
				"MOD001" => &MOD001,
				"MOD002" => &MOD002,
				_ => &MOD003,
			};
			let diagnostic = Diagnostic::new(
				code,
				Location {
					file,
					span: SourceSpan::new(0, 0),
				},
				message,
			);
			self.push_diagnostic(file, diagnostic);
		}
	}

	/// The program: its files in dependency order, the initialization order
	/// of its module-level values (`MOD007` on a cycle).
	fn assemble(
		&mut self,
		files: Vec<FileId>,
		entry: Option<FileId>,
		tests: Vec<FileId>,
	) -> Program {
		let files = self.dependency_order(&files);
		let init_order = self.initialization_order(&files, entry, &tests);
		Program {
			files,
			entry,
			init_order,
			tests,
			configurations: Vec::new(),
		}
	}

	/// The files a file's resolutions designate.
	fn file_dependencies(&self, file: FileId) -> Vec<FileId> {
		let mut found: Vec<FileId> = self
			.analysis(file)
			.resolutions
			.iter()
			.filter_map(|&resolution| match resolution {
				Resolution::External(module, _) => {
					self.loaded_module(self.types.module_path(module))
				}
				_ => None,
			})
			.collect();
		found.sort_by(|&a, &b| self.module(a).cmp(self.module(b)));
		found.dedup();
		found
	}

	/// The files, each after the files it uses (a cycle in path order).
	fn dependency_order(&self, files: &[FileId]) -> Vec<FileId> {
		fn visit(
			session: &Session,
			file: FileId,
			files: &[FileId],
			seen: &mut FxHashSet<FileId>,
			out: &mut Vec<FileId>,
		) {
			if !seen.insert(file) {
				return;
			}
			for dependency in session.file_dependencies(file) {
				if files.contains(&dependency) {
					visit(session, dependency, files, seen, out);
				}
			}
			out.push(file);
		}
		let mut seen = FxHashSet::default();
		let mut out = Vec::new();
		for &file in files {
			visit(self, file, files, &mut seen, &mut out);
		}
		out
	}

	/// The declaration a resolution of `file` designates, when it is a
	/// module-level declaration (or a namespace member): `(file, decl)`.
	fn module_decl(&self, file: FileId, resolution: Resolution) -> Option<Decl> {
		match resolution {
			Resolution::Local(symbol) => {
				let symbols = &self.analysis(file).symbols;
				let data = symbols.symbol(symbol);
				let scope = symbols.scope(data.scope);
				matches!(scope.kind, ScopeKind::Module | ScopeKind::Namespace)
					.then_some((file, data.decl_node))
			}
			Resolution::External(module, decl) => {
				Some((self.loaded_module(self.types.module_path(module))?, decl))
			}
			_ => None,
		}
	}

	/// The kind of a module-level declaration.
	fn decl_kind(&self, (file, decl): Decl) -> Option<SymbolKind> {
		let symbols = &self.analysis(file).symbols;
		symbols
			.symbol_of_decl(decl)
			.map(|symbol| symbols.symbol(symbol).kind)
	}

	/// The module-level declarations a subtree uses: names resolved to
	/// them, and the functions it calls directly. Lambdas are included: a
	/// function value may be called.
	fn used_decls(&self, file: FileId, root: NodeId) -> Vec<Decl> {
		let tree = self.tree(file);
		let analysis = self.analysis(file);
		let mut found = Vec::new();
		let mut stack = vec![root];
		while let Some(node) = stack.pop() {
			if node.index() >= tree.nodes.len() {
				continue;
			}
			if let Some(decl) = self.module_decl(file, analysis.resolution(node)) {
				found.push(decl);
			}
			if let Some(call) = analysis.calls.get(&node)
				&& let Callee::Function(function) = call.callee
			{
				found.push((file, function));
			}
			stack.extend(syntax::children(tree, node));
		}
		found.sort();
		found.dedup();
		found
	}

	/// Whether a declaration is a module-level value (initialized at runtime).
	fn is_value(&self, decl: Decl) -> bool {
		matches!(
			self.decl_kind(decl),
			Some(SymbolKind::Let | SymbolKind::Mutable)
		) && self.role(decl.0).is_module()
	}

	/// The module-level values a declaration reads, directly or through the
	/// functions it calls.
	fn value_dependencies(&self, decl: Decl, cache: &mut FxHashMap<Decl, Vec<Decl>>) -> Vec<Decl> {
		let mut values = Vec::new();
		let mut seen = FxHashSet::default();
		let mut stack = vec![decl];
		seen.insert(decl);
		while let Some(current) = stack.pop() {
			let used = cache
				.entry(current)
				.or_insert_with(|| self.used_decls(current.0, current.1))
				.clone();
			for next in used {
				if next == current || !seen.insert(next) {
					if next == decl && current != decl && self.is_value(decl) {
						values.push(decl);
					}
					continue;
				}
				if self.is_value(next) {
					values.push(next);
				} else if matches!(self.decl_kind(next), Some(SymbolKind::Function)) {
					stack.push(next);
				}
			}
		}
		values.sort();
		values.dedup();
		values
	}

	/// The initialization order of the module-level values the program
	/// reaches (advanced brief §15.7), reporting `MOD007` on cycles.
	fn initialization_order(
		&mut self,
		files: &[FileId],
		entry: Option<FileId>,
		tests: &[FileId],
	) -> Vec<Decl> {
		let mut cache: FxHashMap<Decl, Vec<Decl>> = FxHashMap::default();
		// the roots: the entry file and the test files (or every file of a library)
		let roots: Vec<FileId> = match entry {
			Some(entry) => std::iter::once(entry)
				.chain(tests.iter().copied())
				.collect(),
			None if !tests.is_empty() => tests.to_vec(),
			None => files.to_vec(),
		};
		let reached = self.reached_values(&roots, &mut cache);
		// every value is checked for cycles (D42), the reached ones are initialized
		let mut all = self.reached_values(files, &mut cache);
		all.extend(reached.iter().copied());
		all.sort_by(|&(a_file, a), &(b_file, b)| {
			(self.module(a_file), a.index()).cmp(&(self.module(b_file), b.index()))
		});
		all.dedup();
		let dependencies: FxHashMap<Decl, Vec<Decl>> = all
			.iter()
			.map(|&decl| (decl, self.value_dependencies(decl, &mut cache)))
			.collect();
		let mut order = Vec::new();
		let mut state: FxHashMap<Decl, u8> = FxHashMap::default();
		let mut cycles: Vec<Vec<Decl>> = Vec::new();
		for &decl in &all {
			let mut path = Vec::new();
			visit_value(
				decl,
				&dependencies,
				&mut state,
				&mut path,
				&mut order,
				&mut cycles,
			);
		}
		let reached: FxHashSet<Decl> = reached.into_iter().collect();
		order.retain(|decl| reached.contains(decl));
		for cycle in cycles {
			self.report_initializer_cycle(&cycle);
		}
		order
	}

	/// The module-level values the files use, directly or through the
	/// declarations they use, in declaration order then file path.
	fn reached_values(
		&self,
		roots: &[FileId],
		cache: &mut FxHashMap<Decl, Vec<Decl>>,
	) -> Vec<Decl> {
		let mut reached: Vec<Decl> = Vec::new();
		let mut seen = FxHashSet::default();
		let mut stack: Vec<Decl> = roots
			.iter()
			.map(|&file| (file, NodeId::new(self.tree(file).root)))
			.collect();
		while let Some(decl) = stack.pop() {
			if !seen.insert(decl) {
				continue;
			}
			if self.is_value(decl) {
				reached.push(decl);
			}
			let used = cache
				.entry(decl)
				.or_insert_with(|| self.used_decls(decl.0, decl.1))
				.clone();
			stack.extend(used);
		}
		reached.sort_by(|&(a_file, a), &(b_file, b)| {
			(self.module(a_file), a.index()).cmp(&(self.module(b_file), b.index()))
		});
		reached
	}

	/// `MOD007` on each value of a cycle (unless the walk reported it).
	fn report_initializer_cycle(&mut self, cycle: &[Decl]) {
		let names: Vec<String> = cycle.iter().map(|&decl| self.decl_name(decl)).collect();
		let shown = match names.as_slice() {
			[one] => format!("the initializer of `{one}` depends on itself"),
			[first, second] => {
				format!("the initializers of `{first}` and `{second}` depend on each other")
			}
			_ => format!(
				"the initializers of {} depend on each other",
				names
					.iter()
					.map(|name| format!("`{name}`"))
					.collect::<Vec<_>>()
					.join(", ")
			),
		};
		for &(file, decl) in cycle {
			let already = self
				.diagnostics(file)
				.iter()
				.any(|diagnostic| std::ptr::eq(diagnostic.code, &MOD007));
			if already {
				continue;
			}
			let diagnostic = Diagnostic::new(
				&MOD007,
				Location {
					file,
					span: self.span(file, decl),
				},
				shown.clone(),
			);
			self.push_diagnostic(file, diagnostic);
		}
	}

	/// The name of a module-level declaration, for messages.
	fn decl_name(&self, (file, decl): Decl) -> String {
		let symbols = &self.analysis(file).symbols;
		match symbols.symbol_of_decl(decl) {
			Some(symbol) => self
				.types
				.syms
				.resolve(symbols.symbol(symbol).name)
				.to_string(),
			None => String::new(),
		}
	}

	/// `MOD015`: the entry function takes `(arguments: Array(String))` or
	/// nothing and returns an `Integer` or nothing. In an application, the
	/// entry file must export one.
	fn check_entry_function(&mut self, entry: FileId, required: bool) {
		let interface = self.source_file(entry).interface.export;
		let function = interface.filter(|export| export.kind == SymbolKind::Function);
		let Some(function) = function else {
			if required {
				let diagnostic = Diagnostic::new(
					&MOD015,
					Location {
						file: entry,
						span: SourceSpan::new(0, 0),
					},
					"the entry file exports the function the program starts with, ex: `export function = () => { … }`",
				);
				self.push_diagnostic(entry, diagnostic);
			}
			return;
		};
		let TypeKind::Function(sig) = self.types.kind(function.ty) else {
			return;
		};
		let sig = self.types.sig(sig);
		let params = self.types.params(sig.params).to_vec();
		let strings = self
			.types
			.collection(CollKind::Array, TypeId::STRING, TypeId::NONE);
		let params_ok = match params.as_slice() {
			[] => true,
			[param] => param.ty == strings || self.types.is_recovering(param.ty),
			_ => false,
		};
		let returns_ok = matches!(sig.ret, TypeId::VOID | TypeId::NEVER | TypeId::UNKNOWN)
			|| matches!(self.types.kind(sig.ret), TypeKind::Int(_))
				&& self
					.types
					.assignable(sig.ret, TypeId::INTEGER, crate::types::Mode::Copy)
					.is_ok();
		if params_ok && returns_ok {
			return;
		}
		let diagnostic = Diagnostic::new(
			&MOD015,
			Location {
				file: entry,
				span: self.span(entry, function.decl),
			},
			"the entry function takes `(arguments: Array(String))` or nothing, and returns an `Integer` exit code or nothing",
		);
		self.push_diagnostic(entry, diagnostic);
	}

	/// Re-parses a module after an edit (the language server, watchers):
	/// re-analyzes it, then the files that read it when its interface
	/// changed, and the files whose lookups may now find a new file. Returns
	/// the files analyzed again (their diagnostics changed).
	pub fn update(&mut self, module: &str, text: Arc<str>) -> Vec<FileId> {
		let previous = self.loaded_module(module);
		let before = previous.map(|file| self.source_file(file).interface.hash(&self.types));
		let file = self.set_source(module, text);
		if let Some(previous) = previous {
			self.forget(previous);
		}
		self.analyze_file(file);
		let mut changed = vec![file];
		let after = self.source_file(file).interface.hash(&self.types);
		let interface_changed = before != Some(after) || previous.is_none();
		if !interface_changed {
			return changed;
		}
		// the dependents, transitively: the files whose reads touched the module
		let name = self.own_name(file).unwrap_or_default();
		let mut worklist = vec![(module.to_string(), name)];
		let mut done: FxHashSet<String> = FxHashSet::default();
		while let Some((module, name)) = worklist.pop() {
			if !done.insert(module.clone()) {
				continue;
			}
			let dependents: Vec<FileId> = (0..self.file_count())
				.map(FileId::new)
				.filter(|&other| {
					let source = self.source_file(other);
					source.analyzed
						&& self.loaded_module(&source.module) == Some(other)
						&& source.module != module
						&& source
							.reads
							.iter()
							.any(|read| *read == module || *read == name)
				})
				.collect();
			for dependent in dependents {
				let hash = self.source_file(dependent).interface.hash(&self.types);
				self.reset_analysis(dependent);
				self.analyze_file(dependent);
				changed.push(dependent);
				if self.source_file(dependent).interface.hash(&self.types) != hash {
					let module = self.module(dependent).to_string();
					let name = self.own_name(dependent).unwrap_or_default();
					worklist.push((module, name));
				}
			}
		}
		changed.sort();
		changed.dedup();
		changed
	}
}

/// Depth-first visit of the initialization graph: dependencies first; a
/// value met again on the path closes a cycle.
fn visit_value(
	decl: Decl,
	dependencies: &FxHashMap<Decl, Vec<Decl>>,
	state: &mut FxHashMap<Decl, u8>,
	path: &mut Vec<Decl>,
	order: &mut Vec<Decl>,
	cycles: &mut Vec<Vec<Decl>>,
) {
	match state.get(&decl) {
		Some(2) => return,
		Some(1) => {
			if let Some(start) = path.iter().position(|&other| other == decl) {
				cycles.push(path[start..].to_vec());
			}
			return;
		}
		_ => {}
	}
	state.insert(decl, 1);
	path.push(decl);
	for &dependency in dependencies.get(&decl).map_or(&[][..], Vec::as_slice) {
		if dependencies.contains_key(&dependency) {
			visit_value(dependency, dependencies, state, path, order, cycles);
		}
	}
	path.pop();
	state.insert(decl, 2);
	order.push(decl);
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::modules::CodebaseResolver;

	/// A session over a package of `(path, text)` files, analyzed.
	fn package(files: &[(&str, &str)]) -> (Session, Program) {
		let files: Vec<(String, String)> = files
			.iter()
			.map(|(path, text)| (path.to_string(), text.to_string()))
			.collect();
		let mut session = Session::new();
		session.set_resolver(Box::new(CodebaseResolver::from_texts(&files, None)));
		let program = session.analyze_package(Some("application/main"));
		(session, program)
	}

	fn codes(session: &Session, module: &str) -> Vec<&'static str> {
		let file = session.loaded_module(module).expect("a loaded module");
		session
			.diagnostics(file)
			.iter()
			.map(|diagnostic| diagnostic.code.id)
			.collect()
	}

	#[test]
	fn initialization_follows_dependencies() {
		let (session, program) = package(&[
			("library/a.fa", "export = b + c\n"),
			("library/b.fa", "export = c * 2\n"),
			("library/c.fa", "export = 1\n"),
			("library/unused.fa", "export = 5\n"),
			(
				"application/main.fa",
				"export function = () => console.log(library.a)\n",
			),
		]);
		assert!(!program.has_errors(&session));
		let order: Vec<&str> = program
			.init_order
			.iter()
			.map(|&(file, _)| session.module(file))
			.collect();
		assert_eq!(order, ["library/c", "library/b", "library/a"]);
		let files: Vec<&str> = program
			.files
			.iter()
			.map(|&file| session.module(file))
			.collect();
		let position = |module: &str| files.iter().position(|&file| file == module);
		assert!(position("library/c") < position("library/b"));
		assert!(position("library/a") < position("application/main"));
		assert!(
			files.contains(&"library/unused"),
			"every file is analyzed (D42)"
		);
	}

	#[test]
	fn an_edit_reanalyzes_the_files_using_a_changed_interface() {
		let (mut session, _) = package(&[
			("library/answer.fa", "export function = (): Integer => 42\n"),
			("library/other.fa", "export = 1\n"),
			(
				"application/main.fa",
				"export function = () => {\n\tlet value: Integer = library.answer()\n\tconsole.log(value)\n}\n",
			),
		]);
		assert!(codes(&session, "application/main").is_empty());
		// the body changes, not the interface: only the file itself
		let changed = session.update(
			"library/answer",
			Arc::from("export function = (): Integer => 43\n"),
		);
		assert_eq!(changed.len(), 1);
		// the interface changes: its users are analyzed again
		let changed = session.update(
			"library/answer",
			Arc::from("export function = (): String => \"x\"\n"),
		);
		let modules: Vec<&str> = changed.iter().map(|&file| session.module(file)).collect();
		assert!(modules.contains(&"application/main"), "{modules:?}");
		assert!(!modules.contains(&"library/other"), "{modules:?}");
		assert_eq!(codes(&session, "application/main"), ["E-TYPE-1"]);
	}
}
