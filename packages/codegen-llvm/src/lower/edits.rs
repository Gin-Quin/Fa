//! Strings edited in place (`design/decisions.md`, "`calc`'s edit uses
//! string slices"): an assignment that replaces a string with slices of
//! itself,
//!
//! ```fa
//! text = text[..index] + replacement + text[index + 1..]
//! text = text[..length]
//! ```
//!
//! writes the replacement over the replaced bytes, or shortens the length,
//! when the string owns its heap block (no other value shares it), is simple
//! (a character is a byte), the bounds are in it, and the replacement is
//! simple with the replaced bytes' length. Otherwise the assignment runs as
//! written. The bounds and the replacement must be pure (names, literals,
//! fields, lengths and arithmetic of them: no call), since the general path
//! evaluates them again; the place is the same on both sides.

use fa_analyzer::ids::NodeId;
use fa_analyzer::symbols::Resolution;
use fa_analyzer::syntax::{ArithOp, View};
use inkwell::IntPredicate;
use inkwell::values::IntValue;

use crate::error::Error;
use crate::lower::Lowering;
use crate::lower::values::{IntRepr, Repr, Val};

/// The shape of an edit.
enum Edit {
	/// `place[..start] + replacement + place[end..]`.
	Replace {
		start: NodeId,
		replacement: NodeId,
		end: NodeId,
	},
	/// `place[..length]`.
	Truncate { length: NodeId },
}

