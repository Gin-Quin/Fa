//! Multi-file programs (advanced brief §15.7, D10): one ES module per Fa
//! file.
//!
//! - A declaration other files use (a file's export, a member of one of its
//!   namespaces) is exported under its *export name*: the file's name, then
//!   the namespace path, joined with `$` (`counter$count`). Namespace members
//!   are module-level declarations named that way.
//! - A name resolved to another file ([`Resolution::External`]) is imported:
//!   `import { counter$count as counter$count } from "../library/counter.js"`.
//!   An `export mutable` value is read through its ES live binding; it is
//!   written through an accessor object exported next to it (`people$ref.v =
//!   …`), since an importer can't assign a binding.
//! - The module-level values of a module are module-level `let`s, each
//!   assigned by an exported initialization function (`$init12`); the entry
//!   module calls them in the order the analyzer computed
//!   ([`fa_analyzer::Program::init_order`]), never relying on the order of
//!   `import` evaluation, and again before each test (D33).
//! - The top-level statements of a program file (a standalone file, a
//!   script) are its exported `$main`.

use std::collections::{BTreeMap, HashMap};

use fa_analyzer::ids::{FileId, NodeId, SymbolId};
use fa_analyzer::symbols::{Resolution, SymbolKind};
use fa_analyzer::syntax::{self, BindingKind, View};
use fa_analyzer::{FileRole, Session};

use crate::js::{Expr, Writer};

use super::Generator;

/// A declaration of another module.
#[derive(Clone, Debug)]
pub(crate) struct External {
	/// The path of its JavaScript module, relative to the output folder.
	pub path: String,
	/// The name it is exported as.
	pub export: String,
	/// An `export mutable` value (or a `mutable` member): written through
	/// its accessor `<export>$ref`.
	pub mutable: bool,
}

/// What the generator of one module knows about the program (the other
/// modules are read through their interfaces, `Session::interface`).
#[derive(Clone, Debug, Default)]
pub(crate) struct ProgramCx {
	/// The path of this module (`library/math.js`).
	pub path: String,
	/// The export names of this module's declarations.
	pub exports: HashMap<NodeId, String>,
	/// Whether the file is a program (top-level statements in `$main`).
	pub is_program: bool,
	/// `fa test`: a test file exports its tests.
	pub tests: bool,
}

/// The JavaScript path of a Fa module (`library/(utils)/format` →
/// `library/(utils)/format.js`).
pub(crate) fn js_path(module: &str) -> String {
	// a standalone `main.fa` beside other files: `main.js` is the entry module
	if module == "main" {
		return "main$.js".to_string();
	}
	format!("{module}.js")
}

/// The path of `to` relative to the folder of `from` (`./x.js`, `../a/b.js`).
pub(crate) fn relative(from: &str, to: &str) -> String {
	let from_parts: Vec<&str> = from.split('/').collect();
	let from_folder = &from_parts[..from_parts.len().saturating_sub(1)];
	let to_parts: Vec<&str> = to.split('/').collect();
	let common = from_folder
		.iter()
		.zip(&to_parts)
		.take_while(|(a, b)| a == b)
		.count();
	let mut out = String::new();
	if common == from_folder.len() {
		out.push_str("./");
	} else {
		for _ in common..from_folder.len() {
			out.push_str("../");
		}
	}
	out.push_str(&to_parts[common..].join("/"));
	out
}

/// The export names of a file's declarations that other files may use:
/// its export, the members of its namespaces (recursively), the functions
/// of its types, as its interface lists them (the analyzer computes them,
/// so that no other file's tree is read).
pub(crate) fn export_names(session: &Session, file: FileId) -> HashMap<NodeId, String> {
	session
		.source_file(file)
		.interface
		.export_names
		.iter()
		.map(|(&decl, name)| (decl, name.clone()))
		.collect()
}

/// The statements of a namespace's braces.
pub(crate) fn namespace_items(tree: &fa_parser::TypedSyntaxTree, value: NodeId) -> Vec<NodeId> {
	match syntax::view(tree, value) {
		View::Block { statements } | View::Object { items: statements } => {
			statements.iter().collect()
		}
		_ => Vec::new(),
	}
}

