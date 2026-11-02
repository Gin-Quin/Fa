//! The code generator: one walk over the analyzed tree of a file.
//!
//! Statements are written into a [`Writer`]; expressions are built as
//! [`Expr`] strings. An expression that needs statements (an `if` with
//! blocks, a `do`, a `loop`, a `return` in a value…) is *lifted*: the
//! statements are written before the statement being built, and the value
//! goes through a temporary (values brief §19). The operands evaluated before
//! a lifted construct are stored in temporaries first when they are not
//! stable, so the left-to-right order of evaluation is kept. No IIFE.
//!
//! The submodules split the walk by area: `statements` (blocks, control
//! flow, declarations), `expressions` (names, operators, lifting),
//! `numbers` (arithmetic and the checks the analyzer asks for), `strings`
//! (literals, interpolation, printing, string methods) and `functions`
//! (functions, lambdas and calls).

pub(crate) mod collections;
mod constants;
mod errors;
mod expressions;
mod functions;
mod generics;
pub(crate) mod modules;
mod number_api;
mod numbers;
mod objects;
mod patterns;
pub(crate) mod references;
mod relations;
mod statements;
mod strings;
mod when;

use std::collections::{BTreeSet, HashMap, HashSet};

use fa_analyzer::analysis::{Analysis, NodeFlags};
use fa_analyzer::diagnostics::codes::analyzer::AN001;
use fa_analyzer::diagnostics::{Code, Location, SourceView};
use fa_analyzer::ids::{FileId, NodeId, ScopeId, SymbolId, TypeId};
use fa_analyzer::symbols::{Resolution, ScopeKind, SymbolKind};
use fa_analyzer::syntax::{self, View};
use fa_analyzer::types::{FloatBits, IntStorage, TypeKind, TypeStore};
use fa_analyzer::{Diagnostic, Session};
use fa_parser::TypedSyntaxTree;

use crate::js::{Expr, Writer};
use crate::names::Scopes;
use crate::{Options, Target};

/// How a value is represented in JavaScript.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub(crate) enum Repr {
	/// A number: integers up to 32 bits, `Integer`, ranged integers, floats, percentages.
	Number,
	/// A bigint: `Int64`, `UInt64`, `Int128`, `UInt128`, `BigInt`.
	BigInt,
	/// A string: `String`, `Character`.
	String,
	/// A boolean.
	Boolean,
	/// `undefined`: `none`.
	None,
	/// Anything else (objects, collections, functions, unions).
	Other,
}

/// The number kind of a type, for the arithmetic.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub(crate) enum Kind {
	/// An integer of this storage.
	Int(IntStorage),
	/// `Float` or `Float32`.
	Float(FloatBits),
	/// `BigInt`.
	BigInt,
	/// `Percentage`.
	Percentage,
	/// `String` or `Character`.
	Text,
	/// `Boolean`.
	Boolean,
	/// Anything else.
	Other,
}

/// Where the value of a block or a construct goes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Dest {
	/// Dropped: a statement.
	Discard,
	/// Returned from the current function.
	Return,
	/// Assigned to a JavaScript name (a temporary or a declared value).
	Assign(String),
}

/// What a loop does with the value of a `break`.
#[derive(Clone, Debug)]
struct LoopCx {
	/// Where a `break value` puts its value.
	dest: Dest,
}

/// The state of the JavaScript function being generated.
#[derive(Default, Debug)]
struct FnCx {
	/// The next temporary number.
	temps: u32,
	/// Temporaries assigned inside expressions (`($t0 = f())`), declared at
	/// the start of the function.
	inline_temps: Vec<String>,
	/// The enclosing loops, innermost last.
	loops: Vec<LoopCx>,
	/// The function or lambda node (`FnInfo::lambda`); `None` in `main`.
	function: Option<NodeId>,
}

