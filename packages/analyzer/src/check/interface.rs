//! The interface of a file (design/parallel-pipeline.md §2.2), completed at
//! the end of its walk: everything the other files, the program analysis
//! and the backends of the other files read of it, so that nothing reads
//! another file's tree or analysis.
//!
//! - The functions of its types and their parameters (methods called from
//!   other files, named arguments).
//! - The values of its `static`s (folded where other files read them).
//! - The export names of its declarations (`counter$count`, `Item$score`).
//! - Its module-level declarations, each with the module-level declarations
//!   it uses: the initialization order of the program and `MOD007`
//!   (`program.rs`). They are collected in one pass over the tree, while it
//!   is still there.

use rustc_hash::FxHashMap;

use crate::analysis::Callee;
use crate::check::Walker;
use crate::check::modules::namespace_block;
use crate::ids::{DeclKey, NodeId, SymbolId};
use crate::modules::{Declared, KeyedDecl, Parameter, TypeFunction};
use crate::symbols::{Resolution, ScopeKind, SymbolKind};
use crate::syntax::{self, View};

impl Walker<'_> {
	/// Completes the interface of the file (its export and the members of
	/// its namespaces are published already).
	pub(super) fn publish_interface(&mut self, root: NodeId) {
		// the function node of each function declaration
		let mut function_nodes: FxHashMap<NodeId, NodeId> = FxHashMap::default();
		for (&node, info) in &self.a.functions {
			function_nodes.insert(node, node);
			if let Some(declaration) = info.declaration {
				function_nodes.entry(declaration).or_insert(node);
			}
		}
		let function_of = |decl: NodeId| function_nodes.get(&decl).copied();

		// the functions of the types
		let mut type_functions: FxHashMap<NodeId, Vec<TypeFunction>> = FxHashMap::default();
		let mut called: Vec<NodeId> = Vec::new();
		let mut type_decls: Vec<(NodeId, Vec<SymbolId>)> = self
			.a
			.type_decls
			.iter()
			.map(|(&decl, info)| (decl, info.functions.clone()))
			.collect();
		// keys are given in a deterministic order (their disambiguators)
		type_decls.sort_unstable_by_key(|&(decl, _)| decl);
		for (decl, functions) in type_decls {
			let mut published = Vec::with_capacity(functions.len());
			for function in functions {
				let key = self.symbol_key(function);
				let symbol = self.a.symbols.symbol(function);
				let method = function_of(symbol.decl_node)
					.and_then(|node| self.a.function(node))
					.is_some_and(|info| {
						info.params.first().is_some_and(|param| {
							self.a.symbols.symbol(param.symbol).kind == SymbolKind::SelfParameter
						})
					});
				called.push(symbol.decl_node);
				published.push(TypeFunction {
					name: symbol.name,
					decl: symbol.decl_node,
					key,
					ty: symbol.ty,
					method,
				});
			}
			type_functions.insert(decl, published);
		}
		// the default values of the fields of the types, evaluated once here
		// when they are constant expressions (the typed literals of other
		// files fill them in: `FieldValue::Constant`)
		let mut field_defaults: FxHashMap<NodeId, Vec<_>> = FxHashMap::default();
		let mut with_defaults: Vec<(NodeId, Vec<(crate::ids::Sym, NodeId)>)> = self
			.a
			.type_decls
			.iter()
			.map(|(&decl, info)| {
				let defaults = info
					.fields
					.iter()
					.filter_map(|field| field.default.map(|default| (field.name, default)))
					.collect();
				(decl, defaults)
			})
			.filter(|(_, defaults): &(NodeId, Vec<_>)| !defaults.is_empty())
			.collect();
		with_defaults.sort_unstable_by_key(|&(decl, _)| decl);
		for (decl, defaults) in with_defaults {
			let mut published = Vec::with_capacity(defaults.len());
			for (name, default) in defaults {
				let value = if self.is_constant_expression(default, 0) {
					self.quiet += 1;
					let value = self.evaluate_at(default, default);
					self.quiet -= 1;
					value.filter(|value| !value.has_function())
				} else {
					None
				};
				published.push((name, value));
			}
			field_defaults.insert(decl, published);
		}
		// the functions other files call directly: exports, members, type functions
		let interface = &self.s.source_file(self.file).interface;
		called.extend(
			interface
				.export
				.iter()
				.chain(interface.members.values())
				.filter(|exported| exported.kind == SymbolKind::Function)
				.map(|exported| exported.decl),
		);
		let mut parameters: FxHashMap<NodeId, Vec<Parameter>> = FxHashMap::default();
		called.sort_unstable();
		called.dedup();
		for decl in called {
			let Some(info) = function_of(decl).and_then(|node| self.a.function(node)) else {
				continue;
			};
			let params: Vec<(SymbolId, bool)> = info
				.params
				.iter()
				.map(|param| (param.symbol, param.default.is_some()))
				.collect();
			let mut published = Vec::with_capacity(params.len());
			for (param, has_default) in params {
				let key = self.symbol_key(param);
				let symbol = self.a.symbols.symbol(param);
				published.push(Parameter {
					name: symbol.name,
					decl: symbol.decl_node,
					key,
					kind: symbol.kind,
					ty: symbol.ty,
					has_default,
				});
			}
			parameters.insert(decl, published);
		}

		// the module-level declarations, the statics, what each declaration
		// uses. One pass over the tree after the walk, not during it: the
		// walk checks a hoisted declaration on demand, from wherever it is
		// first used, so its stack of declarations doesn't say which one
		// encloses a node in the text; here it is the nesting of the tree.
		// The pass reads only dense tables and allocates nothing per node.
		let module_level = |walker: &Self, symbol: SymbolId| {
			let data = walker.a.symbols.symbol(symbol);
			matches!(
				walker.a.symbols.scope(data.scope).kind,
				ScopeKind::Module | ScopeKind::Namespace
			)
		};
		let mut statics = FxHashMap::default();
		let mut declared: FxHashMap<NodeId, Declared> = FxHashMap::default();
		let mut uses: Vec<DeclKey> = Vec::new();
		// the declarations enclosing the node visited, with the length of the
		// stack when they end
		let mut enclosing: Vec<(NodeId, usize)> = Vec::new();
		let mut stack: Vec<NodeId> = vec![root];
		let tree = std::sync::Arc::clone(&self.tree);
		while let Some(node) = stack.pop() {
			while enclosing
				.last()
				.is_some_and(|&(_, depth)| stack.len() < depth)
			{
				enclosing.pop();
			}
			if node.index() >= tree.nodes.len() {
				continue;
			}
			// a destructuring declares the names of its pattern, which its
			// value initializes: its whole subtree is theirs (the
			// initialization order reads what each value uses)
			let mut candidates = vec![node];
			if let View::Destructuring { pattern, .. } = syntax::view(&tree, node) {
				let mut names = vec![pattern];
				while let Some(name) = names.pop() {
					if self.a.symbols.symbol_of_decl(name).is_some() {
						candidates.push(name);
					}
					names.extend(syntax::children(&tree, name));
				}
			}
			for node in candidates {
				let Some(symbol) = self.a.symbols.symbol_of_decl(node) else {
					continue;
				};
				let data = self.a.symbols.symbol(symbol);
				let tracked = match data.kind {
					SymbolKind::Function => true,
					SymbolKind::Let
					| SymbolKind::Mutable
					| SymbolKind::Reactive
					| SymbolKind::Derived
					| SymbolKind::Namespace
					| SymbolKind::Static
					| SymbolKind::Type => module_level(self, symbol),
					_ => false,
				};
				if tracked && !declared.contains_key(&node) {
					if data.kind == SymbolKind::Static
						&& let Some(value) = self.a.static_values.get(&symbol)
					{
						statics.insert(node, value.clone());
					}
					let declaration = Declared {
						kind: data.kind,
						name: data.name,
						span: self.span(node),
						function: function_of(node),
						uses: Vec::new(),
						thread_safe: matches!(
							self.s.types.kind(data.ty),
							crate::types::TypeKind::Sync(kind, _) if kind.is_handle()
						),
					};
					declared.insert(node, declaration);
					self.symbol_key(symbol);
					// its subtree is what is pushed above the current length
					enclosing.push((node, stack.len()));
				}
			}
			let used = match self.a.resolution(node) {
				Resolution::Local(symbol) if module_level(self, symbol) => {
					Some(self.symbol_key(symbol))
				}
				Resolution::External(key) => Some(key),
				_ => None,
			};
			let called = match self.a.calls.get(&node).map(|call| &call.callee) {
				Some(&Callee::Function(function)) => self.node_key(function),
				_ => None,
			};
			for found in used.into_iter().chain(called) {
				uses.push(found);
				for &(decl, _) in &enclosing {
					if let Some(entry) = declared.get_mut(&decl) {
						entry.uses.push(found);
					}
				}
			}
			stack.extend(syntax::children(&tree, node));
		}
		for entry in declared.values_mut() {
			entry.uses.sort_unstable();
			entry.uses.dedup();
		}
		uses.sort_unstable();
		uses.dedup();

		let worker_work = self.worker_work();
		let export_names = self.export_names(root);
		let reads_target = self.a.reads_target;
		let externs = self.a.externs.clone();
		// where each key given is (the export and the members got theirs
		// when published)
		let keys: FxHashMap<DeclKey, KeyedDecl> = self
			.modules
			.keyed
			.iter()
			.map(|(&key, &symbol)| {
				let data = self.a.symbols.symbol(symbol);
				let keyed = KeyedDecl {
					node: data.decl_node,
					span: self.span(data.decl_node),
					name_span: data.name_span,
				};
				(key, keyed)
			})
			.collect();
		let interface = &mut self.s.source_file_mut(self.file).interface;
		interface.keys = keys;
		interface.externs = externs;
		interface.type_functions = type_functions;
		interface.parameters = parameters;
		interface.statics = statics;
		interface.field_defaults = field_defaults;
		interface.export_names = export_names;
		interface.declared = declared;
		interface.uses = uses;
		interface.reads_target = reads_target;
		interface.worker_work = worker_work;
		interface.complete = true;
	}

	/// What the work given to `WorkerPool.submit` uses (`worker_work`): the
	/// names of module-level declarations and the calls of functions in its
	/// body, or the function given by name.
	fn worker_work(&mut self) -> Vec<(DeclKey, crate::diagnostics::SourceSpan)> {
		let mut found = Vec::new();
		let works = self.a.submitted_works.clone();
		let tree = std::sync::Arc::clone(&self.tree);
		for work in works {
			let mut stack = vec![work];
			while let Some(node) = stack.pop() {
				let used = match self.a.resolution(node) {
					Resolution::Local(symbol) => {
						let data = self.a.symbols.symbol(symbol);
						let module_level = matches!(
							self.a.symbols.scope(data.scope).kind,
							ScopeKind::Module | ScopeKind::Namespace
						);
						(module_level || data.kind == SymbolKind::Function)
							.then(|| self.symbol_key(symbol))
					}
					Resolution::External(key) => Some(key),
					_ => None,
				};
				let called = match self.a.calls.get(&node).map(|call| &call.callee) {
					Some(&Callee::Function(function)) => self.node_key(function),
					_ => None,
				};
				for key in used.into_iter().chain(called) {
					found.push((key, self.span(node)));
				}
				stack.extend(syntax::children(&tree, node));
			}
		}
		found.sort_by_key(|&(key, span)| (span.start, span.end, key));
		found.dedup();
		found
	}

	/// The names the declarations other files may use are exported as: the
	/// export, the members of the namespaces (recursively), the functions
	/// of the types (`Type$name`).
	fn export_names(&self, root: NodeId) -> FxHashMap<NodeId, String> {
		let tree = &self.tree;
		let mut names = FxHashMap::default();
		let View::Module { statements } = syntax::view(tree, root) else {
			return names;
		};
		let own = self.s.own_name(self.file).unwrap_or_else(|| {
			let module = self.s.module(self.file);
			module.rsplit('/').next().unwrap_or(module).to_string()
		});
		for statement in statements.iter() {
			match syntax::view(tree, statement) {
				View::ExportValue { .. }
				| View::ExportFunction { .. }
				| View::ExportType { .. } => {
					names.entry(statement).or_insert_with(|| own.clone());
				}
				View::ExportNamespace(value) => {
					names.entry(statement).or_insert_with(|| own.clone());
					self.namespace_names(value, &own, &mut names);
				}
				View::Namespace { name, value } => {
					let prefix = tree.name(name).to_string();
					self.namespace_names(value, &prefix, &mut names);
				}
				_ => {}
			}
		}
		let types = &self.s.types;
		for decl in self.a.type_decls.values() {
			let type_name = types.syms.resolve(self.a.symbols.symbol(decl.symbol).name);
			for &function in &decl.functions {
				let symbol = self.a.symbols.symbol(function);
				let name = types.syms.resolve(symbol.name);
				if !name.starts_with('#') {
					names
						.entry(symbol.decl_node)
						.or_insert_with(|| format!("{type_name}${name}"));
				}
			}
		}
		names
	}

	/// The export names of the members of a namespace (its braces `value`).
	fn namespace_names(&self, value: NodeId, prefix: &str, names: &mut FxHashMap<NodeId, String>) {
		let tree = &self.tree;
		let statements = match syntax::view(tree, value) {
			View::Block { statements } | View::Object { items: statements } => statements,
			_ => return,
		};
		for member in statements.iter() {
			let Some(symbol) = self.a.symbols.symbol_of_decl(member) else {
				continue;
			};
			let data = self.a.symbols.symbol(symbol);
			let name = self.s.types.syms.resolve(data.name);
			// a private member is only used inside its file (and tests aren't members)
			if name.starts_with('#') || data.kind == SymbolKind::Test {
				continue;
			}
			let qualified = format!("{prefix}${name}");
			if let Some(block) = namespace_block(tree, member) {
				self.namespace_names(block, &qualified, names);
			}
			names.insert(member, qualified);
		}
	}
}
