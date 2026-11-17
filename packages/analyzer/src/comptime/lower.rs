//! Lowering of the checked tree to the interpreter's code (`fa_interpreter::ir`).
//!
//! Everything the analysis decided is made explicit: a local is a frame
//! slot, a module-level `let` a global, a `static` its value, a call its
//! callee and its arguments in parameter order, arithmetic the range of its
//! result under the target (decision D49). Code with an outside effect
//! (`console`, a module-level `mutable`, a requirement, `arguments`) becomes
//! [`Expr::Impure`]; a construct the interpreter doesn't run yet becomes
//! [`Expr::Unsupported`]: both are errors only when compile-time code
//! reaches them (or, for the impure ones, may reach them: `purity`).
//!
//! A compile-time expression (a `static` value, the arguments of an `@`
//! call, an `@if` condition…) is lowered with no frame of an enclosing
//! function: a local of a function there is not known at compile time
//! (`META001`), which [`Lowerer::not_known`] records.

use std::sync::Arc;

use ethnum::I256;
use rustc_hash::{FxHashMap, FxHashSet};

use fa_interpreter::Value;
use fa_interpreter::ir::{
	Arg, ArithOp as IrArith, CompareOp as IrCompare, Expr, FnKey, Function, GlobalKey, Item, Num,
	Param, Part, Place, Slot, Step, TypeTest,
};
use fa_interpreter::value::MemberKind;
use fa_parser::TypedSyntaxTree;

use crate::analysis::{Analysis, Argument, Callee, FieldValue, NodeFlags, PatternBinding};
use crate::builtins;
use crate::ids::{FileId, NodeId, ParamId, SymbolId, TypeId};
use crate::lower::when::{self, Literal, Test};
use crate::session::Session;
use crate::symbols::{Resolution, ScopeKind, SymbolKind};
use crate::syntax::{
	self, ArithOp, BindingKind, CompareOp, CompareOps, LogicOp, NodeList, TemplatePart, View,
};
use crate::types::{CollKind, FloatBits, IntStorage, NominalKind, TypeKind, TypeStore};

#[path = "copy_plans.rs"]
mod copies;

#[path = "borrows.rs"]
mod borrows;

thread_local! {
	/// The modules a lowering needed that the session doesn't hold.
	static MISSING: std::cell::RefCell<Vec<crate::ids::ModuleId>> =
		const { std::cell::RefCell::new(Vec::new()) };
}

/// The modules the lowerings of this thread needed and didn't find in their
/// session (a unit of a parallel run): load them, then lower again.
pub fn take_missing() -> Vec<crate::ids::ModuleId> {
	MISSING.with(|missing| std::mem::take(&mut *missing.borrow_mut()))
}

/// The key of a function: its file and its declaration (or lambda) node.
pub fn fn_key(file: FileId, node: NodeId) -> FnKey {
	FnKey(file.0, node.0)
}

/// The key of a module-level value: its file and its declaration node.
pub fn global_key(file: FileId, node: NodeId) -> GlobalKey {
	GlobalKey(file.0, node.0)
}

/// The qualified name of a member of a named union, an `Errors` type or an
/// enumeration, as the JavaScript runtime tags it: `Shape.Circle`; a
/// built-in error is its own member (`ParseError`).
pub fn member_name(types: &TypeStore, nominal: crate::ids::NominalId, index: u16) -> String {
	let info = types.nominal(nominal);
	let owner = types.decl_name(info.key);
	let member = types.member_name(nominal, index);
	if owner == member {
		member.to_string()
	} else {
		format!("{owner}.{member}")
	}
}

/// What lowering a compile-time expression found that is not known at
/// compile time: the node and the name (`META001`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NotKnown {
	/// The node.
	pub node: NodeId,
	/// The name, as written.
	pub name: String,
}

/// Lowers the nodes of one file.
pub struct Lowerer<'a> {
	session: &'a Session,
	file: FileId,
	tree: &'a TypedSyntaxTree,
	a: &'a Analysis,
	/// Arithmetic follows JavaScript's numbers (a JavaScript target).
	js: bool,
	slots: FxHashMap<SymbolId, Slot>,
	borrowed_symbols: FxHashSet<SymbolId>,
	borrowed_results: FxHashSet<NodeId>,
	/// The function whose frame is being lowered, for self handles.
	owner: Option<NodeId>,
	borrowed_values: FxHashSet<NodeId>,
	value_slots: FxHashMap<ParamId, Slot>,
	next: Slot,
	/// Lowering a compile-time expression: no enclosing function's frame.
	compile_time: bool,
	/// A compile-time expression may read module-level `let` values (a
	/// configuration file, the initializer of a module-level value); the
	/// arguments of an `@` call and a `static` value may not (`META001`).
	globals: bool,
	/// The first local that is not known at compile time.
	pub not_known: Option<NotKnown>,
	/// During the walk of the file, what its proxies of other files'
	/// declarations stand for (`check/modules.rs`).
	externals: FxHashMap<SymbolId, Resolution>,
}

impl<'a> Lowerer<'a> {
	/// A lowerer of the nodes of `file`, whose tree and analysis are given
	/// (the analysis of the file being walked is not in the session).
	pub fn new(
		session: &'a Session,
		file: FileId,
		tree: &'a TypedSyntaxTree,
		a: &'a Analysis,
	) -> Self {
		Self {
			session,
			file,
			tree,
			a,
			js: super::is_js_target(session.target()),
			slots: FxHashMap::default(),
			borrowed_symbols: FxHashSet::default(),
			borrowed_results: FxHashSet::default(),
			owner: None,
			borrowed_values: FxHashSet::default(),
			value_slots: FxHashMap::default(),
			next: 0,
			compile_time: true,
			globals: false,
			not_known: None,
			externals: FxHashMap::default(),
		}
	}

	/// The resolutions the proxies of the file being walked stand for.
	pub fn with_externals(mut self, externals: FxHashMap<SymbolId, Resolution>) -> Self {
		self.externals = externals;
		self
	}

	/// Lets the compile-time expression read module-level `let` values.
	pub fn with_globals(mut self) -> Self {
		self.globals = true;
		self
	}

	/// The number of slots used so far.
	pub fn slots(&self) -> u32 {
		self.next
	}

	/// Lowers a compile-time expression.
	pub fn compile_time_expression(mut self, n: NodeId) -> Result<(Expr, u32), NotKnown> {
		self.compile_time = true;
		let expr = self.expr(n);
		match self.not_known {
			Some(not_known) => Err(not_known),
			None => Ok((expr, self.next)),
		}
	}

	/// Lowers the function or lambda declared at `node`.
	pub fn function(mut self, node: NodeId) -> Option<Function> {
		self.compile_time = false;
		self.owner = Some(node);
		self.globals = true;
		if let Some(expression) = super::json::default_node(node) {
			let captures: Vec<Slot> = self
				.default_captures(expression)
				.into_iter()
				.map(|symbol| self.bind(symbol))
				.collect();
			let body = self.expr(expression);
			return Some(Function {
				name: "<JSON field default>".into(),
				params: Vec::new(),
				captures,
				slots: self.next,
				body,
			});
		}
		let info = self.a.function(node)?.clone();
		if info
			.sig
			.is_some_and(|sig| self.types().sig(sig).reference.is_some())
		{
			self.mark_reference_result(info.body);
			let mut nodes = vec![info.body];
			while let Some(node) = nodes.pop() {
				match syntax::view(self.tree, node) {
					View::Return(Some(value)) => self.mark_reference_result(value),
					View::Lambda { .. } | View::Function { .. } => continue,
					_ => {}
				}
				nodes.extend(syntax::children(self.tree, node));
			}
		}
		let name = match info.symbol {
			Some(symbol) => self
				.session
				.types
				.syms
				.resolve(self.a.symbols.symbol(symbol).name)
				.to_string(),
			None => "<lambda>".to_string(),
		};
		let mut params = Vec::with_capacity(info.params.len());
		// Value parameters remain values in the VM, including when a
		// generic helper forwards them to another generic helper.
		let owner = self.a.function_owners.get(&node).copied();
		let mut generics = owner
			.and_then(|owner| self.a.type_decls.get(&owner))
			.map(|owner| owner.generics.clone())
			.unwrap_or_default();
		generics.extend(info.generics.iter().copied());
		for param in generics {
			let data = self.types().param(param);
			if !data.value && !self.a.key_set_params.contains_key(&param) {
				continue;
			}
			let symbol = self
				.a
				.value_parameter_symbol(self.types(), self.tree, node, param);
			let slot = match symbol {
				Some(symbol) => self.bind(symbol),
				None => self.temp(),
			};
			self.value_slots.insert(param, slot);
			params.push(Param {
				slot,
				default: None,
			});
		}
		let hidden_count = params.len();
		for param in &info.params {
			let slot = self.bind(param.symbol);
			if !self
				.types()
				.parameter_can_be_consumed(self.a.symbols.symbol(param.symbol).ty)
			{
				self.borrowed_symbols.insert(param.symbol);
			}
			params.push(Param {
				slot,
				default: None,
			});
		}
		let captures: Vec<Slot> = self
			.a
			.environment_captures(info.lambda)
			.into_iter()
			.map(|symbol| {
				self.borrowed_symbols.insert(symbol);
				self.bind(symbol)
			})
			.collect();
		// the defaults, then the patterns of destructured parameters
		for (index, param) in info.params.iter().enumerate() {
			let reference = info.sig.is_some_and(|sig| {
				self.types()
					.params(self.types().sig(sig).params)
					.get(index)
					.is_some_and(|param| param.reference.is_some())
			});
			params[hidden_count + index].default = param.default.map(|default| {
				if reference {
					self.borrow_value(default)
				} else {
					self.bound_argument(default)
				}
			});
		}
		let mut prologue = Vec::new();
		if let Some(symbol) = info.symbol {
			let slot = self.bind(symbol);
			self.borrowed_symbols.insert(symbol);
			prologue.push(Expr::Let(
				slot,
				Expr::CurrentFunction(fn_key(self.file, node)).boxed(),
			));
		}
		for param in &info.params {
			if matches!(
				syntax::view(self.tree, param.node),
				View::DestructuredParameter { .. }
			) && let View::DestructuredParameter { pattern, .. } =
				syntax::view(self.tree, param.node)
			{
				let source = Expr::Local(self.slots[&param.symbol]);
				let borrow = !self
					.types()
					.parameter_can_be_consumed(self.a.symbols.symbol(param.symbol).ty);
				let source = if borrow {
					Expr::Lend(Place {
						slot: self.slots[&param.symbol],
						steps: Vec::new(),
					})
				} else {
					source
				};
				prologue.push(self.destructure(pattern, source, borrow));
			}
		}
		let body = self.body(info.body);
		let body = self.owned_result(body);
		let body = if prologue.is_empty() {
			body
		} else {
			prologue.push(body);
			Expr::Block(prologue)
		};
		Some(Function {
			name,
			params,
			captures,
			slots: self.next,
			body,
		})
	}

	/// The free values of a field default, in a stable order shared by its
	/// synthetic function and the closure constructed at a JSON call.
	fn default_captures(&self, expression: NodeId) -> Vec<SymbolId> {
		let mut nodes = FxHashSet::default();
		let mut pending = vec![expression];
		while let Some(node) = pending.pop() {
			if nodes.insert(node) {
				pending.extend(syntax::children(self.tree, node));
			}
		}
		let mut captures = Vec::new();
		for &node in &nodes {
			if !matches!(syntax::view(self.tree, node), View::Name(_)) {
				continue;
			}
			let Resolution::Local(symbol) = self.a.resolution(node) else {
				continue;
			};
			let data = self.a.symbols.symbol(symbol);
			if nodes.contains(&data.decl_node) || self.is_module_level(symbol) {
				continue;
			}
			match data.kind {
				SymbolKind::Function => {
					if let Some(info) = self.a.function(data.decl_node) {
						captures.extend(info.captures.iter().copied());
					}
				}
				SymbolKind::Static
				| SymbolKind::Type
				| SymbolKind::TypeParameter
				| SymbolKind::Namespace => {}
				_ => captures.push(symbol),
			}
		}
		captures.sort_unstable_by_key(|symbol| symbol.0);
		captures.dedup();
		captures
	}