/// The generator of one file.
pub(crate) struct Generator<'s> {
	pub(crate) tree: &'s TypedSyntaxTree,
	pub(crate) analysis: &'s Analysis,
	pub(crate) types: &'s TypeStore,
	pub(crate) options: &'s Options,
	file: FileId,
	source: SourceView<'s>,
	/// The runtime helpers used.
	pub(crate) helpers: BTreeSet<&'static str>,
	/// Errors: constructs not supported yet, `E-NUM-19`.
	pub(crate) errors: Vec<Diagnostic>,
	/// The JavaScript name of each symbol declared so far.
	names: HashMap<SymbolId, String>,
	/// The names visible at the current point.
	scopes: Scopes,
	/// The statements being written.
	pub(crate) out: Writer,
	/// The current function.
	fcx: FnCx,
	/// The scope of the module.
	module_scope: Option<ScopeId>,
	/// Top-level values read by functions: declared at the module level.
	module_values: HashSet<SymbolId>,
	/// Whether the program reads `arguments`.
	uses_arguments: bool,
	/// Module-level constants (union singletons, enumeration tables), in
	/// the order they were first needed: `(name, code)`.
	constants: Vec<(String, String)>,
	/// Symbols read as another expression: the variable of a `for` loop
	/// over an array whose items are written through it (`items[$t0]`).
	aliases: HashMap<SymbolId, String>,
	/// The aliases that read a value of a map (`references.rs`): the map and
	/// the key a write sets.
	pub(crate) map_aliases: HashMap<SymbolId, (String, String)>,
	/// What `references.rs` knows of each reference (`use`, `mutable use`).
	pub(crate) references: HashMap<SymbolId, references::Reference>,
	/// The declarations of object types, by type (`relations.rs`).
	decls_of_types: relations::DeclsOfTypes,
	/// Module-level `let`s of the references read by functions: the
	/// container and the key of a primitive place (`references.rs`).
	pub(crate) module_slots: Vec<String>,
	/// The roots of the places a reference may designate: a whole write of
	/// an object or a collection there is done in place (`faAssign`, D60).
	/// Computed at the first write.
	referenced_roots: Option<HashSet<SymbolId>>,
	/// The hidden parameters of the requirements of the enclosing named
	/// functions, innermost last: `(key, JavaScript name)` (`references.rs`).
	pub(crate) requirement_scopes: Vec<Vec<(TypeId, String)>>,
	/// The value of each `provide` statement: the name of its temporary, or
	/// `None` for a primitive place, read again at each call.
	pub(crate) provided: HashMap<NodeId, Option<String>>,
	/// The generated copy function of each type (empty while generated).
	copies: HashMap<TypeId, String>,
	/// The hidden parameters of the generic functions (`generics.rs`).
	hidden_names: generics::HiddenNames,
	/// The array locals that `faCopyInto` may reuse (`objects.rs`).
	reusable_arrays: HashMap<SymbolId, bool>,
	/// The generated `==` of each object type (empty while generated).
	equalities: HashMap<TypeId, String>,
	/// The generated error classes, by qualified member name.
	classes: HashMap<String, String>,
	/// Module-level class declarations, in the order they were needed.
	declarations: Vec<Vec<String>>,
	/// The types whose formatter is being generated (recursive types).
	formatting: HashSet<TypeId>,
	/// Module values read by functions whose value is a literal: module-level
	/// `const`s (`(name, value)`) instead of `let`s assigned in `main`.
	module_constants: Vec<(String, String)>,
	/// In a program of several modules: what this module knows of the others.
	pub(crate) program: Option<&'s modules::ProgramCx>,
	/// The imports of other modules: path → export → local name.
	pub(crate) imports: HashMap<String, std::collections::BTreeMap<String, String>>,
	/// Whether the module exports `$main`.
	has_main: bool,
}

/// The result of a walk: the code of the module body and what it needs.
pub(crate) struct Generated {
	pub(crate) code: String,
	pub(crate) helpers: Vec<&'static str>,
	pub(crate) errors: Vec<Diagnostic>,
	/// In a program: whether the module exports `$main` (its top-level statements).
	pub(crate) has_main: bool,
}

