//! Packed arrays (`design/decisions.md`, "Packed arrays"): an `Array` of a
//! boxed named union whose arrays the program only builds with `push` from
//! an empty literal, reads with `for` and measures with `length` stores its
//! items one after the other in one block, each as the index of its member
//! (a byte) followed by its data without padding, a string as its length
//! and its bytes: no box per item, and as many bytes per item as its member
//! needs (the runtime's `FaPacked`). A `for` decodes each item into a box on
//! the stack, whose strings are views of the block, and binds its variable
//! to that box, as it would to an item of an ordinary array.

use fa_analyzer::analysis::{Analysis, Callee};
use fa_analyzer::ids::{FileId, NodeId, NominalId, TypeId};
use fa_analyzer::session::Session;
use fa_analyzer::symbols::Resolution;
use fa_analyzer::syntax::{View, view};
use fa_analyzer::types::{CollKind, TypeKind, TypeStore};
use inkwell::IntPredicate;
use inkwell::types::{BasicType, BasicTypeEnum};
use inkwell::values::{BasicValue, BasicValueEnum, IntValue, PointerValue};
use rustc_hash::FxHashSet;

use super::values::{Repr, Val};
use super::{LoopFrame, Lowering, Slot};
use crate::error::Error;

/// The item type of an `Array` of a named union.
fn union_array_item(types: &TypeStore, ty: TypeId) -> Option<TypeId> {
	match types.kind(ty) {
		TypeKind::Collection(CollKind::Array, item, _)
			if matches!(types.kind(item), TypeKind::NamedUnion(..)) =>
		{
			Some(item)
		}
		_ => None,
	}
}

/// Whether a type mentions a type parameter (an array of it may be an
/// array of any union in an instance).
fn mentions_parameter(types: &TypeStore, ty: TypeId) -> bool {
	let mut stack = vec![ty];
	let mut seen = FxHashSet::default();
	while let Some(at) = stack.pop() {
		if !seen.insert(at) {
			continue;
		}
		match types.kind(at) {
			TypeKind::Param(_) | TypeKind::ImplicitParam(..) => return true,
			TypeKind::Collection(_, item, key) => {
				stack.push(item);
				stack.push(key);
			}
			_ => {}
		}
	}
	false
}

/// The union types whose arrays may be packed: every node of the program
/// holding such an array is a declaration whose value is an empty literal
/// (or that literal, or its annotation), or a name of a local that is the
/// iterable of a `for`, the receiver of `push` or of `length`, outside any
/// closure. Syntactic and conservative, over every file of the program; an
/// array of a type parameter anywhere else packs none (an instance could
/// use it otherwise).
pub(crate) fn packable_items(session: &Session, files: &[(FileId, String)]) -> FxHashSet<TypeId> {
	let types = session.types();
	let mut found = FxHashSet::default();
	let mut refused = FxHashSet::default();
	let mut generic = false;
	for (file, _) in files {
		let tree = session.tree(*file);
		let analysis = session.analysis(*file);
		let parents = crate::facts::parents_of(tree);
		let declared = crate::facts::declaration_lambdas(tree);
		let parent_of = |node: NodeId| {
			let parent = parents[node.index()];
			(parent != usize::MAX).then(|| NodeId::new(parent))
		};
		for index in 0..tree.nodes.len() {
			let node = NodeId::new(index);
			let mut ty = analysis.type_of(node);
			if let View::Declaration { .. } = view(tree, node)
				&& let Some(symbol) = analysis.symbols.symbol_of_decl(node)
			{
				ty = analysis.symbols.symbol(symbol).ty;
			}
			if ty.is_unset() {
				continue;
			}
			let item = union_array_item(types, ty);
			if item.is_none() {
				if let TypeKind::Collection(CollKind::Array, inner, _) = types.kind(ty)
					&& mentions_parameter(types, inner)
					&& !allowed(tree, analysis, node, &parent_of, &declared)
				{
					generic = true;
				}
				continue;
			}
			let item = item.expect("an item");
			found.insert(item);
			if !allowed(tree, analysis, node, &parent_of, &declared) {
				if std::env::var_os("FA_DEBUG_PACKED").is_some() {
					eprintln!(
						"packed: refused at `{}`",
						&tree.source[tree.spans[index].start..tree.spans[index].end]
					);
				}
				refused.insert(item);
			}
		}
	}
	if generic {
		return FxHashSet::default();
	}
	found.retain(|item| !refused.contains(item));
	found
}

