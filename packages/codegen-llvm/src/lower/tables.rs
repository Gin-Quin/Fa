//! Maps and sets: a pointer to a table of the runtime (`fa_runtime.c`,
//! "maps and sets"), its entries `{ i64 live, key, value }` in slot order (a
//! set's entries have no value; `live > 0` marks a live entry), and an index
//! holding the hashes. Maps and growing tables keep their entries in stable
//! chunks. Keys are hashed and compared by functions generated per shape:
//! structural equality, floats with SameValueZero (`NaN` is one key, `-0` and
//! `0` are one key), like `FaHashMap`; integer keys are compared bitwise, in
//! the probe loop.
//!
//! The table owns its keys and values: inserting takes them, deleting drops
//! them, reading one gives a view of it.

use fa_analyzer::analysis::{Argument, CallInfo, Callee, NodeFlags};
use fa_analyzer::ids::{NodeId, TypeId};
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
	/// `{ key, value }` (a set's: `{ key }`).
	pub entry: StructType<'ctx>,
	/// A `GrowingMap` or a `GrowingSet` (the same storage: the entries of
	/// every table live in chunks that never move, `fa_table_entry`).
	pub growing: bool,
	/// A map whose entries are dense (one contiguous array that moves):
	/// no reference into its values outlives an insertion ([`Lowering::dense_map`]).
	pub dense: bool,
	/// A dense map of small entries stored in its probe array itself
	/// (`TABLE_INLINE` of the runtime): integer keys and scalar fields of at most
	/// 64 bits, and no `deleteWhere` on maps of its type.
	pub inline: bool,
}

/// `TABLE_DENSE` of the runtime: a flag of `entry_size`.
const TABLE_DENSE: u64 = 1 << 40;
/// `TABLE_INLINE` of the runtime: a flag of `entry_size`.
const TABLE_INLINE: u64 = 1 << 41;
/// A 32-bit key occupies its own presence word (all ones has a spare slot).
const TABLE_INLINE_KEY32: u64 = 1 << 42;
/// A 64-bit key occupies its own presence word (all ones has a spare slot).
const TABLE_INLINE_KEY64: u64 = 1 << 43;
/// `TABLE_CHUNK_SLOTS` of the runtime: the entries of a chunk.
pub(crate) const TABLE_CHUNK_SLOTS: u64 = 32;
/// `storage` of a table in read-only data: nothing in it is dropped.
pub(crate) const TABLE_STATIC: u64 = 2;

/// What tells a constant key from the other keys of its literal: its bits
/// (the canonical ones of a float), or its text.
#[derive(PartialEq)]
enum ConstantKey {
	Word(u64),
	Text(String),
}

/// The multiplier of the hashes: the golden ratio.
const HASH_MULTIPLIER: u64 = 0x9E37_79B9_7F4A_7C15;

/// `fa_hash_bytes` of the runtime.
fn hash_bytes(bytes: &[u8]) -> u64 {
	let mut hash = 0xcbf2_9ce4_8422_2325 ^ (bytes.len() as u64).wrapping_mul(HASH_MULTIPLIER);
	let (words, rest) = bytes.as_chunks::<8>();
	for word in words {
		let word = u64::from_le_bytes(*word);
		hash = (hash ^ word).wrapping_mul(HASH_MULTIPLIER);
		hash ^= hash >> 29;
	}
	if !rest.is_empty() {
		// the last 1 to 7 bytes, as one word
		let word = if rest.len() >= 4 {
			let low = u32::from_le_bytes(rest[..4].try_into().expect("four bytes"));
			let high = u32::from_le_bytes(rest[rest.len() - 4..].try_into().expect("four bytes"));
			u64::from(low) | (u64::from(high) << 32)
		} else {
			u64::from(rest[0])
				| (u64::from(rest[rest.len() / 2]) << 8)
				| (u64::from(rest[rest.len() - 1]) << 16)
		};
		hash = (hash ^ word).wrapping_mul(HASH_MULTIPLIER);
	}
	hash = (hash ^ (hash >> 33)).wrapping_mul(0xff51_afd7_ed55_8ccd);
	hash ^ (hash >> 33)
}