/// Generates the JavaScript of an analyzed file.
pub(crate) fn generate_file(session: &Session, file: FileId, options: &Options) -> Generated {
	generate_module(session, file, options, None)
}

/// Generates the JavaScript of an analyzed file, as a module of a program
/// of several modules when `program` is given.
pub(crate) fn generate_module(
	session: &Session,
	file: FileId,
	options: &Options,
	program: Option<&modules::ProgramCx>,
) -> Generated {
	let tree = session.tree(file);
	let analysis = session.analysis(file);
	let mut generator = Generator {
		tree,
		analysis,
		types: session.types(),
		options,
		file,
		source: SourceView::new(&options.path, session.text(file)),
		helpers: BTreeSet::new(),
		errors: Vec::new(),
		names: HashMap::new(),
		scopes: Scopes::new(),
		out: Writer::new(),
		fcx: FnCx::default(),
		module_scope: None,
		module_values: HashSet::new(),
		uses_arguments: false,
		constants: Vec::new(),
		aliases: HashMap::new(),
		map_aliases: HashMap::new(),
		references: HashMap::new(),
		referenced_roots: None,
		module_slots: Vec::new(),
		decls_of_types: HashMap::new(),
		requirement_scopes: Vec::new(),
		provided: HashMap::new(),
		copies: HashMap::new(),
		equalities: HashMap::new(),
		reusable_arrays: HashMap::new(),
		hidden_names: HashMap::new(),
		classes: HashMap::new(),
		declarations: Vec::new(),
		formatting: HashSet::new(),
		module_constants: Vec::new(),
		program,
		imports: HashMap::new(),
		has_main: false,
	};
	let code = generator.program();
	Generated {
		code,
		helpers: generator.helpers.into_iter().collect(),
		errors: generator.errors,
		has_main: generator.has_main,
	}
}

impl<'s> Generator<'s> {
	// ── the program ──────────────────────────────────────────────────────