/// See [`packable_items`].
fn allowed(
	tree: &fa_parser::TypedSyntaxTree,
	analysis: &Analysis,
	node: NodeId,
	parent_of: &impl Fn(NodeId) -> Option<NodeId>,
	declared: &std::collections::HashSet<NodeId>,
) -> bool {
	let parent = parent_of(node);
	// a type annotation of a declaration, or a part of it
	let mut child = node;
	while let Some(ancestor) = parent_of(child) {
		if matches!(view(tree, ancestor), View::Declaration { annotation: Some(annotation), .. } if annotation == child)
		{
			return true;
		}
		child = ancestor;
	}
	// a path of a `mutates` or `invalidates` clause
	if let Some(parent) = parent {
		let named = match view(tree, parent) {
			View::Lambda { signature, .. } => {
				signature.mutates.iter().any(|path| path == node)
					|| signature.invalidates.iter().any(|path| path == node)
			}
			View::Effect { paths, .. } => paths.iter().any(|path| path == node),
			_ => false,
		};
		if named {
			return true;
		}
	}
	match view(tree, node) {
		// its value: an empty literal
		View::Declaration { value, .. } => {
			return value.is_some_and(
				|value| matches!(view(tree, value), View::Array { items } if items.is_empty()),
			);
		}
		View::Array { items } if items.is_empty() => {
			return parent.is_some_and(|parent| {
				matches!(view(tree, parent), View::Declaration { value: Some(value), .. } if value == node)
			});
		}
		View::Name(_) => {}
		_ => {
			// a type annotation of a declaration
			return parent.is_some_and(|parent| {
				matches!(view(tree, parent), View::Declaration { annotation: Some(annotation), .. } if annotation == node)
			});
		}
	}
	if !matches!(analysis.resolution(node), Resolution::Local(_)) {
		return false;
	}
	// never captured by a closure
	let mut at = parent;
	while let Some(ancestor) = at {
		if matches!(view(tree, ancestor), View::Lambda { .. }) && !declared.contains(&ancestor) {
			return false;
		}
		at = parent_of(ancestor);
	}
	let Some(parent) = parent else {
		return false;
	};
	match view(tree, parent) {
		View::For {
			iterable,
			parallel: None,
			compile_time: false,
			..
		} => iterable == node,
		View::Access { target, members } if target == node && members.len() == 1 => {
			let member = members.first().expect("a member");
			let Resolution::Builtin(id) = analysis.resolution(member) else {
				return false;
			};
			match fa_analyzer::builtins::get(id).path {
				"Array#length" => true,
				"Array#push" => parent_of(parent).is_some_and(|call| {
					matches!(view(tree, call), View::Call { callee, arguments } if callee == parent && arguments.len() == 1)
						&& analysis
							.call(call)
							.is_some_and(|info| matches!(info.callee, Callee::Builtin(_)))
				}),
				_ => false,
			}
		}
		_ => false,
	}
}

/// One scalar or string of the data of a member, at a path of struct
/// indices inside it.
#[derive(Clone, Debug)]
struct Leaf {
	path: Vec<u32>,
	repr: Repr,
}

/// The layout of a member in a packed array: its data's representation and
/// leaves (`None`: a member without data).
#[derive(Clone, Debug)]
struct PackedMember {
	data: Option<Repr>,
	leaves: Vec<Leaf>,
}

#[derive(Clone, Debug)]
pub(crate) struct PackedLayout {
	nominal: NominalId,
	members: Vec<(u16, PackedMember)>,
	/// The size of the largest box of a member, in 8-byte words.
	box_words: u32,
}

