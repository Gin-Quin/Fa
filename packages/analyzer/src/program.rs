//! The analysis of a whole program (advanced brief §15, §21): the files
//! analyzed, the rounds that settle cycles between files, the module-level
//! graph (initialization order, `MOD007`; the module-level values the
//! methods of `*.worker.fa` files reach, `CONC008`), the entry function
//! (`MOD015`), and the incremental [`Session::update`].

use std::sync::Arc;

use rustc_hash::{FxHashMap, FxHashSet};

use crate::diagnostics::codes::advanced::{
	CONC008, MOD001, MOD002, MOD003, MOD007, MOD015, REQ002,
};
use crate::diagnostics::{Diagnostic, Location, SourceSpan};
use crate::ids::{FileId, ModuleId, TypeId};
use crate::modules::{Declared, FileRole, Program};
use crate::session::Session;
use crate::symbols::SymbolKind;

use crate::types::{CollKind, TypeKind};

/// The most rounds the program analysis runs to settle cycles between files.
const MAX_ROUNDS: usize = 4;

/// A module-level declaration, by its key (its module is in it).
type Decl = crate::ids::DeclKey;

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
		// asynchronous mode, detected (design/async.md A6, check/tasks.rs)
		self.configure_asynchronous(&mut program, None, false);
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
		// every file is read and parsed first, in parallel
		let mut all: Vec<String> = entry.map(str::to_string).into_iter().collect();
		all.extend(modules.iter().cloned());
		self.load_modules(&all);
		let entry_file = entry.and_then(|entry| self.file_of_module(entry));
		let roots: Vec<FileId> = all
			.iter()
			.filter_map(|module| self.loaded_module(module))
			.filter(|&file| !matches!(self.role(file), FileRole::Configuration(_)))
			.collect();
		self.settle_units(&roots);
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
		// asynchronous mode, detected (design/async.md A6, check/tasks.rs)
		self.configure_asynchronous(&mut program, None, entry_file.is_none());
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
	/// previous one published. (A standalone program: its files are found
	/// as its walk reaches them.)
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

	/// Analyzes the files of a package unit by unit (the strongly connected
	/// components of the module graph, dependencies first): a unit's files
	/// read only finished files, except each other's. A unit whose files
	/// read a declaration of one of them not available yet (a cycle) is
	/// walked again, reading the interfaces the previous round published.
	fn settle_units(&mut self, files: &[FileId]) {
		for unit in self.units(files) {
			for &file in &unit {
				self.analyze_file(file);
			}
			for _ in 1..MAX_ROUNDS {
				if !unit.iter().any(|&file| self.source_file(file).deferred) {
					break;
				}
				for &file in &unit {
					self.reset_analysis(file);
				}
				for &file in &unit {
					self.analyze_file(file);
				}
			}
		}
	}

	/// The units of analysis of files: the strongly connected components
	/// of the module graph (the edges of their scans), each after the units
	/// it reads, its files in path order.
	pub fn units(&self, files: &[FileId]) -> Vec<Vec<FileId>> {
		let mut files = files.to_vec();
		files.sort_by(|&a, &b| self.module(a).cmp(self.module(b)));
		files.dedup();
		let index: FxHashMap<&str, usize> = files
			.iter()
			.enumerate()
			.map(|(position, &file)| (self.module(file), position))
			.collect();
		let edges: Vec<Vec<usize>> = files
			.iter()
			.map(|&file| {
				self.source_file(file)
					.scan
					.dependencies
					.iter()
					.filter_map(|dependency| index.get(dependency.as_str()).copied())
					.collect()
			})
			.collect();
		crate::graph::components(&edges)
			.into_iter()
			.map(|unit| unit.into_iter().map(|position| files[position]).collect())
			.collect()
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
		let analyzed: FxHashSet<FileId> = files.iter().copied().collect();
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
				Some(file) if analyzed.contains(&file) => file,
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
		self.check_workers(&files);
		let init_order = self
			.initialization_order(&files, entry, &tests)
			.into_iter()
			.filter_map(|key| {
				let (module, decl) =
					self.external_node(crate::symbols::Resolution::External(key))?;
				Some((self.file_of(module)?, decl))
			})
			.collect();
		Program {
			files,
			entry,
			init_order,
			tests,
			configurations: Vec::new(),
			asynchronous: false,
			asynchronous_reason: String::new(),
		}
	}

	/// The order of a program's files and the initialization order of its
	/// values, computed again from the interfaces of its files only (their
	/// trees and analyses may be dropped: [`Session::forget_tree`]).
	pub fn assemble_again(&mut self, program: &Program) -> Program {
		let mut again = self.assemble(program.files.clone(), program.entry, program.tests.clone());
		again.configurations = program.configurations.clone();
		again.asynchronous = program.asynchronous;
		again.asynchronous_reason = program.asynchronous_reason.clone();
		again
	}

	/// The files, each after the files it uses (a cycle in path order).
	fn dependency_order(&self, files: &[FileId]) -> Vec<FileId> {
		self.units(files).into_iter().flatten().collect()
	}

	/// What the interface of a module says of one of its declarations.
	fn declared(&self, key: Decl) -> Option<&Declared> {
		let module = self.types.decl_key_data(key).module;
		let in_file = self.file_of(module).and_then(|file| {
			let interface = &self.source_file(file).interface;
			interface.declared.get(&interface.node_of(key)?)
		});
		in_file.or_else(|| {
			let interface = self.interface(module)?;
			interface.declared.get(&interface.node_of(key)?)
		})
	}

	/// The module of a declaration.
	fn module_of_decl(&self, key: Decl) -> ModuleId {
		self.types.decl_key_data(key).module
	}

	/// The module-level declarations a declaration uses (its interface).
	fn uses(&self, decl: Decl) -> &[Decl] {
		self.declared(decl)
			.map_or(&[][..], |declared| &declared.uses)
	}

	/// The role of a module: its file's, or the resolver's.
	fn module_role(&self, module: ModuleId) -> FileRole {
		match self.file_of(module) {
			Some(file) => self.role(file),
			None => self.resolver().map_or(FileRole::Module, |resolver| {
				resolver.role(self.types.module_path(module))
			}),
		}
	}

	/// Whether a declaration is a module-level value (initialized at runtime).
	fn is_value(&self, decl: Decl) -> bool {
		self.declared(decl)
			.is_some_and(|declared| matches!(declared.kind, SymbolKind::Let | SymbolKind::Mutable))
			&& self.module_role(self.module_of_decl(decl)).is_module()
	}

	/// For each value of a program's [`Program::init_order`], the indices
	/// (in that order) of the values its initializer reads, directly or
	/// through the functions it calls: `fa develop` initializes again the
	/// values that read the values of a swapped module (decision D43).
	pub fn initializer_reads(&self, program: &Program) -> Vec<Vec<usize>> {
		let keys: Vec<Option<Decl>> = program
			.init_order
			.iter()
			.map(|&(file, node)| {
				self.source_file(file)
					.interface
					.keys
					.iter()
					.find(|(_, keyed)| keyed.node == node)
					.map(|(&key, _)| key)
			})
			.collect();
		keys.iter()
			.map(|key| {
				let Some(key) = key else {
					return Vec::new();
				};
				self.value_dependencies(*key)
					.into_iter()
					.filter_map(|read| keys.iter().position(|other| *other == Some(read)))
					.collect()
			})
			.collect()
	}

	/// The module-level values a declaration reads, directly or through the
	/// functions it calls.
	fn value_dependencies(&self, decl: Decl) -> Vec<Decl> {
		let mut values = Vec::new();
		let mut seen = FxHashSet::default();
		let mut stack = vec![decl];
		seen.insert(decl);
		while let Some(current) = stack.pop() {
			for &next in self.uses(current) {
				if next == current || !seen.insert(next) {
					if next == decl && current != decl && self.is_value(decl) {
						values.push(decl);
					}
					continue;
				}
				if self.is_value(next) {
					values.push(next);
				} else if self
					.declared(next)
					.is_some_and(|declared| declared.kind == SymbolKind::Function)
				{
					stack.push(next);
				}
			}
		}
		values.sort_by(|&a, &b| self.decl_order(a, b));
		values.dedup();
		values
	}

	/// The order of declarations in messages and outputs: module path, then
	/// position.
	fn decl_order(&self, a: Decl, b: Decl) -> std::cmp::Ordering {
		let position = |key: Decl| {
			self.declared(key)
				.map_or(u32::MAX, |declared| declared.span.start)
		};
		(self.types.module_path(self.module_of_decl(a)), position(a))
			.cmp(&(self.types.module_path(self.module_of_decl(b)), position(b)))
			.then_with(|| self.key_cmp(a, b))
	}

	/// The initialization order of the module-level values the program
	/// reaches (advanced brief §15.7), reporting `MOD007` on cycles. Read
	/// from the interfaces of the files only.
	fn initialization_order(
		&mut self,
		files: &[FileId],
		entry: Option<FileId>,
		tests: &[FileId],
	) -> Vec<Decl> {
		let ids = |files: &[FileId]| -> Vec<ModuleId> {
			files.iter().map(|&file| self.module_id(file)).collect()
		};
		let (order, cycles) = self.initialization_order_of(
			&ids(files),
			entry.map(|entry| self.module_id(entry)),
			&ids(tests),
		);
		for (module, diagnostic) in self.initializer_cycle_diagnostics(&cycles) {
			if let Some(file) = self.loaded_module(&module) {
				let already = self
					.diagnostics(file)
					.iter()
					.any(|other| std::ptr::eq(other.code, &MOD007));
				if !already {
					let mut diagnostic = diagnostic;
					diagnostic.set_file(file);
					self.push_diagnostic(file, diagnostic);
				}
			}
		}
		order
	}

	/// The initialization order of the module-level values a program of
	/// modules reaches (its entry and tests, or all of them for a library),
	/// and the cycles between initializers (every value is checked, D42).
	pub(crate) fn initialization_order_of(
		&self,
		modules: &[ModuleId],
		entry: Option<ModuleId>,
		tests: &[ModuleId],
	) -> (Vec<Decl>, Vec<Vec<Decl>>) {
		// the roots: the entry file and the test files (or every file of a library)
		let roots: Vec<ModuleId> = match entry {
			Some(entry) => std::iter::once(entry)
				.chain(tests.iter().copied())
				.collect(),
			None if !tests.is_empty() => tests.to_vec(),
			None => modules.to_vec(),
		};
		let reached = self.reached_values(&roots);
		let mut all = self.reached_values(modules);
		all.extend(reached.iter().copied());
		all.sort_by(|&a, &b| self.decl_order(a, b));
		all.dedup();
		let dependencies: FxHashMap<Decl, Vec<Decl>> = all
			.iter()
			.map(|&decl| (decl, self.value_dependencies(decl)))
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
		(order, cycles)
	}

	/// The module-level values the modules use, directly or through the
	/// declarations they use, in declaration order then file path.
	fn reached_values(&self, roots: &[ModuleId]) -> Vec<Decl> {
		let mut reached: Vec<Decl> = Vec::new();
		let mut seen = FxHashSet::default();
		let mut stack: Vec<Decl> = Vec::new();
		for &module in roots {
			if let Some(interface) = self.interface(module) {
				stack.extend(interface.uses.iter().copied());
			}
		}
		while let Some(decl) = stack.pop() {
			if !seen.insert(decl) {
				continue;
			}
			if self.is_value(decl) {
				reached.push(decl);
			}
			stack.extend(self.uses(decl).iter().copied());
		}
		reached.sort_by(|&a, &b| self.decl_order(a, b));
		reached
	}

	/// `CONC008` on the methods of the `*.worker.fa` files of a program.
	fn check_workers(&mut self, files: &[FileId]) {
		let modules: Vec<ModuleId> = files.iter().map(|&file| self.module_id(file)).collect();
		for (module, diagnostic) in self.worker_diagnostics(&modules) {
			if let Some(file) = self.loaded_module(&module) {
				let already = self.diagnostics(file).iter().any(|other| {
					std::ptr::eq(other.code, diagnostic.code)
						&& other.location.span == diagnostic.location.span
				});
				if !already {
					let mut diagnostic = diagnostic;
					diagnostic.set_file(file);
					self.push_diagnostic(file, diagnostic);
				}
			}
		}
	}

	/// `CONC008` on worker code that uses a module-level `mutable` value
	/// that isn't thread-safe, directly or through the functions it calls
	/// (of any file): it runs on a worker's thread, the value is shared by
	/// every thread. Worker code is each method of a `*.worker.fa` file
	/// (reported at the method) and the work given to `WorkerPool.submit`
	/// (reported at each name or call of its body that leads to such a
	/// value). Read from the interfaces: `(module path, diagnostic)` (the
	/// files of the diagnostics are to be set).
	pub(crate) fn worker_diagnostics(&self, modules: &[ModuleId]) -> Vec<(String, Diagnostic)> {
		let mut found = Vec::new();
		for &module in modules {
			let path = self.types.module_path(module).to_string();
			let Some(interface) = self.interface(module) else {
				continue;
			};
			let mut code: Vec<(String, Vec<Decl>, SourceSpan)> = Vec::new();
			if path.ends_with(".worker") {
				let mut methods: Vec<Decl> = interface
					.type_functions
					.values()
					.flatten()
					.filter(|function| function.method)
					.map(|function| function.key)
					.collect();
				methods.sort_by(|&a, &b| self.decl_order(a, b));
				methods.dedup();
				for method in methods {
					if let Some(declared) = self.declared(method) {
						let what = format!("the method `{}`", self.decl_name(method));
						code.push((what, self.uses(method).to_vec(), declared.span));
					}
				}
			}
			for &(key, span) in &interface.worker_work {
				code.push(("the work given to `submit`".to_string(), vec![key], span));
			}
			// a call and its callee's name lead to the same value: once
			let mut reported: Vec<(SourceSpan, Decl)> = Vec::new();
			for (what, roots, span) in code {
				let Some((value, chain)) = self.shared_value_of(&roots) else {
					continue;
				};
				let overlapping = reported.iter().any(|&(other, other_value)| {
					other_value == value && other.start < span.end && span.start < other.end
				});
				if overlapping {
					continue;
				}
				reported.push((span, value));
				let through = match chain.as_slice() {
					[] => String::new(),
					functions => format!(
						" (through {})",
						functions
							.iter()
							.map(|&function| format!("`{}`", self.decl_name(function)))
							.collect::<Vec<_>>()
							.join(", ")
					),
				};
				let message = format!(
					"{what} runs on a worker's thread: it can't use the module-level `mutable` value `{}`{through}, which every thread shares, unless it is thread-safe (a `Mutex`, an atomic, a `Channel`); keep it in the worker's fields, or pass a copy as an argument",
					self.decl_name(value),
				);
				let diagnostic = Diagnostic::new(
					&CONC008,
					Location {
						file: FileId::new(0),
						span,
					},
					message,
				);
				found.push((path.clone(), diagnostic));
			}
		}
		found
	}

	/// The first module-level `mutable` value that isn't thread-safe among
	/// `roots` (what some code uses) or what the functions among them use,
	/// transitively, and the functions it goes through.
	fn shared_value_of(&self, roots: &[Decl]) -> Option<(Decl, Vec<Decl>)> {
		let mut parent: FxHashMap<Decl, Option<Decl>> = FxHashMap::default();
		let mut seen = FxHashSet::default();
		// `None`: the code itself, which uses the roots
		let mut queue = std::collections::VecDeque::from([None]);
		while let Some(current) = queue.pop_front() {
			let mut uses = match current {
				None => roots.to_vec(),
				Some(function) => self.uses(function).to_vec(),
			};
			uses.sort_by(|&a, &b| self.decl_order(a, b));
			for next in uses {
				if !seen.insert(next) {
					continue;
				}
				let Some(declared) = self.declared(next) else {
					continue;
				};
				// (a script's top-level values too: they are module-level)
				if declared.kind == SymbolKind::Mutable && !declared.thread_safe {
					let mut chain = Vec::new();
					let mut at = current;
					while let Some(function) = at {
						chain.push(function);
						at = parent[&function];
					}
					chain.reverse();
					return Some((next, chain));
				}
				if declared.kind == SymbolKind::Function {
					parent.insert(next, current);
					queue.push_back(Some(next));
				}
			}
		}
		None
	}

	/// `MOD007` on each value of the cycles: `(module path, diagnostic)`
	/// (the files of the diagnostics are to be set).
	pub(crate) fn initializer_cycle_diagnostics(
		&self,
		cycles: &[Vec<Decl>],
	) -> Vec<(String, Diagnostic)> {
		let mut found = Vec::new();
		for cycle in cycles {
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
			for &decl in cycle {
				let span = self
					.declared(decl)
					.map_or(SourceSpan::default(), |declared| declared.span);
				let diagnostic = Diagnostic::new(
					&MOD007,
					Location {
						file: FileId::new(0),
						span,
					},
					shown.clone(),
				);
				found.push((
					self.types
						.module_path(self.module_of_decl(decl))
						.to_string(),
					diagnostic,
				));
			}
		}
		found
	}

	/// The name of a module-level declaration, for messages.
	fn decl_name(&self, decl: Decl) -> String {
		match self.declared(decl) {
			Some(declared) => self.types.syms.resolve(declared.name).to_string(),
			None => String::new(),
		}
	}

	/// `MOD015`: the entry function takes `(arguments: Array(String))` or
	/// nothing and returns an `Integer` or nothing. In an application, the
	/// entry file must export one.
	fn check_entry_function(&mut self, entry: FileId, required: bool) {
		let module = self.module_id(entry);
		if let Some(mut diagnostic) = self.entry_function_diagnostic(module, required) {
			diagnostic.set_file(entry);
			self.push_diagnostic(entry, diagnostic);
		}
	}

	/// `MOD015` on an entry module, from its interface (the file of the
	/// diagnostic is to be set).
	pub(crate) fn entry_function_diagnostic(
		&mut self,
		entry: ModuleId,
		required: bool,
	) -> Option<Diagnostic> {
		let location = |span| Location {
			file: FileId::new(0),
			span,
		};
		let export = self.interface(entry).and_then(|interface| interface.export);
		let function = export.filter(|export| export.kind == SymbolKind::Function);
		let Some(function) = function else {
			return required.then(|| {
				Diagnostic::new(
					&MOD015,
					location(SourceSpan::new(0, 0)),
					"the entry file exports the function the program starts with, ex: `export function = () => { … }`",
				)
			});
		};
		let TypeKind::Function(sig) = self.types.kind(function.ty) else {
			return None;
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
		let requires = !self.types.requirements(sig.requires).is_empty();
		if params_ok && returns_ok && !requires {
			return None;
		}
		let span = self
			.interface(entry)
			.and_then(|interface| interface.declared(function.decl))
			.map_or(SourceSpan::default(), |declared| declared.span);
		// `REQ002`: the entry function provides requirements, it can't need any
		if params_ok && returns_ok {
			return Some(Diagnostic::new(
				&REQ002,
				location(span),
				"the entry function can't declare `requires`: nothing calls it to provide them. Provide them in its body with `provide Type = …`",
			));
		}
		Some(Diagnostic::new(
			&MOD015,
			location(span),
			"the entry function takes `(arguments: Array<String>)` or nothing, and returns an `Integer` exit code or nothing",
		))
	}

	/// Re-parses a module after an edit (the language server, watchers):
	/// re-analyzes it, then, when its interface changed, the files that read
	/// it, found by the reverse edges of the module graph (§4 #10), unit by
	/// unit in dependency order: a unit is analyzed again only when the
	/// interface of a module it reads changed. A new module (no previous
	/// version) also wakes the files whose lookups of its name found
	/// nothing. Stops between units when the cancel flag is set. Returns the
	/// files analyzed again (their diagnostics changed).
	pub fn update(&mut self, module: &str, text: Arc<str>) -> Vec<FileId> {
		use std::sync::atomic::Ordering;
		let previous = self.loaded_module(module);
		let before = previous.map(|file| self.source_file(file).interface.hash(&self.types));
		let file = self.set_source(module, text);
		if let Some(previous) = previous {
			self.forget(previous);
		}
		self.analyze_file(file);
		let mut changed = vec![file];
		let after = self.source_file(file).interface.hash(&self.types);
		if before == Some(after) {
			return changed;
		}
		// the modules whose interface changed, and the readers to look at
		let mut modified: FxHashSet<String> = FxHashSet::default();
		modified.insert(module.to_string());
		let mut candidates: FxHashSet<String> = FxHashSet::default();
		let mut stack: Vec<String> = vec![module.to_string()];
		if previous.is_none() {
			// a new file: the files that looked its name up in vain
			let name = self.own_name(file).unwrap_or_default();
			for other in (0..self.file_count()).map(FileId::new) {
				let source = self.source_file(other);
				if source.analyzed
					&& self.loaded_module(&source.module) == Some(other)
					&& source.module != module
					&& source.reads.contains(&name)
				{
					modified.insert(source.module.clone());
					candidates.insert(source.module.clone());
					stack.push(source.module.clone());
				}
			}
		}
		while let Some(current) = stack.pop() {
			for reader in self.readers(&current) {
				if reader != module && candidates.insert(reader.to_string()) {
					stack.push(reader.to_string());
				}
			}
		}
		let files: Vec<FileId> = candidates
			.iter()
			.filter_map(|candidate| self.loaded_module(candidate))
			.filter(|&other| {
				let source = self.source_file(other);
				source.analyzed && !matches!(source.role, FileRole::Configuration(_))
			})
			.collect();
		let cancel = self.cancel_flag();
		for unit in self.units(&files) {
			if cancel.load(Ordering::Relaxed) {
				break;
			}
			let reads_modified = unit.iter().any(|&other| {
				self.module(other) != module && modified.contains(self.module(other))
					|| self
						.source_file(other)
						.scan
						.dependencies
						.iter()
						.any(|dependency| modified.contains(dependency))
			});
			if !reads_modified {
				continue;
			}
			let hashes: Vec<u128> = unit
				.iter()
				.map(|&other| self.source_file(other).interface.hash(&self.types))
				.collect();
			for &other in &unit {
				self.reset_analysis(other);
			}
			self.settle_unit(&unit);
			for (&other, hash) in unit.iter().zip(hashes) {
				changed.push(other);
				if self.source_file(other).interface.hash(&self.types) != hash {
					modified.insert(self.module(other).to_string());
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
	use crate::ids::NodeId;
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
		// the declaration moves to another node: its users name it by its
		// key, which stays, so they are not analyzed again, and still find it
		let changed = session.update(
			"library/answer",
			Arc::from("\n\nexport function = (): Integer => 40 + 3\n"),
		);
		let modules: Vec<&str> = changed.iter().map(|&file| session.module(file)).collect();
		assert_eq!(modules, ["library/answer"]);
		let main = session.loaded_module("application/main").unwrap();
		let answer = session.loaded_module("library/answer").unwrap();
		let found = (0..session.tree(main).nodes.len())
			.filter_map(|node| {
				session.external(session.analysis(main).resolution(NodeId::new(node)))
			})
			.next();
		let (file, symbol) = found.expect("the use of `answer` resolves");
		assert_eq!(file, answer);
		let decl = session.analysis(answer).symbols.symbol(symbol).decl_node;
		assert_eq!(session.span(answer, decl).start, 2);
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
