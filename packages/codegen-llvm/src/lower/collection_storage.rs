//! Immutable collection storage. Stack allocations have compile-time sizes
//! and live in the entry block, including constructions inside loops.

use fa_analyzer::ids::{NodeId, SymbolId};
use fa_analyzer::symbols::{Resolution, SymbolKind};
use fa_analyzer::syntax::View;
use inkwell::module::Linkage;
use inkwell::types::BasicTypeEnum;
use inkwell::values::{BasicValue, BasicValueEnum, IntValue, PointerValue};

use crate::error::Error;
use crate::lower::Lowering;
use crate::lower::values::{Repr, Val};

const COLLECTION_BUDGET: u64 = 2048;
const FRAME_BUDGET: u64 = 4096;

#[derive(Clone, Copy)]
pub(super) struct CollectionBuffer<'ctx> {
	pub pointer: PointerValue<'ctx>,
	pub bytes: IntValue<'ctx>,
}

impl<'ctx> Lowering<'_, 'ctx> {
	/// A closed proof: every read borrows the collection as an indexed
	/// receiver or iterable. Owning stores, returns and unknown uses choose
	/// heap storage before construction, without introducing a copy-out.
	fn collection_borrowed_only(&self, symbol: SymbolId, returning: bool) -> bool {
		let parents = self.parents();
		self.analysis
			.resolutions
			.iter()
			.enumerate()
			.all(|(index, resolution)| {
				if *resolution != Resolution::Local(symbol) {
					return true;
				}
				let mut at = NodeId::new(index);
				if returning && self.collection_value_returned(at) {
					return true;
				}
				loop {
					let Some(&parent) = parents.get(at.index()) else {
						return false;
					};
					if parent == usize::MAX {
						return false;
					}
					let parent = NodeId::new(parent);
					match self.view(parent) {
						View::Group(_) => at = parent,
						View::Index { target, .. } => return target == at,
						View::Access { target, .. } if target == at => {
							if matches!(self.node_repr(parent), Ok(Repr::Int(_) | Repr::Bool)) {
								return true;
							}
							let Some(&call) = parents.get(parent.index()) else {
								return false;
							};
							if call == usize::MAX {
								return false;
							}
							let call = NodeId::new(call);
							// A deferred iterator or a bound method can carry the
							// receiver beyond this use. Unknown callees stay on heap.
							return !self.is_iterator(call)
								&& self.analysis.call(call).is_some_and(|info| {
									matches!(info.callee, fa_analyzer::analysis::Callee::Builtin(_))
								});
						}
						View::For { iterable, .. } => return iterable == at,
						_ => return false,
					}
				}
			})
	}
	/// Only the initializer of an immutable owner can borrow frame storage.
	pub(crate) fn collection_binding(&self, node: NodeId) -> Option<SymbolId> {
		let parents = self.parents();
		let mut at = node;
		loop {
			let parent = *parents.get(at.index())?;
			if parent == usize::MAX {
				return None;
			}
			let parent = NodeId::new(parent);
			match self.view(parent) {
				View::Group(_) | View::Try(_) => at = parent,
				View::Declaration {
					value: Some(value), ..
				} if value == at => {
					let symbol = self.analysis.symbols.symbol_of_decl(parent)?;
					return (self.analysis.symbols.symbol(symbol).kind == SymbolKind::Let)
						.then_some(symbol);
				}
				_ => return None,
			}
		}
	}

	pub(crate) fn collection_data_size(&self, ty: BasicTypeEnum<'ctx>) -> u64 {
		let data = inkwell::targets::TargetData::create(
			self.module
				.get_data_layout()
				.as_str()
				.to_str()
				.unwrap_or(""),
		);
		data.get_abi_size(&ty)
	}

	/// Fixed-size space; never an alloca whose size depends on user input.
	pub(crate) fn collection_stack(
		&mut self,
		node: NodeId,
		bytes: u64,
	) -> Result<Option<PointerValue<'ctx>>, Error> {
		let bytes = (bytes + 15) & !15;
		let Some(symbol) = self.collection_binding(node) else {
			return Ok(None);
		};
		let data = self.analysis.symbols.symbol(symbol);
		if self.owner_of_scope(data.scope) != self.frame().owner
			|| self.frame().owner.is_none()
			|| self.tasks.asynchronous
			|| self
				.frame()
				.owner
				.and_then(|owner| self.functions.get(&owner))
				.and_then(|function| function.info.sig)
				.is_some_and(|sig| self.types().sig(sig).is_async)
			|| !self.analysis.foreign_args.is_empty()
			|| !self.collection_borrowed_only(symbol, false)
			|| self
				.analysis
				.functions
				.values()
				.any(|f| f.captures.contains(&symbol))
			|| bytes > COLLECTION_BUDGET
			|| self.frame().collection_bytes + bytes > FRAME_BUDGET
		{
			return Ok(None);
		}
		let storage = self.alloca_type(
			self.context.i8_type().array_type(bytes as u32).into(),
			"collection.storage",
		)?;
		storage
			.as_instruction_value()
			.expect("an alloca")
			.set_alignment(16)
			.map_err(|error| Error::Llvm(format!("{error:?}")))?;
		self.frame_mut().collection_bytes += bytes;
		let key = (self.file, symbol);
		self.frame_mut().collection_locals.insert(key);
		Ok(Some(storage))
	}

	pub(super) fn uses_collection_return(&self, repr: Repr) -> bool {
		!self.tasks.asynchronous
			&& matches!(
				repr,
				Repr::Array(_)
					| Repr::Growing(_)
					| Repr::Map(_) | Repr::Set(_)
					| Repr::Pool(_) | Repr::Columnar(_)
			)
	}

	/// Only value-preserving tails reach the return offer. Arguments, fields,
	/// nested collections and closures cannot accidentally consume it.
	fn collection_value_returned(&self, mut node: NodeId) -> bool {
		let Some(root) = self.frame().collection_result_root else {
			return false;
		};
		let parents = self.parents();
		let mut explicit = false;
		loop {
			if node == root {
				return true;
			}
			let Some(&parent) = parents.get(node.index()) else {
				return false;
			};
			if parent == usize::MAX {
				return false;
			}
			let parent = NodeId::new(parent);
			match self.view(parent) {
				View::Lambda { .. } | View::Function { .. } => return false,
				View::Return(Some(value)) if value == node => explicit = true,
				_ if explicit => {}
				View::Group(_) | View::Try(_) | View::Do(_) => {}
				View::Block { statements } if statements.iter().last() == Some(node) => {}
				View::If {
					then_body,
					else_body,
					..
				} if then_body == node || else_body == Some(node) => {}
				View::When { arms, .. } if arms.iter().any(|arm| arm.value == node) => {}
				_ => return false,
			}
			node = parent;
		}
	}

	pub(super) fn collection_returned_binding(&mut self, symbol: SymbolId) -> bool {
		let data = self.analysis.symbols.symbol(symbol);
		if data.kind != SymbolKind::Let
			|| !self.analysis.foreign_args.is_empty()
			|| self.owner_of_scope(data.scope) != self.frame().owner
			|| self
				.analysis
				.functions
				.values()
				.any(|f| f.captures.contains(&symbol))
			|| !self
				.repr_of(data.ty, 0)
				.is_ok_and(|repr| repr == self.frame().ret)
		{
			return false;
		}
		self.analysis.resolutions.iter().enumerate().any(|(i, r)| {
			*r == Resolution::Local(symbol) && self.collection_value_returned(NodeId::new(i))
		}) && self.collection_borrowed_only(symbol, true)
			&& self.analysis.resolutions.iter().enumerate().all(|(i, r)| {
				let node = NodeId::new(i);
				*r != Resolution::Local(symbol)
					|| !self.collection_value_returned(node)
					|| self
						.analysis
						.flags_of(node)
						.contains(fa_analyzer::analysis::NodeFlags::MOVE)
					|| self.is_returned_local(symbol, node, self.frame().ret)
					|| self.is_last_use(
						symbol,
						node,
						self.analysis.symbols.symbol(symbol).decl_node,
					)
			})
	}

	pub(super) fn forwarded_collection_buffer(
		&mut self,
		node: NodeId,
	) -> Option<CollectionBuffer<'ctx>> {
		let offer = self.frame().return_buffer?;
		if !self
			.node_repr(node)
			.is_ok_and(|repr| repr == self.frame().ret)
		{
			return None;
		}
		if self.collection_value_returned(node)
			|| self
				.collection_binding(node)
				.is_some_and(|symbol| self.collection_returned_binding(symbol))
		{
			Some(offer)
		} else {
			None
		}
	}

	pub(super) fn collection_call_buffer(
		&mut self,
		node: NodeId,
	) -> Result<Option<CollectionBuffer<'ctx>>, Error> {
		if let Some(offer) = self.forwarded_collection_buffer(node) {
			return Ok(Some(offer));
		}
		// The existing object-columnar chunk is at least 64 KiB. Reserving
		// a 2 KiB caller offer cannot help this representation.
		if matches!(self.node_repr(node)?, Repr::Columnar(_)) {
			return Ok(None);
		}
		Ok(self
			.collection_stack(node, COLLECTION_BUDGET)?
			.map(|pointer| CollectionBuffer {
				pointer,
				bytes: self.usize.const_int(COLLECTION_BUDGET, false),
			}))
	}

	/// Both branches select final storage before initializing a payload.
	pub(super) fn select_collection_buffer(
		&mut self,
		offer: CollectionBuffer<'ctx>,
		required: u64,
		local: impl FnOnce(&mut Self, PointerValue<'ctx>) -> Result<PointerValue<'ctx>, Error>,
		heap: impl FnOnce(&mut Self) -> Result<PointerValue<'ctx>, Error>,
	) -> Result<PointerValue<'ctx>, Error> {
		let fits = self.builder.build_int_compare(
			inkwell::IntPredicate::UGE,
			self.builder.build_int_z_extend_or_bit_cast(
				offer.bytes,
				self.context.i64_type(),
				"",
			)?,
			self.context.i64_type().const_int(required, false),
			"destination.fits",
		)?;
		let present = self
			.builder
			.build_is_not_null(offer.pointer, "destination.present")?;
		let fits = self.builder.build_and(fits, present, "")?;
		let frame = self.append_block("caller_storage");
		let allocated = self.append_block("heap_storage");
		let end = self.append_block("collection_storage");
		self.builder
			.build_conditional_branch(fits, frame, allocated)?;
		self.builder.position_at_end(frame);
		let frame_value = local(self, offer.pointer)?;
		let frame_end = self.builder.get_insert_block().expect("a block");
		self.builder.build_unconditional_branch(end)?;
		self.builder.position_at_end(allocated);
		let heap_value = heap(self)?;
		let heap_end = self.builder.get_insert_block().expect("a block");
		self.builder.build_unconditional_branch(end)?;
		self.builder.position_at_end(end);
		let result = self
			.builder
			.build_phi(self.pointer_type(), "collection.result")?;
		result.add_incoming(&[(&frame_value, frame_end), (&heap_value, heap_end)]);
		Ok(result.as_basic_value().into_pointer_value())
	}

	pub(crate) fn readonly_global(
		&self,
		value: BasicValueEnum<'ctx>,
		name: &str,
	) -> PointerValue<'ctx> {
		let global = self.module.add_global(value.get_type(), None, name);
		global.set_linkage(Linkage::Internal);
		global.set_constant(true);
		global.set_initializer(&value);
		global.set_alignment(16);
		global.as_pointer_value()
	}

	/// Pure syntax constants only: evaluating a user function to discover a
	/// constant here would change effects.
	fn readonly_item(
		&mut self,
		node: NodeId,
		repr: Repr,
		depth: usize,
	) -> Result<Option<BasicValueEnum<'ctx>>, Error> {
		if depth > 32 {
			return Ok(None);
		}
		let node = self.ungrouped(node);
		if let Some(value) = self.readonly_collection_inner(node, repr, depth + 1)? {
			return Ok(value.value);
		}
		let value = match (self.view(node), repr) {
			(View::Negate(inner), _) => {
				let Some(value) = self.readonly_item(inner, repr, depth + 1)? else {
					return Ok(None);
				};
				match value {
					BasicValueEnum::IntValue(value) => value.const_neg().into(),
					BasicValueEnum::FloatValue(value) => value
						.get_type()
						.const_float(-value.get_constant().expect("a constant float").0)
						.into(),
					_ => return Ok(None),
				}
			}
			(View::Object { .. } | View::TypedObject { .. }, Repr::Object(object)) => {
				if self.has_sides(object) || self.destroy_function(object).is_some() {
					return Ok(None);
				}
				let Some(info) = self.analysis.object(node).cloned() else {
					return Ok(None);
				};
				let fields = self.types().fields_of(object).to_vec();
				let mut values = Vec::new();
				for (index, field) in fields.iter().enumerate() {
					let Some((_, fa_analyzer::analysis::FieldValue::Given(value))) =
						info.fields.iter().find(|(name, _)| *name == field.name)
					else {
						return Ok(None);
					};
					let field_repr = self.field_repr(object, index, node.index())?;
					let Some(value) = self.readonly_item(*value, field_repr, depth + 1)? else {
						return Ok(None);
					};
					values.push(value);
				}
				self.object_type(object).const_named_struct(&values).into()
			}
			(View::Tuple { items }, Repr::Tuple(ty)) => {
				let reprs = self.tuple_reprs(ty);
				let mut values = Vec::new();
				for (node, repr) in items.iter().zip(reprs) {
					let Some(value) = self.readonly_item(node, repr, depth + 1)? else {
						return Ok(None);
					};
					values.push(value);
				}
				self.llvm_type(repr)
					.into_struct_type()
					.const_named_struct(&values)
					.into()
			}
			(View::Integer(value), Repr::Int(int)) => self.const_int(int.bits, value).into(),
			(View::Integer(value), Repr::F64 | Repr::Percentage) => {
				self.context.f64_type().const_float(value.as_f64()).into()
			}
			(View::Integer(value), Repr::F32) => self
				.context
				.f32_type()
				.const_float(value.as_f64() as f32 as f64)
				.into(),
			(View::Float(value), Repr::F64) => self.context.f64_type().const_float(value).into(),
			(View::Float(value), Repr::F32) => self
				.context
				.f32_type()
				.const_float(value as f32 as f64)
				.into(),
			(View::Boolean(value), Repr::Bool) => self
				.context
				.bool_type()
				.const_int(value as u64, false)
				.into(),
			(View::String(value), Repr::Str) => {
				self.string_constant(value).value.expect("a string")
			}
			(View::Name(_), _) => {
				let Resolution::Local(symbol) = self.analysis.resolution(node) else {
					return Ok(None);
				};
				let symbol = self.analysis.symbols.symbol(symbol);
				if symbol.kind != SymbolKind::Let {
					return Ok(None);
				}
				let View::Declaration {
					value: Some(value), ..
				} = self.view(symbol.decl_node)
				else {
					return Ok(None);
				};
				return self.readonly_item(value, repr, depth + 1);
			}
			_ => return Ok(None),
		};
		Ok(Some(value))
	}

	pub(crate) fn readonly_collection(
		&mut self,
		node: NodeId,
		repr: Repr,
	) -> Result<Option<Val<'ctx>>, Error> {
		let key = (self.file, node, repr);
		if let Some(value) = self.readonly_collections.get(&key) {
			return Ok(Some(*value));
		}
		let value = self.readonly_collection_inner(self.ungrouped(node), repr, 0)?;
		if let Some(value) = value {
			self.readonly_collections.insert(key, value);
		}
		Ok(value)
	}

	fn readonly_collection_inner(
		&mut self,
		node: NodeId,
		repr: Repr,
		depth: usize,
	) -> Result<Option<Val<'ctx>>, Error> {
		if depth > 32 {
			return Ok(None);
		}
		if matches!(self.view(node), View::Name(_)) {
			let Resolution::Local(symbol) = self.analysis.resolution(node) else {
				return Ok(None);
			};
			let data = self.analysis.symbols.symbol(symbol);
			if data.kind != SymbolKind::Let {
				return Ok(None);
			}
			let View::Declaration {
				value: Some(value), ..
			} = self.view(data.decl_node)
			else {
				return Ok(None);
			};
			return self.readonly_collection_inner(self.ungrouped(value), repr, depth + 1);
		}
		if matches!(repr, Repr::Map(_) | Repr::Set(_)) {
			return self.readonly_table(node, repr, depth);
		}
		if matches!(repr, Repr::Pool(_) | Repr::Columnar(_)) {
			return self.readonly_pool(node, repr, depth);
		}
		let (Repr::Array(item_type) | Repr::Growing(item_type)) = repr else {
			return Ok(None);
		};
		let View::Array { items } = self.view(node) else {
			return Ok(None);
		};
		let item = self.item_repr(item_type, node)?;
		let mut values = Vec::new();
		for value in items.iter() {
			let Some(value) = self.readonly_item(value, item, depth + 1)? else {
				return Ok(None);
			};
			values.push(value);
		}
		if matches!(repr, Repr::Growing(_)) {
			let count = values.len();
			let capacity = count
				.next_power_of_two()
				.max(1usize << self.segment_bits(item));
			let k = capacity.trailing_zeros();
			let i16 = self.context.i16_type();
			let pointer = self.pointer_type();
			let mut segments = [pointer.const_null(); 3];
			if count != 0 {
				values.resize(capacity, self.llvm_type(item).const_zero());
				segments[0] = self.readonly_global(
					self.context.const_struct(&values, false).into(),
					"fa.collection.segment",
				);
			}
			let ty = self.segmented_type();
			let global = self.module.add_global(ty, None, "fa.collection.static");
			global.set_linkage(Linkage::Internal);
			global.set_constant(true);
			global.set_alignment(16);
			let directory = unsafe {
				global.as_pointer_value().const_gep(
					ty,
					&[
						self.context.i32_type().const_zero(),
						self.context.i32_type().const_int(6, false),
					],
				)
			};
			let metadata = self.segment_metadata_type().const_named_struct(&[
				i16.const_int(u64::from(count != 0), false).into(),
				i16.const_int(2, false).into(),
			]);
			let header = ty.const_named_struct(&[
				self.context
					.i64_type()
					.const_int(count as u64, false)
					.into(),
				metadata.into(),
				self.context.i32_type().const_int(k as u64, false).into(),
				pointer.const_null().into(),
				pointer.const_null().into(),
				directory.into(),
				pointer.const_array(&segments).into(),
			]);
			global.set_initializer(&header);
			return Ok(Some(Val::new(repr, global.as_pointer_value())));
		}
		let items = self.context.const_struct(&values, false);
		let items = self.readonly_global(items.into(), "fa.collection.items");
		let i64 = self.context.i64_type();
		let count = i64.const_int(values.len() as u64, false);
		// The marker follows the regular array header, as for small heap arrays.
		let header = self.context.const_struct(
			&[
				items.into(),
				count.into(),
				count.into(),
				i64.const_int(2, false).into(),
			],
			false,
		);
		let pointer = self.readonly_global(header.into(), "fa.collection.static");
		Ok(Some(Val::new(repr, pointer)))
	}

	fn readonly_pool(
		&mut self,
		node: NodeId,
		repr: Repr,
		depth: usize,
	) -> Result<Option<Val<'ctx>>, Error> {
		let (Repr::Pool(ty) | Repr::Columnar(ty)) = repr else {
			return Ok(None);
		};
		let View::Array { items } = self.view(node) else {
			return Ok(None);
		};
		let element = self.item_repr(ty, node)?;
		if matches!(element, Repr::Object(ty) if self.has_sides(ty)) {
			return Ok(None);
		}
		let mut values = Vec::new();
		for node in items.iter() {
			let Some(value) = self.readonly_item(node, element, depth + 1)? else {
				return Ok(None);
			};
			values.push(value);
		}
		let count = values.len() as u64;
		let i64 = self.context.i64_type();
		let i32 = self.context.i32_type();
		let pointer = self.pointer_type();
		let mut words = vec![i64.const_zero(); count.div_ceil(64) as usize];
		for (word, value) in words.iter_mut().enumerate() {
			let bits = (count - word as u64 * 64).min(64);
			*value = i64.const_int(u64::MAX >> (64 - bits), false);
		}
		let slots;
		let chunk;
		let (slot_size, capacity, pages, slots_offset);
		if let (Repr::Columnar(_), Repr::Object(object)) = (repr, element) {
			let layout = self.column_layout(object);
			if count > layout.capacity {
				return Ok(None);
			}
			slot_size = 4;
			capacity = layout.capacity;
			pages = layout.pages;
			slots_offset = layout.slots_offset;
			words.resize(capacity.div_ceil(64) as usize, i64.const_zero());
			let mut fields: Vec<BasicValueEnum<'ctx>> = vec![
				pointer.const_null().into(),
				i32.const_zero().into(),
				i32.const_int(count, false).into(),
				i32.const_int(count, false).into(),
				i32.const_zero().into(),
				i64.const_array(&words).into(),
			];
			// Packed fields with explicit padding exactly reproduce the normal
			// chunk offsets, including on wasm (whose pointer is four bytes).
			let mut end = u64::from(self.usize.get_bit_width() / 8) + 16;
			let padding = 24 - end;
			fields.insert(
				5,
				self.context
					.i8_type()
					.array_type(padding as u32)
					.const_zero()
					.into(),
			);
			end = 24 + words.len() as u64 * 8;
			fields.push(
				self.context
					.i8_type()
					.array_type((slots_offset - end) as u32)
					.const_zero()
					.into(),
			);
			fields.push(i32.array_type(capacity as u32).const_zero().into());
			end = slots_offset + capacity * 4;
			for (index, (run, size)) in layout.columns.iter().copied().enumerate() {
				fields.push(
					self.context
						.i8_type()
						.array_type((run - end) as u32)
						.const_zero()
						.into(),
				);
				let field_repr = self.field_repr(object, index, node.index())?;
				let mut column = Vec::new();
				for value in &values {
					column.push(
						value
							.into_struct_value()
							.get_field_at_index(index as u32)
							.expect("a constant field"),
					);
				}
				column.resize(capacity as usize, self.llvm_type(field_repr).const_zero());
				fields.push(self.context.const_struct(&column, false).into());
				end = run + capacity * size;
			}
			fields.push(
				self.context
					.i8_type()
					.array_type((pages * 65536 - end) as u32)
					.const_zero()
					.into(),
			);
			let initializer = self.context.const_struct(&fields, true);
			let global =
				self.module
					.add_global(initializer.get_type(), None, "fa.collection.chunk");
			global.set_linkage(Linkage::Internal);
			global.set_constant(true);
			global.set_alignment((pages * 65536) as u32);
			chunk = global.as_pointer_value();
			slots = unsafe {
				chunk.const_gep(
					self.context.i8_type(),
					&[self.usize.const_int(slots_offset, false)],
				)
			};
			fields[0] = slots.into();
			global.set_initializer(&self.context.const_struct(&fields, true));
		} else {
			slot_size = self.collection_data_size(self.llvm_type(element)).max(4);
			capacity = count;
			pages = 0;
			slots_offset = 0;
			// Slab slots are at least four bytes, even for Boolean and Byte.
			let item_size = self.collection_data_size(self.llvm_type(element));
			let values: Vec<_> = values
				.into_iter()
				.map(|value| {
					self.context
						.const_struct(
							&[
								value,
								self.context
									.i8_type()
									.array_type((slot_size - item_size) as u32)
									.const_zero()
									.into(),
							],
							false,
						)
						.into()
				})
				.collect();
			slots = self.readonly_global(
				self.context.const_struct(&values, false).into(),
				"fa.collection.slots",
			);
			let header = self.context.const_struct(
				&[
					slots.into(),
					i32.const_zero().into(),
					i32.const_int(count, false).into(),
					i32.const_int(count, false).into(),
					i32.const_zero().into(),
					i64.const_array(&words).into(),
				],
				false,
			);
			chunk = self.readonly_global(header.into(), "fa.collection.chunk");
		}
		let chunks =
			self.readonly_global(pointer.const_array(&[chunk]).into(), "fa.collection.chunks");
		let header = self.pool_header().const_named_struct(&[
			i64.const_int(count, false).into(),
			chunks.into(),
			pointer.const_null().into(),
			pointer.const_null().into(),
			i32.const_int(u64::from(count != 0), false).into(),
			i32.const_zero().into(),
			i32.const_int(slot_size, false).into(),
			i32.const_int(capacity, false).into(),
			i32.const_int(pages, false).into(),
			i32.const_int(slots_offset, false).into(),
			i32.const_int(u64::from(count != 0), false).into(),
			i64.const_int(2, false).into(),
		]);
		Ok(Some(Val::new(
			repr,
			self.readonly_global(header.into(), "fa.collection.static"),
		)))
	}

	fn readonly_table(
		&mut self,
		node: NodeId,
		repr: Repr,
		depth: usize,
	) -> Result<Option<Val<'ctx>>, Error> {
		let View::Array { items } = self.view(node) else {
			return Ok(None);
		};
		let table = self.table_of(repr)?;
		let i64 = self.context.i64_type();
		let mut entries: Vec<(u64, BasicValueEnum<'ctx>, Option<BasicValueEnum<'ctx>>)> =
			Vec::new();
		for node in items.iter() {
			let (key_node, value_node) = if table.value.is_some() {
				let View::Tuple { items } = self.view(self.ungrouped(node)) else {
					return Ok(None);
				};
				if items.len() != 2 {
					return Ok(None);
				}
				(items.get(0).expect("a key"), items.get(1))
			} else {
				(node, None)
			};
			let Some(key) = self.readonly_item(key_node, table.key, depth + 1)? else {
				return Ok(None);
			};
			let Some(hash) = self.readonly_key_hash(key_node, table.key, key) else {
				return Ok(None);
			};
			let value = if let (Some(node), Some(repr)) = (value_node, table.value) {
				let Some(value) = self.readonly_item(node, repr, depth + 1)? else {
					return Ok(None);
				};
				Some(value)
			} else {
				None
			};
			if let Some(entry) = entries.iter_mut().find(|(_, old, _)| {
				if let (BasicValueEnum::FloatValue(a), BasicValueEnum::FloatValue(b)) = (*old, key)
				{
					let a = a.get_constant().expect("a constant key").0;
					let b = b.get_constant().expect("a constant key").0;
					a == b || (a.is_nan() && b.is_nan())
				} else {
					*old == key
				}
			}) {
				if table.value.is_some() {
					entry.2 = value;
				}
			} else {
				entries.push((hash, key, value));
			}
		}
		let count = entries.len();
		let slots = (count.max(4) * 2).next_power_of_two();
		let mut index = vec![0; slots];
		let mut values = Vec::new();
		for (position, (hash, key, value)) in entries.into_iter().enumerate() {
			let mut slot = hash as usize & (slots - 1);
			while index[slot] != 0 {
				slot = (slot + 1) & (slots - 1);
			}
			index[slot] = position as u64 + 1;
			let mut fields = vec![
				i64.const_int(hash, false).into(),
				i64.const_int(slot as u64 + 1, false).into(),
				key,
			];
			if let Some(value) = value {
				fields.push(value);
			}
			values.push(table.entry.const_named_struct(&fields));
		}
		let capacity = if table.stable() {
			count.max(4).div_ceil(32) * 32
		} else {
			count.max(4)
		};
		values.resize(capacity, table.entry.const_zero());
		let data = if table.stable() {
			let mut chunks = Vec::new();
			for chunk in values.chunks(32) {
				chunks.push(self.readonly_global(
					table.entry.const_array(chunk).into(),
					"fa.collection.entries",
				));
			}
			let directory = self.readonly_global(
				self.pointer_type().const_array(&chunks).into(),
				"fa.collection.directory",
			);
			let count = i64.const_int(chunks.len() as u64, false);
			self.readonly_global(
				self.context
					.const_struct(&[directory.into(), count.into(), count.into()], false)
					.into(),
				"fa.collection.chunks",
			)
		} else {
			self.readonly_global(
				table.entry.const_array(&values).into(),
				"fa.collection.entries",
			)
		};
		let index_type = if slots <= 0x8000_0000 {
			self.context.i32_type()
		} else {
			i64
		};
		let index: Vec<_> = index
			.into_iter()
			.map(|value| index_type.const_int(value, false))
			.collect();
		let index =
			self.readonly_global(index_type.const_array(&index).into(), "fa.collection.index");
		let count = i64.const_int(count as u64, false);
		let kind = if table.value.is_some() && !table.growing {
			2
		} else {
			u64::from(table.growing)
		};
		let header = self.table_header().const_named_struct(&[
			data.into(),
			count.into(),
			count.into(),
			i64.const_int(capacity as u64, false).into(),
			index.into(),
			i64.const_int(slots as u64 - 1, false).into(),
			i64.const_int(kind, false).into(),
			i64.const_zero().into(),
			count.into(),
			i64.const_int(2, false).into(),
		]);
		let pointer = self.readonly_global(header.into(), "fa.collection.static");
		Ok(Some(Val::new(repr, pointer)))
	}

	/// Kept in step with `tables::mix` and the runtime's `fa_hash_bytes`.
	fn readonly_key_hash(
		&self,
		node: NodeId,
		repr: Repr,
		value: BasicValueEnum<'ctx>,
	) -> Option<u64> {
		fn mix(mut value: u64) -> u64 {
			value = (value ^ (value >> 33)).wrapping_mul(0xff51afd7ed558ccd);
			value = (value ^ (value >> 33)).wrapping_mul(0xc4ceb9fe1a85ec53);
			value ^ (value >> 33)
		}
		Some(match repr {
			Repr::Int(int) if int.bits <= 64 => {
				let value = value.into_int_value();
				mix(if int.signed {
					value.get_sign_extended_constant()? as u64
				} else {
					value.get_zero_extended_constant()?
				})
			}
			Repr::Bool => mix(value.into_int_value().get_zero_extended_constant()?),
			Repr::F64 | Repr::F32 | Repr::Percentage => {
				let value = value.into_float_value().get_constant()?.0;
				mix(if value == 0.0 {
					0
				} else if value.is_nan() {
					0x7ff8000000000000
				} else {
					value.to_bits()
				})
			}
			Repr::Str => {
				let text = self.constant_text(node)?;
				let bytes = text.as_bytes();
				let multiplier = 0x9E3779B97F4A7C15u64;
				let mut hash = 0xcbf29ce484222325 ^ (bytes.len() as u64).wrapping_mul(multiplier);
				let (chunks, rest) = bytes.as_chunks::<8>();
				for chunk in chunks {
					hash = (hash ^ u64::from_le_bytes(*chunk)).wrapping_mul(multiplier);
					hash ^= hash >> 29;
				}
				if !rest.is_empty() {
					let word = if rest.len() >= 4 {
						u64::from(u32::from_le_bytes(
							rest[..4].try_into().expect("four bytes"),
						)) | (u64::from(u32::from_le_bytes(
							rest[rest.len() - 4..].try_into().expect("four bytes"),
						)) << 32)
					} else {
						u64::from(rest[0])
							| (u64::from(rest[rest.len() / 2]) << 8)
							| (u64::from(rest[rest.len() - 1]) << 16)
					};
					hash = (hash ^ word).wrapping_mul(multiplier);
				}
				hash = (hash ^ (hash >> 33)).wrapping_mul(0xff51afd7ed558ccd);
				hash ^ (hash >> 33)
			}
			_ => return None,
		})
	}
}