impl<'ctx> Lowering<'_, 'ctx> {
	/// The layout of the items of arrays of `item` when they are packed.
	pub(crate) fn packed_layout(&self, item: TypeId) -> Option<PackedLayout> {
		if !self.whole_program || !self.packable_items.contains(&item) {
			return None;
		}
		if let Some(layout) = self.packed_layouts.borrow().get(&item) {
			return layout.clone();
		}
		let layout = self.compute_packed_layout(item);
		self.packed_layouts
			.borrow_mut()
			.insert(item, layout.clone());
		layout
	}

	fn compute_packed_layout(&self, item: TypeId) -> Option<PackedLayout> {
		let TypeKind::NamedUnion(nominal, mask) = self.types().kind(item) else {
			return None;
		};
		if self.repr_of(item, 0).ok()? != Repr::Union {
			return None;
		}
		let types = self.types();
		let count = types.sym_list(types.nominal(nominal).members).len();
		if count > 256 {
			return None;
		}
		let target = self.target_data();
		let mut members = Vec::new();
		let mut box_bytes = 8;
		for index in 0..count as u16 {
			if !types.mask_has(mask, index) {
				continue;
			}
			let Some(data_type) = self.member_data(nominal, index) else {
				members.push((
					index,
					PackedMember {
						data: None,
						leaves: Vec::new(),
					},
				));
				continue;
			};
			let data = self.repr_of(data_type, 0).ok()?;
			let mut leaves = Vec::new();
			if !self.packed_leaves(data, &mut Vec::new(), &mut leaves, 0) {
				return None;
			}
			box_bytes = box_bytes.max(target.get_abi_size(&self.box_type(Some(data))));
			members.push((
				index,
				PackedMember {
					data: Some(data),
					leaves,
				},
			));
		}
		Some(PackedLayout {
			nominal,
			members,
			box_words: box_bytes.div_ceil(8) as u32,
		})
	}

	/// The leaves of a representation: scalars and strings, inside objects.
	fn packed_leaves(
		&self,
		repr: Repr,
		path: &mut Vec<u32>,
		out: &mut Vec<Leaf>,
		depth: usize,
	) -> bool {
		if depth > 8 {
			return false;
		}
		match repr {
			Repr::Bool | Repr::F64 | Repr::F32 | Repr::Percentage | Repr::Enum(_) | Repr::Str => {
				out.push(Leaf {
					path: path.clone(),
					repr,
				});
				true
			}
			Repr::Int(int) if matches!(int.bits, 8 | 16 | 32 | 64) => {
				out.push(Leaf {
					path: path.clone(),
					repr,
				});
				true
			}
			Repr::Object(ty) => {
				if self.types().is_resource(ty) {
					return false;
				}
				for (index, field) in self.object_fields(ty).into_iter().enumerate() {
					if !field.has_value() {
						continue;
					}
					path.push(index as u32);
					let fine = self.packed_leaves(field, path, out, depth + 1);
					path.pop();
					if !fine {
						return false;
					}
				}
				true
			}
			_ => false,
		}
	}

	/// The size in bytes of a fixed leaf (not a string).
	fn leaf_size(&self, repr: Repr) -> u64 {
		match repr {
			Repr::Bool => 1,
			Repr::Int(int) => u64::from(int.bits / 8),
			Repr::F64 | Repr::Percentage => 8,
			Repr::F32 | Repr::Enum(_) => 4,
			_ => 0,
		}
	}

	/// The address of a leaf inside the data at `pointer`.
	fn leaf_pointer(
		&self,
		data: Repr,
		pointer: PointerValue<'ctx>,
		path: &[u32],
	) -> Result<PointerValue<'ctx>, Error> {
		let mut repr = data;
		let mut pointer = pointer;
		for &index in path {
			let Repr::Object(ty) = repr else {
				return Err(Error::Llvm("a packed leaf outside an object".into()));
			};
			pointer = self
				.builder
				.build_struct_gep(self.object_type(ty), pointer, index, "")?;
			repr = self.object_fields(ty)[index as usize];
		}
		Ok(pointer)
	}

	/// Whether `node` is the argument of a `push` on a packed array: a
	/// member built there lives in a box on the stack, since the push copies
	/// it (`objects.rs`, `build_member`).
	pub(crate) fn packed_push_argument(&self, node: NodeId) -> bool {
		if self.packable_items.is_empty() {
			return false;
		}
		let parent = self.parents()[node.index()];
		if parent == usize::MAX {
			return false;
		}
		let call = NodeId::new(parent);
		let View::Call { callee, arguments } = self.view(call) else {
			return false;
		};
		if arguments.len() != 1 || arguments.first() != Some(node) {
			return false;
		}
		let Some(info) = self.analysis.call(call) else {
			return false;
		};
		let Callee::Builtin(id) = info.callee else {
			return false;
		};
		if fa_analyzer::builtins::get(id).path != "Array#push" {
			return false;
		}
		let View::Access { target, members } = self.view(callee) else {
			return false;
		};
		if members.len() != 1 {
			return false;
		}
		union_array_item(self.types(), self.type_of(target))
			.is_some_and(|item| self.packed_layout(item).is_some())
	}

	/// A new empty packed array of items of `item`.
	pub(crate) fn packed_new(&self, item: TypeId) -> Result<Val<'ctx>, Error> {
		let function = self.runtime.function("fa_packed_new")?;
		let array = self.call_value(function, &[])?;
		Ok(self.fresh(Val::new(Repr::Array(item), array.into_pointer_value())))
	}

	/// `array.push(value)` on a packed array: the value's member and data
	/// copied into the block (the value itself is left to its owner).
	pub(crate) fn packed_push(
		&mut self,
		layout: &PackedLayout,
		array: PointerValue<'ctx>,
		value: Val<'ctx>,
	) -> Result<(), Error> {
		let i64 = self.context.i64_type();
		let i8 = self.context.i8_type();
		let boxed = value.value.expect("a union").into_pointer_value();
		let id = self.builder.build_load(i64, boxed, "")?.into_int_value();
		let end = self.append_block("packed_pushed");
		let otherwise = self.append_block("packed_member");
		let mut cases = Vec::new();
		for (index, member) in &layout.members {
			let block = self.append_block("packed_member");
			cases.push((
				i64.const_int(self.member_id(layout.nominal, *index), false),
				block,
				*index,
				member.clone(),
			));
		}
		self.builder.build_switch(
			id,
			otherwise,
			&cases
				.iter()
				.map(|(id, block, ..)| (*id, *block))
				.collect::<Vec<_>>(),
		)?;
		self.builder.position_at_end(otherwise);
		self.builder.build_unreachable()?;
		let push = self.runtime.function("fa_packed_push")?;
		for (_, block, index, member) in cases {
			self.builder.position_at_end(block);
			let data_pointer = match member.data {
				Some(data) => Some((
					data,
					self.builder
						.build_struct_gep(self.box_type(Some(data)), boxed, 1, "")?,
				)),
				None => None,
			};
			// the leaves' values, and the size of the item
			let mut fixed = 1u64;
			let mut variable: Option<IntValue<'ctx>> = None;
			let mut values = Vec::new();
			for leaf in &member.leaves {
				let (data, pointer) = data_pointer.expect("data");
				let pointer = self.leaf_pointer(data, pointer, &leaf.path)?;
				let loaded = self
					.builder
					.build_load(self.llvm_type(leaf.repr), pointer, "")?;
				if leaf.repr == Repr::Str {
					let (bytes, length) = self.string_parts(Val::new(Repr::Str, loaded))?;
					let capacity = self
						.builder
						.build_extract_value(loaded.into_struct_value(), 2, "")?
						.into_int_value();
					let length64 = self
						.builder
						.build_int_z_extend_or_bit_cast(length, i64, "")?;
					// one byte of length below 127, else a 0xFF and a word
					let short = self.builder.build_int_compare(
						IntPredicate::ULT,
						length64,
						i64.const_int(127, false),
						"",
					)?;
					let header = self.builder.build_select(
						short,
						i64.const_int(1, false),
						i64.const_int(1 + u64::from(self.usize.get_bit_width() / 8), false),
						"",
					)?;
					let size = self
						.builder
						.build_int_add(header.into_int_value(), length64, "")?;
					variable = Some(match variable {
						Some(sum) => self.builder.build_int_add(sum, size, "")?,
						None => size,
					});
					values.push((leaf.repr, loaded, Some((bytes, length, capacity, short))));
				} else {
					fixed += self.leaf_size(leaf.repr);
					values.push((leaf.repr, loaded, None));
				}
			}
			let size = match variable {
				Some(sum) => self
					.builder
					.build_int_add(sum, i64.const_int(fixed, false), "")?,
				None => i64.const_int(fixed, false),
			};
			let mut cursor = self
				.call_value(push, &[array.into(), size.into()])?
				.into_pointer_value();
			let store = self
				.builder
				.build_store(cursor, i8.const_int(u64::from(index), false))?;
			store
				.set_alignment(1)
				.map_err(|e| Error::Llvm(e.to_string()))?;
			cursor = self.packed_offset(cursor, i64.const_int(1, false))?;
			for (repr, loaded, string) in values {
				match string {
					None => {
						let value: BasicValueEnum<'ctx> = if repr == Repr::Bool {
							self.builder
								.build_int_z_extend(loaded.into_int_value(), i8, "")?
								.into()
						} else {
							loaded
						};
						let store = self.builder.build_store(cursor, value)?;
						store
							.set_alignment(1)
							.map_err(|e| Error::Llvm(e.to_string()))?;
						cursor =
							self.packed_offset(cursor, i64.const_int(self.leaf_size(repr), false))?;
					}
					Some((bytes, length, capacity, short)) => {
						cursor =
							self.packed_store_string(cursor, bytes, length, capacity, short)?;
					}
				}
			}
			self.builder.build_unconditional_branch(end)?;
		}
		self.builder.position_at_end(end);
		Ok(())
	}

	fn packed_offset(
		&self,
		pointer: PointerValue<'ctx>,
		offset: IntValue<'ctx>,
	) -> Result<PointerValue<'ctx>, Error> {
		Ok(unsafe {
			self.builder
				.build_in_bounds_gep(self.context.i8_type(), pointer, &[offset], "")?
		})
	}

	/// Writes a string's length (with its `SIMPLE` flag in the low bit) and
	/// bytes at `cursor`; the address after them.
	fn packed_store_string(
		&self,
		cursor: PointerValue<'ctx>,
		bytes: PointerValue<'ctx>,
		length: IntValue<'ctx>,
		capacity: IntValue<'ctx>,
		short: IntValue<'ctx>,
	) -> Result<PointerValue<'ctx>, Error> {
		let i8 = self.context.i8_type();
		let usize = self.usize;
		let simple = self.builder.build_and(capacity, self.simple_flag(), "")?;
		let simple =
			self.builder
				.build_int_compare(IntPredicate::NE, simple, usize.const_zero(), "")?;
		let simple = self.builder.build_int_z_extend(simple, usize, "")?;
		let word = self
			.builder
			.build_left_shift(length, usize.const_int(1, false), "")?;
		let word = self.builder.build_or(word, simple, "")?;
		let short_block = self.append_block("packed_short");
		let long_block = self.append_block("packed_long");
		let join = self.append_block("packed_string");
		self.builder
			.build_conditional_branch(short, short_block, long_block)?;
		self.builder.position_at_end(short_block);
		let byte = self.builder.build_int_truncate(word, i8, "")?;
		let store = self.builder.build_store(cursor, byte)?;
		store
			.set_alignment(1)
			.map_err(|e| Error::Llvm(e.to_string()))?;
		let short_bytes = self.packed_offset(cursor, usize.const_int(1, false))?;
		self.builder.build_unconditional_branch(join)?;
		self.builder.position_at_end(long_block);
		let store = self
			.builder
			.build_store(cursor, i8.const_int(0xFF, false))?;
		store
			.set_alignment(1)
			.map_err(|e| Error::Llvm(e.to_string()))?;
		let word_pointer = self.packed_offset(cursor, usize.const_int(1, false))?;
		let store = self.builder.build_store(word_pointer, word)?;
		store
			.set_alignment(1)
			.map_err(|e| Error::Llvm(e.to_string()))?;
		let word_size = u64::from(usize.get_bit_width() / 8);
		let long_bytes = self.packed_offset(cursor, usize.const_int(1 + word_size, false))?;
		self.builder.build_unconditional_branch(join)?;
		self.builder.position_at_end(join);
		let destination = self.builder.build_phi(self.pointer_type(), "")?;
		destination.add_incoming(&[(&short_bytes, short_block), (&long_bytes, long_block)]);
		let destination = destination.as_basic_value().into_pointer_value();
		self.builder
			.build_memcpy(destination, 1, bytes, 1, length)
			.map_err(|e| Error::Llvm(e.to_string()))?;
		self.packed_offset(destination, length)
	}

	/// Reads a string written by [`Self::packed_store_string`] at `cursor`:
	/// a view of its bytes in the block, and the address after them.
	fn packed_load_string(
		&self,
		cursor: PointerValue<'ctx>,
	) -> Result<(BasicValueEnum<'ctx>, PointerValue<'ctx>), Error> {
		let i8 = self.context.i8_type();
		let usize = self.usize;
		let header = self.builder.build_load(i8, cursor, "")?.into_int_value();
		let long = self.builder.build_int_compare(
			IntPredicate::EQ,
			header,
			i8.const_int(0xFF, false),
			"",
		)?;
		let short_block = self.append_block("packed_short");
		let long_block = self.append_block("packed_long");
		let join = self.append_block("packed_string");
		self.builder
			.build_conditional_branch(long, long_block, short_block)?;
		self.builder.position_at_end(short_block);
		let short_word = self.builder.build_int_z_extend(header, usize, "")?;
		let short_bytes = self.packed_offset(cursor, usize.const_int(1, false))?;
		self.builder.build_unconditional_branch(join)?;
		self.builder.position_at_end(long_block);
		let word_pointer = self.packed_offset(cursor, usize.const_int(1, false))?;
		let load = self.builder.build_load(usize, word_pointer, "")?;
		load.as_instruction_value()
			.expect("a load")
			.set_alignment(1)
			.map_err(|e| Error::Llvm(e.to_string()))?;
		let word_size = u64::from(usize.get_bit_width() / 8);
		let long_bytes = self.packed_offset(cursor, usize.const_int(1 + word_size, false))?;
		self.builder.build_unconditional_branch(join)?;
		self.builder.position_at_end(join);
		let word = self.builder.build_phi(usize, "")?;
		word.add_incoming(&[(&short_word, short_block), (&load, long_block)]);
		let word = word.as_basic_value().into_int_value();
		let bytes = self.builder.build_phi(self.pointer_type(), "")?;
		bytes.add_incoming(&[(&short_bytes, short_block), (&long_bytes, long_block)]);
		let bytes = bytes.as_basic_value().into_pointer_value();
		let length = self
			.builder
			.build_right_shift(word, usize.const_int(1, false), false, "")?;
		// a view (`KIND_MASK`), with the `SIMPLE` flag of the low bit
		let simple = self
			.builder
			.build_and(word, usize.const_int(1, false), "")?;
		let simple = self.builder.build_left_shift(
			simple,
			usize.const_int(u64::from(usize.get_bit_width() - 1), false),
			"",
		)?;
		let capacity = self.builder.build_or(simple, self.string_kind_mask(), "")?;
		let mut value = self.string_type.get_undef();
		for (index, part) in [BasicValueEnum::from(bytes), length.into(), capacity.into()]
			.into_iter()
			.enumerate()
		{
			value = self
				.builder
				.build_insert_value(value, part, index as u32, "")?
				.into_struct_value();
		}
		let after = self.packed_offset(bytes, length)?;
		Ok((value.into(), after))
	}

	/// `for item in array` over a packed array.
	#[allow(clippy::too_many_arguments)]
	pub(crate) fn for_packed(
		&mut self,
		layout: &PackedLayout,
		node: NodeId,
		pattern: Option<NodeId>,
		has_index: bool,
		array: PointerValue<'ctx>,
		body: NodeId,
	) -> Result<Val<'ctx>, Error> {
		let i64 = self.context.i64_type();
		let i8 = self.context.i8_type();
		let pointer_type = self.pointer_type();
		let variable = pattern.and_then(|pattern| self.analysis.symbols.symbol_of_decl(pattern));
		if pattern.is_some_and(|pattern| self.analysis.pattern(pattern).is_some()) {
			return Err(self.unsupported(node.index(), "this pattern in a `for`"));
		}
		let copied = variable.is_some_and(|symbol| {
			matches!(
				self.analysis.symbols.symbol(symbol).kind,
				fa_analyzer::symbols::SymbolKind::LoopVariable { copy: true }
			)
		});
		let index_slot = match has_index
			.then(|| self.analysis.symbols.symbol_of_decl(node))
			.flatten()
		{
			Some(symbol) => Some(self.slot_of(symbol, node)?),
			None => None,
		};
		// the stack box of the current item, and the variable's slot
		let region = self.alloca_type(
			i64.array_type(layout.box_words).as_basic_type_enum(),
			"packed_item",
		)?;
		let item_slot = Slot {
			pointer: self.alloca_type(pointer_type.into(), "packed_view")?,
			repr: Repr::Union,
		};
		self.builder.build_store(item_slot.pointer, region)?;
		// the header: `{ items, length, capacity, used }`
		let header = self.context.struct_type(
			&[pointer_type.into(), i64.into(), i64.into(), i64.into()],
			false,
		);
		let items_pointer = self.builder.build_struct_gep(header, array, 0, "")?;
		let items = self
			.builder
			.build_load(pointer_type, items_pointer, "")?
			.into_pointer_value();
		let used_pointer = self.builder.build_struct_gep(header, array, 3, "")?;
		let used = self
			.builder
			.build_load(i64, used_pointer, "")?
			.into_int_value();
		let end = self.packed_offset(items, used)?;
		let cursor = self.alloca_type(pointer_type.into(), "packed_cursor")?;
		let next_slot = self.alloca_type(pointer_type.into(), "packed_next")?;
		let counter = self.alloca_type(i64.into(), "position")?;
		self.builder.build_store(cursor, items)?;
		self.builder.build_store(counter, i64.const_zero())?;
		let test = self.append_block("for_packed");
		let decode = self.append_block("for_packed_item");
		let body_block = self.append_block("for_packed_body");
		let step = self.append_block("for_packed_step");
		let exit = self.append_block("end_for_packed");
		self.builder.build_unconditional_branch(test)?;
		self.builder.position_at_end(test);
		let at = self
			.builder
			.build_load(pointer_type, cursor, "")?
			.into_pointer_value();
		let more = self
			.builder
			.build_int_compare(IntPredicate::ULT, at, end, "")?;
		self.builder.build_conditional_branch(more, decode, exit)?;
		// decode the item into the stack box
		self.builder.position_at_end(decode);
		let tag = self.builder.build_load(i8, at, "")?.into_int_value();
		let otherwise = self.append_block("packed_member");
		let mut cases = Vec::new();
		for (index, member) in &layout.members {
			let block = self.append_block("packed_member");
			cases.push((
				i8.const_int(u64::from(*index), false),
				block,
				*index,
				member.clone(),
			));
		}
		self.builder.build_switch(
			tag,
			otherwise,
			&cases
				.iter()
				.map(|(tag, block, ..)| (*tag, *block))
				.collect::<Vec<_>>(),
		)?;
		self.builder.position_at_end(otherwise);
		self.builder.build_unreachable()?;
		for (_, block, index, member) in cases {
			self.builder.position_at_end(block);
			let id = i64.const_int(self.member_id(layout.nominal, index), false);
			let ty = self.box_type(member.data);
			let id_pointer = self.builder.build_struct_gep(ty, region, 0, "")?;
			self.builder.build_store(id_pointer, id)?;
			let mut position = self.packed_offset(at, i64.const_int(1, false))?;
			if let Some(data) = member.data {
				let data_pointer = self.builder.build_struct_gep(ty, region, 1, "")?;
				for leaf in &member.leaves {
					let pointer = self.leaf_pointer(data, data_pointer, &leaf.path)?;
					if leaf.repr == Repr::Str {
						let (value, after) = self.packed_load_string(position)?;
						self.builder.build_store(pointer, value)?;
						position = after;
						continue;
					}
					let stored: BasicTypeEnum<'ctx> = if leaf.repr == Repr::Bool {
						i8.into()
					} else {
						self.llvm_type(leaf.repr)
					};
					let load = self.builder.build_load(stored, position, "")?;
					load.as_instruction_value()
						.expect("a load")
						.set_alignment(1)
						.map_err(|e| Error::Llvm(e.to_string()))?;
					let value: BasicValueEnum<'ctx> = if leaf.repr == Repr::Bool {
						self.builder
							.build_int_compare(
								IntPredicate::NE,
								load.into_int_value(),
								i8.const_zero(),
								"",
							)?
							.into()
					} else {
						load
					};
					self.builder.build_store(pointer, value)?;
					position = self
						.packed_offset(position, i64.const_int(self.leaf_size(leaf.repr), false))?;
				}
			}
			self.builder.build_store(next_slot, position)?;
			self.builder.build_unconditional_branch(body_block)?;
		}
		self.builder.position_at_end(body_block);
		let copy_slot = match (variable, copied) {
			(Some(symbol), true) => Some(self.slot_of(symbol, node)?),
			_ => None,
		};
		match (variable, copy_slot) {
			(Some(symbol), None) => {
				self.slots.insert(symbol, item_slot);
			}
			(_, Some(copy)) => {
				let value = self.load(item_slot)?;
				let value = self.emit_copy(value)?;
				self.store(copy, value, node)?;
			}
			_ => {}
		}
		if let Some(index_slot) = index_slot {
			let position = self.builder.build_load(i64, counter, "")?;
			self.store(
				index_slot,
				Val::new(Repr::Int(super::IntRepr::I64), position),
				node,
			)?;
		}
		let mark = self.mark();
		if let Some(copy) = copy_slot {
			self.cleanups
				.borrow_mut()
				.push(super::ownership::Cleanup::Local(copy));
		}
		self.frame_mut().loops.push(LoopFrame {
			exit,
			next: step,
			value: None,
			mark,
		});
		self.coop_point()?;
		self.block(body, false)?;
		self.frame_mut().loops.pop();
		self.end_scope(mark, None)?;
		self.branch_to(step)?;
		self.builder.position_at_end(step);
		let next = self.builder.build_load(pointer_type, next_slot, "")?;
		self.builder.build_store(cursor, next)?;
		let position = self.builder.build_load(i64, counter, "")?.into_int_value();
		let position = self
			.builder
			.build_int_nsw_add(position, i64.const_int(1, false), "")?;
		self.builder.build_store(counter, position)?;
		self.builder.build_unconditional_branch(test)?;
		self.builder.position_at_end(exit);
		Ok(Val::void())
	}
}