	/// The whole module: module-level declarations, the functions, then the
	/// top-level statements in a `main` function run by `faRun`.
	fn program(&mut self) -> String {
		let root = NodeId::new(self.tree.root);
		let View::Module { statements } = self.view(root) else {
			return String::new();
		};
		let statements: Vec<NodeId> = statements.iter().collect();
		self.module_scope = self
			.analysis
			.symbols
			.scopes()
			.iter()
			.position(|scope| scope.kind == ScopeKind::Module)
			.map(ScopeId::new);
		self.find_module_values();

		// module-level names: the functions (hoisted), then the values read by functions
		let mut functions = Vec::new();
		let mut others = Vec::new();
		// the members of namespaces are module-level declarations (modules.rs)
		let members = self.namespace_members(&statements);
		let module_file = self.program.is_some_and(|program| !program.is_program);
		let mut values = Vec::new();
		let mut all = statements.clone();
		all.extend(members.iter().copied());
		self.type_function_names(&all);
		for &statement in &all {
			match self.view(statement) {
				View::Function { .. } | View::ExportFunction { .. } => {
					if let Some(symbol) = self.analysis.symbols.symbol_of_decl(statement) {
						self.declare(symbol);
					}
					functions.push(statement);
				}
				View::TypeDecl { .. } | View::ExportType { .. } => functions.push(statement),
				View::Test { .. } | View::Namespace { .. } | View::ExportNamespace(_) => {}
				_ if self.is_name_alias(statement) => {}
				View::Declaration { .. } | View::ExportValue { .. }
					if module_file || members.contains(&statement) =>
				{
					if let Some(symbol) = self.value_symbol(statement) {
						self.module_values.insert(symbol);
					}
					values.push(statement);
				}
				_ => others.push(statement),
			}
		}
		let mut module_values: Vec<SymbolId> = self.module_values.iter().copied().collect();
		module_values.sort();
		module_values.retain(|&symbol| {
			// a reference to a primitive place has no name of its own
			!matches!(
				self.analysis.symbols.symbol(symbol).kind,
				SymbolKind::Use | SymbolKind::MutableUse
			) || self.module_reference_has_name(symbol)
		});
		let module_lets: Vec<String> = module_values
			.iter()
			.map(|&symbol| self.declare(symbol))
			.collect();
		self.prepare_module_references();

		// the functions, at the module level
		let mut body = Writer::new();
		for &function in &functions {
			self.out = Writer::new();
			self.statement(function);
			body.blank();
			body.append(std::mem::take(&mut self.out));
		}

		// the module-level values of a module: one initialization function each
		let mut inits = Vec::new();
		for &value in &values {
			let function = self.scopes.declare_global("$init");
			self.fcx = FnCx::default();
			self.out = Writer::at(1);
			self.statement(value);
			let inner = std::mem::take(&mut self.out);
			let temps = std::mem::take(&mut self.fcx.inline_temps);
			body.blank();
			body.line(format!("function {function}() {{"));
			if !temps.is_empty() {
				body.line(format!("\tlet {};", temps.join(", ")));
			}
			body.append(inner);
			body.line("}");
			inits.push((function, format!("$init{}", value.index())));
		}

		// the top-level statements, in `main`
		let main_name = if others.is_empty() {
			None
		} else {
			Some(self.scopes.declare("main"))
		};
		let mut main = Writer::new();
		if let Some(main_name) = &main_name {
			self.scopes.push();
			self.fcx = FnCx::default();
			self.out = Writer::at(1);
			self.statements(&others, Dest::Discard);
			let inner = std::mem::take(&mut self.out);
			let temps = std::mem::take(&mut self.fcx.inline_temps);
			self.scopes.pop();
			main.line(format!("function {main_name}() {{"));
			if !temps.is_empty() {
				main.line(format!("\tlet {};", temps.join(", ")));
			}
			main.append(inner);
			main.line("}");
			main.blank();
			if self.program.is_some() {
				// run by the entry module of the program (modules.rs)
				inits.push((main_name.clone(), "$main".to_string()));
				self.has_main = true;
			} else {
				self.helper("faRun");
				main.line(format!("faRun({main_name});"));
			}
		}
		// `fa test`: the tests of a test file, as functions (modules.rs)
		let tests = self.test_functions(&all);
		body.append(tests);
		let exports = self.export_lines(&inits);

		// assembly
		let mut out = Writer::new();
		for line in self.import_lines() {
			out.line(line);
		}
		if self.uses_arguments {
			let value = match self.options.target {
				Target::Web => "[]".to_string(),
				Target::Deno => "Deno.args".to_string(),
				Target::Bun | Target::Node => "process.argv.slice(2)".to_string(),
			};
			out.line(format!("const $arguments = {value};"));
		}
		for declaration in std::mem::take(&mut self.declarations) {
			out.blank();
			for line in declaration {
				out.line(line);
			}
		}
		out.blank();
		for (name, code) in std::mem::take(&mut self.constants) {
			out.line(format!("const {name} = {code};"));
		}
		let module_constants = std::mem::take(&mut self.module_constants);
		for (name, value) in &module_constants {
			out.line(format!("const {name} = {value};"));
		}
		let mut module_lets: Vec<String> = module_lets
			.into_iter()
			.filter(|name| {
				module_constants
					.iter()
					.all(|(constant, _)| constant != name)
			})
			.collect();
		module_lets.append(&mut self.module_slots);
		if !module_lets.is_empty() {
			out.line(format!("let {};", module_lets.join(", ")));
		}
		out.blank();
		out.append(body);
		out.blank();
		out.append(main);
		if !exports.is_empty() {
			out.blank();
			for line in exports {
				out.line(line);
			}
		}
		out.finish()
	}

