//! Maps and sets: a pointer to a table of the runtime (`fa_runtime.c`,
//! "maps and sets"), its entries `{ i64 hash, i64 live, key, value }` in
//! slot order (a set's entries have no value). Maps and growing tables
//! keep their entries in stable segments. Keys are hashed and compared by functions
//! generated per shape: structural equality, floats with SameValueZero
//! (`NaN` is one key, `-0` and `0` are one key), like `FaHashMap`.
//!
//! The table owns its keys and values: inserting takes them, deleting drops
//! them, reading one gives a view of it.

use fa_analyzer::analysis::{Argument, CallInfo, NodeFlags};
use fa_analyzer::ids::NodeId;
use fa_analyzer::syntax::{NodeList, View};
use fa_analyzer::types::{CollKind, TypeKind};
use inkwell::IntPredicate;
use inkwell::builder::Builder;
use inkwell::module::Linkage;
use inkwell::types::{BasicTypeEnum, StructType};
use inkwell::values::{BasicValueEnum, FunctionValue, IntValue, PointerValue};

use crate::error::Error;
use crate::lower::values::{IntRepr, Repr, Scalar, Val};
use crate::lower::{LoopFrame, Lowering, Slot};

/// The layout of the entries of a table.
#[derive(Copy, Clone)]
pub(crate) struct Table<'ctx> {
	pub key: Repr,
	/// `None` for a set.
	pub value: Option<Repr>,
	pub entry: StructType<'ctx>,
	/// A `GrowingMap` or a `GrowingSet`: the entries live in segments that
	/// never move (`fa_table_entry`), so references into them stay valid.
	pub growing: bool,
}

impl Table<'_> {
	pub(crate) fn stable(self) -> bool {
		self.growing || self.value.is_some()
	}
}

