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

use fa_analyzer::ids::{FileId, ModuleId, NodeId, SymbolId};
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

/// What the generator of one module knows about the program.
#[derive(Clone, Debug, Default)]
pub(crate) struct ProgramCx {
	/// The declarations other modules export, by module and declaration node.
	pub externals: HashMap<(ModuleId, NodeId), External>,
	/// The path of this module (`library/math.js`).
	pub path: String,
	/// The export names of this module's declarations.
	pub exports: HashMap<NodeId, String>,
	/// Whether the file is a program (top-level statements in `$main`).
	pub is_program: bool,
	/// The namespace declarations of every module.
	pub namespace_decls: std::collections::HashSet<(ModuleId, NodeId)>,
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
/// its export, the members of its namespaces (recursively).
pub(crate) fn export_names(session: &Session, file: FileId) -> HashMap<NodeId, String> {
	let tree = session.tree(file);
	let mut names = HashMap::new();
	let root = NodeId::new(tree.root);
	let View::Module { statements } = syntax::view(tree, root) else {
		return names;
	};
	let own = session.own_name(file).unwrap_or_else(|| {
		let module = session.module(file);
		module.rsplit('/').next().unwrap_or(module).to_string()
	});
	for statement in statements.iter() {
		match syntax::view(tree, statement) {
			View::ExportValue { .. } | View::ExportFunction { .. } | View::ExportType { .. } => {
				names.entry(statement).or_insert_with(|| own.clone());
			}
			View::ExportNamespace(value) => {
				names.entry(statement).or_insert_with(|| own.clone());
				namespace_names(session, file, value, &own, &mut names);
			}
			View::Namespace { name, value } => {
				let prefix = tree.name(name).to_string();
				namespace_names(session, file, value, &prefix, &mut names);
			}
			_ => {}
		}
	}
	// the functions of the types (methods called from other files): `Type$name`
	let analysis = session.analysis(file);
	let mut type_functions = Vec::new();
	for decl in analysis.type_decls.values() {
		let type_name = session
			.types()
			.syms
			.resolve(analysis.symbols.symbol(decl.symbol).name);
		for &function in &decl.functions {
			let symbol = analysis.symbols.symbol(function);
			let name = session.types().syms.resolve(symbol.name);
			if !name.starts_with('#') {
				type_functions.push((symbol.decl_node, format!("{type_name}${name}")));
			}
		}
	}
	for (decl, name) in type_functions {
		names.entry(decl).or_insert(name);
	}
	names
}

/// The export names of the members of a namespace (its braces `value`).
fn namespace_names(
	session: &Session,
	file: FileId,
	value: NodeId,
	prefix: &str,
	names: &mut HashMap<NodeId, String>,
) {
	let tree = session.tree(file);
	let analysis = session.analysis(file);
	for member in namespace_items(tree, value) {
		let Some(symbol) = analysis.symbols.symbol_of_decl(member) else {
			continue;
		};
		let name = session
			.types()
			.syms
			.resolve(analysis.symbols.symbol(symbol).name);
		// a private member is only used inside its file (and tests aren't members)
		if name.starts_with('#') || analysis.symbols.symbol(symbol).kind == SymbolKind::Test {
			continue;
		}
		let qualified = format!("{prefix}${name}");
		if let View::Namespace { value, .. } = syntax::view(tree, member) {
			namespace_names(session, file, value, &qualified, names);
		}
		names.insert(member, qualified);
	}
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

/// The context of every module of a program: the externals of each file
/// (`ids`: the module id of each file).
pub(crate) fn program_contexts(
	session: &Session,
	files: &[FileId],
	ids: &HashMap<FileId, ModuleId>,
) -> Vec<ProgramCx> {
	let mut externals = HashMap::new();
	let mut namespace_decls = std::collections::HashSet::new();
	let mut exports_by_file = Vec::new();
	for &file in files {
		let module = session.module(file).to_string();
		let id = ids.get(&file).copied();
		let exports = export_names(session, file);
		let tree = session.tree(file);
		for &decl in exports.keys() {
			if let (Some(id), View::Namespace { .. } | View::ExportNamespace(_)) =
				(id, syntax::view(tree, decl))
			{
				namespace_decls.insert((id, decl));
			}
		}
		let analysis = session.analysis(file);
		for (&decl, export) in &exports {
			let mutable = analysis
				.symbols
				.symbol_of_decl(decl)
				.is_some_and(|symbol| analysis.symbols.symbol(symbol).kind == SymbolKind::Mutable);
			if let Some(id) = id {
				externals.insert(
					(id, decl),
					External {
						path: js_path(&module),
						export: export.clone(),
						mutable,
					},
				);
			}
		}
		exports_by_file.push(exports);
	}
	files
		.iter()
		.zip(exports_by_file)
		.map(|(&file, exports)| ProgramCx {
			externals: externals.clone(),
			path: js_path(session.module(file)),
			exports,
			is_program: session.role(file) == FileRole::Program,
			namespace_decls: namespace_decls.clone(),
			tests: false,
		})
		.collect()
}

impl Generator<'_> {
	/// The local name of an import (declared on first use).
	pub(crate) fn import(&mut self, path: &str, export: &str) -> String {
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
	fn external(&self, resolution: Resolution) -> Option<External> {
		let Resolution::External(module, decl) = resolution else {
			return None;
		};
		self.program?.externals.get(&(module, decl)).cloned()
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
		let Resolution::External(module, decl) = resolution else {
			return false;
		};
		self.program
			.is_some_and(|program| program.namespace_decls.contains(&(module, decl)))
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
				_ => return false,
			}
		}
		match self.analysis.resolution(current) {
			Resolution::Folder(_) | Resolution::External(..) => true,
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
			let Some(local) = self.names.get(&symbol).cloned() else {
				continue;
			};
			if self.analysis.symbols.symbol(symbol).kind == SymbolKind::Mutable {
				lines.push(format!(
					"export const {export}$ref = {{ get v() {{ return {local}; }}, set v(value) {{ {local} = value; }} }};"
				));
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
			SymbolKind::Let | SymbolKind::Mutable
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
			self.out = Writer::at(1);
			self.scopes.push();
			self.block_into(body, super::Dest::Discard);
			self.scopes.pop();
			let inner = std::mem::take(&mut self.out);
			let temps = std::mem::take(&mut self.fcx.inline_temps);
			out.blank();
			out.line(format!("function {function}() {{"));
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
			entries.push(format!(
				"\t{{ name: {}, file: {}, line: {line}, modifier: {modifier}, run: {function} }},",
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
