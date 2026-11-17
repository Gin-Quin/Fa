//! Bounded inline lists. Capacity is the second collection argument, a
//! compile-time integer type, so substitutions and interfaces retain it.

use ethnum::I256;

use crate::analysis::{Callee, NodeFlags};
use crate::check::Walker;
use crate::diagnostics::codes::{types::TY003, values::E_TYPE_1};
use crate::ids::{NodeId, TypeId};
use crate::syntax::{NodeList, View};
use crate::types::{CollKind, FnSig, Interval, TypeKind};

impl Walker<'_> {
	pub(super) fn inline_array_type(
		&mut self,
		n: NodeId,
		callee: NodeId,
		arguments: NodeList<'_>,
	) -> TypeId {
		self.set_flag(callee, NodeFlags::TYPE_EXPR);
		let item = arguments.first().map(|item| self.check_type_expr(item));
		let capacity = arguments
			.get(1)
			.and_then(|capacity| self.constant_type(capacity));
		let valid = capacity
			.and_then(|ty| {
				let ty = if let TypeKind::Param(param) = self.s.types.kind(ty) {
					self.s.types.param(param).constraint
				} else {
					ty
				};
				self.s.types.as_int(ty)
			})
			.is_some_and(|int| int.min >= I256::ZERO && int.max <= I256::from(u32::MAX));
		if arguments.len() != 2 || item.is_none() || !valid {
			let diagnostic = self.diagnostic(&TY003, n,
				"`InlineArray` needs an item type and a nonnegative capacity known at compile time, ex: `InlineArray<String, 4>`; use a constant from 0 through 4294967295, or a compile-time parameter constrained to UInt32");
			self.report(diagnostic);
			return self.memo(n, |_| TypeId::UNKNOWN);
		}
		let item = item.expect("checked item");
		if item == TypeId::VOID {
			let diagnostic = self.diagnostic(
				&E_TYPE_1,
				n,
				"`Void` has no value: use an item type that can be stored in the inline array",
			);
			self.report(diagnostic);
			return self.memo(n, |_| TypeId::UNKNOWN);
		}
		self.check_stored_iterator_type(n, item, "stored in an inline array");
		let capacity = capacity.expect("checked capacity");
		self.memo(n, |w| {
			w.s.types.collection(CollKind::InlineArray, item, capacity)
		})
	}

	pub(super) fn check_inline_literal(
		&mut self,
		n: NodeId,
		items: NodeList<'_>,
		expected: TypeId,
		item: TypeId,
		capacity: u32,
	) -> TypeId {
		for value in items.iter() {
			if let View::Spread(inner) = self.view(value) {
				self.check(inner, None);
				let diagnostic = self.diagnostic(&E_TYPE_1, value,
					"write the inline array's items explicitly so their count is known; spreads cannot prove that the literal fits its capacity");
				self.report(diagnostic);
			} else {
				let ty = self.check(value, Some(item));
				self.sink(value, ty, item);
				self.store_closures(value, n);
			}
		}
		if items.len() > capacity as usize {
			let message = format!(
				"this inline array literal has {} items, but its capacity is {capacity}: remove items or choose a larger capacity",
				items.len()
			);
			let diagnostic = self.diagnostic(&E_TYPE_1, n, message);
			self.report(diagnostic);
		}
		self.memo(n, |_| expected)
	}

	pub(super) fn inline_member_type(&mut self, ty: TypeId, text: &str, method: TypeId) -> TypeId {
		let Some(capacity) = self.s.types.inline_capacity_range(ty) else {
			return method;
		};
		match text {
			"capacity" => {
				self.s
					.types
					.int_range(capacity.lo, capacity.hi, crate::types::IntStorage::Default)
			}
			"length" => {
				self.s
					.types
					.int_range(I256::ZERO, capacity.hi, crate::types::IntStorage::Default)
			}
			"push" | "delete" | "take" => {
				let TypeKind::Function(sig) = self.s.types.kind(method) else {
					return method;
				};
				let sig = self.s.types.sig(sig);
				let throws = self.builtin_error(if text == "push" {
					"CapacityError"
				} else {
					"IndexError"
				});
				self.s.types.function(FnSig { throws, ..sig })
			}
			_ => method,
		}
	}

	/// The error belongs to the call, rather than the general method type:
	/// exactly as indexing can discharge IndexError using local facts.
	pub(super) fn inline_push_throws(
		&mut self,
		n: NodeId,
		callee: NodeId,
		kind: Callee,
		throws: TypeId,
	) -> TypeId {
		if !matches!(kind, Callee::Builtin(id) if matches!(crate::builtins::get(id).path, "Array#push" | "Array#delete" | "Array#take"))
		{
			return throws;
		}
		let View::Access { target, members } = self.view(self.strip_groups(callee)) else {
			return throws;
		};
		let receiver_type = if members.len() == 1 {
			self.type_of(target)
		} else {
			self.type_of(members.get(members.len() - 2).expect("receiver member"))
		};
		let Some(capacity) = self.s.types.inline_capacity_range(receiver_type) else {
			return throws;
		};
		if !matches!(kind, Callee::Builtin(id) if crate::builtins::get(id).path == "Array#push") {
			let View::Call { arguments, .. } = self.view(n) else {
				return throws;
			};
			if let Some(index) = arguments.first() {
				self.check_index_bounds_of(
					n,
					if members.len() == 1 { target } else { callee },
					index,
					receiver_type,
				);
			}
			return if self.a.has_flags(n, NodeFlags::INDEX_CHECK) {
				throws
			} else {
				TypeId::NEVER
			};
		}
		let bound = capacity;
		let capacity = bound.hi;
		let length = self
			.expression_length(target)
			.unwrap_or(Interval::new(I256::ZERO, capacity));
		self.clear_flag(n, NodeFlags::CAPACITY_CHECK);
		if length.lo >= capacity {
			let message = format!(
				"this inline array is full (capacity {capacity}): remove an item or choose a larger capacity"
			);
			let diagnostic = self.diagnostic(&E_TYPE_1, n, message);
			self.report(diagnostic);
			return TypeId::NEVER;
		}
		if length.hi < bound.lo {
			return TypeId::NEVER;
		}
		self.set_flag(n, NodeFlags::CAPACITY_CHECK);
		throws
	}
}
