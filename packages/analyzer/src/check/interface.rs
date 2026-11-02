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
use crate::ids::{ModuleId, NodeId, SymbolId};
use crate::modules::{Declared, Parameter, TypeFunction};
use crate::symbols::{Resolution, ScopeKind, SymbolKind};
use crate::syntax::{self, View};

impl Walker<'_> {
	/// Completes the interface of the file (its export and the members of
	/// its namespaces are published already).
	pub(super) fn publish_interface(&mut self, root: NodeId) {
		let own = self.s.module_id(self.file);
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
		for (&decl, info) in &self.a.type_decls {
			let functions = info
				.functions
				.iter()
				.map(|&function| {
					let symbol = self.a.symbols.symbol(function);
					let method = function_of(symbol.decl_node)
						.and_then(|node| self.a.function(node))
						.is_some_and(|info| {
							info.params.first().is_some_and(|param| {
								self.a.symbols.symbol(param.symbol).kind
									== SymbolKind::SelfParameter
							})
						});
					called.push(symbol.decl_node);
					TypeFunction {
						name: symbol.name,
						decl: symbol.decl_node,
						ty: symbol.ty,
						method,
					}
				})
				.collect();
			type_functions.insert(decl, functions);
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
		for decl in called {
			let Some(info) = function_of(decl).and_then(|node| self.a.function(node)) else {
				continue;
			};
			let params = info
				.params
				.iter()
				.map(|param| {
					let symbol = self.a.symbols.symbol(param.symbol);
					Parameter {
						name: symbol.name,
						decl: symbol.decl_node,
						kind: symbol.kind,
						ty: symbol.ty,
						has_default: param.default.is_some(),
					}
				})
				.collect();
			parameters.insert(decl, params);
		}

		// the module-level declarations, the statics, what each declaration uses
		let module_level = |symbol: SymbolId| {
			let data = self.a.symbols.symbol(symbol);
			matches!(
				self.a.symbols.scope(data.scope).kind,
				ScopeKind::Module | ScopeKind::Namespace
			)
		};
		let mut statics = FxHashMap::default();
		let mut declared: FxHashMap<NodeId, Declared> = FxHashMap::default();
		let mut uses: Vec<(ModuleId, NodeId)> = Vec::new();
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
			if let Some(symbol) = self.a.symbols.symbol_of_decl(node) {
				let data = self.a.symbols.symbol(symbol);
				let tracked = match data.kind {
					SymbolKind::Function => true,
					SymbolKind::Let
					| SymbolKind::Mutable
					| SymbolKind::Namespace
					| SymbolKind::Static
					| SymbolKind::Type => module_level(symbol),
					_ => false,
				};
				if tracked && !declared.contains_key(&node) {
					if data.kind == SymbolKind::Static
						&& let Some(value) = self.a.static_values.get(&symbol)
					{
						statics.insert(node, value.clone());
					}
					declared.insert(
						node,
						Declared {
							kind: data.kind,
							name: data.name,
							span: self.span(node),
							function: function_of(node),
							uses: Vec::new(),
						},
					);
					// its subtree is what is pushed above the current length
					enclosing.push((node, stack.len()));
				}
			}
			let used = match self.a.resolution(node) {
				Resolution::Local(symbol) if module_level(symbol) => {
					Some((own, self.a.symbols.symbol(symbol).decl_node))
				}
				Resolution::External(module, decl) => Some((module, decl)),
				_ => None,
			};
			let called = match self.a.calls.get(&node).map(|call| &call.callee) {
				Some(&Callee::Function(function)) => Some((own, function)),
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

		let export_names = self.export_names(root);
		let reads_target = self.a.reads_target;
		let externs = self.a.externs.clone();
		let interface = &mut self.s.source_file_mut(self.file).interface;
		interface.externs = externs;
		interface.type_functions = type_functions;
		interface.parameters = parameters;
		interface.statics = statics;
		interface.export_names = export_names;
		interface.declared = declared;
		interface.uses = uses;
		interface.reads_target = reads_target;
		interface.complete = true;
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