	fn bind(&mut self, symbol: SymbolId) -> Slot {
		let slot = self.next;
		self.next += 1;
		self.slots.insert(symbol, slot);
		slot
	}

	fn temp(&mut self) -> Slot {
		let slot = self.next;
		self.next += 1;
		slot
	}

	fn types(&self) -> &TypeStore {
		&self.session.types
	}

	fn text(&self, n: NodeId) -> &str {
		self.tree.text(n.index())
	}

	fn unsupported(&self, what: impl Into<String>) -> Expr {
		Expr::Unsupported(Arc::from(what.into()))
	}

	fn impure(&self, what: impl Into<String>) -> Expr {
		Expr::Impure(Arc::from(what.into()))
	}

	/// Locale comparison reads machine data at evaluation time, without
	/// changing the runtime purity rules for reactive expressions.
	fn builtin_impurity(&self, path: &str) -> Option<Expr> {
		if path == "String.compareLocale" {
			Some(
				self.impure(
					"the machine's locale data (`String.compareLocale`; call it at runtime)",
				),
			)
		} else if builtins::std_library::impurity(path).is_some() {
			Some(self.impure(path))
		} else {
			None
		}
	}

	// ── numbers ──────────────────────────────────────────────────────────

	/// The kind of number of a type, with the range of the target (D49):
	/// on JavaScript, a number-represented integer is bounded by ±(2^53 − 1).
	pub fn num(&self, ty: TypeId) -> Num {
		let types = self.types();
		match types.kind(ty) {
			TypeKind::Int(_) => {
				let storage = types
					.as_int(ty)
					.map_or(IntStorage::Default, |int| int.storage);
				let range = storage.range();
				let wide = matches!(
					storage,
					IntStorage::I64 | IntStorage::U64 | IntStorage::I128 | IntStorage::U128
				);
				if self.js && !wide {
					let safe = crate::types::int::js_safe_range();
					Num::Int {
						min: range.lo.max(safe.lo),
						max: range.hi.min(safe.hi),
					}
				} else {
					Num::Int {
						min: range.lo,
						max: range.hi,
					}
				}
			}
			TypeKind::BigInt | TypeKind::BigIntLit(_) => Num::Int {
				min: I256::MIN,
				max: I256::MAX,
			},
			TypeKind::Float(FloatBits::F32) | TypeKind::FloatLit(FloatBits::F32, _) => Num::Float32,
			TypeKind::Percentage => Num::Percentage,
			_ => Num::Float,
		}
	}

	fn arith_op(op: ArithOp) -> IrArith {
		match op {
			ArithOp::Add => IrArith::Add,
			ArithOp::Subtract => IrArith::Subtract,
			ArithOp::Multiply => IrArith::Multiply,
			ArithOp::Divide => IrArith::Divide,
			ArithOp::IntegerDivide => IrArith::IntegerDivide,
			ArithOp::Remainder => IrArith::Remainder,
			ArithOp::Power => IrArith::Power,
		}
	}

	/// A number literal with the type the analysis gave it.
	fn number(&self, n: NodeId, value: f64, integer: Option<I256>) -> Expr {
		let ty = self.a.type_of(n);
		let value = match (self.types().kind(ty), integer) {
			(TypeKind::Int(_) | TypeKind::BigInt | TypeKind::BigIntLit(_), Some(integer)) => {
				Value::Integer(integer)
			}
			(TypeKind::Percentage, _) => Value::Percentage(value / 100.0),
			(TypeKind::Float(FloatBits::F32) | TypeKind::FloatLit(FloatBits::F32, _), _) => {
				Value::Float(value as f32 as f64)
			}
			(_, Some(integer))
				if !matches!(
					self.types().kind(ty),
					TypeKind::Float(_) | TypeKind::FloatLit(..)
				) =>
			{
				Value::Integer(integer)
			}
			_ => Value::Float(value),
		};
		Expr::Const(value)
	}

	// ── type tests ───────────────────────────────────────────────────────

	/// A pattern may compare a Fields value or list alternatives, whose
	/// expression type alone does not describe what the pattern matches.
	fn is_pattern(&self, pattern: NodeId) -> TypeTest {
		match syntax::view(self.tree, pattern) {
			View::Group(inner) => self.is_pattern(inner),
			View::Union { operands } => TypeTest::Union(
				operands
					.iter()
					.map(|operand| self.is_pattern(operand))
					.collect(),
			),
			_ if self.a.fields_patterns.contains(&pattern) => self
				.a
				.constant(pattern)
				.cloned()
				.map_or(TypeTest::Any, TypeTest::Literal),
			_ => self.type_test(self.a.type_of(pattern)),
		}
	}

	/// The test of a type (`is`, `when`).
	pub fn type_test(&self, ty: TypeId) -> TypeTest {
		let types = self.types();
		match types.kind(ty) {
			TypeKind::None => TypeTest::None,
			TypeKind::Boolean => TypeTest::Boolean,
			TypeKind::BoolLit(value) => TypeTest::Literal(Value::Boolean(value)),
			TypeKind::Int(_) => match types.as_int(ty) {
				Some(int) if int.min == int.max => TypeTest::Literal(Value::Integer(int.min)),
				Some(int) => TypeTest::Integer {
					min: int.min,
					max: int.max,
				},
				None => TypeTest::Any,
			},
			TypeKind::Float(_) | TypeKind::FloatLit(..) => TypeTest::Float,
			TypeKind::String => TypeTest::String,
			TypeKind::StringLit(text) => TypeTest::Literal(Value::string(types.syms.resolve(text))),
			TypeKind::Bytes => TypeTest::Bytes,
			TypeKind::Object(_) => TypeTest::Object,
			TypeKind::Tuple(list) => TypeTest::Tuple(types.list_items(list).len()),
			TypeKind::Union(list) => TypeTest::Union(
				types
					.list_items(list)
					.iter()
					.map(|&member| self.type_test(member))
					.collect(),
			),
			TypeKind::NamedUnion(nominal, mask) | TypeKind::Enum(nominal, mask) => {
				let count = types.sym_list(types.nominal(nominal).members).len();
				TypeTest::Union(
					(0..count as u16)
						.filter(|&index| types.mask_has(mask, index))
						.map(|index| {
							TypeTest::Member(Arc::from(member_name(types, nominal, index)))
						})
						.collect(),
				)
			}
			TypeKind::Member(nominal, index) | TypeKind::EnumMember(nominal, index) => {
				TypeTest::Member(Arc::from(member_name(types, nominal, index)))
			}
			TypeKind::Collection(CollKind::Map | CollKind::GrowingMap, ..) => TypeTest::Map,
			TypeKind::Collection(CollKind::Set | CollKind::GrowingSet, ..) => TypeTest::Set,
			TypeKind::Collection(..) => TypeTest::Array,
			TypeKind::Function(_) => TypeTest::Function,
			TypeKind::Range => TypeTest::Range,
			TypeKind::ErrorTop => TypeTest::Error,
			TypeKind::Rec(_) => self.type_test(types.unfold(ty)),
			_ => TypeTest::Any,
		}
	}

	fn member_kind(&self, nominal: crate::ids::NominalId) -> MemberKind {
		match self.types().nominal(nominal).kind {
			NominalKind::Enum => MemberKind::Enumeration,
			NominalKind::Errors => MemberKind::Error,
			_ => MemberKind::Union,
		}
	}

	// ── names ────────────────────────────────────────────────────────────

	/// Whether a symbol is a module-level declaration of its file.
	fn is_module_level(&self, symbol: SymbolId) -> bool {
		let scope = self.a.symbols.symbol(symbol).scope;
		matches!(
			self.a.symbols.scope(scope).kind,
			ScopeKind::Module | ScopeKind::Namespace
		)
	}

	/// The read of a symbol of this file.
	fn read_symbol(&mut self, n: NodeId, symbol: SymbolId) -> Expr {
		if let Some(&slot) = self.slots.get(&symbol) {
			return if !self.borrowed_symbols.contains(&symbol)
				&& (self.a.has_flags(n, NodeFlags::MOVE) || self.a.moved_reads.contains(&n))
			{
				Expr::Move(slot)
			} else {
				Expr::Local(slot)
			};
		}
		let data = self.a.symbols.symbol(symbol);
		let name = self.session.types.syms.resolve(data.name).to_string();
		// the variable of an `@for` analyzed per item (check/comptime.rs)
		if let Some(value) = self.a.comptime_locals.get(&data.decl_node) {
			return Expr::Const(value.clone());
		}
		match data.kind {
			SymbolKind::Static => match self.a.static_values.get(&symbol) {
				Some(value) => Expr::Const(value.clone()),
				None => self.not_known(n, &name),
			},
			SymbolKind::Function => {
				if self.owner == Some(data.decl_node) {
					return Expr::CurrentFunction(fn_key(self.file, data.decl_node));
				}
				let info = self.a.function(data.decl_node);
				let captures = self.a.environment_captures(data.decl_node);
				let key = fn_key(self.file, data.decl_node);
				if captures.is_empty() {
					Expr::Function(key)
				} else {
					if self
						.a
						.escapes
						.contains(&info.expect("a captured function").lambda)
					{
						return self.not_known(n, "the function's declaration environment");
					}
					let mut slots = Vec::with_capacity(captures.len());
					for capture in captures {
						match self.slots.get(&capture) {
							Some(&slot) => slots.push(slot),
							None => return self.not_known(n, &name),
						}
					}
					Expr::Closure {
						function: key,
						captures: slots,
						borrow: !self
							.a
							.escapes
							.contains(&info.expect("a captured function").lambda),
					}
				}
			}
			SymbolKind::Let if self.is_module_level(symbol) && self.globals => {
				Expr::Global(global_key(self.file, data.decl_node))
			}
			SymbolKind::Mutable | SymbolKind::Reactive | SymbolKind::Derived
				if self.is_module_level(symbol) =>
			{
				self.impure(format!("the module-level `{name}`"))
			}
			SymbolKind::Implicit => self.impure(name),
			SymbolKind::Requirement { .. } => self.impure(format!("the requirement `{name}`")),
			SymbolKind::Type | SymbolKind::Namespace | SymbolKind::TypeParameter => {
				self.unsupported(format!("`{name}` as a value"))
			}
			_ => self.not_known(n, &name),
		}
	}

	/// A local of a function read by a compile-time expression (`META001`);
	/// inside a function body, an unknown local (an error of the program).
	fn not_known(&mut self, n: NodeId, name: &str) -> Expr {
		if self.compile_time && self.not_known.is_none() {
			self.not_known = Some(NotKnown {
				node: n,
				name: name.to_string(),
			});
		}
		self.unsupported(format!("`{name}`"))
	}