	/// The top-level values read inside a function or a lambda: they must be
	/// declared at the module level, outside of `main`. A reference read by
	/// a function also needs the values its place is made of.
	fn find_module_values(&mut self) {
		let Some(module_scope) = self.module_scope else {
			return;
		};
		// the function bodies, nested ones merged into their outer one
		let mut bodies: Vec<(usize, usize)> = self
			.analysis
			.functions
			.values()
			.map(|info| {
				let span = self.tree.spans[info.lambda.index()];
				(span.start, span.end)
			})
			.collect();
		bodies.sort_unstable();
		let mut merged: Vec<(usize, usize)> = Vec::with_capacity(bodies.len());
		for (start, end) in bodies {
			match merged.last_mut() {
				Some(last) if start <= last.1 => last.1 = last.1.max(end),
				_ => merged.push((start, end)),
			}
		}
		let inside = |start: usize, end: usize| {
			let index = merged.partition_point(|&(body_start, _)| body_start <= start);
			index > 0 && end <= merged[index - 1].1
		};
		let module_level = |symbol: SymbolId| {
			let info = self.analysis.symbols.symbol(symbol);
			info.scope == module_scope
				&& matches!(
					info.kind,
					SymbolKind::Let
						| SymbolKind::Mutable
						| SymbolKind::Use | SymbolKind::MutableUse
				)
		};
		let mut found = Vec::new();
		for index in 0..self.tree.nodes.len() {
			let Resolution::Local(symbol) = self.analysis.resolution(NodeId::new(index)) else {
				continue;
			};
			let span = self.tree.spans[index];
			if module_level(symbol) && inside(span.start, span.end) {
				found.push(symbol);
			}
		}
		// the names of the place of a reference read by a function
		while let Some(symbol) = found.pop() {
			if !self.module_values.insert(symbol) {
				continue;
			}
			let info = self.analysis.symbols.symbol(symbol);
			if !matches!(info.kind, SymbolKind::Use | SymbolKind::MutableUse) {
				continue;
			}
			let mut pending = vec![info.decl_node];
			while let Some(node) = pending.pop() {
				if let Resolution::Local(part) = self.analysis.resolution(node)
					&& part != symbol
					&& module_level(part)
				{
					found.push(part);
				}
				pending.extend(syntax::children(self.tree, node));
			}
		}
	}

	// ── names ────────────────────────────────────────────────────────────

	/// Declares the JavaScript name of a symbol in the current scope.
	pub(crate) fn declare(&mut self, symbol: SymbolId) -> String {
		if let Some(name) = self.names.get(&symbol) {
			return name.clone();
		}
		let wanted = self
			.types
			.syms
			.resolve(self.analysis.symbols.symbol(symbol).name)
			.to_string();
		let name = self.scopes.declare(&wanted);
		self.names.insert(symbol, name.clone());
		name
	}

	/// Declares the JavaScript name of a symbol, spelled `wanted`.
	pub(crate) fn declare_as(&mut self, symbol: SymbolId, wanted: &str) -> String {
		let name = self.scopes.declare(wanted);
		self.names.insert(symbol, name.clone());
		name
	}

	/// The JavaScript name of a symbol (declared on first use if needed), or
	/// the expression it is an alias of.
	pub(crate) fn name_of(&mut self, symbol: SymbolId) -> String {
		if let Some(alias) = self.aliases.get(&symbol) {
			return alias.clone();
		}
		match self.names.get(&symbol) {
			Some(name) => name.clone(),
			None => self.declare(symbol),
		}
	}

	/// Makes a symbol an alias of an expression (until the end of the walk).
	pub(crate) fn alias(&mut self, symbol: SymbolId, expression: String) {
		self.aliases.insert(symbol, expression);
	}

