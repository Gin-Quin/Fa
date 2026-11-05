//! `Fields` types and key sets (types brief §3.9, `types/fields.md`).
//!
//! A `Fields` type is a `TypeKind::Fields` flag tree: its identity is its
//! keys in order, nested the same way (`TypeStore::fields_type`), so two
//! declarations with the same flags in the same order are one type, and a
//! key set built by `Keys<T>` is one of them. A value is an unsigned integer
//! with one bit per leaf, in declaration order (`TypeStore::fields_leaves`).
//!
//! - `Fields { read, write }` in a type: unique names per level (`TY090`),
//!   at most 128 leaves (`TY091`).
//! - Values: `T { read, nested { a } }`, `T.read`, `T.nested`, or a key set
//!   `{ read }` where a `Fields` type is expected. A name that is not a flag
//!   is `TY092` (`TF010` for a key set given as a compile-time argument).
//!   Each of these nodes is a compile-time value (`Analysis::constants`,
//!   the bits as an `Integer`): backends emit the number.
//! - `a + b` (union), `a - b` (difference), `x is in s` (every flag of `x`
//!   is in `s`), `==`; both sides of the same `Fields` type. Constant
//!   operands give a constant result.
//! - `Integer(s)` (any integer type holding every value of the set) and
//!   `s.toStringArray()` (`Fields#toStringArray`: the dotted paths of its
//!   flags, in bit order).
//!
//! Key sets as compile-time values (`Analysis::key_set_params`,
//! `key_loops`, `key_reads`):
//!
//! - `keys: Keys<Value>` / `DeepKeys<Value>`, a compile-time value parameter
//!   whose subject is a type parameter of the same declaration: an
//!   application binds it to `Tuple(Fields type, bits)`, the key set
//!   checked against the keys of the subject's argument (`TF010`). In the
//!   body, it is the iterable of an `@for` or the keys of `pick`/`omit`
//!   (a derived generic parameter, `type_ops.rs`).
//! - `@for key in keys` over such a parameter, a `static` key set or a key
//!   set literal: `key` is the dotted path of each key (a `String`), and
//!   `value[key]` reads the field at that path. Its type is a type
//!   parameter of its own (unconstrained), bound per key to the field's
//!   type: the body is checked once, like generic code (`TF011` for a
//!   target that doesn't hold the keys).

use std::sync::Arc;

use ethnum::I256;
use smallvec::SmallVec;

use rustc_hash::FxHashMap;

use crate::check::Walker;
use crate::diagnostics::Code;
use crate::diagnostics::codes::types::{TF010, TF011, TY090, TY091, TY092};
use crate::diagnostics::codes::values::E_TYPE_1;
use crate::ids::{NodeId, ParamId, Sym, SymbolId, TypeId};
use crate::symbols::{DeclState, Resolution, SymbolKind};
use crate::syntax::{self, ArithOp, Name, NodeList, View};
use crate::types::{FieldEntry, FieldFlags, ParamInfo, TypeKind};

/// The most leaves a `Fields` type may have (its bits fit 128 bits).
const MAX_FLAGS: u32 = 128;

/// The key sets of a walk.
#[derive(Default)]
pub(crate) struct KeySetState {
	/// The compile-time value parameters holding a key set, by symbol.
	params: FxHashMap<SymbolId, KeySetParam>,
	/// The variables of the key loops (by pattern node): what they go through.
	loop_variables: FxHashMap<NodeId, KeyLoop>,
	/// The iterable of the `@for` being checked: a key-set parameter is
	/// read there only.
	iterable: Option<NodeId>,
	/// The type whose keys a key set being checked names (`TF010` messages).
	owner: Option<TypeId>,
	/// The order the fields of anonymous object types were written in
	/// (`Keys<T>` lists them in that order).
	field_orders: FxHashMap<TypeId, Vec<Sym>>,
}