	/// The read of what a name resolves to.
	fn resolved(&mut self, n: NodeId, resolution: Resolution) -> Expr {
		// a proxy of another file's declaration, during the walk
		let resolution = match resolution {
			Resolution::Local(symbol) => self.externals.get(&symbol).copied().unwrap_or(resolution),
			other => other,
		};
		match resolution {
			Resolution::Local(symbol) => self.read_symbol(n, symbol),
			Resolution::External(..) => self.external(n, resolution),
			Resolution::Member(nominal, index) => Expr::Member {
				name: Arc::from(member_name(self.types(), nominal, index)),
				kind: self.member_kind(nominal),
				data: None,
			},
			Resolution::Builtin(id) => {
				let path = builtins::get(id).path;
				match path {
					"arguments" => self.impure("arguments"),
					"Float.nan" => Expr::Const(Value::Float(f64::NAN)),
					"Float.infinity" => Expr::Const(Value::Float(f64::INFINITY)),
					"Float.maxValue" => Expr::Const(Value::Float(f64::MAX)),
					"Float.epsilon" => Expr::Const(Value::Float(f64::EPSILON)),
					_ => match self.literal_of(n) {
						Some(value) => Expr::Const(value),
						None => self
							.builtin_impurity(path)
							.unwrap_or_else(|| self.unsupported(format!("`{path}`"))),
					},
				}
			}
			_ => self.unsupported(format!("`{}`", self.text(n))),
		}
	}

	/// A declaration of another file.
	/// The file of the module an external resolution designates and what
	/// its interface says of the declaration. In a unit of a parallel run,
	/// a module of another unit is not in the session: it is noted
	/// ([`take_missing`]), to be loaded again and lowered again.
	fn declared_of(&self, resolution: Resolution) -> Option<(FileId, &crate::modules::Declared)> {
		let found = self.session.external_declared(resolution);
		if found.is_none()
			&& let Resolution::External(key) = resolution
			&& let module = self.session.types.decl_key_data(key).module
			&& self.session.can_rehydrate(module)
		{
			MISSING.with(|missing| missing.borrow_mut().push(module));
		}
		found
	}

	fn external(&mut self, n: NodeId, resolution: Resolution) -> Expr {
		// read from the other file's interface (check/interface.rs)
		let Some((file, declared)) = self.declared_of(resolution) else {
			return self.unsupported(format!("`{}`", self.text(n)));
		};
		let Some((_, decl)) = self.session.external_node(resolution) else {
			return self.unsupported(format!("`{}`", self.text(n)));
		};
		let name = self.session.types.syms.resolve(declared.name).to_string();
		match declared.kind {
			SymbolKind::Function => Expr::Function(fn_key(file, declared.function.unwrap_or(decl))),
			SymbolKind::Let if self.globals => Expr::Global(global_key(file, decl)),
			SymbolKind::Let => self.not_known(n, &name),
			SymbolKind::Static => match self.session.source_file(file).interface.statics.get(&decl)
			{
				Some(value) => Expr::Const(value.clone()),
				None => self.unsupported(format!("`{name}`")),
			},
			SymbolKind::Mutable | SymbolKind::Reactive | SymbolKind::Derived => {
				self.impure(format!("the module-level `{name}`"))
			}
			_ => self.unsupported(format!("`{name}`")),
		}
	}

	/// The value of a node whose type is a literal (built-in constants).
	fn literal_of(&self, n: NodeId) -> Option<Value> {
		self.literal_type(self.a.type_of(n))
	}

	fn literal_type(&self, ty: TypeId) -> Option<Value> {
		let types = self.types();
		match types.kind(ty) {
			TypeKind::Int(_) => types
				.as_int(ty)
				.filter(|int| int.min == int.max)
				.map(|int| Value::Integer(int.min)),
			TypeKind::BoolLit(value) => Some(Value::Boolean(value)),
			TypeKind::StringLit(text) => Some(Value::string(types.syms.resolve(text))),
			_ => None,
		}
	}

	// ── expressions ──────────────────────────────────────────────────────

	/// The body of a function: a block's value, or an expression.
	fn body(&mut self, body: NodeId) -> Expr {
		self.expr(body)
	}

	/// Lowers an expression or a statement.
	pub fn expr(&mut self, n: NodeId) -> Expr {
		if self.borrowed_values.contains(&n) {
			return self.borrow_value(n);
		}
		if self.borrowed_results.contains(&n)
			&& let View::Call { callee, .. } = syntax::view(self.tree, n)
			&& (self.call_borrows(callee) || self.a.call(n).is_some_and(|call| matches!(call.callee, Callee::Builtin(builtin) if Self::builtin_borrows(builtins::get(builtin).path)))) {
			return self.borrow_value(n);
		}
		if self.borrowed_results.contains(&n)
			&& !self
				.a
				.copy_inputs
				.get(&n)
				.is_some_and(|input| input.produces)
			&& let Some(place) = self.borrowable_place(n)
		{
			return Expr::Lend(place);
		}
		let value = self.expr_raw(n);
		if let Some(input) = self.a.copy_inputs.get(&n).filter(|input| input.produces) {
			return Expr::Copy(self.copy_plan(input.source, input.target), value.boxed());
		}
		let source = self.a.type_of(n);
		let target = self.a.coercions.get(&n).copied().unwrap_or(source);
		if !self.borrowed_results.contains(&n)
			&& !self.a.copy_inputs.contains_key(&n)
			&& self.types().ownership(target) == crate::types::ownership::Ownership::Ordinary
			&& matches!(
				self.types().kind(target),
				TypeKind::Object(_) | TypeKind::Tuple(_) | TypeKind::Union(_)
			) {
			return Expr::Copy(self.copy_plan(source, target), value.boxed());
		}
		value
	}

	fn expr_raw(&mut self, n: NodeId) -> Expr {
		// a value the analysis computed at compile time (`@` calls, statics)
		if let Some(value) = self.a.constants.get(&n) {
			return Expr::Const(value.clone());
		}
		match syntax::view(self.tree, n) {
			View::Module { statements } | View::Block { statements } => Expr::Block(
				statements
					.iter()
					.map(|statement| self.expr(statement))
					.collect(),
			),
			View::Error => self.unsupported("code with a syntax error"),
			View::Unsupported(kind) => self.unsupported(kind),
			View::Await(_) | View::Start(_) => self.unsupported("async code at compile time"),
			View::Integer(value) => self.number(n, value.as_f64(), Some(value)),
			View::Float(value) => self.number(n, value, None),
			View::BigInt(text) => self.big_int(text),
			View::String(text) => Expr::Const(Value::string(text)),
			View::Template { parts } => Expr::Template(
				parts
					.iter()
					.map(|part| match part {
						TemplatePart::Text(text) => Part::Text(Arc::from(text)),
						TemplatePart::Expression(value) => Part::Value(self.printed(value)),
					})
					.collect(),
			),
			View::Boolean(value) => Expr::Const(Value::Boolean(value)),
			View::None => Expr::none(),
			View::Name(_) => {
				let resolution = self.a.resolution(n);
				let value = self.resolved(n, resolution);
				if self.a.instantiation(n).is_some() {
					self.bind_generics(n, n, value)
				} else {
					value
				}
			}
			View::ImplicitMember(_) => match self.a.resolution(n) {
				Resolution::Member(..) => {
					let resolution = self.a.resolution(n);
					self.resolved(n, resolution)
				}
				_ => match self.literal_of(n) {
					Some(value) => Expr::Const(value),
					None => self.unsupported(format!("`{}`", self.text(n))),
				},
			},
			// Producers fulfill explicit Copy boundaries in `expr`.
			View::Group(inner) | View::Copy(inner) => self.expr(inner),
			View::EmptyGroup => Expr::none(),
			View::Array { items } => self.array(n, items),
			View::Tuple { items } => {
				Expr::Tuple(items.iter().map(|item| self.expr(item)).collect())
			}
			View::Spread(value) => self.expr(value),
			View::Object { .. } | View::TypedObject { .. } => self.object(n),
			View::Range {
				start,
				end,
				inclusive,
			} => Expr::Range(
				start.map(|start| self.expr(start).boxed()),
				end.map(|end| self.expr(end).boxed()),
				inclusive,
			),
			View::Access { target, members } => self.access(n, target, members, false),
			View::OptionalAccess { target, members } => self.access(n, target, members, true),
			View::Index { target, index } => {
				Expr::Index(self.expr(target).boxed(), self.expr(index).boxed())
			}
			View::Call { callee, arguments } => {
				let call = self.call(n, callee, arguments);
				if self.call_borrows(callee) {
					Expr::Dereference(call.boxed())
				} else {
					call
				}
			}
			View::Arithmetic { op, operands } => self.arithmetic(n, op, operands),
			View::Negate(value) => {
				if let View::Integer(integer) = syntax::view(self.tree, value) {
					return self.number(n, -integer.as_f64(), Some(-integer));
				}
				let num = self.num(self.a.type_of(n));
				Expr::Negate(num, self.expr(value).boxed())
			}
			View::Percentage(value) => Expr::Convert(
				Num::Percentage,
				Expr::Arith(
					IrArith::Divide,
					Num::Float,
					self.expr(value).boxed(),
					Expr::Const(Value::Float(100.0)).boxed(),
				)
				.boxed(),
			),
			View::CompoundAssignment { op, target, value } => {
				self.compound_assignment(op, target, value)
			}
			View::Equality {
				left,
				right,
				negated,
			} => Expr::Equals(self.expr(left).boxed(), self.expr(right).boxed(), negated),
			View::Comparison {
				operands,
				operators,
			} => self.comparison(operands, operators),
			View::Logic { op, operands } => {
				let mut values = operands.iter().map(|operand| self.expr(operand));
				let mut result = values.next().unwrap_or_else(Expr::none);
				for right in values {
					result = match op {
						LogicOp::And => Expr::And(result.boxed(), right.boxed()),
						LogicOp::Or => Expr::Or(result.boxed(), right.boxed()),
					};
				}
				result
			}
			View::Not(value) => Expr::Not(self.expr(value).boxed()),
			View::Fallback { operands } => {
				let mut values = operands.iter().map(|operand| self.expr(operand));
				let mut result = values.next().unwrap_or_else(Expr::none);
				for right in values {
					result = Expr::Fallback(result.boxed(), right.boxed());
				}
				result
			}
			View::Is { value, ty, negated } => {
				let test = self.is_pattern(ty);
				Expr::Is(self.expr(value).boxed(), test, negated)
			}
			// a set of flags: a bit test
			View::IsIn {
				value,
				collection,
				negated,
			} if matches!(
				self.types().kind(self.a.type_of(collection)),
				TypeKind::Fields(_)
			) =>
			{
				let test = Expr::Builtin(
					"Fields#includes",
					vec![self.expr(collection), self.expr(value)],
				);
				if negated {
					Expr::Not(test.boxed())
				} else {
					test
				}
			}
			View::IsIn {
				value,
				collection,
				negated,
			} => Expr::IsIn(
				self.expr(value).boxed(),
				self.expr(collection).boxed(),
				negated,
			),
			View::Declaration { kind, value, .. } => match (kind, value) {
				(BindingKind::Let | BindingKind::Mutable, Some(value)) => {
					let value = self.expr(value);
					match self.a.symbols.symbol_of_decl(n) {
						Some(symbol) => {
							let slot = self.bind(symbol);
							Expr::Let(slot, value.boxed())
						}
						None => value,
					}
				}
				// a `static` is a constant; the others are runtime constructs
				(BindingKind::Static, _) => Expr::none(),
				_ => self.unsupported(format!("`{}` declarations", kind.keyword())),
			},
			View::Use { value, .. } => self.use_binding(n, value),
			View::Reuse { name: _, value } => match self.a.resolution(n) {
				Resolution::Local(symbol) => {
					let Some(&slot) = self.slots.get(&symbol) else {
						return self.unsupported("an unbound reseatable reference");
					};
					self.borrowed_symbols.insert(symbol);
					Expr::Bind(slot, self.bound(value).boxed())
				}
				_ => Expr::none(),
			},
			View::Destructuring { pattern, value, .. } => {
				let value = self.expr(value);
				self.destructure(pattern, value, false)
			}
			View::Assignment { target, value, .. } => self.assignment(target, value),
			View::UpdateFields { target, value } => self.update_fields(target, value),
			View::Function { .. } => self.function_declaration(n),
			View::TypeDecl { .. } | View::Namespace { .. } => Expr::none(),
			View::ExportValue { value, .. } => self.expr(value),
			View::ExportFunction { .. } | View::ExportType { .. } | View::ExportNamespace(_) => {
				Expr::none()
			}
			View::Lambda { .. } => self.lambda(n),
			View::Parameters { .. } | View::DestructuredParameter { .. } => Expr::none(),
			View::Compose { .. } => self.unsupported("`then`"),
			View::PartialApplication { .. } => self.unsupported("`with`"),
			View::Provide { .. } => self.impure("`provide`"),
			View::Watch(_) => self.impure("a `watch` block"),
			View::If {
				condition,
				then_body,
				else_body,
				compile_time,
			} => self.if_expr(n, condition, then_body, else_body, compile_time),
			View::When { .. } => self.when(n),
			View::While { condition, body } => {
				Expr::While(self.expr(condition).boxed(), self.expr(body).boxed())
			}
			View::WhileLet {
				pattern,
				value,
				body,
			} => self.while_let(pattern, value, body),
			View::For {
				pattern,
				index,
				iterable,
				body,
				parallel,
				compile_time,
			} => self.for_loop(
				n,
				pattern,
				index.is_some(),
				iterable,
				body,
				parallel.is_some(),
				compile_time,
			),
			View::Loop(body) => Expr::Loop(self.expr(body).boxed()),
			View::Do(body) => self.expr(body),
			View::Break(value) => Expr::Break(value.map(|value| self.expr(value).boxed())),
			View::Continue => Expr::Continue,
			View::Return(value) => Expr::Return(value.map(|value| {
				let value = self.expr(value);
				self.owned_result(value).boxed()
			})),
			View::Try(value) => self.expr(value),
			View::Throw(value) => Expr::Throw(self.expr(value).boxed()),
			View::Catch {
				expression,
				handler,
				..
			} => self.catch(n, expression, handler),
			View::Test { .. } => Expr::none(),
			View::Assert(condition) => {
				let text = Arc::from(self.text(condition));
				Expr::Assert(self.expr(condition).boxed(), text)
			}
			View::ObjectPattern { .. }
			| View::ArrayPattern { .. }
			| View::TuplePattern { .. }
			| View::Readonly(_)
			| View::Link(_)
			| View::ErrorType
			| View::Union { .. }
			| View::Optional(_)
			| View::RefType { .. }
			| View::Throws { .. }
			| View::Requires { .. }
			| View::Effect { .. } => self.unsupported(format!("`{}`", self.text(n))),
		}
	}