impl<'ctx> Lowering<'_, 'ctx> {
	/// The edit in place of `target = value` (see the module): `None` when
	/// the assignment isn't one, else the assignment lowered.
	pub(crate) fn edit_in_place(
		&mut self,
		target: NodeId,
		value: NodeId,
	) -> Result<Option<Val<'ctx>>, Error> {
		if self.tasks.asynchronous || self.node_repr(target)? != Repr::Str {
			return Ok(None);
		}
		let Some(edit) = self.edit_shape(target, value) else {
			return Ok(None);
		};
		let slot = self.place(target)?;
		let header = self
			.load(slot)?
			.value
			.expect("a string")
			.into_struct_value();
		let bytes = self
			.builder
			.build_extract_value(header, 0, "")?
			.into_pointer_value();
		let length = self
			.builder
			.build_extract_value(header, 1, "")?
			.into_int_value();
		let capacity = self
			.builder
			.build_extract_value(header, 2, "")?
			.into_int_value();
		let zero = self.usize.const_zero();
		// a simple string owning its block (its length word is its length)
		let simple = self
			.builder
			.build_int_compare(IntPredicate::SLT, capacity, zero, "")?;
		let owned = self.owned_room(capacity, length)?;
		let mut fast = self.builder.build_and(simple, owned, "")?;
		let write = self.append_block("edit_in_place");
		let general = self.append_block("edit_general");
		let join = self.append_block("edited");
		match edit {
			Edit::Replace {
				start,
				replacement,
				end,
			} => {
				let Some(start) = self.bound(start)? else {
					return Ok(Some(Val::never()));
				};
				let text = self.expr(replacement)?;
				if text.repr == Repr::Never {
					return Ok(Some(text));
				}
				let Some(end) = self.bound(end)? else {
					return Ok(Some(Val::never()));
				};
				let (text_bytes, text_length) = self.string_parts(text)?;
				let text_simple = self.builder.build_int_compare(
					IntPredicate::SLT,
					self.string_capacity(text)?,
					zero,
					"",
				)?;
				// 0 <= start <= end <= length, as unsigned words
				let checks = [
					self.builder
						.build_int_compare(IntPredicate::ULE, start, end, "")?,
					self.builder
						.build_int_compare(IntPredicate::ULE, end, length, "")?,
					text_simple,
				];
				for check in checks {
					fast = self.builder.build_and(fast, check, "")?;
				}
				let replaced = self.builder.build_int_sub(end, start, "")?;
				let same =
					self.builder
						.build_int_compare(IntPredicate::EQ, replaced, text_length, "")?;
				fast = self.builder.build_and(fast, same, "")?;
				let branch = self
					.builder
					.build_conditional_branch(fast, write, general)?;
				self.likely(branch)?;
				self.builder.position_at_end(write);
				let destination = unsafe {
					self.builder
						.build_in_bounds_gep(self.context.i8_type(), bytes, &[start], "")?
				};
				self.call_runtime(
					"fa_copy_bytes",
					&[destination.into(), text_bytes.into(), text_length.into()],
				)?;
			}
			Edit::Truncate { length: kept } => {
				let Some(kept) = self.bound(kept)? else {
					return Ok(Some(Val::never()));
				};
				let inside = self
					.builder
					.build_int_compare(IntPredicate::ULE, kept, length, "")?;
				fast = self.builder.build_and(fast, inside, "")?;
				let branch = self
					.builder
					.build_conditional_branch(fast, write, general)?;
				self.likely(branch)?;
				self.builder.position_at_end(write);
				let length_pointer =
					self.builder
						.build_struct_gep(self.string_type, slot.pointer, 1, "")?;
				self.builder.build_store(length_pointer, kept)?;
			}
		}
		self.builder.build_unconditional_branch(join)?;
		// the assignment as written: its bounds and parts are pure
		self.builder.position_at_end(general);
		// its temporaries are dropped before the paths join
		let mark = self.mark();
		let assigned = self.assignment_as_written(target, value)?;
		self.end_scope(mark, None)?;
		if assigned.repr != Repr::Never {
			self.builder.build_unconditional_branch(join)?;
		}
		self.builder.position_at_end(join);
		Ok(Some(Val::void()))
	}

	/// A bound as a word (`usize`), or `None` when it never ends.
	fn bound(&mut self, node: NodeId) -> Result<Option<IntValue<'ctx>>, Error> {
		let value = self.expr(node)?;
		if value.repr == Repr::Never {
			return Ok(None);
		}
		let value = self.convert(value, Repr::Int(IntRepr::I64), node.index())?;
		Ok(Some(self.builder.build_int_truncate_or_bit_cast(
			value.int(),
			self.usize,
			"",
		)?))
	}

	/// The edit `target = value` is, if any.
	fn edit_shape(&self, target: NodeId, value: NodeId) -> Option<Edit> {
		let value = self.unwrapped(value);
		match self.view(value) {
			View::Arithmetic {
				op: ArithOp::Add,
				operands,
			} if operands.len() == 3 => {
				let first = operands.first()?;
				let replacement = operands.get(1)?;
				let last = operands.get(2)?;
				let (None, Some(start)) = self.slice_of(target, first)? else {
					return None;
				};
				let (Some(end), None) = self.slice_of(target, last)? else {
					return None;
				};
				let replacement = self.unwrapped(replacement);
				let literal = matches!(self.view(replacement), View::String(_));
				let local = matches!(self.view(replacement), View::Name(_))
					&& matches!(self.analysis.resolution(replacement), Resolution::Local(_))
					&& !self.reads_root_of(replacement, target);
				((literal || local) && self.pure(start) && self.pure(end) && self.pure(target))
					.then_some(Edit::Replace {
						start,
						replacement,
						end,
					})
			}
			View::Index { .. } => match self.slice_of(target, value)? {
				(None, Some(length)) if self.pure(length) && self.pure(target) => {
					Some(Edit::Truncate { length })
				}
				_ => None,
			},
			_ => None,
		}
	}

	/// The bounds of `place[start..end]` (exclusive, `try` and groups
	/// around it), when the node is a slice of the place.
	fn slice_of(&self, place: NodeId, node: NodeId) -> Option<(Option<NodeId>, Option<NodeId>)> {
		let node = self.unwrapped(node);
		let View::Index { target, index } = self.view(node) else {
			return None;
		};
		let View::Range {
			start,
			end,
			inclusive: false,
		} = self.view(self.unwrapped(index))
		else {
			return None;
		};
		self.same_place(target, place).then_some((start, end))
	}

	/// The node inside `try` and groups.
	fn unwrapped(&self, mut node: NodeId) -> NodeId {
		while let View::Try(inner) | View::Group(inner) = self.view(node) {
			node = inner;
		}
		node
	}

	/// Whether two nodes designate the same place: a local, or the same
	/// fields of the same place.
	fn same_place(&self, a: NodeId, b: NodeId) -> bool {
		let (a, b) = (self.unwrapped(a), self.unwrapped(b));
		match (self.view(a), self.view(b)) {
			(View::Name(_), View::Name(_)) => {
				matches!(self.analysis.resolution(a), Resolution::Local(_))
					&& self.analysis.resolution(a) == self.analysis.resolution(b)
			}
			(
				View::Access {
					target: first,
					members: first_members,
				},
				View::Access {
					target: second,
					members: second_members,
				},
			) => {
				first_members.len() == second_members.len()
					&& first_members
						.iter()
						.zip(second_members.iter())
						.all(|(x, y)| {
							matches!(self.analysis.resolution(x), Resolution::Field(..))
								&& self.analysis.resolution(x) == self.analysis.resolution(y)
						}) && self.same_place(first, second)
			}
			_ => false,
		}
	}

	/// Whether evaluating a node twice is the same as once: literals,
	/// locals, fields, lengths, and arithmetic of them.
	fn pure(&self, node: NodeId) -> bool {
		match self.view(node) {
			View::Integer(_) | View::String(_) => true,
			View::Group(inner) => self.pure(inner),
			View::Name(_) => matches!(self.analysis.resolution(node), Resolution::Local(_)),
			View::Arithmetic { op, operands } => {
				matches!(
					op,
					ArithOp::Add | ArithOp::Subtract | ArithOp::Multiply | ArithOp::IntegerDivide
				) && operands.iter().all(|operand| self.pure(operand))
			}
			View::Access { target, members } => {
				members
					.iter()
					.all(|member| match self.analysis.resolution(member) {
						Resolution::Field(..) => true,
						Resolution::Builtin(id) => matches!(
							fa_analyzer::builtins::get(id).path,
							"String#length" | "Array#length"
						),
						_ => false,
					}) && self.pure(target)
			}
			_ => false,
		}
	}
}