impl<'ctx> Lowering<'_, 'ctx> {
	// ── layouts ─────────────────────────────────────────────────────────

	/// `{ ptr entries, i64 length, i64 used, i64 capacity, ptr index, i64
	/// mask, i64 storage, i64 free, i64 occupied }`.
	pub(crate) fn table_header(&self) -> StructType<'ctx> {
		let i64 = self.context.i64_type();
		let pointer = self.pointer_type();
		self.context.struct_type(
			&[
				pointer.into(),
				i64.into(),
				i64.into(),
				i64.into(),
				pointer.into(),
				i64.into(),
				i64.into(),
				i64.into(),
				i64.into(),
				i64.into(),
			],
			false,
		)
	}

	/// The key and value of a map or set type.
	pub(crate) fn table_of(&self, repr: Repr) -> Result<Table<'ctx>, Error> {
		let (Repr::Map(ty) | Repr::Set(ty)) = repr else {
			return Err(Error::Llvm(format!("{repr:?} is not a table")));
		};
		let TypeKind::Collection(kind, item, key) = self.types().kind(ty) else {
			return Err(Error::Llvm("a table without a collection type".into()));
		};
		let (key, value) = match kind {
			CollKind::Map | CollKind::GrowingMap => (
				self.item_repr(key, NodeId::new(0))?,
				Some(self.item_repr(item, NodeId::new(0))?),
			),
			_ => (self.item_repr(item, NodeId::new(0))?, None),
		};
		let i64 = self.context.i64_type();
		let mut fields: Vec<BasicTypeEnum<'ctx>> =
			vec![i64.into(), i64.into(), self.llvm_type(key)];
		if let Some(value) = value {
			fields.push(self.llvm_type(value));
		}
		Ok(Table {
			key,
			value,
			entry: self.context.struct_type(&fields, false),
			growing: matches!(kind, CollKind::GrowingMap | CollKind::GrowingSet),
		})
	}

	/// The size of an entry, an `i64` constant.
	fn entry_size(&self, table: Table<'ctx>) -> IntValue<'ctx> {
		table.entry.size_of().expect("a sized entry")
	}

	// ── hashing and equality ────────────────────────────────────────────

	/// `fa.hash.<shape>(value) -> i64`: equal keys have equal hashes.
	pub(crate) fn hash_function(&self, repr: Repr) -> Result<FunctionValue<'ctx>, Error> {
		let key = format!("hash {repr:?}");
		if let Some(function) = self.drop_functions.borrow().get(&key) {
			return Ok(*function);
		}
		let i64 = self.context.i64_type();
		let function = self.module.add_function(
			"fa.hash",
			i64.fn_type(&[self.llvm_type(repr).into()], false),
			Some(Linkage::Internal),
		);
		self.runtime.add_attributes(function, &["nounwind"]);
		self.drop_functions.borrow_mut().insert(key, function);
		let builder = self.context.create_builder();
		builder.position_at_end(self.context.append_basic_block(function, "entry"));
		let value = function.get_nth_param(0).expect("a value");
		let hash = self.build_hash(&builder, function, repr, value)?;
		builder.build_return(Some(&hash))?;
		Ok(function)
	}

	/// The murmur finalizer.
	fn mix(&self, builder: &Builder<'ctx>, value: IntValue<'ctx>) -> Result<IntValue<'ctx>, Error> {
		let i64 = self.context.i64_type();
		let shift = i64.const_int(33, false);
		let mut hash = value;
		for multiplier in [0xff51afd7ed558ccd_u64, 0xc4ceb9fe1a85ec53] {
			let shifted = builder.build_right_shift(hash, shift, false, "")?;
			hash = builder.build_xor(hash, shifted, "")?;
			hash = builder.build_int_mul(hash, i64.const_int(multiplier, false), "")?;
		}
		let shifted = builder.build_right_shift(hash, shift, false, "")?;
		Ok(builder.build_xor(hash, shifted, "")?)
	}

	/// Combines a hash with the next one.
	fn combine(
		&self,
		builder: &Builder<'ctx>,
		hash: IntValue<'ctx>,
		next: IntValue<'ctx>,
	) -> Result<IntValue<'ctx>, Error> {
		let i64 = self.context.i64_type();
		let scaled = builder.build_int_mul(hash, i64.const_int(0x9E3779B97F4A7C15, false), "")?;
		let sum = builder.build_int_add(scaled, next, "")?;
		self.mix(builder, sum)
	}

	fn build_hash(
		&self,
		builder: &Builder<'ctx>,
		function: FunctionValue<'ctx>,
		repr: Repr,
		value: BasicValueEnum<'ctx>,
	) -> Result<IntValue<'ctx>, Error> {
		let i64 = self.context.i64_type();
		let call_hash =
			|repr: Repr, value: BasicValueEnum<'ctx>| -> Result<IntValue<'ctx>, Error> {
				let function = self.hash_function(repr)?;
				Ok(builder
					.build_call(function, &[value.into()], "")?
					.try_as_basic_value()
					.basic()
					.expect("a hash")
					.into_int_value())
			};
		Ok(match repr {
			Repr::Int(int) => {
				let raw = value.into_int_value();
				let word = if int.bits > 64 {
					let low = builder.build_int_truncate(raw, i64, "")?;
					let high = builder.build_right_shift(
						raw,
						raw.get_type().const_int(64, false),
						false,
						"",
					)?;
					let high = builder.build_int_truncate(high, i64, "")?;
					builder.build_xor(low, high, "")?
				} else if int.signed {
					builder.build_int_s_extend_or_bit_cast(raw, i64, "")?
				} else {
					builder.build_int_z_extend_or_bit_cast(raw, i64, "")?
				};
				self.mix(builder, word)?
			}
			Repr::Bool | Repr::Enum(_) => {
				let word = builder.build_int_z_extend(value.into_int_value(), i64, "")?;
				self.mix(builder, word)?
			}
			Repr::F64 | Repr::F32 | Repr::Percentage => {
				// SameValueZero: `-0` is `0`, every `NaN` is one
				let float = value.into_float_value();
				let float = if repr == Repr::F32 {
					builder.build_float_ext(float, self.context.f64_type(), "")?
				} else {
					float
				};
				let zero = self.context.f64_type().const_zero();
				let is_zero =
					builder.build_float_compare(inkwell::FloatPredicate::OEQ, float, zero, "")?;
				let is_nan =
					builder.build_float_compare(inkwell::FloatPredicate::UNO, float, float, "")?;
				let bits = builder.build_bit_cast(float, i64, "")?.into_int_value();
				let bits = builder
					.build_select(is_zero, i64.const_zero(), bits, "")?
					.into_int_value();
				let bits = builder
					.build_select(is_nan, i64.const_int(0x7ff8000000000000, false), bits, "")?
					.into_int_value();
				self.mix(builder, bits)?
			}
			Repr::Str => {
				let raw = value.into_struct_value();
				let first = builder
					.build_extract_value(raw, 0, "")?
					.into_pointer_value();
				let first = builder.build_ptr_to_int(first, self.usize, "")?;
				let second = builder.build_extract_value(raw, 1, "")?;
				let third = builder.build_extract_value(raw, 2, "")?;
				let hash = builder
					.build_call(
						self.runtime.function("fa_hash_string_bits")?,
						&[first.into(), second.into(), third.into()],
						"",
					)?
					.try_as_basic_value()
					.basic()
					.expect("a hash")
					.into_int_value();
				builder.build_int_z_extend_or_bit_cast(hash, i64, "")?
			}
			Repr::Object(_) | Repr::Tuple(_) => {
				let fields = match repr {
					Repr::Object(ty) => self.object_fields(ty),
					Repr::Tuple(ty) => self.tuple_reprs(ty),
					_ => unreachable!(),
				};
				let structure = value.into_struct_value();
				let mut hash = i64.const_int(fields.len() as u64, false);
				for (index, field) in fields.into_iter().enumerate() {
					// relation sides don't count (D21)
					if matches!(field, Repr::Link(_) | Repr::Links(_)) {
						continue;
					}
					let raw = builder.build_extract_value(structure, index as u32, "")?;
					let next = call_hash(field, raw)?;
					hash = self.combine(builder, hash, next)?;
				}
				hash
			}
			Repr::Optional(scalar) => {
				let (present, inner) = self.split_optional(builder, Val::new(repr, value))?;
				let inner = inner.value.expect("an optional value");
				let then = self.context.append_basic_block(function, "present");
				let join = self.context.append_basic_block(function, "hashed");
				let start = builder.get_insert_block().expect("a block");
				builder.build_conditional_branch(present, then, join)?;
				builder.position_at_end(then);
				let hash = call_hash(scalar.repr(), inner)?;
				let hash = builder.build_xor(hash, i64.const_int(1, false), "")?;
				let then_end = builder.get_insert_block().expect("a block");
				builder.build_unconditional_branch(join)?;
				builder.position_at_end(join);
				let phi = builder.build_phi(i64, "")?;
				phi.add_incoming(&[(&i64.const_zero(), start), (&hash, then_end)]);
				phi.as_basic_value().into_int_value()
			}
			Repr::Union => {
				// the member: values of different members are different
				let pointer = value.into_pointer_value();
				let id = builder.build_load(i64, pointer, "")?.into_int_value();
				self.mix(builder, id)?
			}
			Repr::InlineUnion(_) => {
				let id = builder
					.build_extract_value(value.into_struct_value(), 0, "")?
					.into_int_value();
				self.mix(builder, id)?
			}
			Repr::Rec(target) => {
				let target_repr = self.repr_of(target, 0)?;
				let loaded = builder.build_load(
					self.llvm_type(target_repr),
					value.into_pointer_value(),
					"",
				)?;
				call_hash(target_repr, loaded)?
			}
			Repr::Array(item_type) => {
				let item = self.item_repr(item_type, NodeId::new(0))?;
				let header = self.header_type();
				let array = value.into_pointer_value();
				if item == Repr::Str {
					builder.build_call(
						self.runtime.function("fa_array_materialize")?,
						&[array.into()],
						"",
					)?;
				}
				let items_pointer = builder.build_struct_gep(header, array, 0, "")?;
				let items = builder
					.build_load(self.pointer_type(), items_pointer, "")?
					.into_pointer_value();
				let length_pointer = builder.build_struct_gep(header, array, 1, "")?;
				let length = builder
					.build_load(i64, length_pointer, "")?
					.into_int_value();
				let accumulator = builder.build_alloca(i64, "hash")?;
				builder.build_store(accumulator, length)?;
				let item_type = self.llvm_type(item);
				let before = builder.get_insert_block().expect("a block");
				let test = self.context.append_basic_block(function, "each");
				let body = self.context.append_basic_block(function, "each_body");
				let end = self.context.append_basic_block(function, "each_end");
				builder.build_unconditional_branch(test)?;
				builder.position_at_end(test);
				let index = builder.build_phi(i64, "index")?;
				index.add_incoming(&[(&i64.const_zero(), before)]);
				let current = index.as_basic_value().into_int_value();
				let more = builder.build_int_compare(IntPredicate::SLT, current, length, "")?;
				builder.build_conditional_branch(more, body, end)?;
				builder.position_at_end(body);
				let pointer =
					unsafe { builder.build_in_bounds_gep(item_type, items, &[current], "")? };
				let loaded = builder.build_load(item_type, pointer, "")?;
				let next = call_hash(item, loaded)?;
				let hash = builder.build_load(i64, accumulator, "")?.into_int_value();
				let hash = self.combine(builder, hash, next)?;
				builder.build_store(accumulator, hash)?;
				let following = builder.build_int_add(current, i64.const_int(1, false), "")?;
				index.add_incoming(&[(&following, builder.get_insert_block().expect("a block"))]);
				builder.build_unconditional_branch(test)?;
				builder.position_at_end(end);
				builder.build_load(i64, accumulator, "")?.into_int_value()
			}
			Repr::Range => {
				let structure = value.into_struct_value();
				let mut hash = i64.const_int(3, false);
				for index in 0..3 {
					let raw = builder
						.build_extract_value(structure, index, "")?
						.into_int_value();
					let raw = builder.build_int_z_extend_or_bit_cast(raw, i64, "")?;
					hash = self.combine(builder, hash, raw)?;
				}
				hash
			}
			// the other keys only collide
			_ => i64.const_zero(),
		})
	}

	/// `fa.key_equals.<shape>(ptr a, ptr b) -> i32`, called by the runtime:
	/// Fa `==`, floats with SameValueZero.
	pub(crate) fn equals_function(&mut self, repr: Repr) -> Result<FunctionValue<'ctx>, Error> {
		self.comparison_function(repr, true)
	}

	/// Structural equality inherits the enclosing comparison's NaN rule.
	/// Caching the declaration before its body also permits recursive items.
	pub(super) fn nested_equals_function(
		&mut self,
		repr: Repr,
	) -> Result<FunctionValue<'ctx>, Error> {
		self.comparison_function(repr, self.frame().same_value_zero)
	}

	fn comparison_function(
		&mut self,
		repr: Repr,
		same_value_zero: bool,
	) -> Result<FunctionValue<'ctx>, Error> {
		let key = format!("equals {same_value_zero} {}", self.shape_key(repr));
		if let Some(function) = self.drop_functions.borrow().get(&key) {
			return Ok(*function);
		}
		let pointer = self.pointer_type();
		let i32 = self.context.i32_type();
		let function = self.module.add_function(
			"fa.key_equals",
			i32.fn_type(&[pointer.into(), pointer.into()], false),
			Some(Linkage::Internal),
		);
		self.runtime.add_attributes(function, &["nounwind"]);
		self.drop_functions.borrow_mut().insert(key, function);
		// lowered as a function of its own
		let saved_frame = self.frame.take();
		let saved_block = self.builder.get_insert_block();
		self.begin_function(function, Repr::Void, None);
		self.frame_mut().same_value_zero = same_value_zero;
		let a = function.get_nth_param(0).expect("a").into_pointer_value();
		let b = function.get_nth_param(1).expect("b").into_pointer_value();
		let a = self.load(Slot { pointer: a, repr })?;
		let b = self.load(Slot { pointer: b, repr })?;
		let node = NodeId::new(0);
		let equal = self.values_equal_any(a, b, node)?;
		let result = self.builder.build_int_z_extend(equal, i32, "")?;
		self.end_scope(0, None)?;
		self.builder.build_return(Some(&result))?;
		self.end_function()?;
		self.frame = saved_frame;
		if let Some(block) = saved_block {
			self.builder.position_at_end(block);
		}
		Ok(function)
	}

	// ── operations ──────────────────────────────────────────────────────

	/// A new empty table.
	pub(crate) fn table_new(
		&mut self,
		table: Table<'ctx>,
		capacity: IntValue<'ctx>,
	) -> Result<PointerValue<'ctx>, Error> {
		let new = self.runtime.function(if table.growing {
			"fa_table_new_growing"
		} else if table.value.is_some() {
			"fa_table_new_map"
		} else {
			"fa_table_new"
		})?;
		Ok(self
			.call_value(new, &[capacity.into(), self.entry_size(table).into()])?
			.into_pointer_value())
	}

	/// A field of the header of a table (1: length, 2: used).
	pub(crate) fn table_field(
		&self,
		pointer: PointerValue<'ctx>,
		index: u32,
	) -> Result<IntValue<'ctx>, Error> {
		let field = self
			.builder
			.build_struct_gep(self.table_header(), pointer, index, "")?;
		Ok(self
			.builder
			.build_load(self.context.i64_type(), field, "")?
			.into_int_value())
	}

	/// `set.size`, `map.size`.
	pub(crate) fn table_size(&self, value: Val<'ctx>) -> Result<Val<'ctx>, Error> {
		let pointer = value.value.expect("a table").into_pointer_value();
		Ok(Val::new(
			Repr::Int(IntRepr::I64),
			self.table_field(pointer, 1)?,
		))
	}

	/// The entry at a position.
	pub(crate) fn entry_at(
		&self,
		pointer: PointerValue<'ctx>,
		table: Table<'ctx>,
		position: IntValue<'ctx>,
	) -> Result<PointerValue<'ctx>, Error> {
		if table.stable() {
			let entry = self.runtime.function("fa_table_entry")?;
			return Ok(self
				.call_value(
					entry,
					&[
						pointer.into(),
						position.into(),
						self.entry_size(table).into(),
					],
				)?
				.into_pointer_value());
		}
		let entries_field = self
			.builder
			.build_struct_gep(self.table_header(), pointer, 0, "")?;
		let entries = self
			.builder
			.build_load(self.pointer_type(), entries_field, "")?
			.into_pointer_value();
		Ok(unsafe {
			self.builder
				.build_in_bounds_gep(table.entry, entries, &[position], "")?
		})
	}

	/// The slot of the key (2) or the value (3) of an entry.
	pub(crate) fn entry_slot(
		&self,
		entry: PointerValue<'ctx>,
		table: Table<'ctx>,
		value: bool,
	) -> Result<Slot<'ctx>, Error> {
		let (index, repr) = match (value, table.value) {
			(true, Some(repr)) => (3, repr),
			_ => (2, table.key),
		};
		Ok(Slot {
			pointer: self
				.builder
				.build_struct_gep(table.entry, entry, index, "")?,
			repr,
		})
	}

	/// The hash of a key.
	fn hash_of(&mut self, key: Val<'ctx>) -> Result<IntValue<'ctx>, Error> {
		let function = self.hash_function(key.repr)?;
		Ok(self
			.call_value(function, &[key.value.expect("a key").into()])?
			.into_int_value())
	}

	/// The position of the entry of a key (converted to the table's key), or
	/// -1 on a miss; and its hash.
	fn table_find(
		&mut self,
		pointer: PointerValue<'ctx>,
		table: Table<'ctx>,
		key: Val<'ctx>,
		node: NodeId,
	) -> Result<(IntValue<'ctx>, IntValue<'ctx>), Error> {
		self.table_search(pointer, table, key, node, false)
	}

	/// Insertion searches additionally retain a negative insertion probe.
	/// No table mutation may occur before reusing it; ownership copies do
	/// not call user code. Read-only searches need no tombstone bookkeeping.
	fn table_search(
		&mut self,
		pointer: PointerValue<'ctx>,
		table: Table<'ctx>,
		key: Val<'ctx>,
		node: NodeId,
		insertion: bool,
	) -> Result<(IntValue<'ctx>, IntValue<'ctx>), Error> {
		let key = self.convert(key, table.key, node.index())?;
		let hash = self.hash_of(key)?;
		let slot = self.alloca(table.key, "key")?;
		self.builder
			.build_store(slot.pointer, key.value.expect("a key"))?;
		// `build_hash` sign/zero-extends these integers, then applies `mix`:
		// xor-shifts and multiplication by odd words are bijections modulo
		// 2^64. Full hash equality therefore proves key equality. Wider
		// integers, truncated hashes and composite keys need a comparator.
		let equals = if matches!(table.key, Repr::Int(int) if int.bits <= 64) {
			self.pointer_type().const_null()
		} else {
			self.equals_function(table.key)?
				.as_global_value()
				.as_pointer_value()
		};
		let find = self.runtime.function(match (table.stable(), insertion) {
			(true, true) => "fa_table_probe_growing",
			(false, true) => "fa_table_probe",
			(true, false) => "fa_table_find_growing",
			(false, false) => "fa_table_find",
		})?;
		let position = self
			.call_value(
				find,
				&[
					pointer.into(),
					hash.into(),
					slot.pointer.into(),
					equals.into(),
					self.entry_size(table).into(),
				],
			)?
			.into_int_value();
		Ok((position, hash))
	}

	/// A new entry at the end of the table, its key and value written.
	fn table_append(
		&mut self,
		pointer: PointerValue<'ctx>,
		table: Table<'ctx>,
		hash: IntValue<'ctx>,
		key: Val<'ctx>,
		value: Option<Val<'ctx>>,
		probe: Option<IntValue<'ctx>>,
	) -> Result<PointerValue<'ctx>, Error> {
		let append = self.runtime.function("fa_table_append")?;
		let entry = self
			.call_value(
				append,
				&[
					pointer.into(),
					hash.into(),
					self.entry_size(table).into(),
					probe
						.unwrap_or_else(|| self.context.i64_type().const_zero())
						.into(),
				],
			)?
			.into_pointer_value();
		let key_slot = self.entry_slot(entry, table, false)?;
		self.builder
			.build_store(key_slot.pointer, key.value.expect("a key"))?;
		if table.value.is_some() {
			let value_slot = self.entry_slot(entry, table, true)?;
			let raw = match value {
				Some(value) => value.value.expect("a value"),
				None => self.llvm_type(value_slot.repr).const_zero(),
			};
			self.builder.build_store(value_slot.pointer, raw)?;
		}
		Ok(entry)
	}

	/// Inserts an owned key (and value): a map replaces the value of an equal
	/// key (dropping the old value and the new key), a set keeps its element
	/// (dropping the new one). Whether it was added.
	pub(crate) fn table_insert(
		&mut self,
		pointer: PointerValue<'ctx>,
		table: Table<'ctx>,
		key: Val<'ctx>,
		value: Option<Val<'ctx>>,
		node: NodeId,
	) -> Result<IntValue<'ctx>, Error> {
		let (position, hash) = self.table_search(pointer, table, key, node, true)?;
		let found = self.builder.build_int_compare(
			IntPredicate::SGE,
			position,
			position.get_type().const_zero(),
			"",
		)?;
		let present = self.append_block("insert_present");
		let absent = self.append_block("insert_absent");
		let end = self.append_block("inserted");
		self.builder
			.build_conditional_branch(found, present, absent)?;
		self.builder.position_at_end(present);
		// a map replaces the value; the new key (a set's new element) isn't kept
		if let Some(value) = value {
			let entry = self.entry_at(pointer, table, position)?;
			let slot = self.entry_slot(entry, table, true)?;
			let old = self.load(slot)?;
			self.emit_drop(old)?;
			self.store(slot, value, node)?;
		}
		self.emit_drop(key)?;
		self.builder.build_unconditional_branch(end)?;
		self.builder.position_at_end(absent);
		self.table_append(pointer, table, hash, key, value, Some(position))?;
		self.builder.build_unconditional_branch(end)?;
		self.builder.position_at_end(end);
		Ok(self.builder.build_not(found, "")?)
	}

	/// The entry of a key, appended with a zero value when absent (an
	/// assignment `map[key] = value`, `getOrInsert`): the key is copied only
	/// when it is added.
	fn table_entry_for(
		&mut self,
		pointer: PointerValue<'ctx>,
		table: Table<'ctx>,
		key: Val<'ctx>,
		initial: Option<(Val<'ctx>, NodeId)>,
		node: NodeId,
	) -> Result<PointerValue<'ctx>, Error> {
		let (position, hash) = self.table_search(pointer, table, key, node, true)?;
		let found = self.builder.build_int_compare(
			IntPredicate::SGE,
			position,
			position.get_type().const_zero(),
			"",
		)?;
		let result = self.alloca_type(self.pointer_type().into(), "entry")?;
		let present = self.append_block("entry_present");
		let absent = self.append_block("entry_absent");
		let end = self.append_block("entry_found");
		self.builder
			.build_conditional_branch(found, present, absent)?;
		self.builder.position_at_end(present);
		let entry = self.entry_at(pointer, table, position)?;
		self.builder.build_store(result, entry)?;
		self.builder.build_unconditional_branch(end)?;
		self.builder.position_at_end(absent);
		let mark = self.mark();
		let key = self.convert(key, table.key, node.index())?;
		let key = self.emit_copy(key)?;
		let value = match (initial, table.value) {
			(Some((value, value_node)), Some(repr)) => {
				Some(self.owned_as(value, repr, Some(value_node))?)
			}
			_ => None,
		};
		let entry = self.table_append(pointer, table, hash, key, value, Some(position))?;
		self.end_statement(mark, None)?;
		self.builder.build_store(result, entry)?;
		self.builder.build_unconditional_branch(end)?;
		self.builder.position_at_end(end);
		Ok(self
			.builder
			.build_load(self.pointer_type(), result, "")?
			.into_pointer_value())
	}

	/// Drops the key and value of a live entry, and kills it.
	fn table_remove(
		&mut self,
		pointer: PointerValue<'ctx>,
		table: Table<'ctx>,
		position: IntValue<'ctx>,
		keep_value: bool,
	) -> Result<Option<Val<'ctx>>, Error> {
		let entry = self.entry_at(pointer, table, position)?;
		let key_slot = self.entry_slot(entry, table, false)?;
		let key = self.load(key_slot)?;
		let mut kept = None;
		if let Some(value_repr) = table.value {
			let value_slot = self.entry_slot(entry, table, true)?;
			let value = self.load(value_slot)?;
			if keep_value {
				kept = Some(value);
			} else {
				self.emit_drop(value)?;
			}
			self.emit_drop(key)?;
			let _ = value_repr;
		} else if keep_value {
			kept = Some(key);
		} else {
			self.emit_drop(key)?;
		}
		let kill = self.runtime.function("fa_table_kill")?;
		self.builder.build_call(
			kill,
			&[
				pointer.into(),
				position.into(),
				self.entry_size(table).into(),
			],
			"",
		)?;
		Ok(kept)
	}

	/// Runs `body` for every live entry, in order (a region per entry).
	pub(crate) fn for_each_entry(
		&mut self,
		pointer: PointerValue<'ctx>,
		table: Table<'ctx>,
		mut body: impl FnMut(&mut Self, PointerValue<'ctx>, IntValue<'ctx>) -> Result<(), Error>,
	) -> Result<(), Error> {
		let i64 = self.context.i64_type();
		let counter = self.alloca(Repr::Int(IntRepr::I64), "entry_position")?;
		self.builder
			.build_store(counter.pointer, i64.const_zero())?;
		let test = self.append_block("entries");
		let check = self.append_block("entry_live");
		let block = self.append_block("entry_body");
		let step = self.append_block("entry_step");
		let end = self.append_block("entries_end");
		self.builder.build_unconditional_branch(test)?;
		self.builder.position_at_end(test);
		let position = self.load(counter)?.int();
		let used = self.table_field(pointer, 2)?;
		let more = self
			.builder
			.build_int_compare(IntPredicate::SLT, position, used, "")?;
		self.builder.build_conditional_branch(more, check, end)?;
		self.builder.position_at_end(check);
		let entry = self.entry_at(pointer, table, position)?;
		let live_field = self.builder.build_struct_gep(table.entry, entry, 1, "")?;
		let live = self
			.builder
			.build_load(i64, live_field, "")?
			.into_int_value();
		let live = self
			.builder
			.build_int_compare(IntPredicate::NE, live, i64.const_zero(), "")?;
		self.builder.build_conditional_branch(live, block, step)?;
		self.builder.position_at_end(block);
		let mark = self.mark();
		body(self, entry, position)?;
		self.end_statement(mark, None)?;
		self.branch_to(step)?;
		self.builder.position_at_end(step);
		let position = self.load(counter)?.int();
		let next = self
			.builder
			.build_int_nsw_add(position, i64.const_int(1, false), "")?;
		self.builder.build_store(counter.pointer, next)?;
		self.builder.build_unconditional_branch(test)?;
		self.builder.position_at_end(end);
		Ok(())
	}

	// ── values ──────────────────────────────────────────────────────────

	/// A list literal of a set (`[1, 2]`) or a map (`[("a", 1)]`).
	pub(crate) fn table_literal(
		&mut self,
		node: NodeId,
		repr: Repr,
		items: NodeList<'_>,
	) -> Result<Val<'ctx>, Error> {
		let table = self.table_of(repr)?;
		let capacity = self.context.i64_type().const_int(items.len() as u64, false);
		let count = items.len().max(4) as u64;
		let entry_size = self.collection_data_size(table.entry.into());
		let pointer_size = u64::from(self.usize.get_bit_width() / 8);
		let mut bytes = self.collection_data_size(self.table_header().into());
		if table.stable() {
			let chunks = count.div_ceil(32);
			bytes += self.collection_data_size(
				self.context
					.struct_type(
						&[
							self.pointer_type().into(),
							self.context.i64_type().into(),
							self.context.i64_type().into(),
						],
						false,
					)
					.into(),
			) + chunks * pointer_size;
			bytes = (bytes + 15) & !15;
			bytes += chunks * 32 * entry_size;
		} else {
			bytes = (bytes + 15) & !15;
			bytes += count * entry_size;
		}
		bytes = ((bytes + 7) & !7) + (count * 2).next_power_of_two().max(8) * 4;
		let pointer = if let Some(buffer) = self.collection_stack(node, bytes)? {
			let kind = if table.value.is_some() && !table.growing {
				2
			} else {
				u64::from(table.growing)
			};
			self.call_value(
				self.runtime.function("fa_table_local")?,
				&[
					buffer.into(),
					capacity.into(),
					self.entry_size(table).into(),
					self.context.i32_type().const_int(kind, false).into(),
				],
			)?
			.into_pointer_value()
		} else {
			self.table_new(table, capacity)?
		};
		let result = self.fresh(Val::new(repr, pointer));
		for item in items.iter() {
			let mark = self.mark();
			match table.value {
				None => {
					let value = self.expr(item)?;
					if value.repr == Repr::Never {
						return Ok(value);
					}
					let value = self.owned_as(value, table.key, Some(item))?;
					self.table_insert(pointer, table, value, None, item)?;
				}
				Some(value_repr) => {
					let (key, value) = match self.view(item) {
						View::Tuple { items: pair } if pair.len() == 2 => {
							let key_node = pair.get(0).expect("a key");
							let value_node = pair.get(1).expect("a value");
							let key = self.expr(key_node)?;
							let value = self.expr(value_node)?;
							if key.repr == Repr::Never || value.repr == Repr::Never {
								return Ok(Val::never());
							}
							(
								self.owned_as(key, table.key, Some(key_node))?,
								self.owned_as(value, value_repr, Some(value_node))?,
							)
						}
						_ => {
							let pair = self.expr(item)?;
							self.pair_parts(pair, table, item)?
						}
					};
					self.table_insert(pointer, table, key, Some(value), item)?;
				}
			}
			self.end_statement(mark, None)?;
		}
		let _ = node;
		Ok(result)
	}

	/// Owned copies of the key and value of a tuple `(key, value)`.
	fn pair_parts(
		&mut self,
		pair: Val<'ctx>,
		table: Table<'ctx>,
		node: NodeId,
	) -> Result<(Val<'ctx>, Val<'ctx>), Error> {
		let Repr::Tuple(ty) = pair.repr else {
			return Err(self.unsupported(node.index(), "this map entry"));
		};
		let reprs = self.tuple_reprs(ty);
		let structure = pair.value.expect("a tuple").into_struct_value();
		let key = self.builder.build_extract_value(structure, 0, "")?;
		let value = self.builder.build_extract_value(structure, 1, "")?;
		let key = self.convert(Val::new(reprs[0], key), table.key, node.index())?;
		let value = self.convert(
			Val::new(reprs[1], value),
			table.value.expect("a map"),
			node.index(),
		)?;
		Ok((self.emit_copy(key)?, self.emit_copy(value)?))
	}

	/// `map[key]`: the value (a view), `KeyNotFound` when absent and not proven.
	pub(crate) fn table_index(
		&mut self,
		node: NodeId,
		map: Val<'ctx>,
		key: Val<'ctx>,
	) -> Result<Val<'ctx>, Error> {
		let table = self.table_of(map.repr)?;
		let pointer = map.value.expect("a map").into_pointer_value();
		let (position, _) = self.table_find(pointer, table, key, node)?;
		if self.flags(node).contains(NodeFlags::INDEX_CHECK) {
			let absent = self.builder.build_int_compare(
				IntPredicate::SLT,
				position,
				position.get_type().const_zero(),
				"",
			)?;
			self.guard(absent, |this| {
				this.call_runtime("fa_b_reset", &[])?;
				this.append_value(key, node)?;
				this.raise_runtime("fa_error_key_not_found", &[])
			})?;
		}
		let entry = self.entry_at(pointer, table, position)?;
		let slot = self.entry_slot(entry, table, true)?;
		self.load(slot)
	}

	/// The place `map[key]`: the value of its entry, added with a zero value
	/// (dropped as nothing) when absent.
	pub(crate) fn table_place(
		&mut self,
		node: NodeId,
		map: Val<'ctx>,
		key: Val<'ctx>,
	) -> Result<Slot<'ctx>, Error> {
		let table = self.table_of(map.repr)?;
		let pointer = map.value.expect("a map").into_pointer_value();
		let entry = self.table_entry_for(pointer, table, key, None, node)?;
		self.entry_slot(entry, table, true)
	}

	/// `map.getOrInsert(key, value)` as a place (`use count = …`).
	pub(crate) fn get_or_insert_place(
		&mut self,
		node: NodeId,
		info: &CallInfo,
	) -> Result<Slot<'ctx>, Error> {
		let map = self.receiver(node)?;
		let given: Vec<NodeId> = info
			.arguments
			.iter()
			.filter_map(|argument| match argument {
				Argument::Given(value) => Some(*value),
				_ => None,
			})
			.collect();
		let [key_node, value_node] = given.as_slice() else {
			return Err(self.unsupported(node.index(), "this `getOrInsert`"));
		};
		let key = self.expr(*key_node)?;
		let value = self.expr(*value_node)?;
		let table = self.table_of(map.repr)?;
		let pointer = map.value.expect("a map").into_pointer_value();
		let entry = self.table_entry_for(pointer, table, key, Some((value, *value_node)), node)?;
		self.entry_slot(entry, table, true)
	}

	/// Appends `[1, 2]` (a set) or `[("a", 1)]` (a map).
	pub(crate) fn append_table(&mut self, value: Val<'ctx>, node: NodeId) -> Result<(), Error> {
		let table = self.table_of(value.repr)?;
		let pointer = value.value.expect("a table").into_pointer_value();
		let first = self.alloca(Repr::Bool, "first")?;
		self.builder
			.build_store(first.pointer, self.context.bool_type().const_int(1, false))?;
		let open = self.string_constant("[");
		self.append_value(open, node)?;
		self.for_each_entry(pointer, table, |this, entry, _| {
			let is_first = this.load(first)?.int();
			let separator = this.append_block("print_separator");
			let item = this.append_block("print_entry");
			this.builder
				.build_conditional_branch(is_first, item, separator)?;
			this.builder.position_at_end(separator);
			let comma = this.string_constant(", ");
			this.append_value(comma, node)?;
			this.builder.build_unconditional_branch(item)?;
			this.builder.position_at_end(item);
			this.builder
				.build_store(first.pointer, this.context.bool_type().const_zero())?;
			let key = this.load(this.entry_slot(entry, table, false)?)?;
			match table.value {
				None => this.append_nested(key, node),
				Some(_) => {
					let open = this.string_constant("(");
					this.append_value(open, node)?;
					this.append_nested(key, node)?;
					let comma = this.string_constant(", ");
					this.append_value(comma, node)?;
					let value = this.load(this.entry_slot(entry, table, true)?)?;
					this.append_nested(value, node)?;
					let close = this.string_constant(")");
					this.append_value(close, node)
				}
			}
		})?;
		let close = self.string_constant("]");
		self.append_value(close, node)
	}

	/// `a == b` on maps and sets: the same size, and every entry of `a` in
	/// `b` (with an equal value).
	pub(crate) fn tables_equal(
		&mut self,
		a: Val<'ctx>,
		b: Val<'ctx>,
		node: NodeId,
	) -> Result<IntValue<'ctx>, Error> {
		let table = self.table_of(a.repr)?;
		let first = a.value.expect("a table").into_pointer_value();
		let second = self.convert(b, a.repr, node.index())?;
		let second = second.value.expect("a table").into_pointer_value();
		let result = self.alloca(Repr::Bool, "tables_equal")?;
		let a_length = self.table_field(first, 1)?;
		let b_length = self.table_field(second, 1)?;
		let same = self
			.builder
			.build_int_compare(IntPredicate::EQ, a_length, b_length, "")?;
		self.builder.build_store(result.pointer, same)?;
		let compare = self.append_block("compare_entries");
		let end = self.append_block("tables_compared");
		self.builder.build_conditional_branch(same, compare, end)?;
		self.builder.position_at_end(compare);
		self.for_each_entry(first, table, |this, entry, _| {
			let key = this.load(this.entry_slot(entry, table, false)?)?;
			let (position, _) = this.table_find(second, table, key, node)?;
			let found = this.builder.build_int_compare(
				IntPredicate::SGE,
				position,
				position.get_type().const_zero(),
				"",
			)?;
			let holds = match table.value {
				None => found,
				Some(repr) => {
					let check = this.append_block("compare_value");
					let join = this.append_block("compared_value");
					let start = this.builder.get_insert_block().expect("a block");
					this.builder.build_conditional_branch(found, check, join)?;
					this.builder.position_at_end(check);
					let value = this.entry_slot(entry, table, true)?;
					let other_entry = this.entry_at(second, table, position)?;
					let other = this.entry_slot(other_entry, table, true)?;
					let compare = this.nested_equals_function(repr)?;
					let equal = this
						.builder
						.build_call(compare, &[value.pointer.into(), other.pointer.into()], "")?
						.try_as_basic_value()
						.basic()
						.expect("equality")
						.into_int_value();
					let equal = this.builder.build_int_compare(
						IntPredicate::NE,
						equal,
						this.context.i32_type().const_zero(),
						"",
					)?;
					let check_end = this.builder.get_insert_block().expect("a block");
					this.builder.build_unconditional_branch(join)?;
					this.builder.position_at_end(join);
					let phi = this.builder.build_phi(this.context.bool_type(), "")?;
					phi.add_incoming(&[
						(&this.context.bool_type().const_zero(), start),
						(&equal, check_end),
					]);
					phi.as_basic_value().into_int_value()
				}
			};
			let previous = this.load(result)?.int();
			let both = this.builder.build_and(previous, holds, "")?;
			this.builder.build_store(result.pointer, both)?;
			Ok(())
		})?;
		self.builder.build_unconditional_branch(end)?;
		self.builder.position_at_end(end);
		Ok(self.load(result)?.int())
	}

	/// `for x in set`, `for (key, value) in map`: views of the entries.
	pub(crate) fn for_entries(
		&mut self,
		node: NodeId,
		pattern: Option<NodeId>,
		table_value: Val<'ctx>,
		body: NodeId,
	) -> Result<Val<'ctx>, Error> {
		let table = self.table_of(table_value.repr)?;
		let pointer = table_value.value.expect("a table").into_pointer_value();
		let destructured = pattern.and_then(|pattern| self.analysis.pattern(pattern).cloned());
		let variable = pattern.and_then(|pattern| self.analysis.symbols.symbol_of_decl(pattern));
		let exit = self.append_block("end_for_entries");
		let next = self.append_block("for_entries_next");
		// a loop of its own, so that `break` and `continue` find their blocks
		let i64 = self.context.i64_type();
		let counter = self.alloca(Repr::Int(IntRepr::I64), "entry_position")?;
		self.builder
			.build_store(counter.pointer, i64.const_zero())?;
		let test = self.append_block("for_entries");
		let check = self.append_block("for_entry_live");
		let block = self.append_block("for_entry_body");
		self.builder.build_unconditional_branch(test)?;
		self.builder.position_at_end(test);
		let position = self.load(counter)?.int();
		let used = self.table_field(pointer, 2)?;
		let more = self
			.builder
			.build_int_compare(IntPredicate::SLT, position, used, "")?;
		self.builder.build_conditional_branch(more, check, exit)?;
		self.builder.position_at_end(check);
		let entry = self.entry_at(pointer, table, position)?;
		let live_field = self.builder.build_struct_gep(table.entry, entry, 1, "")?;
		let live = self
			.builder
			.build_load(i64, live_field, "")?
			.into_int_value();
		let live = self
			.builder
			.build_int_compare(IntPredicate::NE, live, i64.const_zero(), "")?;
		self.builder.build_conditional_branch(live, block, next)?;
		self.builder.position_at_end(block);
		let key_slot = self.entry_slot(entry, table, false)?;
		// `for (key, value) in map`: the names of the tuple pattern
		let names: Option<Vec<Option<fa_analyzer::ids::SymbolId>>> =
			match pattern.map(|pattern| self.view(pattern)) {
				Some(View::TuplePattern { items }) if destructured.is_none() => Some(
					items
						.iter()
						.map(|item| self.analysis.symbols.symbol_of_decl(item))
						.collect(),
				),
				_ => None,
			};
		if let (Some(names), Some(_)) = (&names, table.value) {
			let value_slot = self.entry_slot(entry, table, true)?;
			for (symbol, slot) in names.iter().zip([key_slot, value_slot]) {
				if let Some(symbol) = symbol {
					self.slots.insert(*symbol, slot);
				}
			}
		}
		match (destructured, variable, table.value) {
			_ if names.is_some() => {}
			(Some(info), _, Some(_)) => {
				let value_slot = self.entry_slot(entry, table, true)?;
				for (element, slot) in info.elements.iter().zip([key_slot, value_slot]) {
					if let fa_analyzer::analysis::PatternBinding::Symbol(symbol) = element.binding {
						self.slots.insert(symbol, slot);
					}
				}
			}
			(None, Some(symbol), None) => {
				self.slots.insert(symbol, key_slot);
			}
			(None, None, _) => {}
			_ => return Err(self.unsupported(node.index(), "`for` over this map")),
		}
		let mark = self.mark();
		self.frame_mut().loops.push(LoopFrame {
			exit,
			next,
			value: None,
			mark,
		});
		self.coop_point()?;
		self.block(body, false)?;
		self.frame_mut().loops.pop();
		self.branch_to(next)?;
		self.builder.position_at_end(next);
		let position = self.load(counter)?.int();
		let following = self
			.builder
			.build_int_nsw_add(position, i64.const_int(1, false), "")?;
		self.builder.build_store(counter.pointer, following)?;
		self.builder.build_unconditional_branch(test)?;
		self.builder.position_at_end(exit);
		Ok(Val::void())
	}

	/// `value is in set`, `key is in map`.
	pub(crate) fn table_includes(
		&mut self,
		container: Val<'ctx>,
		key: Val<'ctx>,
		node: NodeId,
	) -> Result<IntValue<'ctx>, Error> {
		let table = self.table_of(container.repr)?;
		let pointer = container.value.expect("a table").into_pointer_value();
		let (position, _) = self.table_find(pointer, table, key, node)?;
		Ok(self.builder.build_int_compare(
			IntPredicate::SGE,
			position,
			position.get_type().const_zero(),
			"",
		)?)
	}

	/// A new table of the same kind holding copies of the entries for which
	/// `keep` holds (a set operation, `filter`).
	fn table_filtered(
		&mut self,
		source: PointerValue<'ctx>,
		table: Table<'ctx>,
		repr: Repr,
		mut keep: impl FnMut(&mut Self, PointerValue<'ctx>) -> Result<IntValue<'ctx>, Error>,
		node: NodeId,
	) -> Result<Val<'ctx>, Error> {
		let zero = self.context.i64_type().const_zero();
		let result = self.table_new(table, zero)?;
		self.for_each_entry(source, table, |this, entry, _| {
			let holds = keep(this, entry)?;
			let add = this.append_block("filter_add");
			let skip = this.append_block("filter_skip");
			this.builder.build_conditional_branch(holds, add, skip)?;
			this.builder.position_at_end(add);
			let key = this.load(this.entry_slot(entry, table, false)?)?;
			let key = this.emit_copy(key)?;
			let value = match table.value {
				Some(_) => {
					let value = this.load(this.entry_slot(entry, table, true)?)?;
					Some(this.emit_copy(value)?)
				}
				None => None,
			};
			let hash = this.hash_of(key)?;
			this.table_append(result, table, hash, key, value, None)?;
			this.builder.build_unconditional_branch(skip)?;
			this.builder.position_at_end(skip);
			Ok(())
		})?;
		let _ = node;
		Ok(self.fresh(Val::new(repr, result)))
	}

	/// Calls a callback with the key (a set's element) and, for a map, the value.
	fn entry_callback(
		&mut self,
		callback: Val<'ctx>,
		callback_node: NodeId,
		entry: PointerValue<'ctx>,
		table: Table<'ctx>,
		before: &[Val<'ctx>],
	) -> Result<Val<'ctx>, Error> {
		let mut items = vec![self.entry_slot(entry, table, false)?];
		if table.value.is_some() {
			items.push(self.entry_slot(entry, table, true)?);
		}
		self.call_collection_callback(callback, callback_node, before, &items, &[])
	}

	/// The methods of sets and maps, `Set.from` and `Map.from`.
	pub(crate) fn table_method(
		&mut self,
		node: NodeId,
		path: &str,
		info: &CallInfo,
	) -> Result<Val<'ctx>, Error> {
		if path == "Set.from" || path == "Map.from" {
			return self.table_from(node, info);
		}
		let receiver = self.receiver(node)?;
		if receiver.repr == Repr::Never {
			return Ok(receiver);
		}
		let mut arguments = Vec::new();
		let mut nodes = Vec::new();
		for argument in &info.arguments {
			match argument {
				Argument::Given(value) => {
					let lowered = self.expr(*value)?;
					if lowered.repr == Repr::Never {
						return Ok(lowered);
					}
					arguments.push(lowered);
					nodes.push(*value);
				}
				Argument::Receiver(_) | Argument::Default => {}
				_ => return Err(self.unsupported(node.index(), path)),
			}
		}
		let receiver = match receiver.repr {
			Repr::Map(_) | Repr::Set(_) => receiver,
			_ => return Err(self.unsupported(node.index(), path)),
		};
		let table = self.table_of(receiver.repr)?;
		let pointer = receiver.value.expect("a table").into_pointer_value();
		let method = path.split('#').nth(1).unwrap_or(path);
		let i64 = self.context.i64_type();
		let bool_type = self.context.bool_type();
		match (method, arguments.as_slice()) {
			("size", []) => Ok(Val::new(
				Repr::Int(IntRepr::I64),
				self.table_field(pointer, 1)?,
			)),
			("isEmpty", []) => {
				let length = self.table_field(pointer, 1)?;
				let empty = self.builder.build_int_compare(
					IntPredicate::EQ,
					length,
					i64.const_zero(),
					"",
				)?;
				Ok(Val::new(Repr::Bool, empty))
			}
			("includes", [key]) => {
				let found = self.table_includes(receiver, *key, node)?;
				Ok(Val::new(Repr::Bool, found))
			}
			("add", [value]) => {
				let value = self.owned_as(*value, table.key, Some(nodes[0]))?;
				let added = self.table_insert(pointer, table, value, None, node)?;
				Ok(Val::new(Repr::Bool, added))
			}
			("get", [key]) => {
				let repr = self.collection_optional_repr(
					self.node_repr(node)?,
					table.value.unwrap_or(table.key),
				);
				let (position, _) = self.table_find(pointer, table, *key, node)?;
				let result = self.alloca(repr, "got")?;
				self.builder
					.build_store(result.pointer, self.llvm_type(repr).const_zero())?;
				let found = self.builder.build_int_compare(
					IntPredicate::SGE,
					position,
					i64.const_zero(),
					"",
				)?;
				let present = self.append_block("get_present");
				let end = self.append_block("got");
				self.builder.build_conditional_branch(found, present, end)?;
				self.builder.position_at_end(present);
				let entry = self.entry_at(pointer, table, position)?;
				let value = self.load(self.entry_slot(entry, table, true)?)?;
				let value = self.convert(value, repr, node.index())?;
				self.builder
					.build_store(result.pointer, value.value.expect("a value"))?;
				self.builder.build_unconditional_branch(end)?;
				self.builder.position_at_end(end);
				// a view of the value in the map
				self.load(result)
			}
			("getOrInsert", [key, value]) => {
				let entry =
					self.table_entry_for(pointer, table, *key, Some((*value, nodes[1])), node)?;
				let slot = self.entry_slot(entry, table, true)?;
				self.load(slot)
			}
			("delete" | "take", [key]) => {
				let (position, _) = self.table_find(pointer, table, *key, node)?;
				let found = self.builder.build_int_compare(
					IntPredicate::SGE,
					position,
					i64.const_zero(),
					"",
				)?;
				let repr = self.collection_optional_repr(
					self.node_repr(node)?,
					table.value.unwrap_or(table.key),
				);
				let result = (method == "take")
					.then(|| self.alloca(repr, "taken"))
					.transpose()?;
				if let Some(result) = result {
					self.builder
						.build_store(result.pointer, self.llvm_type(repr).const_zero())?;
				}
				let present = self.append_block("remove_present");
				let end = self.append_block("removed");
				self.builder.build_conditional_branch(found, present, end)?;
				self.builder.position_at_end(present);
				let kept = self.table_remove(pointer, table, position, result.is_some())?;
				if let (Some(result), Some(kept)) = (result, kept) {
					let kept = self.convert(kept, repr, node.index())?;
					self.builder
						.build_store(result.pointer, kept.value.expect("a value"))?;
				}
				self.builder.build_unconditional_branch(end)?;
				self.builder.position_at_end(end);
				match result {
					// the element leaves the table: owned by the statement
					Some(result) => Ok(self.fresh(self.load(result)?)),
					None => Ok(Val::new(Repr::Bool, found)),
				}
			}
			("clear", []) => {
				if self.needs_drop(table.key)
					|| table.value.is_some_and(|value| self.needs_drop(value))
				{
					self.for_each_entry(pointer, table, |this, entry, _| {
						let key = this.load(this.entry_slot(entry, table, false)?)?;
						this.emit_drop(key)?;
						if table.value.is_some() {
							let value = this.load(this.entry_slot(entry, table, true)?)?;
							this.emit_drop(value)?;
						}
						Ok(())
					})?;
				}
				self.call_runtime("fa_table_clear", &[pointer.into()])?;
				Ok(Val::void())
			}
			("deleteWhere", [callback]) => {
				let callback_node = nodes[0];
				self.for_each_entry(pointer, table, |this, entry, position| {
					let delete =
						this.entry_callback(*callback, callback_node, entry, table, &[])?;
					let remove = this.append_block("delete_entry");
					let keep = this.append_block("keep_entry");
					this.builder
						.build_conditional_branch(delete.int(), remove, keep)?;
					this.builder.position_at_end(remove);
					this.table_remove(pointer, table, position, false)?;
					this.builder.build_unconditional_branch(keep)?;
					this.builder.position_at_end(keep);
					Ok(())
				})?;
				Ok(Val::void())
			}
			("every" | "some", [callback]) => {
				let callback_node = nodes[0];
				let every = method == "every";
				let result = self.alloca(Repr::Bool, "holds")?;
				self.builder
					.build_store(result.pointer, bool_type.const_int(every as u64, false))?;
				self.for_each_entry(pointer, table, |this, entry, _| {
					let holds = this.entry_callback(*callback, callback_node, entry, table, &[])?;
					let previous = this.load(result)?.int();
					let combined = if every {
						this.builder.build_and(previous, holds.int(), "")?
					} else {
						this.builder.build_or(previous, holds.int(), "")?
					};
					this.builder.build_store(result.pointer, combined)?;
					Ok(())
				})?;
				self.load(result)
			}
			("find", [callback]) => {
				let callback_node = nodes[0];
				let repr = self.collection_optional_repr(self.node_repr(node)?, table.key);
				let result = self.alloca(repr, "found")?;
				self.builder
					.build_store(result.pointer, self.llvm_type(repr).const_zero())?;
				let done = self.alloca(Repr::Bool, "done")?;
				self.builder
					.build_store(done.pointer, bool_type.const_zero())?;
				self.for_each_entry(pointer, table, |this, entry, _| {
					let finished = this.load(done)?.int();
					let search = this.append_block("find_entry");
					let skip = this.append_block("found_entry");
					this.builder
						.build_conditional_branch(finished, skip, search)?;
					this.builder.position_at_end(search);
					let holds = this.entry_callback(*callback, callback_node, entry, table, &[])?;
					let take = this.append_block("find_take");
					this.builder
						.build_conditional_branch(holds.int(), take, skip)?;
					this.builder.position_at_end(take);
					let key = this.entry_slot(entry, table, false)?;
					let found = match (table.value, repr) {
						(Some(_), Repr::Optional(Scalar::Tuple(tuple))) => {
							let fields = this.tuple_reprs(tuple);
							let key = this.collection_item_view(key, fields[0], node.index())?;
							let value = this.entry_slot(entry, table, true)?;
							let value =
								this.collection_item_view(value, fields[1], node.index())?;
							let structure = this.tuple_type(tuple);
							let pair = this.builder.build_insert_value(
								structure.get_undef(),
								key.value.expect("a key"),
								0,
								"",
							)?;
							let pair = this.builder.build_insert_value(
								pair,
								value.value.expect("a value"),
								1,
								"",
							)?;
							Val::new(Repr::Tuple(tuple), pair.into_struct_value())
						}
						_ => this.load(key)?,
					};
					let found = this.convert(found, repr, node.index())?;
					this.builder
						.build_store(result.pointer, found.value.expect("a value"))?;
					this.builder
						.build_store(done.pointer, bool_type.const_int(1, false))?;
					this.builder.build_unconditional_branch(skip)?;
					this.builder.position_at_end(skip);
					Ok(())
				})?;
				// a view of the entry
				self.load(result)
			}
			("filter", [callback]) => {
				let callback_node = nodes[0];
				let repr = receiver.repr;
				self.table_filtered(
					pointer,
					table,
					repr,
					|this, entry| {
						let holds =
							this.entry_callback(*callback, callback_node, entry, table, &[])?;
						Ok(holds.int())
					},
					node,
				)
			}
			("map", [callback]) => {
				let callback_node = nodes[0];
				let result_repr = self.node_repr(node)?;
				match result_repr {
					Repr::Array(item_type) => {
						// a set's map: an array of the results
						let item = self.item_repr(item_type, node)?;
						let length = self.table_field(pointer, 1)?;
						let array = self.new_array(item, i64.const_zero(), length)?;
						let array_value = Val::new(result_repr, array);
						self.for_each_entry(pointer, table, |this, entry, _| {
							let mapped =
								this.entry_callback(*callback, callback_node, entry, table, &[])?;
							this.array_push(array_value, mapped, node, None)
						})?;
						Ok(self.fresh(array_value))
					}
					Repr::Map(_) => {
						// a map's map: the same keys, the results as values
						let result_table = self.table_of(result_repr)?;
						let length = self.table_field(pointer, 1)?;
						let result = self.table_new(result_table, length)?;
						self.for_each_entry(pointer, table, |this, entry, _| {
							let mapped =
								this.entry_callback(*callback, callback_node, entry, table, &[])?;
							let mapped =
								this.owned_as(mapped, result_table.value.expect("a map"), None)?;
							let key = this.load(this.entry_slot(entry, table, false)?)?;
							let key = this.emit_copy(key)?;
							let hash = this.hash_of(key)?;
							this.table_append(result, result_table, hash, key, Some(mapped), None)?;
							Ok(())
						})?;
						Ok(self.fresh(Val::new(result_repr, result)))
					}
					_ => Err(self.unsupported(node.index(), path)),
				}
			}
			("reduce", [callback, initial]) => {
				let callback_node = nodes[0];
				let repr = self.node_repr(node)?;
				let accumulator = self.alloca(repr, "accumulator")?;
				let initial = self.owned_as(*initial, repr, Some(nodes[1]))?;
				self.store(accumulator, initial, node)?;
				self.for_each_entry(pointer, table, |this, entry, _| {
					let current = this.load(accumulator)?;
					let next =
						this.entry_callback(*callback, callback_node, entry, table, &[current])?;
					let next = this.owned_as(next, repr, None)?;
					this.emit_drop(current)?;
					this.store(accumulator, next, node)?;
					Ok(())
				})?;
				Ok(self.fresh(self.load(accumulator)?))
			}
			("sum", []) => {
				let repr = self.node_repr(node)?;
				let total = self.alloca(repr, "sum")?;
				self.builder
					.build_store(total.pointer, self.llvm_type(repr).const_zero())?;
				self.for_each_entry(pointer, table, |this, entry, _| {
					let value = this.load(this.entry_slot(entry, table, false)?)?;
					let value = this.convert(value, repr, node.index())?;
					let current = this.load(total)?;
					let sum: BasicValueEnum<'ctx> = match repr {
						Repr::Int(_) => this
							.builder
							.build_int_add(current.int(), value.int(), "")?
							.into(),
						_ => this
							.builder
							.build_float_add(
								current.value.expect("a float").into_float_value(),
								value.value.expect("a float").into_float_value(),
								"",
							)?
							.into(),
					};
					this.builder.build_store(total.pointer, sum)?;
					Ok(())
				})?;
				self.load(total)
			}
			("union", [other]) => {
				let other_value = self.convert(*other, receiver.repr, node.index())?;
				let other_pointer = other_value.value.expect("a set").into_pointer_value();
				let copy = self.emit_copy(receiver)?;
				let result = copy.value.expect("a set").into_pointer_value();
				self.for_each_entry(other_pointer, table, |this, entry, _| {
					let key = this.load(this.entry_slot(entry, table, false)?)?;
					let key = this.emit_copy(key)?;
					this.table_insert(result, table, key, None, node)?;
					Ok(())
				})?;
				Ok(self.fresh(copy))
			}
			("intersection" | "difference", [other]) => {
				let other_value = self.convert(*other, receiver.repr, node.index())?;
				let intersection = method == "intersection";
				self.table_filtered(
					pointer,
					table,
					receiver.repr,
					|this, entry| {
						let key = this.load(this.entry_slot(entry, table, false)?)?;
						let found = this.table_includes(other_value, key, node)?;
						Ok(if intersection {
							found
						} else {
							this.builder.build_not(found, "")?
						})
					},
					node,
				)
			}
			("isSubsetOf", [other]) => {
				let other_value = self.convert(*other, receiver.repr, node.index())?;
				let result = self.alloca(Repr::Bool, "subset")?;
				self.builder
					.build_store(result.pointer, bool_type.const_int(1, false))?;
				self.for_each_entry(pointer, table, |this, entry, _| {
					let key = this.load(this.entry_slot(entry, table, false)?)?;
					let found = this.table_includes(other_value, key, node)?;
					let previous = this.load(result)?.int();
					let both = this.builder.build_and(previous, found, "")?;
					this.builder.build_store(result.pointer, both)?;
					Ok(())
				})?;
				self.load(result)
			}
			_ => Err(self.unsupported(node.index(), &format!("`{path}`"))),
		}
	}

	/// `Set.from(source)`, `Map.from(source)`: a new table of copies of the
	/// items of an array, a set or a map (`capacity` a hint).
	fn table_from(&mut self, node: NodeId, info: &CallInfo) -> Result<Val<'ctx>, Error> {
		let repr = self.node_repr(node)?;
		let table = self.table_of(repr)?;
		let Some(Argument::Given(source_node)) = info.arguments.first() else {
			return Err(self.unsupported(node.index(), "this `from`"));
		};
		if matches!(repr, Repr::Set(_)) && matches!(info.arguments.get(1), Some(Argument::Given(_)))
		{
			return Err(self.unsupported(node.index(), "the `key` of `Set.from`"));
		}
		let source = self.expr(*source_node)?;
		if source.repr == Repr::Never {
			return Ok(source);
		}
		let hint_index = if table.value.is_some() { 1 } else { 2 };
		let capacity = match info.arguments.get(hint_index) {
			Some(Argument::Given(hint)) => {
				let hint = self.expr(*hint)?;
				if hint.repr == Repr::Never {
					return Ok(hint);
				}
				self.convert(hint, Repr::Int(IntRepr::I64), node.index())?
					.int()
			}
			_ => self.context.i64_type().const_zero(),
		};
		let result = self.table_new(table, capacity)?;
		self.reserve_storage(Val::new(repr, result), capacity, node)?;
		match source.repr {
			Repr::Array(item_type) => {
				let item = self.item_repr(item_type, node)?;
				let array = source.value.expect("an array").into_pointer_value();
				let (_, length) = self.array_parts(array)?;
				self.for_each_item_value(array, item, length, |this, value| {
					let (key, value) = match table.value {
						None => (
							this.emit_copy(this.convert(value, table.key, node.index())?)?,
							None,
						),
						Some(_) => {
							let (key, value) = this.pair_parts(value, table, node)?;
							(key, Some(value))
						}
					};
					this.table_insert(result, table, key, value, node)?;
					Ok(())
				})?;
			}
			Repr::Set(_) | Repr::Map(_) => {
				let source_table = self.table_of(source.repr)?;
				let source_pointer = source.value.expect("a table").into_pointer_value();
				self.for_each_entry(source_pointer, source_table, |this, entry, _| {
					let key = this.load(this.entry_slot(entry, source_table, false)?)?;
					let key = this.emit_copy(this.convert(key, table.key, node.index())?)?;
					let value = match (table.value, source_table.value) {
						(Some(repr), Some(_)) => {
							let value = this.load(this.entry_slot(entry, source_table, true)?)?;
							Some(this.emit_copy(this.convert(value, repr, node.index())?)?)
						}
						_ => None,
					};
					this.table_insert(result, table, key, value, node)?;
					Ok(())
				})?;
			}
			_ => return Err(self.unsupported(node.index(), "this `from`")),
		}
		Ok(self.fresh(Val::new(repr, result)))
	}

	/// An array of copies of the elements of a set (`Array.from(set)`).
	pub(crate) fn array_of_table(
		&mut self,
		source: Val<'ctx>,
		repr: Repr,
		node: NodeId,
	) -> Result<Val<'ctx>, Error> {
		let table = self.table_of(source.repr)?;
		let pointer = source.value.expect("a table").into_pointer_value();
		let Repr::Array(item_type) = repr else {
			return Err(self.unsupported(node.index(), "`Array.from`"));
		};
		let item = self.item_repr(item_type, node)?;
		let length = self.table_field(pointer, 1)?;
		let array = self.new_array(item, self.context.i64_type().const_zero(), length)?;
		let array_value = Val::new(repr, array);
		self.for_each_entry(pointer, table, |this, entry, _| {
			let key = this.load(this.entry_slot(entry, table, false)?)?;
			let element = match table.value {
				None => key,
				Some(_) => {
					let Repr::Tuple(tuple) = item else {
						return Err(this.unsupported(node.index(), "`Array.from` of a map"));
					};
					let value = this.load(this.entry_slot(entry, table, true)?)?;
					let pair = this.builder.build_insert_value(
						this.tuple_type(tuple).get_undef(),
						key.value.expect("a key"),
						0,
						"",
					)?;
					let pair = this.builder.build_insert_value(
						pair,
						value.value.expect("a value"),
						1,
						"",
					)?;
					Val::new(item, pair.into_struct_value())
				}
			};
			this.array_push(array_value, element, node, None)
		})?;
		Ok(self.fresh(array_value))
	}

	// ── drops and copies (`ownership.rs`) ───────────────────────────────

	/// The body of the drop of a table: its live keys and values, then its blocks.
	pub(crate) fn build_table_drop(
		&self,
		builder: &Builder<'ctx>,
		function: FunctionValue<'ctx>,
		repr: Repr,
		value: BasicValueEnum<'ctx>,
	) -> Result<(), Error> {
		let table = self.table_of(repr)?;
		let pointer = value.into_pointer_value();
		let present = builder.build_is_not_null(pointer, "")?;
		let then = self.context.append_basic_block(function, "table");
		let none = self.context.append_basic_block(function, "no_table");
		builder.build_conditional_branch(present, then, none)?;
		builder.position_at_end(none);
		builder.build_return(None)?;
		builder.position_at_end(then);
		let field = builder.build_struct_gep(self.table_header(), pointer, 9, "")?;
		let storage = builder
			.build_load(self.context.i64_type(), field, "")?
			.into_int_value();
		let owned = builder.build_int_compare(
			IntPredicate::NE,
			storage,
			storage.get_type().const_int(2, false),
			"",
		)?;
		self.helper_when(builder, function, owned, |builder| {
			builder.build_return(None)?;
			Ok(())
		})?;
		let owning =
			self.needs_drop(table.key) || table.value.is_some_and(|value| self.needs_drop(value));
		if owning {
			self.helper_entries(builder, function, pointer, table, |builder, entry| {
				for (index, repr) in [(2, Some(table.key)), (3, table.value)] {
					let Some(repr) = repr else { continue };
					if self.needs_drop(repr) {
						let field = builder.build_struct_gep(table.entry, entry, index, "")?;
						let loaded = builder.build_load(self.llvm_type(repr), field, "")?;
						builder.build_call(self.drop_function(repr)?, &[loaded.into()], "")?;
					}
				}
				Ok(())
			})?;
		}
		builder.build_call(
			self.runtime.function("fa_table_free")?,
			&[pointer.into(), self.entry_size(table).into()],
			"",
		)?;
		Ok(())
	}

	/// The body of the copy of a table: its live entries, copied.
	pub(crate) fn build_table_copy(
		&self,
		builder: &Builder<'ctx>,
		function: FunctionValue<'ctx>,
		repr: Repr,
		value: BasicValueEnum<'ctx>,
	) -> Result<BasicValueEnum<'ctx>, Error> {
		let table = self.table_of(repr)?;
		let pointer = value.into_pointer_value();
		let present = builder.build_is_not_null(pointer, "")?;
		let then = self.context.append_basic_block(function, "table");
		let none = self.context.append_basic_block(function, "no_table");
		builder.build_conditional_branch(present, then, none)?;
		builder.position_at_end(none);
		builder.build_return(Some(&pointer))?;
		builder.position_at_end(then);
		let copy = builder
			.build_call(
				self.runtime.function("fa_table_copy")?,
				&[pointer.into(), self.entry_size(table).into()],
				"",
			)?
			.try_as_basic_value()
			.basic()
			.expect("a table")
			.into_pointer_value();
		let owning =
			self.needs_drop(table.key) || table.value.is_some_and(|value| self.needs_drop(value));
		if owning {
			self.helper_entries(builder, function, copy, table, |builder, entry| {
				for (index, repr) in [(2, Some(table.key)), (3, table.value)] {
					let Some(repr) = repr else { continue };
					if self.needs_drop(repr) {
						let field = builder.build_struct_gep(table.entry, entry, index, "")?;
						let loaded = builder.build_load(self.llvm_type(repr), field, "")?;
						let copied = builder
							.build_call(self.copy_function(repr)?, &[loaded.into()], "")?
							.try_as_basic_value()
							.basic()
							.expect("a copy");
						builder.build_store(field, copied)?;
					}
				}
				Ok(())
			})?;
		}
		Ok(copy.into())
	}

	/// A loop over the live entries of a table in a helper.
	fn helper_entries(
		&self,
		builder: &Builder<'ctx>,
		function: FunctionValue<'ctx>,
		pointer: PointerValue<'ctx>,
		table: Table<'ctx>,
		mut body: impl FnMut(&Builder<'ctx>, PointerValue<'ctx>) -> Result<(), Error>,
	) -> Result<(), Error> {
		let i64 = self.context.i64_type();
		let header = self.table_header();
		let entries_field = builder.build_struct_gep(header, pointer, 0, "")?;
		let entries = builder
			.build_load(self.pointer_type(), entries_field, "")?
			.into_pointer_value();
		let used_field = builder.build_struct_gep(header, pointer, 2, "")?;
		let used = builder.build_load(i64, used_field, "")?.into_int_value();
		let before = builder.get_insert_block().expect("a block");
		let test = self.context.append_basic_block(function, "entries");
		let check = self.context.append_basic_block(function, "entry");
		let live_block = self.context.append_basic_block(function, "live_entry");
		let step = self.context.append_basic_block(function, "next_entry");
		let end = self.context.append_basic_block(function, "entries_end");
		builder.build_unconditional_branch(test)?;
		builder.position_at_end(test);
		let index = builder.build_phi(i64, "position")?;
		index.add_incoming(&[(&i64.const_zero(), before)]);
		let current = index.as_basic_value().into_int_value();
		let more = builder.build_int_compare(IntPredicate::SLT, current, used, "")?;
		builder.build_conditional_branch(more, check, end)?;
		builder.position_at_end(check);
		let entry = if table.stable() {
			builder
				.build_call(
					self.runtime.function("fa_table_entry")?,
					&[
						pointer.into(),
						current.into(),
						self.entry_size(table).into(),
					],
					"",
				)?
				.try_as_basic_value()
				.basic()
				.expect("an entry")
				.into_pointer_value()
		} else {
			unsafe { builder.build_in_bounds_gep(table.entry, entries, &[current], "")? }
		};

		let live_field = builder.build_struct_gep(table.entry, entry, 1, "")?;
		let live = builder.build_load(i64, live_field, "")?.into_int_value();
		let live = builder.build_int_compare(IntPredicate::NE, live, i64.const_zero(), "")?;
		builder.build_conditional_branch(live, live_block, step)?;
		builder.position_at_end(live_block);
		body(builder, entry)?;
		builder.build_unconditional_branch(step)?;
		builder.position_at_end(step);
		let next = builder.build_int_add(current, i64.const_int(1, false), "")?;
		index.add_incoming(&[(&next, step)]);
		builder.build_unconditional_branch(test)?;
		builder.position_at_end(end);
		Ok(())
	}
}