	/// A big integer literal.
	fn big_int(&self, text: &str) -> Expr {
		let digits: String = text
			.chars()
			.filter(|c| c.is_ascii_digit() || *c == '-')
			.collect();
		match I256::from_str_radix(&digits, 10) {
			Ok(value) => Expr::Const(Value::Integer(value)),
			Err(_) => self.unsupported("this big integer"),
		}
	}

	/// An array literal: an array, a set or a map by its type.
	fn array(&mut self, n: NodeId, items: NodeList<'_>) -> Expr {
		let lowered = self.items(items.iter());
		match self.types().kind(self.a.type_of(n)) {
			TypeKind::Collection(
				CollKind::Pool
				| CollKind::GrowingPool
				| CollKind::Columnar
				| CollKind::GrowingColumnar,
				..,
			) => Expr::Pool(lowered),
			TypeKind::Collection(CollKind::Set | CollKind::GrowingSet, ..) => Expr::Set(lowered),
			TypeKind::Collection(CollKind::Map | CollKind::GrowingMap, ..) => Expr::Map(lowered),
			_ => Expr::Array(lowered),
		}
	}

	/// A value interpolated in a template: a set of flags prints its flags.
	fn printed(&mut self, n: NodeId) -> Expr {
		let value = self.expr(n);
		let ty = self.a.type_of(n);
		if !matches!(self.types().kind(ty), TypeKind::Fields(_)) {
			return value;
		}
		let names = self.fields_names(ty);
		Expr::Builtin("Fields#format", vec![value, names])
	}

	/// The dotted paths of the flags of a `Fields` type, by bit.
	fn fields_names(&self, ty: crate::ids::TypeId) -> Expr {
		let names = self
			.types()
			.fields_leaves(ty)
			.iter()
			.map(|name| Value::string(name))
			.collect();
		Expr::Const(Value::array(names))
	}

	/// A chain of one arithmetic operator, by its associativity.
	fn arithmetic(&mut self, n: NodeId, op: ArithOp, operands: NodeList<'_>) -> Expr {
		// `a + b`, `a - b` of field sets: bit operations
		if matches!(self.types().kind(self.a.type_of(n)), TypeKind::Fields(_)) {
			let path = if op == ArithOp::Add {
				"Fields#union"
			} else {
				"Fields#difference"
			};
			let mut values = operands.iter().map(|operand| self.expr(operand));
			let first = values.next().unwrap_or_else(Expr::none);
			return values.fold(first, |acc, value| Expr::Builtin(path, vec![acc, value]));
		}
		// `"a" + "b"`: a concatenation
		if op == ArithOp::Add
			&& matches!(
				self.types().kind(self.a.type_of(n)),
				TypeKind::String | TypeKind::StringLit(_)
			) {
			let parts = operands
				.iter()
				.map(|operand| Part::Value(self.expr(operand)))
				.collect();
			return Expr::Template(parts);
		}
		let num = self.num(self.a.type_of(n));
		let op_ir = Self::arith_op(op);
		let mut values: Vec<Expr> = operands.iter().map(|operand| self.expr(operand)).collect();
		if op.is_right_associative() {
			let mut result = values.pop().unwrap_or_else(Expr::none);
			while let Some(left) = values.pop() {
				result = Expr::Arith(op_ir, num, left.boxed(), result.boxed());
			}
			result
		} else {
			let mut values = values.into_iter();
			let mut result = values.next().unwrap_or_else(Expr::none);
			for right in values {
				result = Expr::Arith(op_ir, num, result.boxed(), right.boxed());
			}
			result
		}
	}

	/// `target op= value`: designate the target once before reading or computing.
	fn compound_assignment(&mut self, op: ArithOp, target: NodeId, value: NodeId) -> Expr {
		let Some(source) = self.place(target) else {
			return self.unsupported("this assignment");
		};
		let slot = self.temp();
		let place = Place {
			slot,
			steps: Vec::new(),
		};
		let before = Expr::Read(place.clone());
		let value = self.expr(value);
		let updated = if matches!(
			self.types().kind(self.a.type_of(target)),
			TypeKind::Fields(_)
		) {
			let path = if op == ArithOp::Add {
				"Fields#union"
			} else {
				"Fields#difference"
			};
			Expr::Builtin(path, vec![before, value])
		} else if op == ArithOp::Add
			&& matches!(self.types().kind(self.a.type_of(target)), TypeKind::String)
		{
			Expr::Template(vec![Part::Value(before), Part::Value(value)])
		} else {
			Expr::Arith(
				Self::arith_op(op),
				self.num(self.a.type_of(target)),
				before.boxed(),
				value.boxed(),
			)
		};
		Expr::Block(vec![
			Expr::Bind(slot, Expr::Lend(source).boxed()),
			Expr::Assign(place, updated.boxed()),
		])
	}

	/// A chain of comparisons: `a < b <= c` is `a < b and b <= c`.
	fn comparison(&mut self, operands: NodeList<'_>, operators: CompareOps<'_>) -> Expr {
		let values: Vec<Expr> = operands.iter().map(|operand| self.expr(operand)).collect();
		let mut result: Option<Expr> = None;
		for (index, op) in operators.iter().enumerate() {
			let op = match op {
				CompareOp::Less => IrCompare::Less,
				CompareOp::LessOrEqual => IrCompare::LessOrEqual,
				CompareOp::Greater => IrCompare::Greater,
				CompareOp::GreaterOrEqual => IrCompare::GreaterOrEqual,
			};
			let test = Expr::Compare(
				op,
				values[index].clone().boxed(),
				values[index + 1].clone().boxed(),
			);
			result = Some(match result {
				Some(previous) => Expr::And(previous.boxed(), test.boxed()),
				None => test,
			});
		}
		result.unwrap_or(Expr::Const(Value::Boolean(true)))
	}

	/// `use name = value`: an alias of a place, or a temporary bound once.
	fn use_binding(&mut self, n: NodeId, value: NodeId) -> Expr {
		let Some(symbol) = self.a.symbols.symbol_of_decl(n) else {
			return Expr::none();
		};
		let slot = self.bind(symbol);
		self.borrowed_symbols.insert(symbol);
		Expr::Bind(slot, self.bound(value).boxed())
	}

	fn call_borrows(&self, callee: NodeId) -> bool {
		matches!(self.types().kind(self.a.type_of(callee)), TypeKind::Function(sig) if self.types().sig(sig).reference.is_some())
	}

	fn builtin_borrows(path: &str) -> bool {
		matches!(
			path,
			"Pool#add"
				| "Set#find" | "Pool#find"
				| "Columnar#add"
				| "Columnar#find"
				| "OrderedColumnar#add"
				| "OrderedColumnar#find"
				| "OrderedColumnar#get"
				| "Array#get"
				| "Array#find"
				| "Map#get" | "Map#getOrInsert"
		)
	}

	fn mark_reference_result(&mut self, value: NodeId) {
		match syntax::view(self.tree, value) {
			View::Group(inner) | View::Try(inner) | View::Do(inner) | View::Return(Some(inner)) => {
				self.mark_reference_result(inner)
			}
			View::Block { statements } => {
				if let Some(last) = statements.last() {
					self.mark_reference_result(last);
				}
			}
			View::If {
				then_body,
				else_body,
				..
			} => {
				self.mark_reference_result(then_body);
				if let Some(body) = else_body {
					self.mark_reference_result(body);
				}
			}
			View::When { arms, .. } => {
				for arm in arms.iter() {
					self.mark_reference_result(arm.value);
				}
			}
			View::Loop(_) => {
				for value in self.a.loop_results.get(&value).cloned().unwrap_or_default() {
					self.mark_reference_result(value);
				}
			}
			View::Catch {
				expression,
				handler,
				..
			} => {
				self.mark_reference_result(expression);
				self.mark_reference_result(handler);
			}
			View::None
			| View::Name(_)
			| View::Access { .. }
			| View::Index { .. }
			| View::Call { .. } => {
				self.borrowed_values.insert(value);
			}
			_ => {}
		}
	}

	fn receiver_place(&mut self, callee: NodeId) -> Option<Place> {
		let View::Access { target, members } = syntax::view(self.tree, callee) else {
			return None;
		};
		let mut place = self.place(target)?;
		for member in members.iter().take(members.len().saturating_sub(1)) {
			let View::Name(name) = syntax::view(self.tree, member) else {
				return None;
			};
			place
				.steps
				.push(Step::Field(Arc::from(self.tree.name(name))));
		}
		Some(place)
	}