	/// A module-level class declaration (its lines, `{name}` standing for
	/// its name): returns its name.
	pub(crate) fn class(&mut self, wanted: &str, lines: Vec<String>) -> String {
		let name = self.scopes.declare_global(wanted);
		let lines = lines
			.into_iter()
			.map(|line| line.replace("{name}", &name))
			.collect();
		self.declarations.push(lines);
		name
	}

	/// A module-level constant, declared once: returns its name.
	pub(crate) fn constant(&mut self, wanted: &str, code: String) -> String {
		if let Some((name, _)) = self
			.constants
			.iter()
			.find(|(_, existing)| *existing == code)
		{
			return name.clone();
		}
		let name = self.scopes.declare_global(wanted);
		self.constants.push((name.clone(), code));
		name
	}

	/// Whether a symbol is never reassigned.
	pub(crate) fn is_constant(&self, symbol: SymbolId) -> bool {
		!fa_analyzer::lower::copies::is_reassignable(self.analysis, symbol)
	}

	/// Records a module value as a module-level constant.
	pub(crate) fn module_constant(&mut self, name: String, value: String) {
		self.module_constants.push((name, value));
	}

	/// Whether a top-level value is declared at the module level.
	pub(crate) fn is_module_value(&self, symbol: SymbolId) -> bool {
		self.module_values.contains(&symbol)
	}

	/// A new temporary name.
	pub(crate) fn temp(&mut self) -> String {
		let name = format!("$t{}", self.fcx.temps);
		self.fcx.temps += 1;
		name
	}

	/// A new temporary assigned inside an expression, declared at the start
	/// of the function.
	pub(crate) fn inline_temp(&mut self) -> String {
		let name = self.temp();
		self.fcx.inline_temps.push(name.clone());
		name
	}

	/// Opens a JavaScript scope (a block).
	pub(crate) fn push_scope(&mut self) {
		self.scopes.push();
	}

	/// Closes the innermost JavaScript scope.
	pub(crate) fn pop_scope(&mut self) {
		self.scopes.pop();
	}

	/// The function or lambda being generated (`None` in `main`).
	pub(crate) fn current_function(&self) -> Option<NodeId> {
		self.fcx.function
	}

	/// Runs `f` with an empty function state (the body of `function`, a
	/// `FnInfo::lambda`), and returns its result with the inline temporaries
	/// the body declared.
	pub(crate) fn in_function<T>(
		&mut self,
		function: NodeId,
		f: impl FnOnce(&mut Self) -> T,
	) -> (T, Vec<String>) {
		let saved = std::mem::replace(
			&mut self.fcx,
			FnCx {
				function: Some(function),
				..FnCx::default()
			},
		);
		self.scopes.push();
		let result = f(self);
		self.scopes.pop();
		let fcx = std::mem::replace(&mut self.fcx, saved);
		(result, fcx.inline_temps)
	}

	/// Runs `f` with an empty writer at an indentation: returns the lines it
	/// wrote and its result.
	pub(crate) fn capture_at<T>(
		&mut self,
		indent: usize,
		f: impl FnOnce(&mut Self) -> T,
	) -> (Writer, T) {
		let saved = std::mem::replace(&mut self.out, Writer::at(indent));
		let result = f(self);
		let lines = std::mem::replace(&mut self.out, saved);
		(lines, result)
	}

	/// [`Generator::capture_at`] at the current indentation.
	pub(crate) fn capture<T>(&mut self, f: impl FnOnce(&mut Self) -> T) -> (Writer, T) {
		let indent = self.out.indentation();
		self.capture_at(indent, f)
	}

	// ── the analysis ─────────────────────────────────────────────────────