/// The context of a module of a program: its path, its export names.
pub(crate) fn program_context(session: &Session, file: FileId) -> ProgramCx {
	ProgramCx {
		path: js_path(session.module(file)),
		exports: export_names(session, file),
		is_program: session.role(file) == FileRole::Program,
		tests: false,
	}
}

impl Generator<'_> {
	/// The local name of an import (declared on first use). In a hot
	/// build (D43), the expression reading it through the registry of its
	/// module: `$h0.ns.name`.
	pub(crate) fn import(&mut self, path: &str, export: &str) -> String {
		if self.options.hot {
			self.helper("faHotModule");
			let holder = match self.hot_modules.get(path) {
				Some(holder) => holder.clone(),
				None => {
					let holder = self.scopes.declare_global("$h");
					self.hot_modules.insert(path.to_string(), holder.clone());
					holder
				}
			};
			return format!("{holder}.ns.{export}");
		}
		if let Some(local) = self.imports.get(path).and_then(|names| names.get(export)) {
			return local.clone();
		}
		let local = self.scopes.declare_global(export);
		self.imports
			.entry(path.to_string())
			.or_default()
			.insert(export.to_string(), local.clone());
		local
	}

	/// The external declaration a resolution designates.
	pub(crate) fn external(&self, resolution: Resolution) -> Option<External> {
		self.program?;
		let (module, decl) = self.session.external_node(resolution)?;
		// read from the other module's interface (one shared lookup, §4 #13)
		let interface = self.session.interface(module)?;
		let export = interface.export_names.get(&decl)?.clone();
		let kind = interface.declared(decl).map(|declared| declared.kind);
		Some(External {
			path: js_path(self.types.module_path(module)),
			export,
			mutable: kind == Some(SymbolKind::Mutable),
		})
	}

	/// A name of another module, as a value.
	pub(crate) fn external_value(&mut self, n: NodeId) -> Option<Expr> {
		let external = self.external(self.analysis.resolution(n))?;
		let local = self.import(&external.path, &external.export);
		Some(Expr::name(local, !external.mutable))
	}

	/// A name of another module as an assignment target: its accessor.
	pub(crate) fn external_place(&mut self, n: NodeId) -> Option<String> {
		let external = self.external(self.analysis.resolution(n))?;
		let accessor = self.import(&external.path, &format!("{}$ref", external.export));
		Some(format!("{accessor}.v"))
	}

	/// `library.math.add`, `counter.count`: the prefix of an access that
	/// goes through namespaces (folders, other files' exports, namespaces of
	/// this file), as the value of the member it reaches and the number of
	/// members used. `None` when the target is not a namespace.
	pub(crate) fn namespace_prefix(
		&mut self,
		target: NodeId,
		members: &[NodeId],
	) -> Option<(Expr, usize)> {
		if !self.is_namespace_node(target) {
			return None;
		}
		// the last member of the module path: a function of a type follows its type
		let mut reached = None;
		for (index, &member) in members.iter().enumerate() {
			// a type declared inside the type reached (`library.Farm.Animal.make`)
			if reached.is_some()
				&& self
					.analysis
					.has_flags(member, fa_analyzer::analysis::NodeFlags::TYPE_EXPR)
			{
				continue;
			}
			match self.analysis.resolution(member) {
				Resolution::Folder(_) => {}
				Resolution::External(..) => {
					if !self.is_namespace_node(member) {
						reached = Some(index);
					}
				}
				Resolution::Local(symbol) => {
					if self.analysis.symbols.symbol(symbol).kind != SymbolKind::Namespace {
						reached = Some(index);
					}
				}
				_ => break,
			}
		}
		let index = reached?;
		let member = members[index];
		let value = match self.analysis.resolution(member) {
			Resolution::External(..) => self.external_value(member)?,
			Resolution::Local(symbol) => {
				let name = self.name_of(symbol);
				let stable = self.is_constant(symbol);
				Expr::name(name, stable)
			}
			_ => return None,
		};
		Some((value, index + 1))
	}

	/// Whether a node names a namespace: a folder, a namespace of this file
	/// or another file's (an export or a member that is a namespace).
	fn is_namespace_node(&self, n: NodeId) -> bool {
		match self.analysis.resolution(n) {
			Resolution::Folder(_) => true,
			Resolution::Local(symbol) => {
				self.analysis.symbols.symbol(symbol).kind == SymbolKind::Namespace
			}
			resolution @ Resolution::External(..) => self.is_namespace_decl(resolution),
			_ => false,
		}
	}

	/// Whether an external declaration is a namespace (from its node kind).
	fn is_namespace_decl(&self, resolution: Resolution) -> bool {
		let Some((module, decl)) = self.session.external_node(resolution) else {
			return false;
		};
		self.program.is_some()
			&& self.session.interface(module).is_some_and(|interface| {
				interface.export_names.contains_key(&decl)
					&& interface
						.declared(decl)
						.is_some_and(|declared| declared.kind == SymbolKind::Namespace)
			})
	}

	/// Whether a `use` statement is a name alias of a module path (D14):
	/// the analyzer resolved its uses to their targets, nothing to generate.
	pub(crate) fn is_name_alias(&self, n: NodeId) -> bool {
		let value = match self.view(n) {
			View::Use {
				value,
				mutable: false,
				..
			} => value,
			View::Destructuring {
				kind: BindingKind::Use,
				value,
				..
			} => value,
			_ => return false,
		};
		// `use c = std.c` (foreign code): its types only
		if let Some(symbol) = self.analysis.symbols.symbol_of_decl(n)
			&& self.analysis.symbols.symbol(symbol).kind
				== fa_analyzer::symbols::SymbolKind::NameAlias
		{
			return true;
		}
		let mut current = self.strip_groups(value);
		loop {
			match self.view(current) {
				View::Access { target, .. } => current = target,
				View::Name(_) => break,
				// `use weather = @import("…")`: resolved by the analyzer
				View::Call { .. } => {
					return fa_analyzer::scan::import_source(self.tree, current).is_some();
				}
				_ => return false,
			}
		}
		match self.analysis.resolution(current) {
			// `use { Mutex } = std.concurrency`
			Resolution::Folder(_) | Resolution::External(..) | Resolution::Builtin(_) => true,
			Resolution::Local(symbol) => {
				self.analysis.symbols.symbol(symbol).kind == SymbolKind::Namespace
			}
			_ => false,
		}
	}

	/// The member declarations of the namespaces among `statements`
	/// (recursively), their JavaScript names declared with their path
	/// (`counter$count`).
	pub(crate) fn namespace_members(&mut self, statements: &[NodeId]) -> Vec<NodeId> {
		let mut members = Vec::new();
		for &statement in statements {
			let (value, prefix) = match self.view(statement) {
				View::ExportNamespace(value) => {
					let Some(symbol) = self.analysis.symbols.symbol_of_decl(statement) else {
						continue;
					};
					let name = self
						.types
						.syms
						.resolve(self.analysis.symbols.symbol(symbol).name);
					(value, name.to_string())
				}
				View::Namespace { name, value } => (value, self.tree.name(name).to_string()),
				_ => continue,
			};
			self.namespace_members_of(value, &prefix, &mut members);
		}
		members
	}

	fn namespace_members_of(&mut self, value: NodeId, prefix: &str, out: &mut Vec<NodeId>) {
		for member in namespace_items(self.tree, value) {
			let Some(symbol) = self.analysis.symbols.symbol_of_decl(member) else {
				continue;
			};
			let name = self
				.types
				.syms
				.resolve(self.analysis.symbols.symbol(symbol).name)
				.to_string();
			let qualified = format!("{prefix}${}", name.trim_start_matches('#'));
			self.declare_as(symbol, &qualified);
			if let View::Namespace { value, .. } = self.view(member) {
				self.namespace_members_of(value, &qualified, out);
			} else {
				out.push(member);
			}
		}
	}

	/// The `import` lines of the module: its imports of other modules (the
	/// runtime helpers are imported by `generate_program`).
	pub(crate) fn import_lines(&self) -> Vec<String> {
		let Some(program) = self.program else {
			return Vec::new();
		};
		let mut lines = Vec::new();
		// a hot build: each module through its registry (`faHotModule`)
		let hot: BTreeMap<&String, &String> = self.hot_modules.iter().collect();
		for (path, holder) in hot {
			lines.push(format!(
				"import * as {holder}$m from \"{}\";",
				relative(&program.path, path)
			));
			lines.push(format!(
				"const {holder} = faHotModule({}, {holder}$m);",
				crate::js::string_literal(path)
			));
		}
		let imports: BTreeMap<&String, &BTreeMap<String, String>> = self.imports.iter().collect();
		for (path, names) in imports {
			let names: Vec<String> = names
				.iter()
				.map(|(export, local)| {
					if export == local {
						export.clone()
					} else {
						format!("{export} as {local}")
					}
				})
				.collect();
			lines.push(format!(
				"import {{ {} }} from \"{}\";",
				names.join(", "),
				relative(&program.path, path)
			));
		}
		lines
	}

	/// The TypeScript declarations of an export of the module (`.d.ts`): a
	/// function with its instances when it may suspend (as
	/// `instance_exports` exports them), a value with its accessor cell.
	fn export_typings(&mut self, decl: NodeId, symbol: SymbolId, export: &str) {
		let info = self.analysis.symbols.symbol(symbol);
		let (kind, ty) = (info.kind, info.ty);
		match kind {
			SymbolKind::Function => {
				let value = match self.view(decl) {
					View::Function { value, .. } | View::ExportFunction { value, .. } => value,
					_ => decl,
				};
				let Some((parameters, ret, is_async)) =
					crate::declarations::signature(self.types, self.analysis, value)
				else {
					self.typings
						.push(format!("export declare const {export}: unknown;"));
					return;
				};
				let task_first = if parameters.is_empty() {
					"task: unknown".to_string()
				} else {
					format!("task: unknown, {parameters}")
				};
				let suspends =
					is_async && self.summary(decl).is_some_and(|summary| summary.suspends);
				if !suspends {
					self.typings.push(format!(
						"export declare function {export}({parameters}): {ret};"
					));
					return;
				}
				match self.tasks.instances.get(&decl) {
					// synchronous mode: one instance, blocking
					None => {
						self.typings.push(format!(
							"export declare function {export}({parameters}): {ret};"
						));
						self.typings.push(format!(
							"export declare function {export}$sync({parameters}): {ret};"
						));
					}
					Some(instances) => {
						let (promise, sync, asynchronous) = (
							instances.promise_name.is_some(),
							instances.sync_name.is_some(),
							instances.async_name.is_some(),
						);
						if promise {
							self.typings.push(format!(
								"/** Runs the call as a task: the promise rejects with its Fa error. */\nexport declare function {export}({parameters}): Promise<{ret}>;"
							));
						} else if sync {
							self.typings.push(format!(
								"export declare function {export}({parameters}): {ret};"
							));
						} else if asynchronous {
							self.typings.push(format!(
								"export declare function {export}({task_first}): Promise<{ret}>;"
							));
						}
						if sync {
							self.typings.push(format!(
								"/** The synchronous instance: a blocking call (Bun and Node). */\nexport declare function {export}$sync({parameters}): {ret};"
							));
						}
						if asynchronous {
							self.typings.push(format!(
								"/** The async instance, for Fa modules: the caller's task first. */\nexport declare function {export}$async({task_first}): Promise<{ret}>;"
							));
						}
					}
				}
			}
			SymbolKind::Let => {
				let ty = crate::declarations::typescript(self.types, ty);
				self.typings
					.push(format!("export declare const {export}: {ty};"));
			}
			SymbolKind::Mutable => {
				let ty = crate::declarations::typescript(self.types, ty);
				self.typings
					.push(format!("export declare let {export}: {ty};"));
				self.typings.push(format!(
					"/** The value, read and written by other modules. */\nexport declare const {export}$ref: {{ v: {ty} }};"
				));
			}
			_ => {}
		}
	}

	/// The `export` lines of the module: its declarations other files use
	/// (and the accessors of its mutable ones), its initialization functions.
	pub(crate) fn export_lines(&mut self, inits: &[(String, String)]) -> Vec<String> {
		let Some(program) = self.program else {
			return Vec::new();
		};
		let mut lines = Vec::new();
		let mut exported: Vec<String> = Vec::new();
		let mut exports: Vec<(&NodeId, &String)> = program.exports.iter().collect();
		exports.sort();
		for (&decl, export) in exports {
			let Some(symbol) = self.analysis.symbols.symbol_of_decl(decl) else {
				continue;
			};
			// types, namespaces and `static`s have no runtime binding: their
			// uses in other modules are folded (statics) or need nothing
			if matches!(
				self.analysis.symbols.symbol(symbol).kind,
				SymbolKind::Type | SymbolKind::Namespace | SymbolKind::Static
			) {
				continue;
			}
			let Some(local) = self.names.get(&symbol).cloned() else {
				continue;
			};
			if self.options.declarations {
				self.export_typings(decl, symbol, export);
			}
			// a function that may suspend: its instances too (tasks.rs)
			if let Some(instances) = self.instance_exports(decl, &local, export) {
				exported.extend(instances);
				continue;
			}
			if self.analysis.symbols.symbol(symbol).kind == SymbolKind::Mutable {
				lines.push(format!(
					"export const {export}$ref = {{ get v() {{ return {local}; }}, set v(value) {{ {local} = value; }} }};"
				));
			}
			let generics = self.function_generics(decl);
			let mut numbers = vec![None];
			numbers.extend(
				self.integer_variants
					.get(&decl)
					.cloned()
					.unwrap_or_default()
					.into_iter()
					.map(Some),
			);
			for references in self.reference_variants(decl) {
				let reference_suffix = self.reference_suffix(&generics, &references);
				let mut layouts = vec![fa_analyzer::analysis::ReferenceLayout::General];
				let saved = self.reference_bindings.clone();
				self.reference_bindings.extend(references.iter().copied());
				if let Some(layout) = self.analysis.reference_layouts.get(&decl).copied() {
					let object = self
						.analysis
						.function(decl)
						.and_then(|info| info.sig)
						.is_some_and(|sig| self.is_object_reference(self.types.sig(sig).ret));
					if !object && layout != fa_analyzer::analysis::ReferenceLayout::General {
						layouts.push(layout);
					}
				}
				self.reference_bindings = saved;
				for numeric in &numbers {
					for &layout in &layouts {
						let numeric_suffix = numeric
							.as_ref()
							.map(|variant| self.variant_suffix(decl, variant))
							.unwrap_or_default();
						let suffix = format!(
							"{numeric_suffix}{reference_suffix}{}",
							Self::reference_layout_suffix(layout)
						);
						if !suffix.is_empty() {
							exported.push(format!("{local}{suffix} as {export}{suffix}"));
						}
					}
				}
			}
			exported.push(if local == *export {
				local
			} else {
				format!("{local} as {export}")
			});
		}
		for (function, export) in inits {
			exported.push(format!("{function} as {export}"));
		}
		if !exported.is_empty() {
			lines.push(format!("export {{ {} }};", exported.join(", ")));
		}
		lines
	}

	/// The symbol of a declaration, when it is a module-level value.
	pub(crate) fn value_symbol(&self, decl: NodeId) -> Option<SymbolId> {
		let symbol = self.analysis.symbols.symbol_of_decl(decl)?;
		matches!(
			self.analysis.symbols.symbol(symbol).kind,
			SymbolKind::Let | SymbolKind::Mutable | SymbolKind::Use | SymbolKind::MutableUse
		)
		.then_some(symbol)
	}

	/// A place reached through namespaces: a `mutable` member of a namespace
	/// of this file, an `export mutable` value of another, then fields.
	pub(crate) fn namespace_prefix_place(
		&mut self,
		target: NodeId,
		members: &[NodeId],
	) -> Option<super::objects::Place> {
		if !self.is_namespace_node(target) {
			return None;
		}
		let used = members
			.iter()
			.position(|&member| match self.analysis.resolution(member) {
				Resolution::Folder(_) => false,
				Resolution::Local(symbol) => {
					self.analysis.symbols.symbol(symbol).kind != SymbolKind::Namespace
				}
				resolution @ Resolution::External(..) => !self.is_namespace_decl(resolution),
				_ => true,
			})?;
		let reached = members[used];
		let mut text = match self.analysis.resolution(reached) {
			Resolution::External(..) if used + 1 == members.len() => {
				self.external_place(reached)?
			}
			Resolution::External(..) => self.external_value(reached)?.text,
			Resolution::Local(symbol) => self.name_of(symbol),
			_ => return None,
		};
		for &member in &members[used + 1..] {
			let Resolution::Field(shape, index) = self.analysis.resolution(member) else {
				return None;
			};
			let field = self.types.shape_fields(shape)[index as usize];
			text = format!(
				"{text}.{}",
				Self::property(self.types.syms.resolve(field.name))
			);
		}
		let last = *members.last()?;
		Some(super::objects::Place::new(text, self.type_of(last)))
	}

	/// `fa test` (advanced brief §18.3): the tests of a test file as
	/// functions, and the exported list `tests` the runner reads
	/// (`faRunTests`): `{ name, file, line, modifier, run }`.
	pub(crate) fn test_functions(&mut self, statements: &[NodeId]) -> Writer {
		let mut out = Writer::new();
		if !self.program.is_some_and(|program| program.tests) {
			return out;
		}
		let mut entries = Vec::new();
		for &statement in statements {
			let View::Test {
				name,
				modifier,
				body,
				..
			} = self.view(statement)
			else {
				continue;
			};
			let function = self.scopes.declare_global("$test");
			self.fcx = super::FnCx::default();
			// a test that may suspend, in an asynchronous program: an async
			// function taking its task (`faTestTask`); the helper marks the
			// program for the entry module (`entry_module`)
			let asynchronous = self.tasks.asynchronous
				&& self
					.analysis
					.async_summaries
					.get(&statement)
					.is_some_and(|summary| summary.suspends);
			if asynchronous {
				self.fcx.mode = super::tasks::Mode::Async { coop: true };
				self.helper("faTestTask");
			}
			self.out = Writer::at(1);
			self.scopes.push();
			self.block_into(body, super::Dest::Discard);
			self.scopes.pop();
			let inner = std::mem::take(&mut self.out);
			let temps = std::mem::take(&mut self.fcx.inline_temps);
			out.blank();
			if asynchronous {
				out.line(format!("async function {function}($task) {{"));
			} else {
				out.line(format!("function {function}() {{"));
			}
			if !temps.is_empty() {
				out.line(format!("\tlet {};", temps.join(", ")));
			}
			out.append(inner);
			out.line("}");
			let text = match self.view(name) {
				View::String(text) => text.to_string(),
				_ => self.text(name).trim_matches('"').to_string(),
			};
			let modifier = match modifier {
				Some(syntax::TestModifier::Skip) => "\"skip\"",
				Some(syntax::TestModifier::Only) => "\"only\"",
				None => "undefined",
			};
			let span = self.tree.spans[statement.index()];
			let (line, _) = self.source.position(span.start as u32);
			let kind = if asynchronous { ", async: true" } else { "" };
			entries.push(format!(
				"\t{{ name: {}, file: {}, line: {line}, modifier: {modifier}, run: {function}{kind} }},",
				crate::js::string_literal(&text),
				crate::js::string_literal(&self.options.path)
			));
		}
		out.blank();
		out.line("export const tests = [");
		for entry in entries {
			out.line(entry);
		}
		out.line("];");
		out
	}
}