	fn borrow_value(&mut self, value: NodeId) -> Expr {
		match syntax::view(self.tree, value) {
			View::If { .. }
			| View::When { .. }
			| View::Catch { .. }
			| View::Do(_)
			| View::Block { .. }
			| View::Loop(_) => {
				self.mark_reference_result(value);
				return self.expr(value);
			}
			View::Group(inner) | View::Try(inner) => return self.borrow_value(inner),
			View::None => return Expr::none(),
			View::Call { callee, arguments } if self.call_borrows(callee) => {
				return self.call(value, callee, arguments);
			}
			View::Call { callee, .. } => {
				if let Some(info) = self.a.call(value)
					&& let Callee::Builtin(builtin) = info.callee
					&& Self::builtin_borrows(builtins::get(builtin).path)
				{
					let values = info
						.arguments
						.iter()
						.filter_map(|argument| match argument {
							Argument::Given(value) => Some(self.expr(*value)),
							_ => None,
						})
						.collect();
					return self.borrow_builtin(callee, builtins::get(builtin).path, values);
				}
			}
			_ => {}
		}
		if let Some(place) = self.place(value) {
			let optional = matches!(syntax::view(self.tree, value), View::Name(_))
				&& self
					.types()
					.union_members(&self.a.type_of(value))
					.contains(&TypeId::NONE);
			return Expr::Borrow(place, optional);
		}
		match syntax::view(self.tree, value) {
			View::Name(_) => {
				let read = match self.a.resolution(value) {
					Resolution::Local(symbol) => self.read_symbol(value, symbol),
					resolution @ Resolution::External(_) => self.external(value, resolution),
					_ => self.unsupported("this reference"),
				};
				match read {
					Expr::Global(key) => Expr::BorrowGlobal(key),
					other @ (Expr::Impure(_) | Expr::Unsupported(_)) => other,
					Expr::Const(value) => {
						let slot = self.temp();
						Expr::Block(vec![
							Expr::Let(slot, Expr::Const(value).boxed()),
							Expr::Borrow(
								Place {
									slot,
									steps: Vec::new(),
								},
								false,
							),
						])
					}
					_ => self.unsupported("a reference to this constant"),
				}
			}
			View::Access { target, members } => {
				let owner = self.borrow_value(target);
				let slot = self.temp();
				let mut steps = Vec::new();
				for member in members.iter() {
					let View::Name(name) = syntax::view(self.tree, member) else {
						return self.unsupported("this reference field");
					};
					steps.push(Step::Field(Arc::from(self.tree.name(name))));
				}
				Expr::Block(vec![
					Expr::Bind(slot, owner.boxed()),
					Expr::Borrow(Place { slot, steps }, false),
				])
			}
			View::Index { target, index } => {
				let owner = self.borrow_value(target);
				let slot = self.temp();
				let index = self.expr(index);
				Expr::Block(vec![
					Expr::Bind(slot, owner.boxed()),
					Expr::Borrow(
						Place {
							slot,
							steps: vec![Step::Index(index)],
						},
						false,
					),
				])
			}
			_ => {
				// A builtin may borrow an owned receiver temporary, e.g.
				// `[value].find(predicate)`. Keep it until its result is read.
				let marked = self.borrowed_values.remove(&value);
				let data = self.expr(value);
				if marked {
					self.borrowed_values.insert(value);
				}
				let slot = self.temp();
				Expr::Block(vec![
					Expr::Let(slot, data.boxed()),
					Expr::Borrow(
						Place {
							slot,
							steps: Vec::new(),
						},
						false,
					),
				])
			}
		}
	}

	/// Borrow a builtin's receiver, including immutable module owners and
	/// nested reference-returning expressions, without copying its data.
	fn borrow_builtin(&mut self, callee: NodeId, path: &'static str, values: Vec<Expr>) -> Expr {
		if let Some(place) = self.receiver_place(callee) {
			return Expr::BorrowBuiltin(path, place, values);
		}
		let View::Access { target, members } = syntax::view(self.tree, callee) else {
			return self.unsupported("this borrowed receiver");
		};
		let owner = self.borrow_value(target);
		let slot = self.temp();
		let mut steps = Vec::new();
		for member in members.iter().take(members.len().saturating_sub(1)) {
			let View::Name(name) = syntax::view(self.tree, member) else {
				return self.unsupported("this borrowed receiver");
			};
			steps.push(Step::Field(Arc::from(self.tree.name(name))));
		}
		Expr::Block(vec![
			Expr::Bind(slot, owner.boxed()),
			Expr::BorrowBuiltin(path, Place { slot, steps }, values),
		])
	}

	/// `target = value`.
	fn assignment(&mut self, target: NodeId, value: Option<NodeId>) -> Expr {
		let Some(value) = value else {
			return Expr::none();
		};
		let value = self.expr(value);
		match self.place(target) {
			Some(place) => Expr::Assign(place, value.boxed()),
			None => match self.module_target(target) {
				Some(name) => self.impure(format!("a change of the module-level `{name}`")),
				None => self.unsupported("this assignment"),
			},
		}
	}

	/// `target .= { field = value, … }`: one assignment per field.
	fn update_fields(&mut self, target: NodeId, value: NodeId) -> Expr {
		let Some(place) = self.place(target) else {
			return self.unsupported("this `.=`");
		};
		let View::Object { items } = syntax::view(self.tree, value) else {
			return self.unsupported("this `.=`");
		};
		let mut statements = Vec::new();
		for item in items.iter() {
			if let View::Assignment {
				target: field,
				value: Some(field_value),
				..
			} = syntax::view(self.tree, item)
				&& let View::Name(name) = syntax::view(self.tree, field)
			{
				let mut place = place.clone();
				place
					.steps
					.push(Step::Field(Arc::from(self.tree.name(name))));
				let field_value = self.expr(field_value);
				statements.push(Expr::Assign(place, field_value.boxed()));
			}
		}
		Expr::Block(statements)
	}

	/// A normal result owns its value; a declared reference result forwards
	/// its loan. An already owned/moved value transfers without another copy.
	fn owned_result(&self, value: Expr) -> Expr {
		if self
			.owner
			.and_then(|owner| self.a.function(owner))
			.and_then(|info| info.sig)
			.is_some_and(|sig| self.types().sig(sig).reference.is_some())
		{
			return value;
		}
		Expr::Own(value.boxed())
	}

	/// A function literal: a closure over the slots of its captures.
	fn lambda(&mut self, n: NodeId) -> Expr {
		let captures = self.a.environment_captures(n);
		let mut slots = Vec::with_capacity(captures.len());
		let lambda = self.a.function(n).map_or(n, |info| info.lambda);
		let owned = self.a.escapes.contains(&lambda);
		let mut preparation = Vec::new();
		let mut temporary = Vec::new();
		for capture in captures {
			match self.slots.get(&capture) {
				Some(&slot) => {
					let ty = self.a.symbols.symbol(capture).ty;
					let plan = self.copy_plan(ty, ty);
					if owned
						&& self.types().ownership(ty) == crate::types::Ownership::Ordinary
						&& !matches!(
							plan.steps[plan.root as usize],
							fa_interpreter::ir::CopyStep::Value
						) {
						let snapshot = self.temp();
						preparation.push(Expr::Let(
							snapshot,
							Expr::Copy(plan, Expr::Local(slot).boxed()).boxed(),
						));
						temporary.push(snapshot);
						slots.push(snapshot);
					} else {
						slots.push(slot);
					}
				}
				None => {
					let name = self
						.session
						.types
						.syms
						.resolve(self.a.symbols.symbol(capture).name)
						.to_string();
					return self.not_known(n, &name);
				}
			}
		}
		let closure = Expr::Closure {
			function: fn_key(self.file, n),
			captures: slots,
			borrow: !owned,
		};
		if preparation.is_empty() {
			return closure;
		}
		let result = self.temp();
		preparation.push(Expr::Let(result, closure.boxed()));
		preparation.extend(temporary.into_iter().map(Expr::Move));
		preparation.push(Expr::Move(result));
		Expr::Block(preparation)
	}

	fn function_declaration(&mut self, node: NodeId) -> Expr {
		if self.a.environment_captures(node).is_empty() {
			return Expr::none();
		}
		let Some(symbol) = self.a.function(node).and_then(|info| info.symbol) else {
			return self.unsupported("a function declaration without a binding");
		};
		let value = self.lambda(node);
		let slot = self.bind(symbol);
		Expr::Let(slot, value.boxed())
	}

	/// `if`, or the branch an `@if` took.
	fn if_expr(
		&mut self,
		n: NodeId,
		condition: NodeId,
		then_body: NodeId,
		else_body: Option<NodeId>,
		compile_time: bool,
	) -> Expr {
		if compile_time {
			return match self.a.comptime_branches.get(&n) {
				Some(true) => self.expr(then_body),
				Some(false) => else_body.map_or_else(Expr::none, |body| self.expr(body)),
				None => self.unsupported("this `@if`"),
			};
		}
		let condition = self.expr(condition);
		let then = self.expr(then_body);
		let otherwise = else_body.map(|body| self.expr(body).boxed());
		Expr::If(condition.boxed(), then.boxed(), otherwise)
	}

	/// `while let x = value { … }`: until the value is `none`.
	fn while_let(&mut self, pattern: NodeId, value: NodeId, body: NodeId) -> Expr {
		// `while let x = value { … }`: until the value is `none`
		let temp = self.temp();
		let value = self.expr(value);
		let bind = match self.a.symbols.symbol_of_decl(pattern) {
			Some(symbol) => {
				let slot = self.bind(symbol);
				Expr::Let(slot, Expr::Local(temp).boxed())
			}
			None => self.destructure(pattern, Expr::Local(temp), false),
		};
		let body = self.expr(body);
		Expr::Loop(
			Expr::Block(vec![
				Expr::Let(temp, value.boxed()),
				Expr::If(
					Expr::Is(Expr::Local(temp).boxed(), TypeTest::None, false).boxed(),
					Expr::Break(None).boxed(),
					None,
				),
				bind,
				body,
			])
			.boxed(),
		)
	}

	/// `for item in iterable { … }` (an `@for` iterates over its unrolled items).
	#[allow(clippy::too_many_arguments)]
	fn for_loop(
		&mut self,
		n: NodeId,
		pattern: Option<NodeId>,
		index: bool,
		iterable: NodeId,
		body: NodeId,
		parallel: bool,
		compile_time: bool,
	) -> Expr {
		if parallel {
			return self.unsupported("`for parallel`");
		}
		let unrolled = compile_time
			.then(|| self.a.unrolled.get(&n).cloned())
			.flatten();
		let utf8_source =
			if unrolled.is_none() && self.a.has_flags(iterable, NodeFlags::UTF8_ITERATION) {
				self.string_call_receiver(iterable)
			} else {
				None
			};
		let utf8 = utf8_source.is_some();
		let iterable = match unrolled {
			Some(items) => Expr::Const(Value::array(items)),
			None => utf8_source.unwrap_or_else(|| self.bound(iterable)),
		};

		let (item, prologue) = match pattern {
			Some(pattern) => match syntax::view(self.tree, pattern) {
				View::Name(_) => match self.a.symbols.symbol_of_decl(pattern) {
					Some(symbol) => {
						self.borrowed_symbols.insert(symbol);
						(Some(self.bind(symbol)), None)
					}
					None => (None, None),
				},
				_ => {
					let temp = self.temp();
					let destructure = self.destructure(
						pattern,
						Expr::Lend(Place {
							slot: temp,
							steps: Vec::new(),
						}),
						true,
					);
					(Some(temp), Some(destructure))
				}
			},
			None => (None, None),
		};
		let index_slot = if index {
			let symbol = self.a.symbols.symbols().iter().position(|symbol| {
				symbol.decl_node == n
					&& matches!(symbol.kind, SymbolKind::LoopVariable { copy: true })
			});
			symbol.map(|symbol| self.bind(SymbolId::new(symbol)))
		} else {
			None
		};
		let body = self.expr(body);
		let body = match prologue {
			Some(prologue) => Expr::Block(vec![prologue, body]),
			None => body,
		};
		if utf8 {
			Expr::ForUtf8 {
				item,
				index: index_slot,
				iterable: iterable.boxed(),
				body: body.boxed(),
			}
		} else {
			Expr::For {
				item,
				index: index_slot,
				iterable: iterable.boxed(),
				body: body.boxed(),
			}
		}
	}

