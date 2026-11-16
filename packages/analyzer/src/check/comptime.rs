//! Compile-time evaluation in the walk (advanced brief §12): `static`
//! declarations, `@` calls and operations, `@if`, `@for`, `@test`, the
//! provided statics `target` and `configuration`.
//!
//! Each handler checks its code like any other, then lowers it
//! (`comptime/lower.rs`) and runs it on the interpreter, once: the values
//! go to `Analysis::constants` (what the backends emit instead of the node),
//! `static_values`, `comptime_branches` and `unrolled`. The walker is the
//! interpreter's [`Host`]: it lowers the functions compile-time code calls
//! (analyzed by then, on demand) and evaluates module-level values.

use std::sync::Arc;

use fa_interpreter::ir::{Expr, FnKey, Function, GlobalKey};
use fa_interpreter::{Error, Host, Interpreter, Limits, Value};

use crate::check::Walker;
use crate::comptime::lower::{Lowerer, NotKnown};
use crate::comptime::{self, GlobalState, inputs};
use crate::diagnostics::codes::advanced::{
	META001, META002, META003, META005, META006, META007, META008, META009, META012, META013,
	MOD018, TEST003,
};
use crate::diagnostics::codes::analyzer::AN001;
use crate::diagnostics::codes::values::E_DECL_10;
use crate::ids::{FileId, NodeId, TypeId};
use crate::symbols::{DeclState, Resolution, SymbolKind};
use crate::syntax::{self, Name, NodeList, View};
use crate::types::TypeKind;

/// The compile-time operations of the compiler (`@readFile`…).
const OPERATIONS: [&str; 7] = [
	"readFile",
	"readDirectory",
	"parseJson",
	"typeToString",
	"typeToSchema",
	"import",
	"extern",
];