/// A compile-time value parameter holding a key set (`keys: DeepKeys<Value>`).
#[derive(Copy, Clone, Debug)]
pub(crate) struct KeySetParam {
	/// The parameter.
	pub param: ParamId,
	/// The type parameter whose keys it holds.
	pub subject: ParamId,
	/// `DeepKeys`: the keys of nested objects too.
	pub deep: bool,
}

/// What the variable of a key loop goes through.
#[derive(Clone, Debug)]
enum KeyLoop {
	/// The keys of a key-set parameter: of a value of its subject.
	Param(ParamId),
	/// These keys (dotted paths).
	Keys(Vec<String>),
}

impl Walker<'_> {
	/// Whether a type is a `Fields` type.
	pub(super) fn is_fields(&self, ty: TypeId) -> bool {
		!ty.is_unset() && matches!(self.s.types.kind(ty), TypeKind::Fields(_))
	}

	/// The `Fields` type an expected type asks for: itself, or the only
	/// `Fields` member of an optional.
	pub(super) fn expected_fields(&self, expected: Option<TypeId>) -> Option<TypeId> {
		let expected = expected?;
		if self.is_fields(expected) {
			return Some(expected);
		}
		match self.s.types.union_members(&expected) {
			[a, b] if *b == TypeId::NONE && self.is_fields(*a) => Some(*a),
			[a, b] if *a == TypeId::NONE && self.is_fields(*b) => Some(*b),
			_ => None,
		}
	}

	// ── types ────────────────────────────────────────────────────────────

	/// `Fields { read, write, nested { a, b } }` in a type.
	pub(super) fn fields_type_expr(&mut self, n: NodeId, members: NodeList<'_>) -> TypeId {
		self.memo(n, |w| {
			let Some(ty) = w.fields_level(members, 0) else {
				return TypeId::UNKNOWN;
			};
			let width = w.s.types.fields_width(ty);
			if width > MAX_FLAGS {
				let message = format!(
					"a `Fields` type has at most {MAX_FLAGS} flags, counting the members of its groups; this one has {width}: split it"
				);
				let diagnostic = w.diagnostic(&TY091, n, message);
				w.report(diagnostic);
				return TypeId::UNKNOWN;
			}
			ty
		})
	}

	/// One level of a `Fields` declaration: its flags and groups.
	fn fields_level(&mut self, members: NodeList<'_>, depth: u32) -> Option<TypeId> {
		let tree = Arc::clone(&self.tree);
		let mut keys: Vec<(String, TypeId)> = Vec::with_capacity(members.len());
		let mut complete = true;
		for member in members.iter() {
			self.set_flag(member, crate::analysis::NodeFlags::TYPE_EXPR);
			let (name, node, children) = match syntax::view(&tree, member) {
				View::Name(name) => (tree.name(name).to_string(), member, TypeId::UNSET),
				View::TypedObject { target, members }
					if matches!(syntax::view(&tree, target), View::Name(_)) =>
				{
					let View::Name(name) = syntax::view(&tree, target) else {
						unreachable!("checked by the guard");
					};
					self.set_flag(target, crate::analysis::NodeFlags::TYPE_EXPR);
					if members.is_empty() {
						let message = format!(
							"the group `{}` has no flag: list its flags, ex: `{} {{ a, b }}`",
							tree.name(name),
							tree.name(name)
						);
						let diagnostic = self.diagnostic(&TY090, member, message);
						self.report(diagnostic);
						complete = false;
						continue;
					}
					if depth >= 16 {
						let diagnostic = self.diagnostic(
							&TY090,
							member,
							"the groups of `Fields` are nested too deeply",
						);
						self.report(diagnostic);
						return None;
					}
					let Some(children) = self.fields_level(members, depth + 1) else {
						complete = false;
						continue;
					};
					(tree.name(name).to_string(), target, children)
				}
				_ => {
					let diagnostic = self.diagnostic(
						&TY090,
						member,
						"a `Fields` type lists flag names and groups, ex: `Fields { read, write, nested { a, b } }`",
					);
					self.report(diagnostic);
					complete = false;
					continue;
				}
			};
			if keys.iter().any(|(other, _)| *other == name) {
				let message = format!("the flag `{name}` is listed twice at this level");
				let diagnostic = self.diagnostic(&TY090, node, message);
				self.report(diagnostic);
				complete = false;
				continue;
			}
			keys.push((name, children));
		}
		complete.then(|| self.s.types.fields_type(&keys))
	}

	// ── values ───────────────────────────────────────────────────────────

	/// `T { read, nested { a } }`, or a key set `{ read }` where the `Fields`
	/// type `ty` is expected: the value, a compile-time constant.
	pub(super) fn check_fields_literal(
		&mut self,
		n: NodeId,
		ty: TypeId,
		items: NodeList<'_>,
		key_set: bool,
	) -> TypeId {
		self.memo(n, |w| {
			let code = if key_set { &TF010 } else { &TY092 };
			let bits = w.fields_items_bits(ty, items, &mut Vec::new(), code);
			if let Some(bits) = bits {
				w.record_fields_constant(n, bits);
			}
			ty
		})
	}

	/// The bits a list of flags and groups names, below the group `prefix`.
	/// Reports the names that are not keys (`code`) and returns `None` then.
	fn fields_items_bits(
		&mut self,
		ty: TypeId,
		items: NodeList<'_>,
		prefix: &mut Vec<String>,
		code: Code,
	) -> Option<u128> {
		let tree = Arc::clone(&self.tree);
		let mut bits = Some(0u128);
		for item in items.iter() {
			self.set_flag(item, crate::analysis::NodeFlags::TYPE_EXPR);
			let found = match syntax::view(&tree, item) {
				View::Name(name) => self.fields_name_bits(ty, prefix, tree.name(name), item, code),
				View::TypedObject { target, members }
					if matches!(syntax::view(&tree, target), View::Name(_)) =>
				{
					let View::Name(name) = syntax::view(&tree, target) else {
						unreachable!("checked by the guard");
					};
					self.set_flag(target, crate::analysis::NodeFlags::TYPE_EXPR);
					let name = tree.name(name).to_string();
					match self.fields_name_bits(ty, prefix, &name, target, code) {
						Some(_) if prefix.len() >= 16 => None,
						Some(_) => {
							prefix.push(name);
							let inner = self.fields_items_bits(ty, members, prefix, code);
							prefix.pop();
							inner
						}
						None => None,
					}
				}
				_ => {
					let diagnostic = self.diagnostic(
						code,
						item,
						format!(
							"a value of `{}` lists its flags, ex: `{{ {} }}`",
							self.display(ty),
							self.s
								.types
								.fields_leaves(ty)
								.first()
								.map_or("", String::as_str)
						),
					);
					self.report(diagnostic);
					None
				}
			};
			bits = match (bits, found) {
				(Some(bits), Some(found)) => Some(bits | found),
				_ => None,
			};
		}
		bits
	}

	/// The bits of the flag or group `name` below `prefix`; reports it with
	/// `code` when it is not one.
	fn fields_name_bits(
		&mut self,
		ty: TypeId,
		prefix: &[String],
		name: &str,
		node: NodeId,
		code: Code,
	) -> Option<u128> {
		let path: Vec<&str> = prefix
			.iter()
			.map(String::as_str)
			.chain(std::iter::once(name))
			.collect();
		let found = self.s.types.fields_path_bits(ty, &path);
		if found.is_none() {
			let message = if let Some(owner) = self.key_sets.owner {
				format!(
					"`{}` is not a key of `{}`",
					path.join("."),
					self.display(owner)
				)
			} else if prefix.is_empty() {
				format!("`{name}` is not a flag of `{}`", self.display(ty))
			} else {
				format!(
					"`{name}` is not a flag of the group `{}` of `{}`",
					prefix.join("."),
					self.display(ty)
				)
			};
			let diagnostic = self.diagnostic(code, node, message);
			self.report(diagnostic);
		}
		found
	}

	/// Records the value of a field set computed at compile time.
	fn record_fields_constant(&mut self, n: NodeId, bits: u128) {
		let value = I256::from(bits);
		self.a
			.constants
			.insert(n, fa_interpreter::Value::Integer(value));
	}

	/// The bits of a field set known at compile time.
	pub(super) fn fields_constant(&self, n: NodeId) -> Option<u128> {
		let n = self.strip_groups(n);
		match self.a.constants.get(&n)? {
			fa_interpreter::Value::Integer(value) => u128::try_from(*value).ok(),
			_ => None,
		}
	}

	/// `T.read`, `T.nested`, `T.nested.a`: a value of `T`.
	pub(super) fn fields_member_value(
		&mut self,
		n: NodeId,
		ty: TypeId,
		members: NodeList<'_>,
	) -> TypeId {
		let tree = Arc::clone(&self.tree);
		let mut path = Vec::with_capacity(members.len());
		for member in members.iter() {
			let View::Name(name) = syntax::view(&tree, member) else {
				return TypeId::UNKNOWN;
			};
			path.push(tree.name(name));
			self.set_type(member, ty);
		}
		match self.s.types.fields_path_bits(ty, &path) {
			Some(bits) => {
				self.record_fields_constant(n, bits);
				ty
			}
			None => {
				let message = format!(
					"`{}` is not a flag of `{}`",
					path.join("."),
					self.display(ty)
				);
				let node = members.last().unwrap_or(n);
				let diagnostic = self.diagnostic(&TY092, node, message);
				self.report(diagnostic);
				TypeId::UNKNOWN
			}
		}
	}

	/// Whether the integer type `target` holds every value of the `Fields`
	/// type `fields` (`Integer(s)`).
	pub(super) fn fields_fit(&self, fields: TypeId, target: TypeId) -> bool {
		let Some(range) = self.s.types.interval_of(target) else {
			return false;
		};
		let max = I256::from(crate::types::store::fields_mask(
			self.s.types.fields_width(fields),
		));
		range.lo <= I256::ZERO && max <= range.hi
	}

	// ── operations ───────────────────────────────────────────────────────

	/// The operands of `a + b`, `a - b` when one is a key set literal or a
	/// `Fields` type is expected: checked here, a key set with the `Fields`
	/// type of the others. `None` otherwise (they are checked as numbers).
	/// Then [`Walker::fields_operation`] types the operation when an operand
	/// is a `Fields` value.
	pub(super) fn check_fields_operands(
		&mut self,
		op: ArithOp,
		nodes: &[NodeId],
		expected: Option<TypeId>,
	) -> Option<SmallVec<[TypeId; 4]>> {
		if !matches!(op, ArithOp::Add | ArithOp::Subtract) {
			return None;
		}
		let is_key_set = |w: &Self, node: NodeId| {
			matches!(
				syntax::view(&w.tree, w.strip_groups(node)),
				View::Object { .. }
			)
		};
		let expected = self.expected_fields(expected);
		if expected.is_none() && !nodes.iter().any(|&node| is_key_set(self, node)) {
			return None;
		}
		let mut fields = expected;
		let mut types: SmallVec<[TypeId; 4]> = SmallVec::from_elem(TypeId::UNSET, nodes.len());
		for (index, &node) in nodes.iter().enumerate() {
			if is_key_set(self, node) && fields.is_none() {
				continue;
			}
			types[index] = self.check(node, fields);
			if fields.is_none() && self.is_fields(types[index]) {
				fields = Some(types[index]);
			}
		}
		for (index, &node) in nodes.iter().enumerate() {
			if types[index].is_unset() {
				types[index] = self.check(node, fields);
			}
		}
		Some(types)
	}

	/// The type of `a + b` or `a - b` whose operands are checked, one of them
	/// a `Fields` type: that type (`E-TYPE-1` for an operand of another type).
	/// Folds constant operands.
	pub(super) fn fields_operation(
		&mut self,
		n: NodeId,
		op: ArithOp,
		nodes: &[NodeId],
		types: &[TypeId],
	) -> TypeId {
		self.clear_arith(n);
		let ty = self.memo(n, |w| w.fields_operation_type(n, op, nodes, types));
		if self.is_fields(ty) {
			let constants: Option<Vec<u128>> = nodes
				.iter()
				.map(|&node| self.fields_constant(node))
				.collect();
			if let Some(constants) = constants
				&& let Some((&first, rest)) = constants.split_first()
			{
				let bits = rest.iter().fold(first, |acc, &bits| match op {
					ArithOp::Add => acc | bits,
					_ => acc & !bits,
				});
				self.record_fields_constant(n, bits);
			}
		}
		ty
	}

	/// The type of `a + b`, `a - b` (and `+=`, `-=`) whose operands are of
	/// `types`, one of them a `Fields` type: that type, `E-TYPE-1` for
	/// another operator or an operand of another type.
	pub(super) fn fields_operation_type(
		&mut self,
		n: NodeId,
		op: ArithOp,
		nodes: &[NodeId],
		types: &[TypeId],
	) -> TypeId {
		let Some(fields) = types.iter().copied().find(|&ty| self.is_fields(ty)) else {
			return TypeId::UNKNOWN;
		};
		if !matches!(op, ArithOp::Add | ArithOp::Subtract) {
			let message = format!(
				"a `{}` is combined with `+` and `-`, and tested with `is in`",
				self.display(fields)
			);
			let diagnostic = self.diagnostic(&E_TYPE_1, n, message);
			self.report(diagnostic);
			return TypeId::UNKNOWN;
		}
		let mut valid = true;
		for (&node, &ty) in nodes.iter().zip(types) {
			if ty.is_unset() || self.s.types.is_recovering(ty) {
				valid = false;
				continue;
			}
			if ty != fields {
				let message = format!(
					"cannot combine a `{}` with a `{}`: both sides of `{}` are the same `Fields` type",
					self.display(fields),
					self.display(self.s.types.widen(ty)),
					if op == ArithOp::Add { "+" } else { "-" }
				);
				let diagnostic = self.diagnostic(&E_TYPE_1, node, message);
				self.report(diagnostic);
				valid = false;
			}
		}
		if valid { fields } else { TypeId::UNKNOWN }
	}

	/// `x is in s` with `s` of the `Fields` type `fields`: `x` is a value of
	/// the same type (a key set literal allowed).
	pub(super) fn check_fields_is_in(
		&mut self,
		n: NodeId,
		value: NodeId,
		fields: TypeId,
	) -> TypeId {
		let value_ty = self.check(value, Some(fields));
		self.sink(value, value_ty, fields);
		self.memo(n, |_| TypeId::BOOLEAN)
	}

	// ── key sets as compile-time values ──────────────────────────────────

	/// `Keys<Value>`, `DeepKeys<Value>` as the type of a compile-time value
	/// parameter, `Value` a type parameter: the subject and whether it is
	/// deep. The nodes are typed with the placeholder of a key set.
	fn key_set_annotation(&mut self, annotation: NodeId) -> Option<(ParamId, bool)> {
		let tree = Arc::clone(&self.tree);
		let View::Call { callee, arguments } = syntax::view(&tree, annotation) else {
			return None;
		};
		let View::Name(function) = syntax::view(&tree, callee) else {
			return None;
		};
		let deep = match tree.name(function) {
			"Keys" => false,
			"DeepKeys" => true,
			_ => return None,
		};
		if self.is_declared(function) || arguments.len() != 1 {
			return None;
		}
		let subject = arguments.first()?;
		let View::Name(name) = syntax::view(&tree, subject) else {
			return None;
		};
		let sym = self.s.types.syms.intern(tree.name(name));
		let symbol = self.a.symbols.lookup(self.body.scope, sym)?;
		if self.a.symbols.symbol(symbol).kind != SymbolKind::TypeParameter {
			return None;
		}
		let subject_ty = self.a.symbols.symbol(symbol).ty;
		let TypeKind::Param(param) = self.s.types.kind(subject_ty) else {
			return None;
		};
		self.set_resolution(subject, Resolution::Local(symbol));
		let placeholder = self.s.types.fields_type(&[]);
		for (node, ty) in [
			(subject, subject_ty),
			(callee, placeholder),
			(annotation, placeholder),
		] {
			self.set_flag(node, crate::analysis::NodeFlags::TYPE_EXPR);
			self.memo(node, |_| ty);
		}
		Some((param, deep))
	}

	/// Declares a compile-time value parameter whose type is a key set of a
	/// type parameter. `false` when its annotation is not one.
	pub(super) fn declare_key_set_param(
		&mut self,
		param: ParamId,
		generic: NodeId,
		target: NodeId,
		name: Name,
		annotation: NodeId,
	) -> bool {
		let Some((subject, deep)) = self.key_set_annotation(annotation) else {
			return false;
		};
		// checked at each application (`key_set_argument`)
		self.s
			.types
			.set_param_bounds(param, TypeId::UNKNOWN, TypeId::UNSET);
		let ty = self.s.types.fields_type(&[]);
		let symbol = self.declare(generic, name, SymbolKind::Static);
		let data = self.a.symbols.symbol_mut(symbol);
		data.ty = ty;
		data.state = DeclState::Done;
		self.set_type(target, ty);
		self.key_sets.params.insert(
			symbol,
			KeySetParam {
				param,
				subject,
				deep,
			},
		);
		self.a.key_set_params.insert(param);
		true
	}

	/// The key-set parameter a name node reads (`keys` in `Pick<Entity,
	/// keys>`, `pick(entity, keys)`), resolved; `None` for another node.
	pub(super) fn key_set_param_named(&mut self, n: NodeId) -> Option<KeySetParam> {
		let tree = Arc::clone(&self.tree);
		let View::Name(name) = syntax::view(&tree, self.strip_groups(n)) else {
			return None;
		};
		let sym = self.s.types.syms.intern(tree.name(name));
		let symbol = self.a.symbols.lookup(self.body.scope, sym)?;
		let entry = *self.key_sets.params.get(&symbol)?;
		self.set_resolution(n, Resolution::Local(symbol));
		let ty = self.a.symbols.symbol(symbol).ty;
		self.memo(n, |_| ty);
		Some(entry)
	}

	/// The key-set parameter declared by `param`, if it is one.
	fn key_set_param(&self, param: ParamId) -> Option<KeySetParam> {
		self.key_sets
			.params
			.values()
			.copied()
			.find(|entry| entry.param == param)
	}

	/// The argument of a key-set parameter at an application: its key set
	/// checked against the keys of its subject's argument (`TF010`); the
	/// literal it is bound to, `Tuple(Fields type, bits)`. `None` when
	/// `param` is not a key-set parameter.
	pub(super) fn key_set_argument(
		&mut self,
		param: ParamId,
		argument: NodeId,
		params: &[ParamId],
		given: &[TypeId],
	) -> Option<TypeId> {
		let entry = self.key_set_param(param)?;
		self.set_flag(argument, crate::analysis::NodeFlags::TYPE_EXPR);
		let subject = params
			.iter()
			.position(|&other| other == entry.subject)
			.and_then(|index| given.get(index).copied());
		let Some(subject) = subject.filter(|ty| !ty.is_unset()) else {
			let diagnostic = self.diagnostic(
				&TF010,
				argument,
				"the keys of a key set are known once the type before it is given: give the type first",
			);
			self.report(diagnostic);
			return Some(TypeId::UNKNOWN);
		};
		if self.s.types.is_recovering(subject) {
			return Some(TypeId::UNKNOWN);
		}
		let fields = self.keys_type(argument, subject, entry.deep);
		if !self.is_fields(fields) {
			return Some(TypeId::UNKNOWN);
		}
		let tree = Arc::clone(&self.tree);
		let View::Object { items } = syntax::view(&tree, self.strip_groups(argument)) else {
			let diagnostic = self.diagnostic(
				&TF010,
				argument,
				"a key set is written in braces, ex: `{ id, name }`",
			);
			self.report(diagnostic);
			return Some(TypeId::UNKNOWN);
		};
		let saved = self.key_sets.owner.replace(subject);
		let bits = self.fields_items_bits(fields, items, &mut Vec::new(), &TF010);
		self.key_sets.owner = saved;
		let Some(bits) = bits else {
			return Some(TypeId::UNKNOWN);
		};
		self.memo(argument, |_| fields);
		let bits = self.s.types.int_literal(I256::from(bits));
		Some(self.s.types.tuple(&[fields, bits]))
	}

	/// A read of a name: a key-set parameter is only the iterable of an
	/// `@for` (`AN001` elsewhere).
	pub(super) fn check_key_set_read(&mut self, n: NodeId, symbol: SymbolId) {
		if !self.key_sets.params.contains_key(&symbol) || self.key_sets.iterable == Some(n) {
			return;
		}
		self.unsupported_message(n, "a key set parameter outside of the iterable of `@for`");
	}

	/// The fields of an object type in declaration order: as declared, as
	/// written for an anonymous type, else in the order of its shape.
	pub(super) fn ordered_fields(&mut self, object: TypeId) -> Vec<FieldEntry> {
		let mut fields = self.s.types.fields_of(object).to_vec();
		let order: Option<Vec<Sym>> = match self.declared_fields(object) {
			Some((_, declared)) if !declared.is_empty() => {
				Some(declared.iter().map(|field| field.name).collect())
			}
			_ => self.key_sets.field_orders.get(&object).cloned(),
		};
		if let Some(order) = order {
			fields.sort_by_key(|field| {
				order
					.iter()
					.position(|&name| name == field.name)
					.unwrap_or(usize::MAX)
			});
		}
		fields
	}

	/// Records the order the fields of an anonymous object type are written in.
	pub(super) fn record_field_order(&mut self, ty: TypeId, names: Vec<Sym>) {
		self.key_sets.field_orders.entry(ty).or_insert(names);
	}

	/// `@for key in keys` over a key set (a key-set parameter, a `static`
	/// key set, a literal of a `Fields` type): checked once, `key` the dotted
	/// path of a key. `None` when the iterable is not a key set.
	pub(super) fn check_key_loop(
		&mut self,
		n: NodeId,
		pattern: Option<NodeId>,
		index: Option<Name>,
		iterable: NodeId,
		body: NodeId,
	) -> Option<TypeId> {
		let tree = Arc::clone(&self.tree);
		let parameter = match syntax::view(&tree, iterable) {
			View::Name(name) => {
				let sym = self.s.types.syms.intern(tree.name(name));
				self.a
					.symbols
					.lookup(self.body.scope, sym)
					.and_then(|symbol| self.key_sets.params.get(&symbol).copied())
			}
			_ => None,
		};
		let keys = match parameter {
			Some(entry) => {
				let saved = self.key_sets.iterable.replace(iterable);
				self.check(iterable, None);
				self.key_sets.iterable = saved;
				KeyLoop::Param(entry.subject)
			}
			None => {
				let ty = self.check(iterable, None);
				if !self.is_fields(ty) {
					return None;
				}
				self.evaluate_comptime_for(n, iterable);
				let keys = self
					.a
					.unrolled
					.get(&n)
					.map(|items| {
						items
							.iter()
							.filter_map(|item| item.as_str().map(str::to_string))
							.collect()
					})
					.unwrap_or_default();
				KeyLoop::Keys(keys)
			}
		};
		self.a.key_loops.insert(n);
		if let Some(pattern) = pattern {
			self.key_sets.loop_variables.insert(pattern, keys);
		}
		Some(self.check_for_items(n, pattern, index, (iterable, TypeId::STRING), body))
	}

	/// The keys of a `Fields` value known at compile time: the dotted paths
	/// of its flags (the items of an `@for` over a key set).
	pub(super) fn key_items(&self, ty: TypeId, bits: I256) -> Vec<fa_interpreter::Value> {
		self.s
			.types
			.fields_leaves(ty)
			.iter()
			.enumerate()
			.filter(|(index, _)| (bits >> *index as u32) & I256::ONE == I256::ONE)
			.map(|(_, path)| fa_interpreter::Value::string(path))
			.collect()
	}

	/// `value[key]`, `key` the variable of a key loop: the field of `value`
	/// at that key, of a type parameter of its own. `None` for another index.
	pub(super) fn check_key_read(
		&mut self,
		n: NodeId,
		target: NodeId,
		index: NodeId,
	) -> Option<TypeId> {
		let name = self.strip_groups(index);
		let View::Name(variable) = self.view(name) else {
			return None;
		};
		let sym = self.s.types.syms.intern(self.tree.name(variable));
		let symbol = self.a.symbols.lookup(self.body.scope, sym)?;
		let decl = self.a.symbols.symbol(symbol).decl_node;
		let keys = self.key_sets.loop_variables.get(&decl).cloned()?;
		self.check(index, None);
		let target_ty = self.check(target, None);
		Some(self.key_read_type(n, target, target_ty, keys))
	}

	fn key_read_type(
		&mut self,
		n: NodeId,
		target: NodeId,
		target_ty: TypeId,
		keys: KeyLoop,
	) -> TypeId {
		self.memo(n, |w| {
			if w.s.types.is_recovering(target_ty) {
				return TypeId::UNKNOWN;
			}
			match &keys {
				KeyLoop::Param(subject) => {
					if !matches!(w.s.types.kind(target_ty), TypeKind::Param(param) if param == *subject)
					{
						let name =
							w.s.types
								.syms
								.resolve(w.s.types.param(*subject).name)
								.to_string();
						let message = format!(
							"the keys go through the fields of a `{name}`: `{}` is a `{}`",
							w.tree.text(target.index()),
							w.display(target_ty)
						);
						let diagnostic = w.diagnostic(&TF011, n, message);
						w.report(diagnostic);
						return TypeId::UNKNOWN;
					}
				}
				KeyLoop::Keys(paths) => {
					for path in paths {
						if w.field_at_path(target_ty, path).is_none() {
							let message = format!(
								"for the key `{path}`: a `{}` has no field `{path}`",
								w.display(target_ty)
							);
							let diagnostic = w.diagnostic(&TF011, n, message);
							w.report(diagnostic);
							return TypeId::UNKNOWN;
						}
					}
				}
			}
			let module_name = w.s.module(w.file).to_string();
			let module = w.s.types.module(&module_name);
			let owner =
				w.s.types
					.decl_key(module, &["@key"], (n.index() & 0xffff) as u16);
			let name = w.s.types.syms.intern("Field");
			let param = w.s.types.add_param(ParamInfo {
				owner,
				index: 0,
				name,
				constraint: TypeId::UNKNOWN,
				default: TypeId::UNSET,
				value: false,
			});
			w.a.key_reads.insert(n, param);
			w.s.types.intern(TypeKind::Param(param))
		})
	}

	/// The type of the field of an object type at a dotted path.
	pub(super) fn field_at_path(&mut self, ty: TypeId, path: &str) -> Option<TypeId> {
		let mut current = ty;
		for name in path.split('.') {
			let object = self.s.types.unfold(current);
			let sym = self.s.types.syms.intern(name);
			current = self
				.s
				.types
				.fields_of(object)
				.iter()
				.find(|field| field.name == sym && !field.flags.contains(FieldFlags::PRIVATE))?
				.ty;
		}
		Some(current)
	}
}
