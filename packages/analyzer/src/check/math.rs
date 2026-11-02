//! `Math.max(a, b)` and `Math.min(a, b)` (`operations/arithmetic.md`, values
//! brief §6.14): two numbers of the same kind; the result has that kind. On
//! integers, the result has the base type of the operands (as for `+`) and
//! the interval `[max(a1, b1), max(a2, b2)]` (`min` likewise), so that
//! `Math.max(size, 6)` is known to be at least 6. A literal operand takes the
//! kind of the other one (`Math.max(ratio, 0)` is `Float`).

use std::sync::Arc;

use ethnum::I256;

use crate::analysis::{Argument, CallInfo, Callee, NodeFlags};
use crate::builtins;
use crate::check::Walker;
use crate::check::numbers::NumKind;
use crate::diagnostics::codes::types::{FN024, FN025, FN027};
use crate::diagnostics::codes::values::{E_NUM_8, E_NUM_9, E_NUM_11, E_NUM_18, E_NUM_20};
use crate::ids::{BuiltinId, NodeId, TypeId};
use crate::symbols::Resolution;
use crate::syntax::{self, NodeList, View};
use crate::types::{ConstValue, IntStorage, TypeKind};

impl Walker<'_> {
	/// `Math.max(a, b)` / `Math.min(a, b)`, unless `Math` is a local. `None`
	/// for any other callee.
	pub(super) fn check_math_call(
		&mut self,
		n: NodeId,
		callee: NodeId,
		arguments: NodeList<'_>,
	) -> Option<TypeId> {
		let tree = Arc::clone(&self.tree);
		let View::Access { target, members } = syntax::view(&tree, callee) else {
			return None;
		};
		let View::Name(name) = syntax::view(&tree, target) else {
			return None;
		};
		if tree.name(name) != "Math" || members.len() != 1 {
			return None;
		}
		let sym = self.s.types.syms.intern("Math");
		if self.a.symbols.lookup(self.body.scope, sym).is_some() {
			return None;
		}
		let member = members.first()?;
		let math = builtins::lookup("Math")?;
		let function = builtins::member(math, tree.text(member.index()))?;
		let is_max = builtins::get(function).name() == "max";
		self.memo(callee, |w| {
			w.set_resolution(target, Resolution::Builtin(math));
			w.set_resolution(member, Resolution::Builtin(function));
			builtins::value_type(&mut w.s.types, function).unwrap_or(TypeId::UNKNOWN)
		});
		let values: Vec<NodeId> = arguments.iter().collect();
		let named = values
			.iter()
			.any(|&value| matches!(syntax::view(&tree, value), View::Assignment { .. }));
		let [a, b] = values[..] else {
			return Some(self.math_arity(n, function, &values));
		};
		if named {
			return Some(self.math_arity(n, function, &values));
		}
		// a literal takes the kind of the other operand
		let (a_ty, b_ty) = match (self.is_number_literal(a), self.is_number_literal(b)) {
			(true, false) => {
				let b_ty = self.check(b, None);
				let widened = self.s.types.widen(b_ty);
				(self.check(a, Some(widened)), b_ty)
			}
			(false, true) => {
				let a_ty = self.check(a, None);
				let widened = self.s.types.widen(a_ty);
				(a_ty, self.check(b, Some(widened)))
			}
			// two literals: an integer one takes the kind of a float one
			(true, true) if self.is_float_literal_operand(a) => {
				let a_ty = self.check(a, None);
				(a_ty, self.check(b, Some(self.s.types.widen(a_ty))))
			}
			(true, true) if self.is_float_literal_operand(b) => {
				let b_ty = self.check(b, None);
				(self.check(a, Some(self.s.types.widen(b_ty))), b_ty)
			}
			_ => (self.check(a, None), self.check(b, None)),
		};
		let ty = self.memo(n, |w| {
			w.a.calls.insert(
				n,
				CallInfo {
					callee: Callee::Builtin(function),
					arguments: vec![Argument::Given(a), Argument::Given(b)],
				},
			);
			w.math_result(n, function, a_ty, b_ty)
		});
		for (value, actual) in [(a, a_ty), (b, b_ty)] {
			self.sink(value, actual, ty);
		}
		// the interval of the result
		if let Some(range) = self.s.types.interval_of(ty)
			&& let (Some(left), Some(right)) = (self.value_set(a), self.value_set(b))
		{
			let interval = if is_max {
				left.hull.max(right.hull)
			} else {
				left.hull.min(right.hull)
			};
			match interval.intersect(range) {
				Some(inside) => self.record_interval(n, inside, range),
				None => self.record_interval(n, range, range),
			}
		}
		Some(ty)
	}

	/// Whether a node is a float literal (`2.5`, `-2.5`, `(1.5)`).
	fn is_float_literal_operand(&self, n: NodeId) -> bool {
		match self.view(self.strip_groups(n)) {
			View::Float(_) => true,
			View::Negate(inner) => self.is_float_literal_operand(inner),
			_ => false,
		}
	}

	/// The type of `Math.max(a, b)`: the kind both operands share.
	fn math_result(&mut self, n: NodeId, function: BuiltinId, a: TypeId, b: TypeId) -> TypeId {
		let path = builtins::get(function).path;
		match (self.num_kind(a), self.num_kind(b)) {
			(NumKind::Recovering, _) | (_, NumKind::Recovering) => TypeId::UNKNOWN,
			(NumKind::Int(x), NumKind::Int(y)) => match x.common_base(y) {
				Some(base) => TypeId::of_storage(base),
				None => {
					let message = format!(
						"`{path}` between a `{}` and a `{}`: neither type holds the other, convert one",
						self.display(TypeId::of_storage(x)),
						self.display(TypeId::of_storage(y)),
					);
					let diagnostic = self.diagnostic(&E_NUM_11, n, message);
					self.report(diagnostic);
					TypeId::of_storage(IntStorage::Default)
				}
			},
			(x, y) if x == y && !matches!(x, NumKind::Text | NumKind::Other) => {
				self.s.types.widen(a)
			}
			(x, y) => {
				let (a_name, b_name) = (
					self.display(self.s.types.widen(a)),
					self.display(self.s.types.widen(b)),
				);
				let not_number = |kind: NumKind| matches!(kind, NumKind::Text | NumKind::Other);
				let diagnostic = if not_number(x) || not_number(y) {
					let culprit = if not_number(x) { a_name } else { b_name };
					let message = format!("`{path}` takes numbers, not a `{culprit}`");
					self.diagnostic(&E_NUM_18, n, message)
				} else {
					let message = format!(
						"`{path}` takes two numbers of the same kind, not a `{a_name}` and a `{b_name}`: convert one, ex: `Float(…)`"
					);
					self.diagnostic(&E_NUM_11, n, message)
				};
				self.report(diagnostic);
				TypeId::UNKNOWN
			}
		}
	}

	/// `Math.max(a)`, `Math.max(a, b, c)`, named arguments: two positional values.
	fn math_arity(&mut self, n: NodeId, function: BuiltinId, values: &[NodeId]) -> TypeId {
		for &value in values {
			let value = match syntax::view(&self.tree, value) {
				View::Assignment {
					value: Some(inner), ..
				} => inner,
				_ => value,
			};
			self.check(value, None);
		}
		self.memo(n, |w| {
			let path = builtins::get(function).path;
			let named = values
				.iter()
				.any(|&value| matches!(w.view(value), View::Assignment { .. }));
			let (code, message) = if named {
				(
					&FN027,
					format!("`{path}` takes its values by position: `{path}(a, b)`"),
				)
			} else if values.len() < 2 {
				(&FN024, format!("`{path}` takes two values: `{path}(a, b)`"))
			} else {
				(&FN025, format!("`{path}` takes two values: `{path}(a, b)`"))
			};
			let diagnostic = w.diagnostic(code, n, message);
			w.report(diagnostic);
			TypeId::UNKNOWN
		})
	}
}