impl Walker<'_> {
	// ── evaluation ───────────────────────────────────────────────────────

	/// Lowers the compile-time expression `n`, checks that it is pure, and
	/// evaluates it; an error is reported at `at`.
	pub(super) fn evaluate_at(&mut self, n: NodeId, at: NodeId) -> Option<Value> {
		// an `@` call evaluated already: its value, or its error reported
		let n = self.strip_groups(n);
		if self.comptime_evaluated.contains(&n) {
			return self.a.constants.get(&n).cloned();
		}
		let lowered = self.with_rehydration(|w| {
			let externals = w.externals();
			let lowerer = Lowerer::new(&*w.s, w.file, &w.tree, &w.a).with_externals(externals);
			lowerer.compile_time_expression(n)
		});
		let (expr, slots) = match lowered {
			Ok(lowered) => lowered,
			Err(not_known) => {
				self.report_not_known(&not_known);
				return None;
			}
		};
		if let Some(what) = self.expr_impurity(&expr) {
			self.report_impure(at, &what);
			return None;
		}
		match self.run(&expr, slots) {
			Ok(value) => Some(value),
			Err(error) => {
				self.report_comptime_error(at, &error);
				None
			}
		}
	}

	/// Runs a lowering; when it needed modules of other units of a parallel
	/// run that this session doesn't hold, loads them again
	/// (`Session::rehydrate`) and runs it again.
	fn with_rehydration<T>(&mut self, mut lower: impl FnMut(&mut Self) -> T) -> T {
		let _ = crate::comptime::lower::take_missing();
		for _ in 0..8 {
			let result = lower(self);
			let missing = crate::comptime::lower::take_missing();
			if missing.is_empty() {
				return result;
			}
			let mut loaded = false;
			for module in missing {
				loaded |= self.s.rehydrate(module);
			}
			if !loaded {
				return result;
			}
		}
		lower(self)
	}

	/// What the proxies of this file stand for (`check/modules.rs`).
	fn externals(&mut self) -> rustc_hash::FxHashMap<crate::ids::SymbolId, Resolution> {
		let symbols: Vec<crate::ids::SymbolId> = self
			.modules
			.proxies
			.keys()
			.chain(self.modules.aliases.keys())
			.copied()
			.collect();
		symbols
			.into_iter()
			.filter_map(|symbol| Some((symbol, self.external_resolution(symbol)?)))
			.collect()
	}

	/// Runs lowered code on the interpreter, on a thread with a large stack.
	fn run(&mut self, expr: &Expr, slots: u32) -> Result<Value, Error> {
		let limits = Limits::default();
		fa_interpreter::on_large_stack(|| Interpreter::new(self, limits).evaluate(expr, slots))
	}

	/// The first impure primitive an expression may reach.
	fn expr_impurity(&mut self, expr: &Expr) -> Option<String> {
		comptime::impurity(expr, &mut |key| self.function_impurity(key))
	}

	/// The impure primitive a function may reach (`None` inside: pure),
	/// memoized; `None` when it can't be lowered (an error later).
	fn function_impurity(&mut self, key: FnKey) -> Option<Option<String>> {
		if let Some(found) = self.s.comptime.purity.get(&key) {
			return Some(found.clone());
		}
		// a cycle is pure until proven otherwise
		self.s.comptime.purity.insert(key, None);
		let function = self.function(key).ok()?;
		let mut found = comptime::impurity(&function.body, &mut |key| self.function_impurity(key));
		for param in &function.params {
			if found.is_none()
				&& let Some(default) = &param.default
			{
				found = comptime::impurity(default, &mut |key| self.function_impurity(key));
			}
		}
		self.s.comptime.purity.insert(key, found.clone());
		Some(found)
	}

	fn report_not_known(&mut self, not_known: &NotKnown) {
		// a compile-time parameter: known per instantiation only
		let parameter = matches!(self.a.resolution(not_known.node), Resolution::Local(symbol)
			if self.a.symbols.symbol(symbol).kind == SymbolKind::Static);
		let diagnostic = if parameter {
			self.diagnostic(
				&AN001,
				not_known.node,
				format!(
					"`{}` is a compile-time parameter: evaluating code that depends on it is not supported yet",
					not_known.name
				),
			)
		} else {
			self.diagnostic(
				&META001,
				not_known.node,
				format!(
					"`{}` is not known at compile time: compile-time code takes literals, static values, compile-time parameters and other compile-time operations",
					not_known.name
				),
			)
		};
		self.report(diagnostic);
	}

	fn report_impure(&mut self, at: NodeId, what: &str) {
		let message = if what.starts_with("the ") || what.starts_with("a ") {
			format!(
				"{what} can't be used at compile time: a function called at compile time must be pure"
			)
		} else {
			let name = what.rsplit('.').next().unwrap_or(what);
			format!("`{name}` is not deterministic: a function called at compile time must be pure")
		};
		let diagnostic = self.diagnostic(&META002, at, message);
		self.report(diagnostic);
	}

	/// Reports why an evaluation failed, at `at`.
	pub(super) fn report_comptime_error(&mut self, at: NodeId, error: &Error) {
		// thrown by the code of a type function's application: its message
		if self.in_type_function_code
			&& let Error::Thrown(value) = error
		{
			let message = match value {
				Value::Member(member) => match &member.data {
					Some(Value::Object(object)) => object
						.get("message")
						.and_then(|message| message.as_str().map(str::to_string)),
					_ => None,
				},
				_ => None,
			}
			.unwrap_or_else(|| format!("this type function's code throws `{}`", value.to_text()));
			let diagnostic = self.diagnostic(&crate::diagnostics::codes::types::TF004, at, message);
			self.report(diagnostic);
			return;
		}
		let (code, message) = match error {
			Error::Thrown(value) => (
				&META005,
				format!(
					"this compile-time evaluation throws `{}`: an error at compile time is a compile error",
					value.to_text()
				),
			),
			Error::Stop(reason) => (
				&META005,
				format!("this compile-time evaluation stops: {reason}"),
			),
			Error::Overflow(what) => (
				&META013,
				format!(
					"{what}: compile-time arithmetic follows the target (`{}`)",
					self.s.target()
				),
			),
			Error::Steps(limit) => (
				&META003,
				format!(
					"this compile-time evaluation runs more than {limit} steps: it may never end"
				),
			),
			Error::Depth(limit) => (
				&META003,
				format!(
					"this compile-time evaluation nests more than {limit} calls: it may never end"
				),
			),
			Error::Memory(limit) => (
				&META003,
				format!(
					"this compile-time evaluation uses more than {} MB of memory",
					limit >> 20
				),
			),
			Error::Impure(what) => {
				self.report_impure(at, what);
				return;
			}
			Error::Unsupported(what) => (
				&AN001,
				format!("{what} can't be evaluated at compile time yet"),
			),
		};
		let diagnostic = self.diagnostic(code, at, message);
		self.report(diagnostic);
	}

	/// The lowered function of a key: of this file (analyzed by now) or of
	/// another file whose analysis is done.
	fn lower_function(&mut self, key: FnKey) -> Result<Function, Error> {
		let (file, node) = (FileId(key.0), NodeId(key.1));
		let not_ready = || {
			Error::Unsupported(
				"a function whose analysis is not finished (a cycle through compile-time code)"
					.into(),
			)
		};
		if file == self.file {
			if let Some(symbol) = self.a.symbols.symbol_of_decl(node)
				&& self.a.symbols.symbol(symbol).state != DeclState::Done
			{
				return Err(not_ready());
			}
			return self
				.with_rehydration(|w| {
					let externals = w.externals();
					let lowerer =
						Lowerer::new(&*w.s, file, &w.tree, &w.a).with_externals(externals);
					lowerer.function(node)
				})
				.ok_or_else(not_ready);
		}
		if !self.s.source_file(file).done {
			return Err(not_ready());
		}
		self.with_rehydration(|w| {
			let tree = Arc::clone(&w.s.source_file(file).tree);
			let session: &crate::Session = w.s;
			let lowerer = Lowerer::new(session, file, &tree, session.analysis(file));
			lowerer.function(node)
		})
		.ok_or_else(not_ready)
	}

	/// The value of a module-level `let`, evaluated once.
	fn global_value(&mut self, key: GlobalKey) -> Result<Value, Error> {
		match self.s.comptime.globals.get(&key) {
			Some(GlobalState::Done(result)) => return result.clone(),
			Some(GlobalState::InProgress) => {
				return Err(Error::Stop("a module-level value depends on itself".into()));
			}
			None => {}
		}
		self.s.comptime.globals.insert(key, GlobalState::InProgress);
		let result = self.evaluate_global(key);
		self.s
			.comptime
			.globals
			.insert(key, GlobalState::Done(result.clone()));
		result
	}

	fn evaluate_global(&mut self, key: GlobalKey) -> Result<Value, Error> {
		let (file, node) = (FileId(key.0), NodeId(key.1));
		let tree = if file == self.file {
			Arc::clone(&self.tree)
		} else {
			if !self.s.source_file(file).done {
				return Err(Error::Unsupported(
					"a value of a file whose analysis is not finished".into(),
				));
			}
			Arc::clone(&self.s.source_file(file).tree)
		};
		let value = match syntax::view(&tree, node) {
			View::Declaration {
				value: Some(value), ..
			} => value,
			View::ExportValue { value, .. } => value,
			_ => return Err(Error::Unsupported("this module-level value".into())),
		};
		let lowered = self.with_rehydration(|w| {
			if file == w.file {
				let externals = w.externals();
				Lowerer::new(&*w.s, file, &tree, &w.a)
					.with_externals(externals)
					.with_globals()
					.compile_time_expression(value)
			} else {
				let session: &crate::Session = w.s;
				Lowerer::new(session, file, &tree, session.analysis(file))
					.with_globals()
					.compile_time_expression(value)
			}
		});
		let (expr, slots) =
			lowered.map_err(|not_known| Error::Unsupported(format!("`{}`", not_known.name)))?;
		if let Some(what) = self.expr_impurity(&expr) {
			return Err(Error::Impure(what));
		}
		Interpreter::new(self, Limits::default()).evaluate(&expr, slots)
	}

	// ── static values ────────────────────────────────────────────────────

	/// `static name = value`, `static name: Type = value` (advanced brief
	/// §12.2): evaluated once; its type is the exact type of its value
	/// (the annotation when there is one), and every read of it is the
	/// value.
	pub(super) fn check_static(
		&mut self,
		n: NodeId,
		name: Name,
		annotation: Option<NodeId>,
		value: Option<NodeId>,
	) -> TypeId {
		let declared = annotation.map(|annotation| self.check_type_expr(annotation));
		self.comptime_values += 1;
		let actual = value.map(|value| (value, self.check(value, declared)));
		self.comptime_values -= 1;
		if let (Some((value, actual)), Some(declared)) = (actual, declared) {
			self.sink(value, actual, declared);
		}
		self.memo(n, |w| {
			let text = w.tree.name(name).to_string();
			if value.is_none() {
				let diagnostic = w.diagnostic(
					&E_DECL_10,
					n,
					format!("`{text}` has no value: give it one, ex: `static {text} = …`"),
				);
				w.report(diagnostic);
			}
			let computed = value.and_then(|value| w.evaluate_at(value, value));
			if let (Some(at), Some(computed)) = (value, &computed) {
				w.check_literal_schema_value(at, computed);
			}
			let computed = match computed {
				Some(computed) if computed.has_function() => {
					let diagnostic = w.diagnostic(
						&AN001,
						value.unwrap_or(n),
						"a static value holding a function is not supported yet",
					);
					w.report(diagnostic);
					None
				}
				other => other,
			};
			let ty = match (declared, &computed, actual) {
				(Some(declared), _, _) => declared,
				(None, Some(computed), Some((_, actual))) => w.static_type(actual, computed),
				(None, None, Some((_, actual))) => actual,
				_ => TypeId::UNKNOWN,
			};
			let symbol = match w.a.symbols.symbol_of_decl(n) {
				Some(symbol) => symbol,
				None if name.is_empty() => return TypeId::VOID,
				None => w.declare(n, name, SymbolKind::Static),
			};
			let data = w.a.symbols.symbol_mut(symbol);
			data.kind = SymbolKind::Static;
			data.ty = ty;
			if let Some(computed) = computed {
				w.a.static_values.insert(symbol, computed.clone());
				w.a.constants.insert(n, computed);
			}
			TypeId::VOID
		})
	}

	// ── module-level constants ───────────────────────────────────────────

	/// A module-level `let` (or `export =` value) declared by `n` whose
	/// value is a constant expression: its value goes to
	/// `Analysis::module_constants` (minimal representations). Nothing is
	/// reported: a value that can't be evaluated is simply not a constant.
	pub(super) fn record_module_constant(&mut self, n: NodeId, value: NodeId) {
		let Some(symbol) = self.a.symbols.symbol_of_decl(n) else {
			return;
		};
		if self.a.module_constants.contains_key(&symbol) {
			return;
		}
		let data = self.a.symbols.symbol(symbol);
		let module_level = matches!(
			self.a.symbols.scope(data.scope).kind,
			crate::symbols::ScopeKind::Module | crate::symbols::ScopeKind::Namespace
		);
		if data.kind != SymbolKind::Let
			|| !module_level
			|| self.s.types.is_recovering(data.ty)
			|| !self.is_constant_expression(value, 0)
		{
			return;
		}
		let key = crate::comptime::lower::global_key(self.file, n);
		if let Ok(computed) = self.global_value(key)
			&& !computed.has_function()
		{
			self.a.module_constants.insert(symbol, computed);
		}
	}

	/// Whether an expression is built from literals and constants only:
	/// numbers, strings (templates of constants), booleans, `none`,
	/// arithmetic, comparisons and logic of constants, lists, tuples and
	/// objects of constants, and the names of statics and module constants.
	pub(super) fn is_constant_expression(&self, n: NodeId, depth: u32) -> bool {
		if depth > 64 {
			return false;
		}
		let constant = |node: NodeId| self.is_constant_expression(node, depth + 1);
		match self.view(n) {
			View::Integer(_)
			| View::Float(_)
			| View::BigInt(_)
			| View::String(_)
			| View::Boolean(_)
			| View::None => true,
			View::Template { parts } => parts.iter().all(|part| match part {
				syntax::TemplatePart::Text(_) => true,
				syntax::TemplatePart::Expression(node) => constant(node),
			}),
			View::Group(inner)
			| View::Negate(inner)
			| View::Not(inner)
			| View::Percentage(inner) => constant(inner),
			View::Equality { left, right, .. } => constant(left) && constant(right),
			View::Arithmetic { operands, .. }
			| View::Logic { operands, .. }
			| View::Comparison { operands, .. }
			| View::Array { items: operands }
			| View::Tuple { items: operands } => operands.iter().all(constant),
			View::Object { items } | View::TypedObject { members: items, .. } => {
				items.iter().all(|item| match self.view(item) {
					View::Assignment {
						value: Some(value), ..
					} => constant(value),
					View::Name(_) => constant(item),
					_ => false,
				})
			}
			View::Name(_) => match self.a.resolution(n) {
				Resolution::Local(symbol) => {
					self.a.module_constants.contains_key(&symbol)
						|| self.a.static_values.contains_key(&symbol)
				}
				_ => false,
			},
			_ => false,
		}
	}

	/// The type of a static value: its exact type, unless the value has a
	/// named type (an object type, a union) whose functions it keeps.
	fn static_type(&mut self, checked: TypeId, value: &Value) -> TypeId {
		let named = self.s.types.type_name(checked).is_some()
			|| matches!(
				self.s.types.kind(checked),
				TypeKind::NamedUnion(..)
					| TypeKind::Member(..)
					| TypeKind::Enum(..)
					| TypeKind::EnumMember(..)
					| TypeKind::Function(_)
			);
		if named || matches!(value, Value::Member(_) | Value::Function(_)) {
			checked
		} else {
			comptime::exact_type(&mut self.s.types, value)
		}
	}

	/// A read of a `static`: the read is its value (constant folding).
	pub(super) fn fold_static_read(&mut self, n: NodeId) {
		let Resolution::Local(symbol) = self.a.resolution(n) else {
			return;
		};
		if let Some(value) = self.a.static_values.get(&symbol) {
			let value = value.clone();
			self.a.constants.insert(n, value);
		} else if let Some(value) = self.external_static(symbol) {
			// a `static` of another file (its interface)
			self.a.constants.insert(n, value);
		}
	}

	/// `settings.port` where `settings` is a compile-time value: the field.
	pub(super) fn fold_static_fields(&mut self, n: NodeId, target: NodeId, members: NodeList<'_>) {
		let Some(mut value) = self.a.constants.get(&target).cloned() else {
			return;
		};
		for member in members.iter() {
			if !matches!(self.a.resolution(member), Resolution::Field(..)) {
				return;
			}
			let View::Name(name) = self.view(member) else {
				return;
			};
			let text = self.tree.name(name).to_string();
			match fa_interpreter::builtins::field(&value, &text) {
				Ok(field) => value = field,
				Err(_) => return,
			}
		}
		self.a.constants.insert(n, value);
	}

	// ── names ────────────────────────────────────────────────────────────

	/// A name written with `@` (type part): `@add` names a function called
	/// at compile time; `@version` on a value is `META012`. `None` when the
	/// name doesn't start with `@`.
	pub(super) fn check_at_name(&mut self, n: NodeId, text: &str) -> Option<TypeId> {
		let plain = text.strip_prefix('@')?;
		if OPERATIONS.contains(&plain) {
			let message = format!("`@{plain}` is called: `@{plain}(…)`");
			let diagnostic = self.diagnostic(&META012, n, message);
			self.report(diagnostic);
			return Some(TypeId::UNKNOWN);
		}
		let sym = self.s.types.syms.intern(plain);
		let Some(symbol) = self.a.symbols.lookup(self.body.scope, sym) else {
			let diagnostic = self.diagnostic(
				&crate::diagnostics::codes::values::E_DECL_1,
				n,
				format!("`{plain}` is not declared: `@` is written before a call of a function"),
			);
			self.report(diagnostic);
			return Some(TypeId::UNKNOWN);
		};
		self.set_resolution(n, Resolution::Local(symbol));
		if self.is_namespace(symbol) {
			return Some(self.namespace_as_value(n, plain));
		}
		let ty = self.symbol_type(symbol, n);
		let function = matches!(self.s.types.kind(ty), TypeKind::Function(_))
			|| self.s.types.is_recovering(ty);
		if !function {
			let message = format!(
				"`@` is written before a compile-time call, never on a value: a static value is used plainly, write `{plain}`"
			);
			let diagnostic = self.diagnostic(&META012, n, message);
			self.report(diagnostic);
		}
		Some(ty)
	}

	/// `target` and `configuration`, the statics the compiler provides
	/// (advanced brief §12.2): `None` for another name.
	pub(super) fn check_provided_static(&mut self, n: NodeId, text: &str) -> Option<TypeId> {
		let value = match text {
			"target" => Value::string(self.s.target()),
			"configuration" => match &self.s.comptime.configuration {
				Some(configuration) => configuration.clone(),
				None => crate::configuration::to_value(&crate::configuration::defaults()),
			},
			_ => return None,
		};
		self.a.reads_target = true;
		let ty = comptime::exact_type(&mut self.s.types, &value);
		self.a.constants.insert(n, value);
		Some(ty)
	}

	// ── `@` calls ────────────────────────────────────────────────────────

	/// Whether a callee is written with `@`.
	pub(super) fn is_at_callee(&self, callee: NodeId) -> bool {
		let mut node = callee;
		// `@tables.squares(5)`: through a namespace
		while let View::Access { target, .. } = self.view(node) {
			node = target;
		}
		matches!(self.view(node), View::Name(name) if self.tree.name(name).starts_with('@'))
	}

	/// The compile-time operations of the compiler (`@readFile(path)`…):
	/// `None` for a call of a function with `@`.
	pub(super) fn check_operation(
		&mut self,
		n: NodeId,
		callee: NodeId,
		arguments: NodeList<'_>,
	) -> Option<TypeId> {
		let View::Name(name) = self.view(callee) else {
			return None;
		};
		let operation = self.tree.name(name).strip_prefix('@')?.to_string();
		if !OPERATIONS.contains(&operation.as_str()) {
			return None;
		}
		let first = arguments.first();
		match operation.as_str() {
			"typeToString" | "typeToSchema" => {
				let ty = first.map(|first| self.check_type_expr(first));
				return Some(self.memo(n, |w| {
					let (Some(ty), Some(first)) = (ty, first) else {
						return TypeId::UNKNOWN;
					};
					if operation == "typeToString" {
						// the spelling at the use site (D49, spelling.rs)
						let text = if w.s.types.is_recovering(ty) {
							w.display(ty)
						} else {
							w.type_spelling(first)
						};
						let value = Value::string(&text);
						w.a.constants.insert(n, value);
						w.s.types.string_literal(&text)
					} else {
						let schema = comptime::schema::schema(&w.s.types, ty);
						let (defaults, order) = w.schema_defaults(ty);
						let schema = comptime::schema::with_defaults(schema, &defaults, &order);
						w.a.constants.insert(n, schema);
						crate::builtins::schema::schema(&mut w.s.types)
					}
				}));
			}
			// `use name = @import("…")` (modules.rs) is the form resolved
			"import" => {
				let tree = Arc::clone(&self.tree);
				let message = match crate::scan::import_source(&tree, n) {
					None => "the source of `@import` is a string literal: `@import(\"https://…/types.fa\")`".to_string(),
					Some(source) if source.starts_with('.') => format!(
						"`@import(\"{source}\")` names a local folder: local files are reached by their names, sibling packages through `dependencies.fa`"
					),
					Some(source) if self.import_proxy(n).is_none() => format!(
						"the source `{source}` of `@import` isn't resolved: `fa` resolves it when it builds the package (`fa check`)"
					),
					Some(_) => "`@import` is written in a `use`: `use weather = @import(\"https://…/types.fa\")`".to_string(),
				};
				let diagnostic =
					self.diagnostic(&crate::diagnostics::codes::advanced::META010, n, message);
				self.report(diagnostic);
				return Some(TypeId::UNKNOWN);
			}
			"extern" => {
				return Some(self.unsupported(n, &format!("`@{operation}`")));
			}
			_ => {}
		}
		let argument_ty = first.map(|first| self.check(first, Some(TypeId::STRING)));
		if let (Some(first), Some(actual)) = (first, argument_ty) {
			self.sink(first, actual, TypeId::STRING);
		}
		Some(self.memo(n, |w| {
			let Some(first) = first else {
				return TypeId::UNKNOWN;
			};
			w.comptime_evaluated.insert(n);
			let Some(argument) = w.evaluate_at(first, first) else {
				return TypeId::UNKNOWN;
			};
			let Some(text) = argument.as_str().map(str::to_string) else {
				return TypeId::UNKNOWN;
			};
			let value = match operation.as_str() {
				"readFile" => w.read_file(first, &text),
				"readDirectory" => w.read_directory(first, &text),
				_ => match fa_interpreter::json::parse(&text) {
					Ok(value) => Some(value),
					Err(error) => {
						w.report_comptime_error(n, &error);
						None
					}
				},
			};
			let Some(value) = value else {
				return TypeId::UNKNOWN;
			};
			let ty = comptime::exact_type(&mut w.s.types, &value);
			w.a.constants.insert(n, value);
			ty
		}))
	}

	/// The root-relative path of a path read from this file (`META008`).
	fn input_path(&mut self, at: NodeId, relative: &str) -> Option<String> {
		let from = self.s.display_path(self.file);
		match inputs::resolve(&from, relative) {
			Ok(path) => Some(path),
			Err(_) => {
				let diagnostic = self.diagnostic(
					&META008,
					at,
					format!(
						"`{relative}` is outside the root of the program: compile-time code reads the files of its package (or of its folder)"
					),
				);
				self.report(diagnostic);
				None
			}
		}
	}

	fn report_input_error(&mut self, at: NodeId, relative: &str, error: inputs::InputError) {
		let (code, message) = match error {
			inputs::InputError::Outside => (
				&META008,
				format!("`{relative}` is outside the root of the program"),
			),
			inputs::InputError::NotFound => (
				&META009,
				format!(
					"`{relative}` is not found: the path is relative to the folder of this file"
				),
			),
			inputs::InputError::NotText => {
				(&META009, format!("`{relative}` is not a text file (UTF-8)"))
			}
		};
		let diagnostic = self.diagnostic(code, at, message);
		self.report(diagnostic);
	}

	/// `@readFile(path)`: the text of a file, a build input.
	fn read_file(&mut self, at: NodeId, relative: &str) -> Option<Value> {
		let path = self.input_path(at, relative)?;
		match self.s.read_input(&path) {
			Ok(text) => {
				self.a.build_inputs.push(path);
				Some(Value::string(&text))
			}
			Err(error) => {
				self.report_input_error(at, relative, error);
				None
			}
		}
	}

	/// `@readDirectory(path)`: its entries sorted by name (D28), a build input.
	fn read_directory(&mut self, at: NodeId, relative: &str) -> Option<Value> {
		let path = self.input_path(at, relative)?;
		match self.s.list_input(&path) {
			Ok(entries) => {
				self.a.build_inputs.push(format!("{path}/"));
				let items = entries
					.into_iter()
					.map(|entry| {
						Value::object(vec![
							("name", Value::string(&entry.name)),
							(
								"path",
								Value::string(&inputs::entry_path(relative, &entry.name)),
							),
							("isDirectory", Value::Boolean(entry.is_directory)),
						])
					})
					.collect();
				Some(Value::array(items))
			}
			Err(error) => {
				self.report_input_error(at, relative, error);
				None
			}
		}
	}

	/// The schemas of the literal default values of an object type's fields
	/// (`@typeToSchema`: `default = Schema.StringLiteral { value = "hello" }`).
	pub(super) fn schema_defaults(&mut self, ty: TypeId) -> (Vec<(String, Value)>, Vec<String>) {
		if !matches!(self.s.types.kind(ty), TypeKind::Object(_)) {
			return (Vec::new(), Vec::new());
		}
		let (_, fields) = self.object_declaration(ty);
		let order: Vec<String> = fields
			.iter()
			.map(|field| self.s.types.syms.resolve(field.name).to_string())
			.collect();
		let mut defaults = Vec::new();
		for field in fields {
			let Some(default) = field.default else {
				continue;
			};
			let default = self.strip_groups(default);
			let tree = Arc::clone(&self.tree);
			let literal = match syntax::view(&tree, default) {
				View::String(text) => self.s.types.string_literal(text),
				View::Integer(_) | View::Boolean(_) | View::Float(_) | View::Negate(_) => {
					self.type_of(default)
				}
				_ => continue,
			};
			if !self.s.types.is_literal(literal) {
				continue;
			}
			let name = self.s.types.syms.resolve(field.name).to_string();
			defaults.push((name, comptime::schema::schema(&self.s.types, literal)));
		}
		(defaults, order)
	}

	/// After the checks of a call written `@f(…)`: evaluates it once.
	pub(super) fn evaluate_call(&mut self, n: NodeId) {
		// known per application of a type function only (schemas.rs)
		if self.comptime_evaluated.contains(&n) || self.deferred_comptime > 0 {
			return;
		}
		let value = self.evaluate_at(n, n);
		self.comptime_evaluated.insert(n);
		if let Some(value) = value {
			self.check_literal_schema_value(n, &value);
			self.a.constants.insert(n, value);
		}
	}

	// ── `@if`, `@for`, `@test` ───────────────────────────────────────────

	/// `@if condition { … } @else { … }` (D22): the condition is a
	/// compile-time `Boolean` (`META006`); only the branch kept is analyzed,
	/// the other one is only parsed.
	pub(super) fn check_comptime_if(
		&mut self,
		n: NodeId,
		condition: NodeId,
		then_body: NodeId,
		else_body: Option<NodeId>,
		expected: Option<TypeId>,
	) -> TypeId {
		let condition_ty = self.check(condition, Some(TypeId::BOOLEAN));
		if self.comptime_evaluated.insert(n) {
			let boolean = self.s.types.is_recovering(condition_ty)
				|| self
					.s
					.types
					.assignable(condition_ty, TypeId::BOOLEAN, crate::types::Mode::Copy)
					.is_ok();
			if !boolean {
				let message = format!(
					"the condition of `@if` is a `Boolean` known at compile time, not a `{}`",
					self.display(condition_ty)
				);
				let diagnostic = self.diagnostic(&META006, condition, message);
				self.report(diagnostic);
			} else if let Some(value) = self.evaluate_at(condition, condition) {
				match value {
					Value::Boolean(taken) => {
						self.a.comptime_branches.insert(n, taken);
					}
					other => {
						let message = format!(
							"the condition of `@if` is a `Boolean` known at compile time, not `{}`",
							other.to_text()
						);
						let diagnostic = self.diagnostic(&META006, condition, message);
						self.report(diagnostic);
					}
				}
			}
		}
		let branch = match self.a.comptime_branches.get(&n) {
			Some(true) => Some(then_body),
			Some(false) => else_body,
			None => None,
		};
		if self.is_statement(n) {
			if let Some(branch) = branch {
				self.check_stmt(branch);
			}
			return self.memo(n, |_| TypeId::VOID);
		}
		let ty = match branch {
			Some(branch) => self.check_block_value(branch, expected).0,
			None => TypeId::UNKNOWN,
		};
		self.memo(n, |_| ty)
	}

	/// `@for item in items { … }`: the items are known at compile time
	/// (`META007`) and the body is unrolled (`unrolled`): analyzed once, with
	/// the variable typed by the items. A range bounded by a compile-time
	/// parameter is known per instantiation only: it stays a loop.
	/// Returns `None` to let the loop be checked as a `for`.
	pub(super) fn evaluate_comptime_for(&mut self, n: NodeId, iterable: NodeId) {
		if !self.comptime_evaluated.insert(n) {
			return;
		}
		// its names resolved first
		if self.type_of(iterable).is_unset() {
			self.check(iterable, None);
		}
		let lowered = self.with_rehydration(|w| {
			let externals = w.externals();
			let lowerer = Lowerer::new(&*w.s, w.file, &w.tree, &w.a).with_externals(externals);
			lowerer.compile_time_expression(iterable)
		});
		let (expr, slots) = match lowered {
			Ok(lowered) => lowered,
			Err(not_known) => {
				let parameter = matches!(self.a.resolution(not_known.node), Resolution::Local(symbol)
					if self.a.symbols.symbol(symbol).kind == SymbolKind::Static);
				if !parameter {
					let message = format!(
						"the items of `@for` are known at compile time: `{}` is not",
						not_known.name
					);
					let diagnostic = self.diagnostic(&META007, iterable, message);
					self.report(diagnostic);
				}
				return;
			}
		};
		if let Some(what) = self.expr_impurity(&expr) {
			self.report_impure(iterable, &what);
			return;
		}
		// a key set: the dotted paths of its keys (fields.rs)
		let key_set = self.type_of(iterable);
		if self.is_fields(key_set) {
			match self.run(&expr, slots) {
				Ok(Value::Integer(bits)) => {
					let items = self.key_items(key_set, bits);
					self.a.unrolled.insert(n, items);
				}
				Ok(_) => {}
				Err(error) => self.report_comptime_error(iterable, &error),
			}
			return;
		}
		match self.run(&expr, slots) {
			Ok(value) => match fa_interpreter::builtins::iterate(&value) {
				Ok(items) => {
					self.a.unrolled.insert(n, items);
				}
				Err(error) => self.report_comptime_error(iterable, &error),
			},
			Err(error) => self.report_comptime_error(iterable, &error),
		}
	}

	/// `MOD018` (decision D27): a module-level `@for` contains no named
	/// declaration of the module (a function, a type, a namespace, an
	/// export): it would be declared once per item.
	pub(super) fn check_module_for_body(&mut self, body: NodeId) {
		let at_module_level = self.body.functions.is_empty()
			&& matches!(
				self.a.symbols.scope(self.body.scope).kind,
				crate::symbols::ScopeKind::Module | crate::symbols::ScopeKind::Namespace
			);
		if !at_module_level {
			return;
		}
		let tree = Arc::clone(&self.tree);
		let View::Block { statements } = syntax::view(&tree, body) else {
			return;
		};
		for statement in statements.iter() {
			// a `let` of the body is a local of each item, like in a `for`;
			// a function, a type, a namespace would be hoisted to the module
			let named = matches!(
				syntax::view(&tree, statement),
				View::TypeDecl { .. }
					| View::Namespace { .. }
					| View::Function { .. }
					| View::ExportValue { .. }
					| View::ExportFunction { .. }
					| View::ExportType { .. }
					| View::ExportNamespace(_)
			);
			if named {
				let message = "a module-level `@for` can't declare a name: it would be declared once per item. Declare it before the loop, or collect the values in a static array".to_string();
				let diagnostic = self.diagnostic(&MOD018, statement, message);
				self.report(diagnostic);
			}
		}
	}

	/// An `@for` whose body reads its variable at compile time (an `@if`
	/// condition or an `@` call argument naming it): the body is analyzed
	/// once per item, the variable known (`Analysis::comptime_locals`), and
	/// the branches and values of each item are kept
	/// (`Analysis::unrolled_items`). `None` for another `@for`.
	pub(super) fn check_comptime_for_per_item(
		&mut self,
		n: NodeId,
		pattern: Option<NodeId>,
		index: Option<crate::syntax::Name>,
		iterable: NodeId,
		body: NodeId,
	) -> Option<TypeId> {
		let pattern = pattern?;
		let items = self.a.unrolled.get(&n).cloned()?;
		if !self.reads_variable_at_compile_time(pattern, body) {
			return None;
		}
		let mut nodes = Vec::new();
		let mut stack = vec![body];
		while let Some(node) = stack.pop() {
			nodes.push(node);
			stack.extend(syntax::children(&self.tree, node));
		}
		let mut results = Vec::with_capacity(items.len());
		let mut ty = TypeId::VOID;
		for item in items {
			// the compile-time results of the previous item are forgotten
			for &node in &nodes {
				if self.comptime_evaluated.remove(&node) {
					self.a.constants.remove(&node);
					self.a.comptime_branches.remove(&node);
					if self.is_at_call(node) {
						self.set_type(node, TypeId::UNSET);
					}
				}
			}
			self.a.comptime_locals.insert(pattern, item);
			ty = self.check_for(n, pattern.into(), index, iterable, body, None, false);
			let mut result = crate::analysis::ItemResults::default();
			for &node in &nodes {
				if let Some(&branch) = self.a.comptime_branches.get(&node) {
					result.branches.insert(node, branch);
				}
				if let Some(value) = self.a.constants.get(&node) {
					result.constants.insert(node, value.clone());
				}
			}
			results.push(result);
		}
		self.a.comptime_locals.remove(&pattern);
		self.a.unrolled_items.insert(n, results);
		Some(ty)
	}

	/// Whether a node is a call written `@f(…)` or `@readFile(…)`.
	fn is_at_call(&self, node: NodeId) -> bool {
		matches!(self.view(node), View::Call { callee, .. }
			if matches!(self.view(callee), View::Name(name) if self.tree.name(name).starts_with('@')))
	}

	/// Whether an `@if` condition or an `@` call argument of `body` names
	/// the variable declared by `pattern`.
	fn reads_variable_at_compile_time(&self, pattern: NodeId, body: NodeId) -> bool {
		let View::Name(variable) = self.view(pattern) else {
			return false;
		};
		let variable = self.tree.name(variable);
		let mentions = |root: NodeId| {
			let mut stack = vec![root];
			while let Some(node) = stack.pop() {
				if matches!(self.view(node), View::Name(name) if self.tree.name(name) == variable) {
					return true;
				}
				stack.extend(syntax::children(&self.tree, node));
			}
			false
		};
		let mut stack = vec![body];
		while let Some(node) = stack.pop() {
			match self.view(node) {
				View::If {
					condition,
					compile_time: true,
					..
				} if mentions(condition) => return true,
				View::Call { arguments, .. }
					if self.is_at_call(node) && arguments.iter().any(mentions) =>
				{
					return true;
				}
				_ => {}
			}
			stack.extend(syntax::children(&self.tree, node));
		}
		false
	}

	/// `@test "name" { … }`: run while checking (advanced brief §18); a
	/// failure is `TEST003` at the test.
	pub(super) fn run_comptime_test(&mut self, n: NodeId, body: NodeId) {
		if !self.comptime_evaluated.insert(n) {
			return;
		}
		let lowered = self.with_rehydration(|w| {
			let externals = w.externals();
			let lowerer = Lowerer::new(&*w.s, w.file, &w.tree, &w.a).with_externals(externals);
			lowerer.compile_time_expression(body)
		});
		let (expr, slots) = match lowered {
			Ok(lowered) => lowered,
			Err(not_known) => {
				self.report_not_known(&not_known);
				return;
			}
		};
		if let Some(what) = self.expr_impurity(&expr) {
			self.report_impure(n, &what);
			return;
		}
		match self.run(&expr, slots) {
			Ok(_) => {}
			Err(Error::Stop(reason)) => {
				let diagnostic = self.diagnostic(
					&TEST003,
					n,
					format!("this compile-time test fails: {reason}"),
				);
				self.report(diagnostic);
			}
			Err(Error::Thrown(error)) => {
				let diagnostic = self.diagnostic(
					&TEST003,
					n,
					format!(
						"this compile-time test fails: it throws `{}`",
						error.to_text()
					),
				);
				self.report(diagnostic);
			}
			Err(error) => self.report_comptime_error(n, &error),
		}
	}
}

impl Host for Walker<'_> {
	fn function(&mut self, key: FnKey) -> Result<Arc<Function>, Error> {
		if let Some(function) = self.s.comptime.functions.get(&key) {
			return Ok(Arc::clone(function));
		}
		let function = Arc::new(self.lower_function(key)?);
		self.s.comptime.functions.insert(key, Arc::clone(&function));
		Ok(function)
	}

	fn global(&mut self, key: GlobalKey) -> Result<Value, Error> {
		self.global_value(key)
	}
}