	/// `expression catch error { handler }`.
	fn catch(&mut self, n: NodeId, expression: NodeId, handler: NodeId) -> Expr {
		let body = self.expr(expression);
		let slot = self
			.a
			.symbols
			.symbol_of_decl(n)
			.map(|symbol| self.bind(symbol));
		let handler = self.expr(handler);
		Expr::Catch {
			body: body.boxed(),
			slot,
			handler: handler.boxed(),
		}
	}

	fn items(&mut self, items: impl Iterator<Item = NodeId>) -> Vec<Item> {
		items
			.map(|item| match syntax::view(self.tree, item) {
				View::Spread(value) => Item::Spread(self.expr(value)),
				_ => Item::One(self.expr(item)),
			})
			.collect()
	}

	/// The name of a module-level value an assignment writes (impure).
	fn module_target(&self, target: NodeId) -> Option<String> {
		let mut node = target;
		loop {
			match syntax::view(self.tree, node) {
				View::Access { target, .. } | View::Index { target, .. } => node = target,
				View::Group(inner) => node = inner,
				View::Name(name) => {
					return match self.a.resolution(node) {
						Resolution::Local(symbol) if self.is_module_level(symbol) => {
							Some(self.tree.name(name).to_string())
						}
						Resolution::External(..) => Some(self.tree.name(name).to_string()),
						_ => None,
					};
				}
				_ => return None,
			}
		}
	}

	/// An assignable place: a local and the fields and items below it.
	fn place(&mut self, n: NodeId) -> Option<Place> {
		match syntax::view(self.tree, n) {
			View::Group(inner) | View::Try(inner) => self.place(inner),
			View::Name(_) => match self.a.resolution(n) {
				Resolution::Local(symbol) => self.slots.get(&symbol).map(|&slot| Place {
					slot,
					steps: Vec::new(),
				}),
				_ => None,
			},
			View::Access { target, members } => {
				let mut place = self.place(target)?;
				for member in members.iter() {
					let View::Name(name) = syntax::view(self.tree, member) else {
						return None;
					};
					place
						.steps
						.push(Step::Field(Arc::from(self.tree.name(name))));
				}
				Some(place)
			}
			View::Index { target, index } => {
				let mut place = self.place(target)?;
				let index = self.expr(index);
				place.steps.push(if self.a.key_reads.contains(&n) {
					Step::Key(index)
				} else {
					Step::Index(index)
				});
				Some(place)
			}
			_ => None,
		}
	}

	fn strip_groups(&self, mut node: NodeId) -> NodeId {
		while let View::Group(inner) = syntax::view(self.tree, node) {
			node = inner;
		}
		node
	}

	/// The receiver prefix, including flattened field chains and grouped
	/// direct callees. Share it with the allocation-free string views.
	fn call_receiver(&mut self, callee: NodeId) -> Option<Expr> {
		if let Some(place) = self.borrowed_receiver(callee) {
			return Some(Expr::Lend(place));
		}
		let callee = self.strip_groups(callee);
		let (target, members, optional) = match syntax::view(self.tree, callee) {
			View::Access { target, members } => (target, members, false),
			View::OptionalAccess { target, members } => (target, members, true),
			_ => return None,
		};
		Some(self.access(
			callee,
			target,
			members.take(members.len().saturating_sub(1)),
			optional,
		))
	}

	fn string_call_receiver(&mut self, call: NodeId) -> Option<Expr> {
		let View::Call { callee, .. } = syntax::view(self.tree, self.strip_groups(call)) else {
			return None;
		};
		self.call_receiver(callee)
	}

	/// `a.b.c`, `a?.b?.c`.
	fn access(
		&mut self,
		n: NodeId,
		target: NodeId,
		members: syntax::NodeList<'_>,
		optional: bool,
	) -> Expr {
		let counted = self.a.has_flags(n, NodeFlags::CODE_POINT_COUNT)
			|| members
				.first()
				.is_some_and(|first| self.a.has_flags(first, NodeFlags::CODE_POINT_COUNT));
		let count = if counted {
			self.string_call_receiver(target)
				.map(|text| Expr::Builtin("String#codePointCount", vec![text]))
		} else {
			None
		};
		let counted = count.is_some();
		// a member whose value the analysis knows (`Int8.maxValue`)
		if !counted
			&& let Some(value) = self.literal_of(n)
			&& matches!(
				members.last().map(|member| self.a.resolution(member)),
				Some(Resolution::Builtin(_))
			) {
			return Expr::Const(value);
		}
		// the target: a value, or a type or namespace whose member is named
		let static_target = self
			.a
			.has_flags(target, crate::analysis::NodeFlags::TYPE_EXPR)
			|| match self.a.resolution(target) {
				Resolution::Local(symbol) => matches!(
					self.a.symbols.symbol(symbol).kind,
					SymbolKind::Type | SymbolKind::Namespace
				),
				Resolution::Builtin(id) => {
					!matches!(builtins::get(id).kind, builtins::BuiltinKind::Value(_))
				}
				Resolution::Folder(_) => true,
				Resolution::External(..) => self
					.declared_of(self.a.resolution(target))
					.is_some_and(|(_, declared)| {
						matches!(declared.kind, SymbolKind::Type | SymbolKind::Namespace)
					}),
				_ => false,
			};
		let mut current = if let Some(count) = count {
			Some(count)
		} else if static_target {
			None
		} else {
			Some(self.expr(target))
		};
		for (index, member) in members.iter().enumerate().skip(usize::from(counted)) {
			let resolution = self.a.resolution(member);
			let name: Arc<str> = match syntax::view(self.tree, member) {
				View::Name(name) => Arc::from(self.tree.name(name)),
				_ => return self.unsupported("this member"),
			};
			current = Some(match (current, resolution) {
				(Some(value), Resolution::Builtin(id)) => {
					let path = builtins::get(id).path;
					match path {
						"Array#capacity"
							if self
								.types()
								.inline_array_types(self.a.type_of(if index == 0 {
									target
								} else {
									members.get(index - 1).expect("a receiver")
								}))
								.is_some() =>
						{
							let receiver = if index == 0 {
								target
							} else {
								members.get(index - 1).expect("a receiver")
							};
							let (_, capacity) = self
								.types()
								.inline_array_types(self.a.type_of(receiver))
								.expect("an inline array");
							Expr::Block(vec![value, self.generic_value(capacity)])
						}
						"Range#start" | "Range#end" | "Range#includesEnd" => {
							Expr::Field(value.boxed(), name, optional)
						}
						"Array#capacity"
							if self
								.types()
								.as_int(self.a.type_of(member))
								.is_some_and(|int| int.min == int.max) =>
						{
							let int = self
								.types()
								.as_int(self.a.type_of(member))
								.expect("a constant capacity");
							Expr::Block(vec![value, Expr::Const(Value::Integer(int.min))])
						}
						_ => Expr::Builtin(path, vec![value]),
					}
				}
				(Some(value), _) => Expr::Field(value.boxed(), name, optional),
				(None, resolution) => self.resolved(member, resolution),
			});
		}
		let value = current.unwrap_or_else(Expr::none);
		if self.a.instantiation(n).is_some() {
			self.bind_generics(n, n, value)
		} else {
			value
		}
	}

	/// An object literal: its fields in declaration order (`objects`).
	fn object(&mut self, n: NodeId) -> Expr {
		let Some(info) = self.a.object(n).cloned() else {
			return self.unsupported("this object");
		};
		let mut spreads: FxHashMap<NodeId, Slot> = FxHashMap::default();
		let mut statements = Vec::new();
		let mut fields = Vec::with_capacity(info.fields.len());
		for (name, value) in &info.fields {
			let name: Arc<str> = Arc::from(self.session.types.syms.resolve(*name));
			let value = match *value {
				FieldValue::Given(value) => self.expr(value),
				FieldValue::Default(value) => self.expr(value),
				FieldValue::Constant(index) => match self.a.field_constants.get(index as usize) {
					Some(value) => Expr::Const(value.clone()),
					None => self.unsupported("this default value"),
				},
				FieldValue::None => Expr::none(),
				FieldValue::Spread(value) => {
					let slot = match spreads.get(&value) {
						Some(&slot) => slot,
						None => {
							let slot = self.temp();
							let lowered = self.expr(value);
							statements.push(Expr::Let(slot, lowered.boxed()));
							spreads.insert(value, slot);
							slot
						}
					};
					Expr::Field(Expr::Local(slot).boxed(), Arc::clone(&name), false)
				}
			};
			fields.push((name, value));
		}
		let object = Expr::Object(fields);
		let value = match info.member {
			Some(member) => match self.types().kind(member) {
				TypeKind::Member(nominal, index) => Expr::Member {
					name: Arc::from(member_name(self.types(), nominal, index)),
					kind: self.member_kind(nominal),
					data: Some(object.boxed()),
				},
				_ => object,
			},
			None => object,
		};
		if statements.is_empty() {
			value
		} else {
			statements.push(value);
			Expr::Block(statements)
		}
	}