	/// The view of a node.
	pub(crate) fn view(&self, n: NodeId) -> View<'s> {
		syntax::view(self.tree, n)
	}

	/// The node with its parentheses removed.
	pub(crate) fn strip_groups(&self, n: NodeId) -> NodeId {
		let mut current = n;
		while let View::Group(inner) = self.view(current) {
			current = inner;
		}
		current
	}

	/// The type of a node.
	pub(crate) fn type_of(&self, n: NodeId) -> TypeId {
		self.analysis.type_of(n)
	}

	/// Whether a node has a flag. Without overflow checks
	/// ([`Options::overflow_checks`]), the overflow and sink flags are never
	/// set: the operations stay plain JavaScript operations.
	pub(crate) fn has(&self, n: NodeId, flag: NodeFlags) -> bool {
		let mut flags = self.analysis.flags_of(n);
		if !self.options.overflow_checks {
			flags.remove(
				NodeFlags::OVERFLOW_JS | NodeFlags::OVERFLOW_NATIVE | NodeFlags::SINK_CHECK,
			);
		}
		flags.contains(flag)
	}

	/// The source text of a node.
	pub(crate) fn text(&self, n: NodeId) -> &'s str {
		self.tree.text(n.index())
	}

	/// How a type is represented.
	pub(crate) fn repr(&self, ty: TypeId) -> Repr {
		match self.types.kind(ty) {
			TypeKind::Int(id) => {
				if self.types.int(id).storage.is_js_bigint() {
					Repr::BigInt
				} else {
					Repr::Number
				}
			}
			TypeKind::Float(_) | TypeKind::FloatLit(..) | TypeKind::Percentage => Repr::Number,
			TypeKind::BigInt | TypeKind::BigIntLit(_) => Repr::BigInt,
			TypeKind::String | TypeKind::StringLit(_) | TypeKind::Character => Repr::String,
			TypeKind::Boolean | TypeKind::BoolLit(_) => Repr::Boolean,
			TypeKind::None => Repr::None,
			_ => Repr::Other,
		}
	}

	/// The number kind of a type.
	pub(crate) fn kind(&self, ty: TypeId) -> Kind {
		match self.types.kind(ty) {
			TypeKind::Int(id) => Kind::Int(self.types.int(id).storage),
			TypeKind::Float(bits) | TypeKind::FloatLit(bits, _) => Kind::Float(bits),
			TypeKind::BigInt | TypeKind::BigIntLit(_) => Kind::BigInt,
			TypeKind::Percentage => Kind::Percentage,
			TypeKind::String | TypeKind::StringLit(_) | TypeKind::Character => Kind::Text,
			TypeKind::Boolean | TypeKind::BoolLit(_) => Kind::Boolean,
			_ => Kind::Other,
		}
	}

	/// The `file:line:column` of a node, as a JavaScript string literal.
	pub(crate) fn location(&self, n: NodeId) -> String {
		let span = self.tree.spans[n.index()];
		let (line, column) = self.source.position(span.start as u32);
		crate::js::string_literal(&format!("{}:{line}:{column}", self.options.path))
	}

	// ── output ───────────────────────────────────────────────────────────

	/// Records that the program uses a runtime helper (and returns its name).
	pub(crate) fn helper(&mut self, name: &'static str) -> &'static str {
		debug_assert!(
			crate::runtime::helper(name).is_some(),
			"unknown helper {name}"
		);
		self.helpers.insert(name);
		name
	}

	/// Reports an error at a node.
	pub(crate) fn error(&mut self, code: Code, n: NodeId, message: impl Into<String>) {
		let span = syntax::span(self.tree, n);
		let location = Location {
			file: self.file,
			span,
		};
		self.errors.push(Diagnostic::new(code, location, message));
	}

	/// A construct the JavaScript backend doesn't generate yet: an error and
	/// a placeholder value.
	pub(crate) fn unsupported(&mut self, n: NodeId, what: &str) -> Expr {
		self.error(
			&AN001,
			n,
			format!("{what} is not supported by the JavaScript backend yet"),
		);
		Expr::constant("undefined", crate::js::Prec::Primary)
	}

	/// Notes that the program reads `arguments`.
	pub(crate) fn use_arguments(&mut self) -> Expr {
		self.uses_arguments = true;
		Expr::name("$arguments", true)
	}
}
