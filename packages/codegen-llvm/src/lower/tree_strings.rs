//! Immutable strings stored in a proven root's region. The ordinary String
//! ABI is reconstructed as a borrowed byte view when the member is read.
//! Large strings retain their normal owner and one region cleanup record.

use fa_analyzer::analysis::FieldValue;
use fa_analyzer::ids::{NodeId, NominalId, TypeId};
use fa_analyzer::symbols::{Resolution, SymbolKind};
use fa_analyzer::syntax::View;
use fa_analyzer::types::FieldFlags;
use inkwell::IntPredicate;
use inkwell::builder::Builder;
use inkwell::values::{BasicValueEnum, PointerValue};

use super::Lowering;
use super::values::{Repr, Val};
use crate::error::Error;

const EXTERNAL: u64 = 1 << 62;
const SIMPLE: u64 = 1 << 63;
const SURROGATES: u64 = 1 << 61;

impl<'ctx> Lowering<'_, 'ctx> {
	fn stable_string_storage(&self, node: NodeId) -> bool {
		match self.view(node) {
			View::Group(inner) => self.stable_string_storage(inner),
			View::Name(_) => matches!(self.analysis.resolution(node), Resolution::Local(symbol)
				if matches!(self.analysis.symbols.symbol(symbol).kind,
					SymbolKind::Let | SymbolKind::Mutable | SymbolKind::Parameter { .. } | SymbolKind::SelfParameter)),
			View::Access { target, members } => self.stable_string_storage(target)
				&& members.iter().all(|member| matches!(self.analysis.resolution(member), Resolution::Field(shape, index)
					if !self.types().shape_fields(shape)[index as usize].flags.intersects(FieldFlags::REACTIVE | FieldFlags::RELATION))),
			View::Index { target, .. } => self.node_repr(target).is_ok_and(|repr| matches!(repr, Repr::Array(_)))
				&& self.stable_string_storage(target),
			_ => false,
		}
	}

	/// The only consumer is this one-field constructor, reached through
	/// transparent syntax. Its source place remains alive until the bytes
	/// are frozen; bounds contain no calls or writes that could change it.
	pub(super) fn slice_into_region(&self, node: NodeId, target: NodeId, index: NodeId) -> bool {
		if !self.stable_string_storage(target) {
			return false;
		}
		let Some(root) = fa_analyzer::lower::copies::place_root(self.tree, self.analysis, target)
		else {
			return false;
		};
		if !matches!(
			self.analysis.symbols.symbol(root).kind,
			SymbolKind::Let
				| SymbolKind::Mutable
				| SymbolKind::Parameter { .. }
				| SymbolKind::SelfParameter
		) {
			return false;
		}
		let mut pending = vec![target, index];
		while let Some(at) = pending.pop() {
			if !matches!(
				self.view(at),
				View::Name(_)
					| View::Access { .. }
					| View::Index { .. }
					| View::Group(_)
					| View::Integer(_)
					| View::Range { .. }
					| View::Arithmetic { .. }
					| View::Negate(_)
			) {
				return false;
			}
			if matches!(self.view(at), View::Name(_))
				&& let fa_analyzer::symbols::Resolution::Local(symbol) =
					self.analysis.resolution(at)
				&& !matches!(
					self.analysis.symbols.symbol(symbol).kind,
					SymbolKind::Let
						| SymbolKind::Mutable
						| SymbolKind::Parameter { .. }
						| SymbolKind::SelfParameter
						| SymbolKind::Static
				) {
				return false;
			}
			match self.analysis.resolution(at) {
				Resolution::Field(shape, index)
					if self.types().shape_fields(shape)[index as usize]
						.flags
						.intersects(FieldFlags::REACTIVE | FieldFlags::RELATION) =>
				{
					return false;
				}
				Resolution::Builtin(id)
					if !matches!(
						fa_analyzer::builtins::get(id).path,
						"String#length" | "Array#length" | "Array#capacity"
					) =>
				{
					return false;
				}
				Resolution::External(..) => return false,
				_ => {}
			}
			self.tree.nodes[at.index()].for_each_child(|child| pending.push(NodeId::new(child)));
		}
		let parents = self.parents();
		let mut current = node;
		let mut field_value = node;
		loop {
			let Some(&parent) = parents.get(current.index()) else {
				return false;
			};
			if parent == usize::MAX {
				return false;
			}
			let parent = NodeId::new(parent);
			if let Some(info) = self.analysis.object(parent)
				&& let [(_, FieldValue::Given(value))] = info.fields.as_slice()
				&& *value == field_value
				&& info
					.member
					.and_then(|ty| self.member_of_type(ty))
					.is_some_and(|(nominal, index)| {
						self.compact_string_payload(nominal, index).is_some()
					}) {
				return true;
			}
			match self.view(parent) {
				View::Group(inner) | View::Try(inner) if inner == current => field_value = parent,
				View::Catch {
					expression,
					handler,
					..
				} if expression == current && matches!(self.view(handler), View::String(_)) => {
					field_value = parent
				}
				View::Assignment {
					value: Some(value), ..
				} if value == current && value == field_value => {}
				View::Object { items } if items.len() == 1 && items.first() == Some(current) => {}
				_ => return false,
			}
			current = parent;
		}
	}

	pub(super) fn compact_string_payload(&self, nominal: NominalId, index: u16) -> Option<TypeId> {
		if !self.arena_member(nominal) {
			return None;
		}
		let data = self.member_data(nominal, index)?;
		(self.object_fields(data).as_slice() == [Repr::Str]).then_some(data)
	}

	pub(super) fn compact_string_box(&self, id: u64) -> Option<TypeId> {
		let member = self
			.union_ids
			.borrow()
			.iter()
			.find_map(|(&member, &candidate)| (candidate == id).then_some(member))?;
		self.compact_string_payload(member.0, member.1)
	}

	pub(super) fn compact_string_data(&self, data: TypeId) -> Vec<u64> {
		let mut members = Vec::new();
		for nominal in self.tree_arenas.unions.iter().copied() {
			let count = self
				.types()
				.sym_list(self.types().nominal(nominal).members)
				.len();
			for index in 0..count as u16 {
				if self.compact_string_payload(nominal, index) == Some(data) {
					members.push(self.member_id(nominal, index));
				}
			}
		}
		members.sort_unstable();
		members
	}

	pub(super) fn string_payload_value(
		&self,
		b: &Builder<'ctx>,
		pointer: PointerValue<'ctx>,
		data: TypeId,
	) -> Result<BasicValueEnum<'ctx>, Error> {
		let i64 = self.context.i64_type();
		let packed = self.packed_headers();
		let flags = if packed {
			let raw = b
				.build_load(
					self.context.i16_type(),
					self.payload_offset(b, pointer, 2)?,
					"",
				)?
				.into_int_value();
			b.build_int_z_extend(raw, i64, "")?
		} else {
			self.load_payload_word(b, pointer, 1)?
		};
		let external = b.build_and(
			flags,
			i64.const_int(if packed { 0x8000 } else { EXTERNAL }, false),
			"",
		)?;
		let external = b.build_int_compare(IntPredicate::NE, external, i64.const_zero(), "")?;
		let function = b
			.get_insert_block()
			.expect("a block")
			.get_parent()
			.expect("a function");
		let outside = self.context.append_basic_block(function, "external_text");
		let inside = b.get_insert_block().expect("a block");
		let end = self.context.append_basic_block(function, "text_view");
		let bytes = self.payload_offset(b, pointer, if packed { 4 } else { 16 })?;
		let inline_length = b.build_and(
			flags,
			i64.const_int(if packed { 0x1fff } else { SURROGATES - 1 }, false),
			"",
		)?;
		b.build_conditional_branch(external, outside, end)?;
		b.position_at_end(outside);
		// An external string is longer than one arena chunk, hence never inline.
		let external_header = self.payload_offset(b, pointer, if packed { 8 } else { 16 })?;
		let external_bytes = b
			.build_load(self.pointer_type(), external_header, "")?
			.into_pointer_value();
		let external_length = if packed {
			let field = b.build_struct_gep(self.string_type, external_header, 1, "")?;
			let value = b.build_load(self.usize, field, "")?.into_int_value();
			b.build_int_cast(value, i64, "")?
		} else {
			inline_length
		};
		b.build_unconditional_branch(end)?;
		b.position_at_end(end);
		let bytes_view = b.build_phi(self.pointer_type(), "text_bytes")?;
		bytes_view.add_incoming(&[(&bytes, inside), (&external_bytes, outside)]);
		let length = b.build_phi(i64, "text_length")?;
		length.add_incoming(&[(&inline_length, inside), (&external_length, outside)]);
		let length = b.build_int_cast(length.as_basic_value().into_int_value(), self.usize, "")?;
		let simple = b.build_and(
			flags,
			i64.const_int(if packed { 0x4000 } else { SIMPLE }, false),
			"",
		)?;
		let simple = b.build_int_compare(IntPredicate::NE, simple, i64.const_zero(), "")?;
		let simple = b.build_select(simple, self.simple_flag(), self.usize.const_zero(), "")?;
		// These bytes belong to a root, including an external string retained
		// by its cleanup. A stored copy must materialize before that root dies.
		// Kind zero denotes an immortal literal and would leak this borrow.
		let surrogates = b.build_and(
			flags,
			i64.const_int(if packed { 0x2000 } else { SURROGATES }, false),
			"",
		)?;
		let surrogates = b.build_int_compare(IntPredicate::NE, surrogates, i64.const_zero(), "")?;
		let surrogates = b
			.build_select(
				surrogates,
				self.string_surrogates_flag(),
				self.usize.const_zero(),
				"",
			)?
			.into_int_value();
		let encoding = b.build_or(simple.into_int_value(), surrogates, "")?;
		let capacity = b.build_or(encoding, self.string_kind_mask(), "")?;
		let value = b.build_insert_value(
			self.string_type.get_undef(),
			bytes_view.as_basic_value(),
			0,
			"",
		)?;
		let value = b.build_insert_value(value, length, 1, "")?;
		let value = b.build_insert_value(value, capacity, 2, "")?;
		Ok(b.build_insert_value(
			self.object_type(data).get_undef(),
			value.into_struct_value(),
			0,
			"",
		)?
		.into_struct_value()
		.into())
	}

	fn register_external_text(
		&self,
		b: &Builder<'ctx>,
		node: PointerValue<'ctx>,
	) -> Result<(), Error> {
		let name = if self.packed_headers() {
			"fa.arena.external_text.compact"
		} else {
			"fa.arena.external_text"
		};
		let drop = match self.module.get_function(name) {
			Some(drop) => drop,
			None => {
				let signature = self
					.context
					.void_type()
					.fn_type(&[self.pointer_type().into()], false);
				let (drop, cleanup) = self.helper_function(name, signature);
				let node = drop.get_nth_param(0).expect("a node").into_pointer_value();
				let source = self.payload_offset(
					&cleanup,
					node,
					if self.packed_headers() { 8 } else { 16 },
				)?;
				let value = cleanup.build_load(self.string_type, source, "")?;
				self.call_drop(&cleanup, Repr::Str, value)?;
				cleanup.build_return(None)?;
				drop
			}
		};
		b.build_call(
			self.runtime.function("fa_arena_cleanup")?,
			&[
				node.into(),
				drop.as_global_value().as_pointer_value().into(),
			],
			"",
		)?;
		Ok(())
	}

	pub(super) fn freeze_string_payload(
		&self,
		b: &Builder<'ctx>,
		id: u64,
		value: BasicValueEnum<'ctx>,
		copy: bool,
	) -> Result<PointerValue<'ctx>, Error> {
		let packed = self.packed_headers();
		let prefix_bytes = if packed { 4 } else { 16 };
		let string = Val::new(Repr::Str, value);
		let (bytes, length) = self.string_parts_with(b, string)?;
		let pair = value.into_struct_value();
		let capacity = b.build_extract_value(pair, 2, "")?.into_int_value();
		let simple = b.build_and(capacity, self.simple_flag(), "")?;
		let simple = b.build_int_compare(IntPredicate::NE, simple, self.usize.const_zero(), "")?;
		let i64 = self.context.i64_type();
		let simple = b
			.build_select(
				simple,
				i64.const_int(if packed { 0x4000 } else { SIMPLE }, false),
				i64.const_zero(),
				"",
			)?
			.into_int_value();
		let surrogates = b.build_and(capacity, self.string_surrogates_flag(), "")?;
		let surrogates =
			b.build_int_compare(IntPredicate::NE, surrogates, self.usize.const_zero(), "")?;
		let surrogates = b
			.build_select(
				surrogates,
				i64.const_int(if packed { 0x2000 } else { SURROGATES }, false),
				i64.const_zero(),
				"",
			)?
			.into_int_value();
		let encoding = b.build_or(simple, surrogates, "")?;
		let long_length = b.build_int_cast(length, i64, "")?;
		let flags = b.build_or(long_length, encoding, "")?;
		let header = (5 * u64::from(self.usize.get_bit_width() / 8)).next_multiple_of(16);
		let maximum = super::trees::ARENA_CHUNK_BYTES - header - prefix_bytes;
		let maximum = if packed { maximum.min(0x1fff) } else { maximum };
		let fits = b.build_int_compare(
			IntPredicate::ULE,
			length,
			self.usize.const_int(maximum, false),
			"",
		)?;
		let function = b
			.get_insert_block()
			.expect("a block")
			.get_parent()
			.expect("a function");
		let inline = self.context.append_basic_block(function, "freeze_text");
		let external = self.context.append_basic_block(function, "keep_text");
		let end = self.context.append_basic_block(function, "frozen_text");
		b.build_conditional_branch(fits, inline, external)?;
		b.position_at_end(inline);
		let size = b.build_int_add(length, self.usize.const_int(prefix_bytes, false), "")?;
		let node = b
			.build_call(
				self.runtime.function("fa_arena_alloc")?,
				&[size.into(), self.usize.const_int(8, false).into()],
				"",
			)?
			.try_as_basic_value()
			.basic()
			.expect("a node")
			.into_pointer_value();
		if packed {
			b.build_store(node, i64.const_zero())?;
			self.store_compact_id(b, node, id)?;
			let flags = b.build_int_truncate(flags, self.context.i16_type(), "")?;
			b.build_store(self.payload_offset(b, node, 2)?, flags)?;
		} else {
			b.build_store(node, i64.const_int(id, false))?;
			b.build_store(self.payload_word(b, node, 1)?, flags)?;
		}
		b.build_memcpy(
			self.payload_offset(b, node, prefix_bytes)?,
			1,
			bytes,
			1,
			length,
		)?;
		if !copy {
			self.call_drop(b, Repr::Str, value)?;
		}
		let inline_end = b.get_insert_block().expect("a block");
		b.build_unconditional_branch(end)?;
		b.position_at_end(external);
		let prefix = self.context.struct_type(
			&[
				i64.array_type(if packed { 1 } else { 2 }).into(),
				self.string_type.into(),
			],
			false,
		);
		let outside = self.allocate_tree_type(b, prefix.into())?;
		if packed {
			b.build_store(outside, i64.const_zero())?;
			self.store_compact_id(b, outside, id)?;
			let flags = b.build_or(encoding, i64.const_int(0x8000, false), "")?;
			let flags = b.build_int_truncate(flags, self.context.i16_type(), "")?;
			b.build_store(self.payload_offset(b, outside, 2)?, flags)?;
		} else {
			b.build_store(outside, i64.const_int(id, false))?;
			let flags = b.build_or(flags, i64.const_int(EXTERNAL, false), "")?;
			b.build_store(self.payload_word(b, outside, 1)?, flags)?;
		}
		let owned = if copy {
			self.call_copy(b, Repr::Str, value)?
		} else {
			value
		};
		b.build_store(
			self.payload_offset(b, outside, if packed { 8 } else { 16 })?,
			owned,
		)?;
		self.register_external_text(b, outside)?;
		let external_end = b.get_insert_block().expect("a block");
		b.build_unconditional_branch(end)?;
		b.position_at_end(end);
		let result = b.build_phi(self.pointer_type(), "frozen_text")?;
		result.add_incoming(&[(&node, inline_end), (&outside, external_end)]);
		Ok(result.as_basic_value().into_pointer_value())
	}
}
