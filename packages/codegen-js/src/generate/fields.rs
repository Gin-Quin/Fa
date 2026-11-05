//! `Fields` values (types brief §3.9, `types/fields.md`): a JavaScript
//! number up to 32 flags, a `bigint` above, one bit per flag in declaration
//! order. The analysis gives the bits of every literal (`T { a }`, `T.a`, a
//! key set) as a compile-time value; the operations are bit operations:
//! `+` is `|`, `-` is `& ~`, `x is in s` is `(s & x) === x`. A 32-flag set
//! has its bit 31: the result of a bit operation goes through `>>> 0`.

use fa_analyzer::ids::{NodeId, TypeId};
use fa_analyzer::syntax::ArithOp;
use fa_analyzer::types::TypeKind;

use crate::generate::Generator;
use crate::js::{self, Expr, Prec};

impl Generator<'_> {
	/// Whether a type is a `Fields` type.
	pub(crate) fn is_fields(&self, ty: TypeId) -> bool {
		matches!(self.types.kind(ty), TypeKind::Fields(_))
	}

	/// Whether the values of a `Fields` type are `bigint`s (more than 32 flags).
	pub(crate) fn fields_are_big(&self, ty: TypeId) -> bool {
		self.types.fields_width(ty) > 32
	}

	/// A number result of a bit operation, unsigned when bit 31 may be set.
	fn fields_unsigned(&self, ty: TypeId, value: Expr) -> Expr {
		if self.types.fields_width(ty) == 32 {
			js::binary(
				&value,
				">>>",
				&Expr::constant("0", Prec::Primary),
				Prec::Shift,
			)
		} else {
			value
		}
	}

	/// `a + b + …` (union) and `a - b - …` (difference) of field sets.
	pub(crate) fn fields_arithmetic(&mut self, n: NodeId, op: ArithOp, nodes: &[NodeId]) -> Expr {
		let ty = self.type_of(n);
		let values = self.in_order(nodes, |g, operand| g.expr(operand));
		let mut values = values.into_iter();
		let Some(mut result) = values.next() else {
			return self.unsupported(n, "this operation");
		};
		for value in values {
			result = self.fields_step(ty, op, &result, &value);
		}
		result
	}

	/// One step of `+` (`|`) or `-` (`& ~`) on values of the `Fields` type `ty`.
	pub(crate) fn fields_step(
		&mut self,
		ty: TypeId,
		op: ArithOp,
		left: &Expr,
		right: &Expr,
	) -> Expr {
		let result = match op {
			ArithOp::Add => js::binary(left, "|", right, Prec::BitOr),
			_ => js::binary(left, "&", &js::prefix("~", right), Prec::BitAnd),
		};
		if self.fields_are_big(ty) {
			result
		} else {
			self.fields_unsigned(ty, result)
		}
	}

	/// `value is in set`: every flag of `value` is in `set`.
	pub(crate) fn fields_is_in(&mut self, set_ty: TypeId, set: &Expr, value: &Expr) -> Expr {
		// the value is used twice
		let (value, set) = (value.clone(), set.clone());
		let value = self.keep(value);
		let common = js::binary(&set, "&", &value, Prec::BitAnd);
		let common = if self.fields_are_big(set_ty) {
			common
		} else {
			self.fields_unsigned(set_ty, common)
		};
		js::binary(&common, "===", &value, Prec::Equality)
	}

	/// The module-level table of the dotted paths of the flags of a `Fields`
	/// type, by bit.
	pub(crate) fn fields_names(&mut self, ty: TypeId) -> String {
		let names: Vec<String> = self
			.types
			.fields_leaves(ty)
			.iter()
			.map(|path| js::string_literal(path))
			.collect();
		let wanted = match self.types.type_name(ty) {
			Some(name) => format!("{name}$flags"),
			None => "fields$flags".to_string(),
		};
		self.constant(&wanted, format!("Object.freeze([{}])", names.join(", ")))
	}

	/// `set.toStringArray()`.
	pub(crate) fn fields_to_string_array(&mut self, ty: TypeId, value: Expr) -> Expr {
		let names = self.fields_names(ty);
		let helper = self.helper("faFieldsToStringArray");
		js::call(helper, &[value.text, names])
	}

	/// The function printing the values of a `Fields` type: `{ a, g { b } }`.
	pub(crate) fn fields_formatter(&mut self, ty: TypeId) -> String {
		let names = self.fields_names(ty);
		let helper = self.helper("faFormatFields");
		format!("(value) => {helper}(value, {names})")
	}
}