#[cfg(test)]
mod tests {
	/// Whether the program's arrays of `Event` may be packed.
	fn packs(body: &str) -> bool {
		let source = format!(
			"type Event = Union {{\n\tTick\n\tSay = {{ id: Int32, text: String }}\n}}\nmutable log: Array<Event> = []\nlog.push(Event.Tick)\n{body}"
		);
		let (session, file) = fa_analyzer::session::analyze_source(&source);
		assert!(
			session.analysis(file).diagnostics.is_empty(),
			"{body}: {:?}",
			session.analysis(file).diagnostics
		);
		!super::packable_items(&session, &[(file, String::new())]).is_empty()
	}

	#[test]
	fn arrays_built_by_push_and_read_by_for_are_packed() {
		assert!(packs(
			"for event in log {\n\tconsole.log(event)\n}\nconsole.log(log.length)\n"
		));
		assert!(packs(
			"function add = () mutates log => {\n\tlog.push(Event.Say { id = 1, text = \"a\" })\n}\nadd()\n"
		));
	}

	#[test]
	fn arrays_used_otherwise_are_not_packed() {
		for body in [
			// read at an index, printed, passed, copied, replaced
			"console.log(log[0])\n",
			"console.log(log)\n",
			"function count = (events: Array<Event>): Integer => events.length\nconsole.log(count(log))\n",
			"let copy = log\nconsole.log(copy.length)\n",
			"log = []\n",
			"mutable other: Array<Event> = [Event.Tick]\nother.push(Event.Tick)\n",
			// captured by a closure
			"console.log([1].map((x) => x + log.length))\n",
			// another method
			"log.reverseInPlace()\n",
		] {
			assert!(!packs(body), "{body}");
		}
	}
}