	/// A call, from its `CallInfo`.
	fn call(&mut self, n: NodeId, callee: NodeId, arguments: syntax::NodeList<'_>) -> Expr {
		let application = self.strip_groups(callee);
		let mut callee = application;
		while self.a.has_flags(callee, NodeFlags::GENERIC_APPLICATION)
			&& let View::Call { callee: inner, .. } = syntax::view(self.tree, callee)
		{
			callee = self.strip_groups(inner);
		}
		let Some(info) = self.a.call(n).cloned() else {
			if self.a.has_flags(n, NodeFlags::GENERIC_APPLICATION) {
				let value = self.expr(callee);
				return self.bind_generics(n, callee, value);
			}
			return self.unsupported(format!("`{}`", self.text(callee)));
		};

		match info.callee {
			Callee::Function(declaration) => {
				let mut args = self.arguments(&info.arguments, Some(callee), arguments);
				self.prepend_generic_arguments(n, application, self.file, declaration, &mut args);
				let symbol = self.a.symbols.symbol_of_decl(declaration);
				let captures = self.a.environment_captures(declaration);
				if captures.is_empty() {
					Expr::CallFunction(fn_key(self.file, declaration), args)
				} else {
					let read = match symbol {
						Some(symbol) => self.read_symbol(callee, symbol),
						None => return self.unsupported("this call"),
					};
					Expr::Call(read.boxed(), args)
				}
			}
			Callee::Builtin(id) => {
				let path = builtins::get(id).path;
				if let Some(impure) = self.builtin_impurity(path) {
					return impure;
				}
				if Self::builtin_borrows(path) {
					let values = info
						.arguments
						.iter()
						.filter_map(|argument| match argument {
							Argument::Given(value) => Some(self.expr(*value)),
							_ => None,
						})
						.collect();
					return Expr::Dereference(self.borrow_builtin(callee, path, values).boxed());
				}
				let method = path.contains('#');
				let mut values = Vec::new();
				let place = if method {
					// the effects table of the built-ins (one source, `builtins/array.rs`)
					if crate::builtins::effects::changes_receiver(path) {
						match self.borrowed_receiver(callee) {
							Some(place) => Some(place),
							None => return self.unsupported(format!("`{path}` on this value")),
						}
					} else {
						match self.call_receiver(callee) {
							Some(value) => values.push(value),
							None => return self.unsupported(format!("`{path}`")),
						}
						None
					}
				} else {
					None
				};
				// Only omitted trailing defaults may disappear. A supplied
				// `none` is an item just like any other value.
				let mut arguments = info.arguments.as_slice();
				if path != "Map#getOrInsert" {
					while matches!(
						arguments.last(),
						Some(Argument::Default | Argument::Missing)
					) {
						arguments = &arguments[..arguments.len() - 1];
					}
				}
				for argument in arguments {
					match argument {
						Argument::Given(value) => values.push(
							if matches!(
								path,
								"Pool#delete" | "Pool#take" | "Columnar#delete" | "Columnar#take"
							) {
								self.borrow_value(*value)
							} else {
								self.expr(*value)
							},
						),
						Argument::Variadic(items) => {
							let items = self.items(items.iter().copied());
							values.push(Expr::Array(items));
						}
						Argument::Receiver(_) => {}
						Argument::Default | Argument::Missing => values.push(Expr::none()),
					}
				}
				if path == "Json.stringify"
					&& let Some(Argument::Given(argument)) = info.arguments.first()
				{
					let (schema, _) = super::json::schema(
						self.types(),
						self.a,
						self.file,
						self.a.type_of(*argument),
					);
					values.push(self.json_schema(&schema));
				}
				if path == "Json.parse" {
					let (schema, defaults) =
						super::json::schema(self.types(), self.a, self.file, self.a.type_of(n));
					let mut expressions = Vec::new();
					let input = self.temp();
					expressions.push(Expr::Let(input, values.remove(0).boxed()));
					values.push(Expr::Local(input));
					let mut closures = Vec::new();
					for key in defaults {
						let expression =
							super::json::default_node(NodeId(key.1)).expect("a JSON default key");
						let mut captures = Vec::new();
						for symbol in self.default_captures(expression) {
							let read = self.read_symbol(expression, symbol);
							let slot = self.temp();
							expressions.push(Expr::Let(slot, read.boxed()));
							captures.push(slot);
						}
						closures.push(Item::One(Expr::Closure {
							function: key,
							captures,
							borrow: false,
						}));
					}
					values.push(Expr::Builtin(
						"$Json.bindDefaults",
						vec![self.json_schema(&schema), Expr::Array(closures)],
					));
					values.push(Expr::Const(Value::Boolean(crate::comptime::is_js_target(
						&self.session.comptime.target,
					))));
					expressions.push(Expr::Builtin(path, values));
					return Expr::Block(expressions);
				}
				// the paths of the flags, by bit
				if path == "Fields#toStringArray"
					&& let View::Access { target, .. } = syntax::view(self.tree, callee)
				{
					values.push(self.fields_names(self.a.type_of(target)));
				}
				if path == "Array#push"
					&& let Some(receiver) = self.receiver_node(callee)
					&& let Some((_, capacity)) =
						self.types().inline_array_types(self.a.type_of(receiver))
				{
					values.push(self.generic_value(capacity));
				}
				match place {
					Some(place) => Expr::Mutate(path, place, values),
					None => Expr::Builtin(path, values),
				}
			}
			Callee::Conversion(ty) => {
				let num = self.num(ty);
				let value = match info.arguments.first() {
					Some(Argument::Given(value)) => self.expr(*value),
					_ => return self.unsupported("this conversion"),
				};
				Expr::Convert(num, value.boxed())
			}
			Callee::Member(member) => {
				let data = match info.arguments.first() {
					Some(Argument::Given(value)) => Some(self.expr(*value).boxed()),
					_ => None,
				};
				match self.types().kind(member) {
					TypeKind::Member(nominal, index) => Expr::Member {
						name: Arc::from(member_name(self.types(), nominal, index)),
						kind: self.member_kind(nominal),
						data,
					},
					_ => self.unsupported("this member"),
				}
			}
			Callee::Value => {
				let mut args = self.arguments(&info.arguments, Some(callee), arguments);
				if let Some((file, node)) = self.callee_function(callee) {
					self.prepend_generic_arguments(n, application, file, node, &mut args);
					return Expr::CallFunction(fn_key(file, node), args);
				}
				let callee = self.expr(callee);
				Expr::Call(callee.boxed(), args)
			}
		}
	}

	fn bind_generics(&mut self, n: NodeId, callee: NodeId, value: Expr) -> Expr {
		let hidden = if let Some((file, function)) = self.callee_function(callee) {
			self.generic_arguments(n, callee, file, function)
				.into_iter()
				.filter_map(|arg| match arg {
					Arg::Value(value) => Some(value),
					_ => None,
				})
				.collect::<Vec<_>>()
		} else {
			let TypeKind::Function(sig) = self.types().kind(self.a.type_of(callee)) else {
				return self.unsupported("this generic function");
			};
			let generics = self
				.types()
				.list_items(self.types().sig(sig).generics)
				.to_vec();
			let bindings = self
				.a
				.instantiation(n)
				.map(|list| self.types().list_items(list).to_vec())
				.unwrap_or_default();
			generics
				.iter()
				.zip(bindings)
				.filter_map(|(&param, ty)| match self.types().kind(param) {
					TypeKind::Param(param) if self.types().param(param).value => {
						Some(self.generic_value(ty))
					}
					_ => None,
				})
				.collect()
		};
		let mut values = vec![value];
		values.extend(hidden);
		if values.len() == 1 {
			values.pop().expect("a callee")
		} else {
			Expr::Builtin("Function#bind", values)
		}
	}

	fn callee_function(&mut self, mut callee: NodeId) -> Option<(FileId, NodeId)> {
		loop {
			callee = match syntax::view(self.tree, callee) {
				View::Group(inner) => inner,
				View::Call { callee: inner, .. }
					if self.a.has_flags(callee, NodeFlags::GENERIC_APPLICATION) =>
				{
					inner
				}
				_ => break,
			};
		}
		let named = match syntax::view(self.tree, callee) {
			View::Access { members, .. } => members.last()?,
			_ => callee,
		};
		let resolution = match self.a.resolution(named) {
			Resolution::Local(symbol) => {
				if let Some(&external) = self.externals.get(&symbol) {
					external
				} else {
					let data = self.a.symbols.symbol(symbol);
					return (data.kind == SymbolKind::Function)
						.then_some((self.file, data.decl_node));
				}
			}
			other => other,
		};
		let (file, declared) = self.declared_of(resolution)?;
		let (_, node) = self.session.external_node(resolution)?;
		(declared.kind == SymbolKind::Function).then_some((file, declared.function.unwrap_or(node)))
	}

	/// Hidden compile-time values in the declaration's generic order.
	fn generic_arguments(
		&mut self,
		call: NodeId,
		callee: NodeId,
		file: FileId,
		function: NodeId,
	) -> Vec<Arg> {
		let analysis = if file == self.file {
			self.a
		} else {
			self.session.analysis(file)
		};
		let Some(info) = analysis.function(function) else {
			return Vec::new();
		};
		let mut generics = analysis
			.function_owners
			.get(&function)
			.and_then(|owner| analysis.type_decls.get(owner))
			.map(|owner| owner.generics.clone())
			.unwrap_or_default();
		generics.extend(info.generics.iter().copied());
		let mut bindings = self
			.a
			.instantiation(call)
			.or_else(|| self.a.instantiation(callee))
			.map(|list| self.types().list_items(list).to_vec())
			.unwrap_or_default();
		let mut target = callee;
		while let View::Call { callee, .. } | View::Group(callee) = syntax::view(self.tree, target)
		{
			target = callee;
		}
		if let View::Access { members, .. } = syntax::view(self.tree, target)
			&& let Some(owner) = members
				.last()
				.and_then(|member| self.a.method_instantiations.get(&member))
		{
			bindings.splice(
				0..0,
				self.types().list_items(owner.arguments).iter().copied(),
			);
		}
		let generics = generics
			.into_iter()
			.enumerate()
			.filter_map(|(index, param)| {
				let key_set = analysis.key_set_params.contains_key(&param);
				(self.types().param(param).value || key_set).then_some((index, key_set))
			})
			.collect::<Vec<_>>();
		generics
			.into_iter()
			.map(|(index, key_set)| {
				Arg::Value(bindings.get(index).map_or_else(
					|| self.unsupported("an unresolved compile-time value parameter"),
					|&ty| {
						if key_set {
							self.key_argument(ty)
						} else {
							self.generic_value(ty)
						}
					},
				))
			})
			.collect()
	}

	fn prepend_generic_arguments(
		&mut self,
		call: NodeId,
		callee: NodeId,
		file: FileId,
		function: NodeId,
		arguments: &mut fa_interpreter::ir::Call,
	) {
		let mut hidden = self.generic_arguments(call, callee, file, function);
		let count = hidden.len();
		if count == 0 {
			return;
		}
		if arguments.order.is_empty() {
			arguments.order.extend(0..arguments.arguments.len());
		}
		for index in &mut arguments.order {
			*index += count;
		}
		arguments.order.splice(0..0, 0..count);
		hidden.append(&mut arguments.arguments);
		arguments.arguments = hidden;
	}

	/// Substitute VM value parameters only along the schema's dynamic paths.
	/// Static subtrees stay shared constants, including lazy field defaults.
	fn json_schema(&self, schema: &Value) -> Expr {
		fn dynamic(lowerer: &Lowerer<'_>, value: &Value) -> Option<Expr> {
			match value {
				Value::Member(member) => Some(Expr::Member {
					name: Arc::clone(&member.name),
					kind: member.kind,
					data: Some(dynamic(lowerer, member.data.as_ref()?)?.boxed()),
				}),
				Value::Object(object) => {
					let capacity = object.get("$capacityType").and_then(Value::as_int);
					let mut changed = capacity.is_some();
					let fields = object
						.fields
						.iter()
						.filter(|(name, _)| name.as_ref() != "$capacityType")
						.map(|(name, value)| {
							let value = if name.as_ref() == "capacity"
								&& let Some(ty) = capacity
							{
								lowerer.generic_value(TypeId(ty.as_u32()))
							} else if let Some(expression) = dynamic(lowerer, value) {
								changed = true;
								expression
							} else {
								Expr::Const(value.clone())
							};
							(Arc::clone(name), value)
						})
						.collect();
					changed.then_some(Expr::Object(fields))
				}
				Value::Array(items) => {
					let mut changed = false;
					let items = items
						.iter()
						.map(|value| {
							Item::One(if let Some(expression) = dynamic(lowerer, value) {
								changed = true;
								expression
							} else {
								Expr::Const(value.clone())
							})
						})
						.collect();
					changed.then_some(Expr::Array(items))
				}
				_ => None,
			}
		}
		dynamic(self, schema).unwrap_or_else(|| Expr::Const(schema.clone()))
	}

	fn key_argument(&self, binding: TypeId) -> Expr {
		match self.types().kind(binding) {
			TypeKind::Param(parameter) => {
				self.value_slots.get(&parameter).copied().map(Expr::Local)
			}
			TypeKind::Tuple(list) => match self.types().list_items(list) {
				&[fields, bits] => {
					self.types()
						.as_int(bits)
						.and_then(|int| int.literal())
						.map(|bits| {
							Expr::Const(Value::array(
								self.types()
									.fields_leaves(fields)
									.iter()
									.enumerate()
									.filter(|(index, _)| {
										(bits >> *index as u32) & I256::ONE == I256::ONE
									})
									.map(|(_, path)| Value::string(path))
									.collect(),
							))
						})
				}
				_ => None,
			},
			_ => None,
		}
		.unwrap_or_else(|| self.unsupported("this key-set argument"))
	}

	fn generic_value(&self, ty: TypeId) -> Expr {
		if let TypeKind::Param(param) = self.types().kind(ty) {
			let info = self.types().param(param);
			return self
				.value_slots
				.iter()
				.find_map(|(&candidate, &slot)| {
					let other = self.types().param(candidate);
					(info.owner == other.owner && info.index == other.index)
						.then_some(Expr::Local(slot))
				})
				.unwrap_or_else(|| self.unsupported("an unresolved compile-time value parameter"));
		}
		match self.literal_type(ty) {
			Some(value) => Expr::Const(value),
			None => self.unsupported("this compile-time value parameter"),
		}
	}

	fn receiver_node(&self, callee: NodeId) -> Option<NodeId> {
		let View::Access { target, members } = syntax::view(self.tree, callee) else {
			return None;
		};
		if members.len() == 1 {
			Some(target)
		} else {
			members.get(members.len().checked_sub(2)?)
		}
	}