// ── the proofs of the number API: conversions and shifts ──

impl Walker<'_> {
	/// The errors a call of the number API can throw once its arguments are
	/// known (every visit): `Integer.round(2.7)` and the other float to
	/// integer conversions can't fail on a literal that fits (otherwise
	/// `CONVERSION_CHECK`); a shift amount proven in `[0, width)` can't fail,
	/// one proven outside is `E-NUM-20` (otherwise `SHIFT_CHECK`). Other
	/// calls keep `throws`.
	pub(super) fn number_api_throws(
		&mut self,
		n: NodeId,
		callee: NodeId,
		kind: Callee,
		arguments: &[Argument],
		throws: TypeId,
	) -> TypeId {
		let Callee::Builtin(builtin) = kind else {
			return throws;
		};
		let path = builtins::get(builtin).path;
		let first = match arguments.first() {
			Some(Argument::Given(value)) => *value,
			_ => return throws,
		};
		match path {
			"Integer.round" | "Integer.floor" | "Integer.ceil" | "Integer.truncate" => {
				self.clear_flag(n, NodeFlags::CONVERSION_CHECK);
				let value = self.float_literal_value(first);
				let converted = value.map(|value| match path {
					"Integer.round" => value.round(),
					"Integer.floor" => value.floor(),
					"Integer.ceil" => value.ceil(),
					_ => value.trunc(),
				});
				// `i64::MIN as f64` is exact; `i64::MAX as f64` rounds up to 2^63
				let fits = converted.is_some_and(|value| {
					value.is_finite() && value >= i64::MIN as f64 && value < i64::MAX as f64
				});
				if fits {
					return TypeId::NEVER;
				}
				self.set_flag(n, NodeFlags::CONVERSION_CHECK);
				throws
			}
			"Integer#shiftLeft" | "Integer#shiftRight" => {
				self.clear_flag(n, NodeFlags::SHIFT_CHECK);
				let View::Access { target, .. } = self.view(callee) else {
					return throws;
				};
				let receiver = self.type_of(self.strip_groups(target));
				let Some(storage) = self.s.types.as_int(receiver).map(|int| int.storage) else {
					return throws;
				};
				let width = storage.bits();
				let Some(amount) = self.value_set(first) else {
					return throws;
				};
				let valid = crate::types::Interval::new(I256::ZERO, I256::from(width - 1));
				if valid.contains_interval(amount.hull) {
					return TypeId::NEVER;
				}
				if amount.hull.intersect(valid).is_none() {
					if self.body.flow.reachable {
						let message = format!(
							"the shift amount is outside of 0..{width}: a `{}` has {width} bits",
							self.display(TypeId::of_storage(storage))
						);
						let diagnostic = self.diagnostic(&E_NUM_20, first, message);
						self.report_flow(diagnostic);
					}
					return TypeId::NEVER;
				}
				self.set_flag(n, NodeFlags::SHIFT_CHECK);
				throws
			}
			_ => throws,
		}
	}

	/// A number literal that its sink doesn't accept: an integer that a float
	/// type can't represent exactly (`E-NUM-8`), a number without `%` where a
	/// `Percentage` is expected (`E-NUM-9`). Flow part: reported here.
	pub(super) fn number_literal_sink(
		&mut self,
		n: NodeId,
		actual: TypeId,
		target: TypeId,
	) -> bool {
		let literal = self.s.types.as_int(actual).and_then(|int| int.literal());
		let float = matches!(self.s.types.kind(actual), TypeKind::FloatLit(..));
		let (code, message) = match self.s.types.kind(target) {
			TypeKind::Float(bits) => {
				let Some(value) = literal else {
					return false;
				};
				let exact = match bits {
					crate::types::FloatBits::F32 => {
						let wide = value.as_i128();
						(wide as f32) as i128 == wide && wide.unsigned_abs() <= 1 << 100
					}
					crate::types::FloatBits::F64 => {
						let wide = value.as_i128();
						(wide as f64) as i128 == wide && wide.unsigned_abs() <= 1 << 100
					}
				};
				if exact {
					return false;
				}
				(
					&E_NUM_8,
					format!(
						"{value} can't be represented exactly by a `{}`: write a float literal, ex: `{value}.0`",
						self.display(target)
					),
				)
			}
			TypeKind::Percentage if literal.is_some() || float => {
				let text = self.tree.text(self.strip_groups(n).index()).to_string();
				(
					&E_NUM_9,
					format!("a `Percentage` is written with `%`: write `{text}%`"),
				)
			}
			_ => return false,
		};
		let diagnostic = self.diagnostic(code, n, message);
		self.report_flow(diagnostic);
		true
	}

	/// The value of a float literal argument (`2.7`, `-2.5`).
	fn float_literal_value(&self, n: NodeId) -> Option<f64> {
		let ty = self.type_of(self.strip_groups(n));
		match self.s.types.kind(ty) {
			TypeKind::FloatLit(_, constant) => match self.s.types.const_value(constant) {
				ConstValue::Float(bits) => Some(f64::from_bits(bits)),
				_ => None,
			},
			_ => None,
		}
	}
}