impl<'ctx> Lowering<'_, 'ctx> {
	// ── layouts ─────────────────────────────────────────────────────────

	/// `{ ptr chunks, i64 length, i64 used, i64 capacity, ptr index, i64
	/// mask, i64 directory, i64 free, ptr entries, i64 room, i64 storage }`.
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
				pointer.into(),
				i64.into(),
				i64.into(),
			],
			false,
		)
	}

	/// `TableChunk` of the runtime: `{ ptr entries, i32 live, i32 next }`.
	pub(crate) fn table_chunk_type(&self) -> StructType<'ctx> {
		let i32 = self.context.i32_type();
		self.context
			.struct_type(&[self.pointer_type().into(), i32.into(), i32.into()], false)
	}

	/// The slots of the index of a table of `count` entries (`index_slots`
	/// of the runtime): at most half of them name an entry.
	pub(crate) fn table_index_slots(count: u64) -> u64 {
		(count * 2).max(8).next_power_of_two()
	}

	/// The bytes of a slot of the probe array of an inline table, and
	/// whether its key is its own presence word (`inline_stride` of the
	/// runtime).
	pub(crate) fn inline_slot(&self, table: Table<'ctx>) -> (u64, bool) {
		let own_key = matches!(table.key, Repr::Int(int) if matches!(int.bits, 32 | 64));
		let entry = self.collection_data_size(table.entry.into());
		// A wide value needs an aligned entry after the 32-bit hash tag.
		let prefix = if own_key {
			0
		} else if entry > 8 {
			8
		} else {
			4
		};
		(entry + prefix, own_key)
	}

	/// The bytes of the block `fa_table_local` lays a table of `capacity`
	/// entries in: its header, then its probe array for inline entries,
	/// else its index, its directory and its chunks of 32 entries.
	fn table_local_bytes(&self, table: Table<'ctx>, capacity: u64) -> u64 {
		let header = self.collection_data_size(self.table_header().into());
		let slots = Self::table_index_slots(capacity);
		if table.inline {
			let (stride, own_key) = self.inline_slot(table);
			return header + (slots + u64::from(own_key)) * stride;
		}
		let chunks = capacity.div_ceil(TABLE_CHUNK_SLOTS);
		let directory = chunks * self.collection_data_size(self.table_chunk_type().into());
		let entry = self.collection_data_size(table.entry.into());
		let chunk = if entry > 0 {
			TABLE_CHUNK_SLOTS * entry
		} else {
			16
		};
		((header + slots * 8 + directory + 15) & !15) + chunks * chunk
	}

	/// Every operation consults the same normalized map ABI, including scalar
	/// views. Concrete applications were resolved before any layout was chosen.
	fn map_excluded(
		&self,
		ty: TypeId,
		kind: &'static str,
		local: &std::collections::BTreeSet<String>,
	) -> bool {
		let ty = self.concrete_type(ty);
		crate::facts::map_excluded(self.types(), ty, |name| {
			if self.whole_program {
				crate::facts::contains_map_fact(local, name)
			} else {
				self.program_facts.as_ref().is_none_or(|facts| {
					facts
						.consulted(kind, self.fact_log.as_deref())
						.contains(name)
				})
			}
		})
	}

	/// No reference into a map of this ABI outlives growth.
	pub(crate) fn dense_map(&self, ty: TypeId) -> bool {
		!self.map_excluded(ty, crate::facts::REFERENCED_MAPS, &self.unsafe_maps)
	}

	fn deleted_while_iterated(&self, ty: TypeId) -> bool {
		self.map_excluded(
			ty,
			crate::facts::DELETED_WHILE_ITERATED_MAPS,
			&self.no_inline_maps,
		)
	}

	/// The key and value of a map or set type.
	pub(crate) fn table_of(&self, repr: Repr) -> Result<Table<'ctx>, Error> {
		let (Repr::Map(ty) | Repr::Set(ty)) = repr else {
			return Err(Error::Llvm(format!("{repr:?} is not a table")));
		};
		let ty = self.concrete_type(ty);
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
		let mut fields: Vec<BasicTypeEnum<'ctx>> = vec![self.llvm_type(key)];
		if let Some(value) = value {
			fields.push(self.llvm_type(value));
		}
		let dense = kind == CollKind::Map && self.dense_map(ty);
		let small = |repr: Repr| match repr {
			Repr::Int(int) => int.bits <= 64,
			Repr::Bool | Repr::F32 | Repr::F64 | Repr::Percentage => true,
			_ => false,
		};
		let inline = dense
			&& matches!(key, Repr::Int(int) if matches!(int.bits, 8 | 16 | 32 | 64))
			&& value.is_some_and(small)
			&& !self.deleted_while_iterated(ty);
		Ok(Table {
			key,
			value,
			entry: self.context.struct_type(&fields, false),
			growing: matches!(kind, CollKind::GrowingMap | CollKind::GrowingSet),
			dense,
			inline,
		})
	}

	/// The size of an entry, an `i64` constant (with `TABLE_DENSE` for
	/// dense entries: the runtime then finds them in one array).
	pub(crate) fn entry_size(&self, table: Table<'ctx>) -> IntValue<'ctx> {
		let size = table.entry.size_of().expect("a sized entry");
		if table.inline {
			let flags = TABLE_INLINE
				| match table.key {
					Repr::Int(int) if int.bits == 32 => TABLE_INLINE_KEY32,
					Repr::Int(int) if int.bits == 64 => TABLE_INLINE_KEY64,
					_ => 0,
				};
			let flag = size.get_type().const_int(flags, false);
			// the size is far below the flag: xor sets it
			return size.const_xor(flag);
		}
		if table.dense {
			let flag = size.get_type().const_int(TABLE_DENSE, false);
			// the size is far below the flag: xor sets it
			return size.const_xor(flag);
		}
		size
	}

	// ── read-only literals ──────────────────────────────────────────────

	/// The hash that [`Self::build_hash`] gives a constant key, and what
	/// tells it from the other keys of its literal. `None` for the keys that
	/// only the program hashes.
	fn constant_key(
		&self,
		node: NodeId,
		repr: Repr,
		value: BasicValueEnum<'ctx>,
	) -> Option<(u64, ConstantKey)> {
		let mix = |mut value: u64| {
			for multiplier in [0xff51_afd7_ed55_8ccd_u64, 0xc4ce_b9fe_1a85_ec53] {
				value = (value ^ (value >> 33)).wrapping_mul(multiplier);
			}
			value ^ (value >> 33)
		};
		let mix_integer = |value: u64| {
			let product = (value ^ (value >> 32)).wrapping_mul(HASH_MULTIPLIER);
			product ^ (product >> 32)
		};
		Some(match repr {
			Repr::Int(int) if int.bits <= 32 => {
				let word = value.into_int_value().get_zero_extended_constant()?;
				let product = word.wrapping_mul(HASH_MULTIPLIER);
				let hash = if int.bits == 32 {
					product.rotate_right(24)
				} else {
					product ^ (product >> 32)
				};
				(hash, ConstantKey::Word(word))
			}
			Repr::Int(int) if int.bits <= 64 => {
				let value = value.into_int_value();
				let word = if int.signed {
					value.get_sign_extended_constant()? as u64
				} else {
					value.get_zero_extended_constant()?
				};
				(mix_integer(word), ConstantKey::Word(word))
			}
			Repr::Bool => {
				let word = value.into_int_value().get_zero_extended_constant()?;
				(mix_integer(word), ConstantKey::Word(word))
			}
			Repr::F64 | Repr::F32 | Repr::Percentage => {
				// SameValueZero: `-0` is `0`, every `NaN` is one
				let float = value.into_float_value().get_constant()?.0;
				let bits = if float == 0.0 {
					0
				} else if float.is_nan() {
					0x7ff8_0000_0000_0000
				} else {
					float.to_bits()
				};
				(mix(bits), ConstantKey::Word(bits))
			}
			Repr::Str => {
				let text = self.constant_text(node)?;
				(hash_bytes(text.as_bytes()), ConstantKey::Text(text))
			}
			_ => return None,
		})
	}

	/// A map or a set literal of constants in read-only data, laid out as
	/// the runtime lays out a table (`fa_runtime.c`, "Tables"): its header
	/// ([`TABLE_STATIC`]), its index and its entries, each key where an
	/// insertion would have put it. `None` when an item isn't a constant, or
	/// a key isn't one [`Self::constant_key`] hashes.
	pub(crate) fn readonly_table(
		&mut self,
		node: NodeId,
		repr: Repr,
		depth: usize,
	) -> Result<Option<Val<'ctx>>, Error> {
		let View::Array { items } = self.view(node) else {
			return Ok(None);
		};
		let table = self.table_of(repr)?;
		let mut keys: Vec<(u64, ConstantKey)> = Vec::new();
		let mut entries: Vec<Vec<BasicValueEnum<'ctx>>> = Vec::new();
		for item in items.iter() {
			let (key_node, value_node) = match table.value {
				Some(_) => {
					let View::Tuple { items: pair } = self.view(self.ungrouped(item)) else {
						return Ok(None);
					};
					if pair.len() != 2 {
						return Ok(None);
					}
					(pair.get(0).expect("a key"), pair.get(1))
				}
				None => (item, None),
			};
			let Some(key) = self.readonly_item(key_node, table.key, depth + 1)? else {
				return Ok(None);
			};
			let Some(known) = self.constant_key(key_node, table.key, key) else {
				return Ok(None);
			};
			let value = match (value_node, table.value) {
				(Some(node), Some(repr)) => {
					let Some(value) = self.readonly_item(node, repr, depth + 1)? else {
						return Ok(None);
					};
					Some(value)
				}
				_ => None,
			};
			// a key written twice keeps its place, with its last value
			let entry = match keys.iter().position(|(_, other)| *other == known.1) {
				Some(position) => position,
				None => {
					keys.push(known);
					entries.push(vec![key]);
					entries.len() - 1
				}
			};
			entries[entry].truncate(1);
			entries[entry].extend(value);
		}
		let entries: Vec<_> = entries
			.iter()
			.map(|fields| table.entry.const_named_struct(fields))
			.collect();
		let count = keys.len() as u64;
		let i64 = self.context.i64_type();
		let i32 = self.context.i32_type();
		let null = self.pointer_type().const_null();
		let slots = Self::table_index_slots(count);
		let mask = slots - 1;
		let header = if table.inline {
			let (stride, own_key) = self.inline_slot(table);
			// a slot: the tag of the hash (0: empty) then the entry, or the
			// entry alone when its 32/64-bit key says whether it is there (the
			// all-ones key then lives in one more slot, after the others)
			let slot_type = if own_key {
				table.entry
			} else {
				let mut fields = vec![i32.into()];
				if stride - self.collection_data_size(table.entry.into()) == 8 {
					fields.push(i32.into());
				}
				fields.push(table.entry.into());
				self.context.struct_type(&fields, true)
			};
			let total = slots + u64::from(own_key);
			let mut probe = vec![None; total as usize];
			let mut sentinel = false;
			for ((hash, key), entry) in keys.iter().zip(entries) {
				let sentinel_bits = match table.key {
					Repr::Int(int) if int.bits == 32 => u64::from(u32::MAX),
					_ => u64::MAX,
				};
				if own_key && *key == ConstantKey::Word(sentinel_bits) {
					sentinel = true;
					probe[slots as usize] = Some(entry);
					continue;
				}
				let mut slot = hash & mask;
				while probe[slot as usize].is_some() {
					slot = (slot + 1) & mask;
				}
				probe[slot as usize] = Some(if own_key {
					entry
				} else {
					let tag = match *hash as u32 {
						0 => 0x8000_0000,
						tag => tag,
					};
					let mut fields = vec![i32.const_int(u64::from(tag), false).into()];
					if slot_type.count_fields() == 3 {
						fields.push(i32.const_zero().into());
					}
					fields.push(entry.into());
					slot_type.const_named_struct(&fields)
				});
			}
			let probe: Vec<_> = probe
				.into_iter()
				.map(|slot| {
					slot.unwrap_or_else(|| {
						if own_key {
							let key = self.llvm_type(table.key).into_int_type().const_all_ones();
							table.entry.const_named_struct(&[
								key.into(),
								self.llvm_type(table.value.expect("an inline map"))
									.const_zero(),
							])
						} else {
							slot_type.const_zero()
						}
					})
				})
				.collect();
			let probe = self.readonly_global(
				slot_type.const_array(&probe).into(),
				"fa.collection.entries",
			);
			[
				null.into(),
				i64.const_int(count, false).into(),
				i64.const_int(total, false).into(),
				i64.const_int(u64::from(sentinel), false).into(),
				probe.into(),
				i64.const_int(mask, false).into(),
				i64.const_zero().into(),
				i64.const_zero().into(),
				null.into(),
				i64.const_zero().into(),
				i64.const_int(TABLE_STATIC, false).into(),
			]
		} else {
			// a slot of the index: the low half of the hash, then the
			// position of the entry + 1, in the first empty slot from the
			// one of the hash
			let mut index = vec![0_u64; slots as usize];
			for (position, (hash, _)) in keys.iter().enumerate() {
				let mut slot = hash & mask;
				while index[slot as usize] != 0 {
					slot = (slot + 1) & mask;
				}
				index[slot as usize] = (hash & 0xffff_ffff) | ((position as u64 + 1) << 32);
			}
			let index: Vec<_> = index
				.into_iter()
				.map(|slot| i64.const_int(slot, false))
				.collect();
			let index = self.readonly_global(i64.const_array(&index).into(), "fa.collection.index");
			// chunks of 32 entries, the last one with free entries; dense
			// entries are one array, and their chunks only say which are live
			let chunks = count.div_ceil(TABLE_CHUNK_SLOTS);
			let capacity = chunks * TABLE_CHUNK_SLOTS;
			let mut entries = entries;
			entries.resize(capacity as usize, table.entry.const_zero());
			let chunk_type = self.table_chunk_type();
			let mut directory = Vec::new();
			for (chunk, entries) in entries.chunks(TABLE_CHUNK_SLOTS as usize).enumerate() {
				let live = (count - chunk as u64 * TABLE_CHUNK_SLOTS).min(TABLE_CHUNK_SLOTS);
				let block = if table.dense {
					null
				} else {
					self.readonly_global(
						table.entry.const_array(entries).into(),
						"fa.collection.entries",
					)
				};
				directory.push(chunk_type.const_named_struct(&[
					block.into(),
					i32.const_int((1 << live) - 1, false).into(),
					i32.const_zero().into(),
				]));
			}
			let (directory, dense) = if chunks == 0 {
				(null, null)
			} else {
				(
					self.readonly_global(
						chunk_type.const_array(&directory).into(),
						"fa.collection.chunks",
					),
					if table.dense {
						self.readonly_global(
							table.entry.const_array(&entries).into(),
							"fa.collection.entries",
						)
					} else {
						null
					},
				)
			};
			let free = if count.is_multiple_of(TABLE_CHUNK_SLOTS) {
				0
			} else {
				chunks
			};
			let room = if table.dense { capacity } else { 0 };
			[
				directory.into(),
				i64.const_int(count, false).into(),
				i64.const_int(count, false).into(),
				i64.const_int(capacity, false).into(),
				index.into(),
				i64.const_int(mask, false).into(),
				i64.const_int(chunks, false).into(),
				i64.const_int(free, false).into(),
				dense.into(),
				i64.const_int(room, false).into(),
				i64.const_int(TABLE_STATIC, false).into(),
			]
		};
		let header = self.table_header().const_named_struct(&header);
		let pointer = self.readonly_global(header.into(), "fa.collection.static");
		Ok(Some(Val::new(repr, pointer)))
	}

	// ── hashing and equality ────────────────────────────────────────────

	/// `fa.hash.<shape>(value) -> i64`: equal keys have equal hashes.
	pub(crate) fn hash_function(&self, repr: Repr) -> Result<FunctionValue<'ctx>, Error> {
		let key = format!("hash {}", self.shape_key(repr));
		if let Some(function) = self.drop_functions.borrow().get(&key) {
			return Ok(*function);
		}
		let i64 = self.context.i64_type();
		let function = self.module.add_function(
			"fa.hash",
			i64.fn_type(
				&[if self.llvm_type(repr).is_struct_type() {
					self.pointer_type().into()
				} else {
					self.llvm_type(repr).into()
				}],
				false,
			),
			Some(Linkage::Internal),
		);
		self.runtime.add_attributes(function, &["nounwind"]);
		// a few instructions for a scalar key: inlined in every probe
		if matches!(
			repr,
			Repr::Int(_)
				| Repr::Bool | Repr::Enum(_)
				| Repr::F64 | Repr::F32
				| Repr::Percentage
				| Repr::Str
		) {
			self.runtime.add_attributes(function, &["alwaysinline"]);
		}
		self.drop_functions.borrow_mut().insert(key, function);
		let builder = self.context.create_builder();
		builder.position_at_end(self.context.append_basic_block(function, "entry"));
		let value = function.get_nth_param(0).expect("a value");
		self.keep_aggregate_helper(function, repr);
		if let Repr::Tuple(ty) = repr
			&& self.inline_info(ty).is_some()
		{
			let hash =
				self.hash_inline_place(&builder, function, ty, value.into_pointer_value())?;
			builder.build_return(Some(&hash))?;
			return Ok(function);
		}
		let value = self.helper_parameter(&builder, repr, value)?;
		let hash = self.build_hash(&builder, function, repr, value)?;
		builder.build_return(Some(&hash))?;
		Ok(function)
	}

	fn hash_inline_place(
		&self,
		builder: &Builder<'ctx>,
		function: FunctionValue<'ctx>,
		ty: TypeId,
		source: PointerValue<'ctx>,
	) -> Result<IntValue<'ctx>, Error> {
		let i64 = self.context.i64_type();
		let (item_type, capacity) = self.inline_info(ty).expect("an inline array");
		if capacity == 0 {
			return Ok(i64.const_zero());
		}
		let slot = Slot {
			pointer: source,
			repr: Repr::Tuple(ty),
		};
		let length_pointer = builder.build_struct_gep(self.inline_type(ty), source, 1, "")?;
		let length = builder
			.build_load(self.inline_length_type(capacity), length_pointer, "")?
			.into_int_value();
		let length = builder.build_int_z_extend(length, i64, "")?;
		let item = self.item_repr(item_type, NodeId::new(0))?;
		let items = builder.build_struct_gep(self.inline_type(ty), slot.pointer, 0, "")?;
		self.hash_items(builder, function, item, items, length)
	}

	fn hash_items(
		&self,
		builder: &Builder<'ctx>,
		function: FunctionValue<'ctx>,
		item: Repr,
		items: PointerValue<'ctx>,
		length: IntValue<'ctx>,
	) -> Result<IntValue<'ctx>, Error> {
		let i64 = self.context.i64_type();
		let accumulator = builder.build_alloca(i64, "hash")?;
		builder.build_store(accumulator, length)?;
		let hash_at = self.hash_at_function(item)?;
		self.helper_loop(builder, function, length, |builder, index| {
			let pointer =
				unsafe { builder.build_in_bounds_gep(self.llvm_type(item), items, &[index], "")? };
			let next = builder
				.build_call(hash_at, &[pointer.into()], "")?
				.try_as_basic_value()
				.basic()
				.expect("a hash")
				.into_int_value();
			let current = builder.build_load(i64, accumulator, "")?.into_int_value();
			let combined = self.combine(builder, current, next)?;
			builder.build_store(accumulator, combined)?;
			Ok(())
		})?;
		Ok(builder.build_load(i64, accumulator, "")?.into_int_value())
	}

	/// `fa.hash_at.<shape>(ptr key) -> i64`, the hash of a key in an entry:
	/// the runtime calls it for the rare rebuilds of the index that need
	/// the hashes of the keys (a set compacting its entries, a wide index).
	pub(crate) fn hash_at_function(&self, repr: Repr) -> Result<FunctionValue<'ctx>, Error> {
		let key = format!("hash_at {}", self.shape_key(repr));
		if let Some(function) = self.drop_functions.borrow().get(&key) {
			return Ok(*function);
		}
		let i64 = self.context.i64_type();
		let function = self.module.add_function(
			"fa.hash_at",
			i64.fn_type(&[self.pointer_type().into()], false),
			Some(Linkage::Internal),
		);
		self.runtime.add_attributes(function, &["nounwind"]);
		self.drop_functions.borrow_mut().insert(key, function);
		let hash = self.hash_function(repr)?;
		let builder = self.context.create_builder();
		builder.position_at_end(self.context.append_basic_block(function, "entry"));
		let pointer = function
			.get_nth_param(0)
			.expect("a key")
			.into_pointer_value();
		let loaded = if self.llvm_type(repr).is_struct_type() {
			pointer.into()
		} else {
			builder.build_load(self.llvm_type(repr), pointer, "")?
		};
		let hashed = builder
			.build_call(hash, &[loaded.into()], "")?
			.try_as_basic_value()
			.basic()
			.expect("a hash");
		builder.build_return(Some(&hashed))?;
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

	/// The hash of an integer key: `x ^ (x >> 32)`, multiplied by an odd
	/// word, then its high half folded into its low half. Each step is a
	/// bijection, and the low 32 bits (the home slot and the tag of the
	/// index) depend on every bit of the key; half the cost of `mix`.
	fn mix_integer(
		&self,
		builder: &Builder<'ctx>,
		value: IntValue<'ctx>,
	) -> Result<IntValue<'ctx>, Error> {
		let i64 = self.context.i64_type();
		let shift = i64.const_int(32, false);
		let shifted = builder.build_right_shift(value, shift, false, "")?;
		let folded = builder.build_xor(value, shifted, "")?;
		let multiplier = i64.const_int(0x9E3779B97F4A7C15, false);
		let product = builder.build_int_mul(folded, multiplier, "")?;
		let shifted = builder.build_right_shift(product, shift, false, "")?;
		Ok(builder.build_xor(product, shifted, "")?)
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
				let value = self.helper_argument(builder, value)?;
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
				} else if int.bits <= 32 {
					// no high half to fold: its bits, multiplied, then the
					// high half of the product folded into the low half
					let word = builder.build_int_z_extend_or_bit_cast(raw, i64, "")?;
					let product = builder.build_int_mul(
						word,
						i64.const_int(0x9E3779B97F4A7C15, false),
						"",
					)?;
					if int.bits == 32 {
						// Multiplication and rotation are bijective: only key zero
						// has hash zero. Probe slots use middle product bits so
						// nearby small keys don't share their low-bit patterns.
						let rotate = self.intrinsic("llvm.fshr", &[i64.into()])?;
						return Ok(builder
							.build_call(
								rotate,
								&[
									product.into(),
									product.into(),
									i64.const_int(24, false).into(),
								],
								"",
							)?
							.try_as_basic_value()
							.basic()
							.expect("a hash")
							.into_int_value());
					}
					let shifted =
						builder.build_right_shift(product, i64.const_int(32, false), false, "")?;
					return Ok(builder.build_xor(product, shifted, "")?);
				} else if int.signed {
					builder.build_int_s_extend_or_bit_cast(raw, i64, "")?
				} else {
					builder.build_int_z_extend_or_bit_cast(raw, i64, "")?
				};
				self.mix_integer(builder, word)?
			}
			Repr::Bool | Repr::Enum(_) => {
				let word = builder.build_int_z_extend(value.into_int_value(), i64, "")?;
				self.mix_integer(builder, word)?
			}
			// the canonical form: an inline word, or the limbs of a block
			Repr::BigInt => {
				let word = builder
					.build_call(
						self.runtime.function("fa_bigint_hash")?,
						&[value.into()],
						"",
					)?
					.try_as_basic_value()
					.basic()
					.expect("a hash")
					.into_int_value();
				self.mix_integer(builder, word)?
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
			Repr::Array(item_type) => {
				let item = self.item_repr(item_type, NodeId::new(0))?;
				let array = value.into_pointer_value();
				if item == Repr::Str {
					builder.build_call(
						self.runtime.function("fa_array_materialize")?,
						&[array.into()],
						"",
					)?;
				}
				let header = self.header_type();
				let items_pointer = builder.build_struct_gep(header, array, 0, "")?;
				let items = builder
					.build_load(self.pointer_type(), items_pointer, "")?
					.into_pointer_value();
				let length_pointer = builder.build_struct_gep(header, array, 1, "")?;
				let length = builder
					.build_load(i64, length_pointer, "")?
					.into_int_value();
				self.hash_items(builder, function, item, items, length)?
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
				let id = self.union_id(builder, pointer)?;
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
				let address = self.edge_address(builder, Repr::Rec(target), value)?;
				let loaded = builder.build_load(self.llvm_type(target_repr), address, "")?;
				call_hash(target_repr, loaded)?
			}
			Repr::Bytes => builder
				.build_call(self.runtime.function("fa_bytes_hash")?, &[value.into()], "")?
				.try_as_basic_value()
				.basic()
				.expect("Bytes hash")
				.into_int_value(),
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

	pub(super) fn nested_equals_pair_function(
		&mut self,
		left: Repr,
		right: Repr,
	) -> Result<FunctionValue<'ctx>, Error> {
		self.comparison_pair_function(left, right, self.frame().same_value_zero)
	}

	fn comparison_function(
		&mut self,
		repr: Repr,
		same_value_zero: bool,
	) -> Result<FunctionValue<'ctx>, Error> {
		self.comparison_pair_function(repr, repr, same_value_zero)
	}

	fn comparison_pair_function(
		&mut self,
		repr: Repr,
		other: Repr,
		same_value_zero: bool,
	) -> Result<FunctionValue<'ctx>, Error> {
		let key = format!(
			"equals {same_value_zero} {} {}",
			self.shape_key(repr),
			self.shape_key(other)
		);
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
		self.keep_aggregate_helper(function, repr);
		self.drop_functions.borrow_mut().insert(key, function);
		// lowered as a function of its own
		let saved_frame = self.frame.take();
		let saved_block = self.builder.get_insert_block();
		self.begin_function(function, Repr::Void, None);
		self.frame_mut().same_value_zero = same_value_zero;
		let a = function.get_nth_param(0).expect("a").into_pointer_value();
		let b = function.get_nth_param(1).expect("b").into_pointer_value();
		let node = NodeId::new(0);
		let equal = if let Repr::Tuple(ty) = repr
			&& repr == other
			&& let Some((item, _)) = self.inline_info(ty)
		{
			let a = self.inline_view_at(Slot { pointer: a, repr }, ty)?;
			let b = self.inline_view_at(Slot { pointer: b, repr }, ty)?;
			self.arrays_equal(a, b, item, node)?
		} else {
			let a = self.load(Slot { pointer: a, repr })?;
			let b = self.load(Slot {
				pointer: b,
				repr: other,
			})?;
			self.values_equal_any(a, b, node)?
		};
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
		let new = self.runtime.function("fa_table_new")?;
		let pointer = self
			.call_value(new, &[capacity.into(), self.entry_size(table).into()])?
			.into_pointer_value();
		if !table.inline && capacity.get_zero_extended_constant() != Some(0) {
			// the entries too, up front (an inline table has them in its
			// probe array)
			let hash_at = self.hash_at_function(table.key)?;
			self.call_runtime(
				"fa_table_reserve",
				&[
					pointer.into(),
					capacity.into(),
					self.entry_size(table).into(),
					hash_at.as_global_value().as_pointer_value().into(),
				],
			)?;
		}
		Ok(pointer)
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
		let entry = self.runtime.function("fa_table_entry")?;
		Ok(self
			.call_value(
				entry,
				&[
					pointer.into(),
					position.into(),
					self.entry_size(table).into(),
				],
			)?
			.into_pointer_value())
	}

	/// Whether the entry at a position (below `used`) is live, an `i1`.
	pub(crate) fn entry_live(
		&self,
		pointer: PointerValue<'ctx>,
		table: Table<'ctx>,
		position: IntValue<'ctx>,
	) -> Result<IntValue<'ctx>, Error> {
		let live = self
			.call_value(
				self.runtime.function("fa_table_live")?,
				&[
					pointer.into(),
					position.into(),
					self.entry_size(table).into(),
				],
			)?
			.into_int_value();
		Ok(self.builder.build_int_compare(
			IntPredicate::NE,
			live,
			live.get_type().const_zero(),
			"",
		)?)
	}

	/// The slot of the key (0) or the value (1) of an entry.
	pub(crate) fn entry_slot(
		&self,
		entry: PointerValue<'ctx>,
		table: Table<'ctx>,
		value: bool,
	) -> Result<Slot<'ctx>, Error> {
		let (index, repr) = match (value, table.value) {
			(true, Some(repr)) => (1, repr),
			_ => (0, table.key),
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
		let argument = self.helper_argument(&self.builder, key.value.expect("a key"))?;
		Ok(self
			.call_value(function, &[argument.into()])?
			.into_int_value())
	}

	/// The number of bytes compared bitwise for keys whose `==` is their bits
	/// (whole-byte integers), or 0 for the others (compared by `fa.key_equals`).
	pub(super) fn bitwise_key_size(table: Table<'ctx>) -> u64 {
		match table.key {
			Repr::Int(int) if matches!(int.bits, 8 | 16 | 32 | 64 | 128) => u64::from(int.bits / 8),
			_ => 0,
		}
	}

	/// The position of the entry of a key (converted to the table's key), or
	/// on a miss the complement of the index slot where it goes (negative);
	/// and its hash. No table mutation may occur before an insertion reuses
	/// the slot; ownership copies do not call user code.
	fn table_find(
		&mut self,
		pointer: PointerValue<'ctx>,
		table: Table<'ctx>,
		key: Val<'ctx>,
		node: NodeId,
	) -> Result<(IntValue<'ctx>, IntValue<'ctx>), Error> {
		self.table_lookup("fa_table_find", pointer, table, key, node)
	}

	/// A lookup by the runtime function `name`: `fa_table_find`, or
	/// `fa_table_remove`, which also takes a found key out of the index (its
	/// entry is then released by `table_remove`).
	fn table_lookup(
		&mut self,
		name: &str,
		pointer: PointerValue<'ctx>,
		table: Table<'ctx>,
		key: Val<'ctx>,
		node: NodeId,
	) -> Result<(IntValue<'ctx>, IntValue<'ctx>), Error> {
		let key = self.convert(key, table.key, node.index())?;
		// Inline removal reads the entry before release shifts the index, so
		// it can consume the same guarded lookup as insertion/replacement.
		if (name == "fa_table_find"
			|| name == "fa_table_remove" && self.map_probe_inline_delete_supported(table))
			&& let Some(result) = self.reuse_map_probe(pointer, table, key, node)?
		{
			return Ok(result);
		}
		self.table_lookup_uncached(name, pointer, table, key, node)
	}

	pub(super) fn table_lookup_uncached(
		&mut self,
		name: &str,
		pointer: PointerValue<'ctx>,
		table: Table<'ctx>,
		key: Val<'ctx>,
		_node: NodeId,
	) -> Result<(IntValue<'ctx>, IntValue<'ctx>), Error> {
		let hash = self.hash_of(key)?;
		// A lookup inspects its needle in its original representation. The
		// callback reads stored keys using the table's layout, independently.
		let slot = self.alloca(key.repr, "key")?;
		self.builder
			.build_store(slot.pointer, key.value.expect("a key"))?;
		// integers are compared bitwise in the probe loop, strings by the
		// runtime (`STRING_KEY`: in place, three words when both are inline),
		// the other keys with their `==`
		let bitwise = if key.repr != table.key {
			0
		} else {
			match table.key {
				Repr::Str => u64::MAX,
				_ => Self::bitwise_key_size(table),
			}
		};
		let equals = if bitwise > 0 {
			self.pointer_type().const_null()
		} else {
			self.comparison_pair_function(table.key, key.repr, true)?
				.as_global_value()
				.as_pointer_value()
		};
		let i64 = self.context.i64_type();
		let find = self.runtime.function(name)?;
		let arguments = [
			pointer.into(),
			hash.into(),
			slot.pointer.into(),
			equals.into(),
			self.entry_size(table).into(),
			i64.const_int(bitwise, false).into(),
		];
		let position =
			if name == "fa_table_remove" && self.map_probe_inline_delete_supported(table) {
				// Inline removal only finds the entry; release below updates a
				// cached miss after the backward shift makes its final hole.
				self.builder
					.build_call(find, &arguments, "")?
					.try_as_basic_value()
					.basic()
					.expect("a position")
			} else {
				self.call_value(find, &arguments)?
			}
			.into_int_value();
		Ok((position, hash))
	}

	/// The un-hashed key for inline sentinel slots, shared by inserts and
	/// converting copies. Other layouts ignore this argument.
	pub(super) fn table_key_bits(
		&self,
		builder: &Builder<'ctx>,
		table: Table<'ctx>,
		key: Val<'ctx>,
	) -> Result<IntValue<'ctx>, Error> {
		let i64 = self.context.i64_type();
		Ok(
			if table.inline && matches!(table.key, Repr::Int(int) if matches!(int.bits, 32 | 64)) {
				builder.build_int_z_extend(key.int(), i64, "")?
			} else {
				i64.const_zero()
			},
		)
	}

	/// A new entry for a key that is absent (`probe`: the result of its
	/// lookup), its key and value written.
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
		let i64 = self.context.i64_type();
		let key_bits = self.table_key_bits(&self.builder, table, key)?;
		let hash_at = self.hash_at_function(table.key)?;
		let entry = self
			.call_value(
				append,
				&[
					pointer.into(),
					hash.into(),
					self.entry_size(table).into(),
					probe.unwrap_or_else(|| i64.const_zero()).into(),
					key_bits.into(),
					hash_at.as_global_value().as_pointer_value().into(),
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
		let (position, hash) = self.table_find(pointer, table, key, node)?;
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
		let (position, hash) = self.table_find(pointer, table, key, node)?;
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

	/// Drops the key and value of a live entry, and kills it: `indexed` when
	/// its key is still in the index (else `fa_table_remove` took it out).
	fn table_remove(
		&mut self,
		pointer: PointerValue<'ctx>,
		table: Table<'ctx>,
		position: IntValue<'ctx>,
		indexed: bool,
		keep_value: bool,
	) -> Result<Option<Val<'ctx>>, Error> {
		let entry = self.entry_at(pointer, table, position)?;
		let key_slot = self.entry_slot(entry, table, false)?;
		let key = self.load(key_slot)?;
		// the slot of the entry in the index is found from the hash
		let hash = if indexed {
			Some(self.hash_of(key)?)
		} else {
			None
		};
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
		if self.release_inline_map_with_probe(pointer, table, position)? {
			return Ok(kept);
		}
		self.invalidate_all_map_probes()?;
		match hash {
			Some(hash) => self.builder.build_call(
				self.runtime.function("fa_table_kill")?,
				&[
					pointer.into(),
					hash.into(),
					position.into(),
					self.entry_size(table).into(),
				],
				"",
			)?,
			None => self.builder.build_call(
				self.runtime.function("fa_table_release")?,
				&[
					pointer.into(),
					position.into(),
					self.entry_size(table).into(),
				],
				"",
			)?,
		};
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
		let live = self.entry_live(pointer, table, position)?;
		self.builder.build_conditional_branch(live, block, step)?;
		self.builder.position_at_end(block);
		let entry = self.entry_at(pointer, table, position)?;
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
		let bytes = self.table_local_bytes(table, items.len() as u64);
		let local =
			|this: &mut Self, buffer: PointerValue<'ctx>| -> Result<PointerValue<'ctx>, Error> {
				Ok(this
					.call_value(
						this.runtime.function("fa_table_local")?,
						&[
							buffer.into(),
							capacity.into(),
							this.entry_size(table).into(),
						],
					)?
					.into_pointer_value())
			};
		let pointer = if let Some(offer) = self.forwarded_collection_buffer(node) {
			self.select_collection_buffer(offer, bytes, local, |this| {
				this.table_new(table, capacity)
			})?
		} else if let Some(buffer) = self.collection_stack(node, bytes)? {
			local(self, buffer)?
		} else {
			self.table_new_at(node, table, capacity)?
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
		Ok(result)
	}

	/// Transfer both parts of a dying tuple; borrowed construction inputs
	/// retain their existing item-copy checks.
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
		let transfer = self.is_temporary(pair) || self.movable_local(node, pair.repr).is_some();
		let pair = if transfer {
			self.take(pair, Some(node))?
		} else {
			pair
		};
		let structure = pair.value.expect("a tuple").into_struct_value();
		let key = self.builder.build_extract_value(structure, 0, "")?;
		let value = self.builder.build_extract_value(structure, 1, "")?;
		let key = self.convert(Val::new(reprs[0], key), table.key, node.index())?;
		let value = self.convert(
			Val::new(reprs[1], value),
			table.value.expect("a map"),
			node.index(),
		)?;
		if transfer {
			Ok((key, value))
		} else {
			Ok((self.emit_copy(key)?, self.emit_copy(value)?))
		}
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

	/// `use value = map.get(key)`: the slot of the value in its entry, a
	/// null pointer when the key is absent (a nullable reference, read as an
	/// optional; narrowed, it is the value's place).
	pub(crate) fn map_get_reference(&mut self, node: NodeId) -> Result<Option<Slot<'ctx>>, Error> {
		let Some(info) = self.analysis.call(node).cloned() else {
			return Ok(None);
		};
		let Callee::Builtin(id) = info.callee else {
			return Ok(None);
		};
		if !matches!(
			fa_analyzer::builtins::get(id).path,
			"Map#get" | "GrowingMap#get"
		) {
			return Ok(None);
		}
		let map = self.receiver(node)?;
		let Repr::Map(_) = map.repr else {
			return Ok(None);
		};
		let given: Vec<NodeId> = info
			.arguments
			.iter()
			.filter_map(|argument| match argument {
				Argument::Given(value) => Some(*value),
				_ => None,
			})
			.collect();
		let [key_node] = given.as_slice() else {
			return Err(self.unsupported(node.index(), "this `get`"));
		};
		let key = self.expr(*key_node)?;
		let table = self.table_of(map.repr)?;
		let Some(repr) = table.value else {
			return Ok(None);
		};
		let pointer = map.value.expect("a map").into_pointer_value();
		let (position, _) = self.table_find(pointer, table, key, node)?;
		let result = self.alloca_type(self.pointer_type().into(), "reference")?;
		self.builder
			.build_store(result, self.pointer_type().const_null())?;
		let found = self.builder.build_int_compare(
			IntPredicate::SGE,
			position,
			position.get_type().const_zero(),
			"",
		)?;
		let present = self.append_block("reference_present");
		let end = self.append_block("referenced");
		self.builder.build_conditional_branch(found, present, end)?;
		self.builder.position_at_end(present);
		let entry = self.entry_at(pointer, table, position)?;
		let slot = self.entry_slot(entry, table, true)?;
		self.builder.build_store(result, slot.pointer)?;
		self.builder.build_unconditional_branch(end)?;
		self.builder.position_at_end(end);
		let pointer = self
			.builder
			.build_load(self.pointer_type(), result, "")?
			.into_pointer_value();
		Ok(Some(Slot { pointer, repr }))
	}

	/// Appends `[1, 2]` (a set) or `[("a", 1)]` (a map).
	pub(crate) fn append_table(&mut self, value: Val<'ctx>, node: NodeId) -> Result<(), Error> {
		let table = self.table_of(value.repr)?;
		// the types of the keys and values: a set of flags prints its flags (fields.rs)
		let (key_type, value_type) = match value.repr {
			Repr::Set(ty) | Repr::Map(ty) => match self.types().kind(ty) {
				fa_analyzer::types::TypeKind::Collection(_, item, key)
					if matches!(value.repr, Repr::Map(_)) =>
				{
					(key, item)
				}
				fa_analyzer::types::TypeKind::Collection(_, item, _) => (item, TypeId::UNKNOWN),
				_ => (TypeId::UNKNOWN, TypeId::UNKNOWN),
			},
			_ => (TypeId::UNKNOWN, TypeId::UNKNOWN),
		};
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
				None => this.append_nested_of(key, key_type, node),
				Some(_) => {
					let open = this.string_constant("(");
					this.append_value(open, node)?;
					this.append_nested_of(key, key_type, node)?;
					let comma = this.string_constant(", ");
					this.append_value(comma, node)?;
					let value = this.load(this.entry_slot(entry, table, true)?)?;
					this.append_nested_of(value, value_type, node)?;
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
		let other_table = self.table_of(b.repr)?;
		let first = a.value.expect("a table").into_pointer_value();
		let second = b.value.expect("a table").into_pointer_value();
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
			let (position, _) =
				this.table_lookup_uncached("fa_table_find", second, other_table, key, node)?;
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
					let other_entry = this.entry_at(second, other_table, position)?;
					let other = this.entry_slot(other_entry, other_table, true)?;
					let compare = if repr == other.repr {
						this.nested_equals_function(repr)?
					} else {
						this.nested_equals_pair_function(repr, other.repr)?
					};
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
		let live = self.entry_live(pointer, table, position)?;
		self.builder.build_conditional_branch(live, block, next)?;
		self.builder.position_at_end(block);
		let entry = self.entry_at(pointer, table, position)?;
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
		mut keep: impl FnMut(&mut Self, PointerValue<'ctx>, Val<'ctx>) -> Result<IntValue<'ctx>, Error>,
		node: NodeId,
	) -> Result<Val<'ctx>, Error> {
		let zero = self.context.i64_type().const_zero();
		let result = self.table_new(table, zero)?;
		self.for_each_entry(source, table, |this, entry, _| {
			let holds = keep(this, entry, Val::new(repr, result))?;
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
	/// Find the first live entry, retaining its address for either data reads
	/// or an explicitly borrowed set result.
	fn find_table_entry(
		&mut self,
		pointer: PointerValue<'ctx>,
		table: Table<'ctx>,
		callback: Val<'ctx>,
		callback_node: NodeId,
	) -> Result<PointerValue<'ctx>, Error> {
		let found = self.alloca_type(self.pointer_type().into(), "found_entry")?;
		self.builder
			.build_store(found, self.pointer_type().const_null())?;
		self.for_each_entry(pointer, table, |this, entry, _| {
			let previous = this
				.builder
				.build_load(this.pointer_type(), found, "")?
				.into_pointer_value();
			let search = this.append_block("find_entry");
			let skip = this.append_block("found_entry");
			let missing = this.builder.build_is_null(previous, "")?;
			this.builder
				.build_conditional_branch(missing, search, skip)?;
			this.builder.position_at_end(search);
			let holds = this.entry_callback(callback, callback_node, entry, table, &[])?;
			let take = this.append_block("find_take");
			this.builder
				.build_conditional_branch(holds.int(), take, skip)?;
			this.builder.position_at_end(take);
			this.builder.build_store(found, entry)?;
			this.builder.build_unconditional_branch(skip)?;
			this.builder.position_at_end(skip);
			Ok(())
		})?;
		Ok(self
			.builder
			.build_load(self.pointer_type(), found, "")?
			.into_pointer_value())
	}

	pub(super) fn set_reference(
		&mut self,
		node: NodeId,
	) -> Result<Option<(Slot<'ctx>, bool)>, Error> {
		let Some(info) = self.analysis.call(node).cloned() else {
			return Ok(None);
		};
		let Callee::Builtin(id) = info.callee else {
			return Ok(None);
		};
		if fa_analyzer::builtins::get(id).path != "Set#find" {
			return Ok(None);
		}
		let receiver = self.receiver(node)?;
		let table = self.table_of(receiver.repr)?;
		let [Argument::Given(callback_node)] = info.arguments.as_slice() else {
			return Err(self.unsupported(node.index(), "this set predicate"));
		};
		let callback = self.expr(*callback_node)?;
		let entry = self.find_table_entry(
			receiver.value.expect("set").into_pointer_value(),
			table,
			callback,
			*callback_node,
		)?;
		let result = self.alloca_type(self.pointer_type().into(), "set_reference")?;
		self.builder
			.build_store(result, self.pointer_type().const_null())?;
		let present = self.builder.build_is_not_null(entry, "")?;
		let take = self.append_block("set_reference_found");
		let end = self.append_block("set_reference_end");
		self.builder.build_conditional_branch(present, take, end)?;
		self.builder.position_at_end(take);
		let key = self.entry_slot(entry, table, false)?;
		self.builder.build_store(result, key.pointer)?;
		self.builder.build_unconditional_branch(end)?;
		self.builder.position_at_end(end);
		let pointer = self
			.builder
			.build_load(self.pointer_type(), result, "")?
			.into_pointer_value();
		Ok(Some((
			Slot {
				pointer,
				repr: table.key,
			},
			true,
		)))
	}

	pub(crate) fn table_method(
		&mut self,
		node: NodeId,
		path: &str,
		info: &CallInfo,
	) -> Result<Val<'ctx>, Error> {
		if matches!(
			path,
			"Set.from" | "Map.from" | "GrowingSet.from" | "GrowingMap.from"
		) {
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
				let (position, _) =
					self.table_lookup("fa_table_remove", pointer, table, *key, node)?;
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
				let kept = self.table_remove(pointer, table, position, false, result.is_some())?;
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
				self.call_runtime(
					"fa_table_clear",
					&[pointer.into(), self.entry_size(table).into()],
				)?;
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
					this.table_remove(pointer, table, position, true, false)?;
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
				let repr = self.collection_optional_repr(self.node_repr(node)?, table.key);
				let result = self.alloca(repr, "found")?;
				self.builder
					.build_store(result.pointer, self.llvm_type(repr).const_zero())?;
				let entry = self.find_table_entry(pointer, table, *callback, nodes[0])?;
				let present = self.builder.build_is_not_null(entry, "")?;
				let take = self.append_block("find_present");
				let end = self.append_block("find_end");
				self.builder.build_conditional_branch(present, take, end)?;
				self.builder.position_at_end(take);
				let key = self.entry_slot(entry, table, false)?;
				let found = match (table.value, repr) {
					(Some(_), Repr::Optional(Scalar::Tuple(tuple))) => {
						let fields = self.tuple_reprs(tuple);
						let key = self.collection_item_view(key, fields[0], node.index())?;
						let value = self.entry_slot(entry, table, true)?;
						let value = self.collection_item_view(value, fields[1], node.index())?;
						let structure = self.tuple_type(tuple);
						let pair = self.builder.build_insert_value(
							structure.get_undef(),
							key.value.expect("a key"),
							0,
							"",
						)?;
						let pair = self.builder.build_insert_value(
							pair,
							value.value.expect("a value"),
							1,
							"",
						)?;
						Val::new(Repr::Tuple(tuple), pair.into_struct_value())
					}
					_ => self.load(key)?,
				};
				let found = self.convert(found, repr, node.index())?;
				self.builder
					.build_store(result.pointer, found.value.expect("a value"))?;
				self.builder.build_unconditional_branch(end)?;
				self.builder.position_at_end(end);
				self.load(result)
			}
			("filter", [callback]) => {
				let callback_node = nodes[0];
				let repr = receiver.repr;
				self.table_filtered(
					pointer,
					table,
					repr,
					|this, entry, built| {
						// an error: the entries kept so far are dropped
						let holds = this.callback_with_fixup(
							callback_node,
							|this| this.entry_callback(*callback, callback_node, entry, table, &[]),
							|this| this.emit_drop(built),
						)?;
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
							let mapped = this.callback_with_fixup(
								callback_node,
								|this| {
									this.entry_callback(*callback, callback_node, entry, table, &[])
								},
								|this| this.emit_drop(array_value),
							)?;
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
							let mapped = this.callback_with_fixup(
								callback_node,
								|this| {
									this.entry_callback(*callback, callback_node, entry, table, &[])
								},
								|this| this.emit_drop(Val::new(result_repr, result)),
							)?;
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
					let next = this.callback_with_fixup(
						callback_node,
						|this| {
							this.entry_callback(*callback, callback_node, entry, table, &[current])
						},
						|this| this.emit_drop(current),
					)?;
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
					|this, entry, _| {
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
		// `capacity = n`: the table is sized for it (`Map.from(source,
		// capacity)`, `Set.from(source, key, capacity)`)
		let capacity_position = if matches!(repr, Repr::Set(_)) { 2 } else { 1 };
		let capacity = match info.arguments.get(capacity_position) {
			Some(Argument::Given(capacity_node)) => {
				let capacity = self.expr(*capacity_node)?;
				if capacity.repr == Repr::Never {
					return Ok(capacity);
				}
				self.convert(capacity, Repr::Int(IntRepr::I64), capacity_node.index())?
					.int()
			}
			_ => self.context.i64_type().const_zero(),
		};
		let result = self.table_new_at(node, table, capacity)?;
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
		let owning =
			self.needs_drop(table.key) || table.value.is_some_and(|value| self.needs_drop(value));
		if owning {
			// the keys and values of read-only data own nothing
			let storage = builder.build_struct_gep(self.table_header(), pointer, 10, "")?;
			let storage = builder
				.build_load(self.context.i64_type(), storage, "")?
				.into_int_value();
			let constant = builder.build_int_compare(
				IntPredicate::EQ,
				storage,
				storage.get_type().const_int(TABLE_STATIC, false),
				"",
			)?;
			let owned = self.context.append_basic_block(function, "owned");
			builder.build_conditional_branch(constant, none, owned)?;
			builder.position_at_end(owned);
			self.helper_entries(builder, function, pointer, table, |builder, entry| {
				for (index, repr) in [(0, Some(table.key)), (1, table.value)] {
					let Some(repr) = repr else { continue };
					if self.needs_drop(repr) {
						let field = builder.build_struct_gep(table.entry, entry, index, "")?;
						let loaded = builder.build_load(self.llvm_type(repr), field, "")?;
						self.call_drop(builder, repr, loaded)?;
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
				for (index, repr) in [(0, Some(table.key)), (1, table.value)] {
					let Some(repr) = repr else { continue };
					if self.needs_drop(repr) {
						let field = builder.build_struct_gep(table.entry, entry, index, "")?;
						let loaded = builder.build_load(self.llvm_type(repr), field, "")?;
						let copied = self.call_copy(builder, repr, loaded)?;
						builder.build_store(field, copied)?;
					}
				}
				Ok(())
			})?;
		}
		Ok(copy.into())
	}

	/// A loop over the live entries of a table in a helper.
	pub(super) fn helper_entries(
		&self,
		builder: &Builder<'ctx>,
		function: FunctionValue<'ctx>,
		pointer: PointerValue<'ctx>,
		table: Table<'ctx>,
		mut body: impl FnMut(&Builder<'ctx>, PointerValue<'ctx>) -> Result<(), Error>,
	) -> Result<(), Error> {
		let i64 = self.context.i64_type();
		let header = self.table_header();
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
		let live = builder
			.build_call(
				self.runtime.function("fa_table_live")?,
				&[
					pointer.into(),
					current.into(),
					self.entry_size(table).into(),
				],
				"",
			)?
			.try_as_basic_value()
			.basic()
			.expect("a bit")
			.into_int_value();
		let live =
			builder.build_int_compare(IntPredicate::NE, live, live.get_type().const_zero(), "")?;
		builder.build_conditional_branch(live, live_block, step)?;
		builder.position_at_end(live_block);
		let entry = builder
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
			.into_pointer_value();
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