	fn arguments(
		&mut self,
		arguments: &[Argument],
		callee: Option<NodeId>,
		written: syntax::NodeList<'_>,
	) -> fa_interpreter::ir::Call {
		let order = crate::lower::calls::parameter_order(self.tree, written, arguments);
		let references: Vec<_> = callee
			.and_then(|callee| match self.types().kind(self.a.type_of(callee)) {
				TypeKind::Function(sig) => Some(
					self.types()
						.params(self.types().sig(sig).params)
						.iter()
						.map(|param| param.reference.is_some())
						.collect(),
				),
				_ => None,
			})
			.unwrap_or_default();
		let skip = arguments.len().saturating_sub(references.len());
		let mut args = Vec::with_capacity(arguments.len());
		for (position, argument) in arguments.iter().enumerate() {
			args.push(match argument {
				Argument::Given(value) => Arg::Value(
					if position
						.checked_sub(skip)
						.and_then(|position| references.get(position))
						.copied()
						.unwrap_or(false)
					{
						self.borrow_value(*value)
					} else {
						self.bound_argument(*value)
					},
				),
				Argument::Default | Argument::Missing => Arg::Default,
				Argument::Variadic(items) => Arg::Variadic(self.items(items.iter().copied())),
				Argument::Receiver(_) => {
					let receiver = callee.and_then(|callee| {
						if let Some(place) = self.borrowed_receiver(callee) {
							return Some(Expr::Lend(place));
						}
						match syntax::view(self.tree, callee) {
							View::Access { target, members }
							| View::OptionalAccess { target, members } => {
								let mut value = self.bound(target);
								for member in members.iter().take(members.len().saturating_sub(1)) {
									let View::Name(name) = syntax::view(self.tree, member) else {
										return None;
									};
									value = Expr::Field(
										value.boxed(),
										Arc::from(self.tree.name(name)),
										false,
									);
								}
								Some(value)
							}
							_ => None,
						}
					});
					Arg::Value(receiver.unwrap_or_else(|| self.unsupported("this receiver")))
				}
			});
		}
		fa_interpreter::ir::Call {
			arguments: args,
			order,
		}
	}

	/// A `when`: its subject in a temporary, then its decisions in order.
	fn when(&mut self, n: NodeId) -> Expr {
		let Ok(list) = when::decision_list(self.tree, self.a, n) else {
			return self.unsupported("this `when`");
		};
		let subject = self.temp();
		let subject_value = self.expr(list.subject);
		let mut chain: Option<Expr> = None;
		for decision in list.decisions.iter().rev() {
			let mut tests: Option<Expr> = None;
			let mut member_data = false;
			for test in &decision.tests {
				let expr = match test {
					Test::Equals(literal) => {
						let value = match literal {
							Literal::Integer(value) => Value::Integer(*value),
							Literal::Float(value) => Value::Float(*value),
							Literal::String(text) => Value::string(text),
							Literal::Boolean(value) => Value::Boolean(*value),
							Literal::None => Value::None,
						};
						Expr::Equals(
							Expr::Local(subject).boxed(),
							Expr::Const(value).boxed(),
							false,
						)
					}
					Test::InRange {
						start,
						end,
						inclusive,
					} => {
						let mut parts = Vec::new();
						if let Some(start) = start {
							parts.push(Expr::Compare(
								IrCompare::GreaterOrEqual,
								Expr::Local(subject).boxed(),
								self.expr(*start).boxed(),
							));
						}
						if let Some(end) = end {
							let op = if *inclusive {
								IrCompare::LessOrEqual
							} else {
								IrCompare::Less
							};
							parts.push(Expr::Compare(
								op,
								Expr::Local(subject).boxed(),
								self.expr(*end).boxed(),
							));
						}
						let integer = Expr::Is(
							Expr::Local(subject).boxed(),
							TypeTest::Integer {
								min: I256::MIN,
								max: I256::MAX,
							},
							false,
						);
						parts
							.into_iter()
							.fold(integer, |all, part| Expr::And(all.boxed(), part.boxed()))
					}
					Test::IsType(ty) => {
						if matches!(self.types().kind(*ty), TypeKind::Member(..)) {
							member_data = true;
						}
						Expr::Is(Expr::Local(subject).boxed(), self.type_test(*ty), false)
					}
				};
				tests = Some(match tests {
					Some(previous) => Expr::Or(previous.boxed(), expr.boxed()),
					None => expr,
				});
			}
			let mut body = Vec::new();
			if let Some(binding) = decision.binding {
				let source = if member_data {
					Expr::MemberData(Expr::Local(subject).boxed())
				} else {
					Expr::Local(subject)
				};
				body.push(self.bind_as(binding, source));
			}
			body.push(self.expr(decision.value));
			let arm = Expr::Block(body);
			chain = Some(match (tests, chain) {
				(None, _) => arm,
				(Some(tests), Some(rest)) => {
					Expr::If(tests.boxed(), arm.boxed(), Some(rest.boxed()))
				}
				(Some(tests), None) => Expr::If(tests.boxed(), arm.boxed(), None),
			});
		}
		Expr::Block(vec![
			Expr::Let(subject, subject_value.boxed()),
			chain.unwrap_or_else(Expr::none),
		])
	}

	/// `as name`, `as { a, b }` of a `when` arm.
	fn bind_as(&mut self, binding: NodeId, source: Expr) -> Expr {
		match syntax::view(self.tree, binding) {
			View::Name(_) => match self.a.symbols.symbol_of_decl(binding) {
				Some(symbol) => {
					let slot = self.bind(symbol);
					Expr::Let(slot, source.boxed())
				}
				None => Expr::none(),
			},
			View::ObjectPattern { fields, .. } => {
				let temp = self.temp();
				let mut statements = vec![Expr::Let(temp, source.boxed())];
				let symbols: Vec<SymbolId> = (0..self.a.symbols.symbols().len())
					.map(SymbolId::new)
					.filter(|&symbol| self.a.symbols.symbol(symbol).decl_node == binding)
					.collect();
				for (field, symbol) in fields.iter().zip(symbols) {
					let name: Arc<str> = Arc::from(self.tree.name(field.name));
					let slot = self.bind(symbol);
					statements.push(Expr::Let(
						slot,
						Expr::Field(Expr::Local(temp).boxed(), name, false).boxed(),
					));
				}
				Expr::Block(statements)
			}
			_ => self.unsupported("this binding"),
		}
	}

	/// A destructuring pattern bound from a value (`patterns`).
	fn destructure(&mut self, pattern: NodeId, source: Expr, borrow: bool) -> Expr {
		let Some(info) = self.a.pattern(pattern).cloned() else {
			return match self.a.symbols.symbol_of_decl(pattern) {
				Some(symbol) => {
					let slot = self.bind(symbol);
					if borrow {
						self.borrowed_symbols.insert(symbol);
						Expr::Bind(slot, source.boxed())
					} else {
						Expr::Let(slot, source.boxed())
					}
				}
				None => self.unsupported("this pattern"),
			};
		};
		let temp = self.temp();
		let mut statements = vec![if borrow {
			Expr::Bind(temp, source.boxed())
		} else {
			Expr::Let(temp, source.boxed())
		}];
		for (position, element) in info.elements.iter().enumerate() {
			let read = match element.field {
				Some(field) => Expr::Field(
					Expr::Local(temp).boxed(),
					Arc::from(self.session.types.syms.resolve(field)),
					false,
				),
				None => Expr::Index(
					Expr::Local(temp).boxed(),
					Expr::Const(Value::int(position as i64)).boxed(),
				),
			};
			let read = if borrow {
				let step = match element.field {
					Some(field) => Step::Field(Arc::from(self.session.types.syms.resolve(field))),
					None => Step::Index(Expr::Const(Value::int(position as i64))),
				};
				Expr::Lend(Place {
					slot: temp,
					steps: vec![step],
				})
			} else {
				read
			};
			let read = if element.absent {
				element
					.default
					.map_or_else(Expr::none, |default| self.expr(default))
			} else {
				match element.default {
					Some(default) => Expr::Fallback(
						read.boxed(),
						(if borrow {
							self.bound(default)
						} else {
							self.expr(default)
						})
						.boxed(),
					),
					None => read,
				}
			};
			match element.binding {
				PatternBinding::Symbol(symbol) => {
					let slot = self.bind(symbol);
					statements.push(if borrow {
						self.borrowed_symbols.insert(symbol);
						Expr::Bind(slot, read.boxed())
					} else {
						Expr::Let(slot, read.boxed())
					});
				}
				PatternBinding::Nested(nested) => {
					statements.push(self.destructure(nested, read, borrow))
				}
				PatternBinding::None => {}
			}
		}
		if let Some((symbol, _)) = info.rest {
			let slot = self.bind(symbol);
			let count = info.elements.len() as i64;
			let rest = match self.types().kind(info.ty) {
				TypeKind::Object(_) => {
					// the fields not named by the pattern
					self.unsupported("`...rest` of an object pattern")
				}
				_ => Expr::Index(
					Expr::Local(temp).boxed(),
					Expr::Range(Some(Expr::Const(Value::int(count)).boxed()), None, false).boxed(),
				),
			};
			statements.push(Expr::Let(slot, rest.boxed()));
		}
		Expr::Block(statements)
	}
}

/// The node of a function's `FnInfo`: its declaration, or the lambda of an
/// exported function.
pub fn function_node(analysis: &Analysis, decl: NodeId) -> NodeId {
	if analysis.function(decl).is_some() {
		return decl;
	}
	analysis
		.functions
		.iter()
		.find(|(_, info)| info.declaration == Some(decl))
		.map_or(decl, |(&node, _)| node)
}

#[cfg(test)]
mod ownership_tests {
	use super::*;

	#[test]
	fn explicit_compile_time_copies_convert_items_and_project_fields() {
		let (session, file) = crate::analyze_source(
			"function converted = (): Array<{ keep: Float }> => {\nlet rows = [{ keep = Float32(1.25), omitted = [2] }]\nlet result: Array<{ keep: Float }> = copy rows\nassert rows[0].omitted.length == 1\nresult\n}\nstatic result = @converted()\n",
		);
		assert!(
			session.diagnostics(file).is_empty(),
			"{:?}",
			session.diagnostics(file)
		);
		let result = session
			.analysis(file)
			.constants
			.values()
			.find_map(|value| match value {
				Value::Array(items) => Some(items),
				_ => None,
			})
			.expect("the computed array");
		let Value::Object(object) = &result[0] else {
			panic!("a converted record")
		};
		assert_eq!(object.fields.len(), 1);
		assert!(matches!(object.get("keep"), Some(Value::Float(value)) if *value == 1.25));
	}

	#[test]
	fn checked_last_uses_lower_to_slot_transfers() {
		let (session, file) = crate::analyze_source(
			"function make = (): Array<Integer> => { let original = [1]\nlet result = original\nresult }\n",
		);
		assert!(
			session
				.diagnostics(file)
				.iter()
				.all(|diagnostic| diagnostic.severity != crate::Severity::Error)
		);
		let tree = session.tree(file);
		let analysis = session.analysis(file);
		let declaration = analysis
			.functions
			.iter()
			.find(|(_, info)| {
				info.symbol.is_some_and(|symbol| {
					session
						.types
						.syms
						.resolve(analysis.symbols.symbol(symbol).name)
						== "make"
				})
			})
			.map(|(&node, _)| node)
			.unwrap();
		let function = Lowerer::new(&session, file, tree, analysis)
			.function(declaration)
			.unwrap();
		let mut moves = 0;
		crate::comptime::visit(&function.body, &mut |expression| {
			moves += usize::from(matches!(expression, Expr::Move(_)));
		});
		assert_eq!(moves, 2);
	}
}
