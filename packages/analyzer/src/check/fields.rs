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
//! Key sets as compile-time values (`Keys<T>` parameters, `@for` over them)
//! are in `key_sets.rs`.

use std::sync::Arc;

use ethnum::I256;
use smallvec::SmallVec;

use crate::check::Walker;
use crate::diagnostics::Code;
use crate::diagnostics::codes::types::{TF010, TY090, TY091, TY092};
use crate::diagnostics::codes::values::E_TYPE_1;
use crate::ids::{NodeId, TypeId};
use crate::syntax::{self, ArithOp, NodeList, View};
use crate::types::TypeKind;

/// The most leaves a `Fields` type may have (its bits fit 128 bits).
const MAX_FLAGS: u32 = 128;

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
	pub(super) fn fields_items_bits(
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

	/// Whether the integer type `target` holds the integer value of every
	/// member an enumeration type may hold (`UInt8(status)`): its pinned
	/// values, else its declaration indices.
	pub(super) fn enum_fits(&self, source: TypeId, target: TypeId) -> bool {
		let types = &self.s.types;
		let Some(range) = types.interval_of(target) else {
			return false;
		};
		let (nominal, members): (_, Vec<u16>) = match types.kind(source) {
			TypeKind::EnumMember(nominal, index) => (nominal, vec![index]),
			TypeKind::Enum(nominal, mask) => {
				let count = types.sym_list(types.nominal(nominal).members).len() as u16;
				(
					nominal,
					(0..count).filter(|&index| types.mask_has(mask, index)).collect(),
				)
			}
			_ => return false,
		};
		members.into_iter().all(|index| {
			let value = types
				.as_int(types.payload(nominal, index))
				.and_then(|ty| ty.literal())
				.unwrap_or(I256::from(index as i128));
			range.contains(value)
		})
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

	/// The pattern of a `when` arm over a set of flags, when it is a value
	/// of the subject's `Fields` type (`T { … }`, `T.flag`, a key set): its
	/// bits, compared with the subject's. `None` for another pattern.
	pub(super) fn fields_when_pattern(
		&mut self,
		pattern: NodeId,
		subject: TypeId,
	) -> Option<TypeId> {
		let fields = self.expected_fields(Some(subject))?;
		let tree = Arc::clone(&self.tree);
		let value = match syntax::view(&tree, self.strip_groups(pattern)) {
			View::Object { .. } | View::TypedObject { .. } => true,
			View::Access { target, .. } => {
				matches!(syntax::view(&tree, target), View::Name(_))
					&& self
						.type_of_target(target)
						.is_some_and(|ty| self.is_fields(ty))
			}
			_ => false,
		};
		if !value {
			return None;
		}
		let ty = self.check(pattern, Some(fields));
		self.sink(pattern, ty, fields);
		self.a.fields_patterns.insert(pattern);
		Some(fields)
	}
}
