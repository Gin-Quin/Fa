//! Strings and printing: literals, interpolation, concatenation, `length`,
//! `toString` through the runtime's text builder (values brief §2.8, §7).

use fa_analyzer::ids::NodeId;
use fa_analyzer::symbols::Resolution;
use fa_analyzer::syntax::{TemplatePart, TemplateParts};
use inkwell::IntPredicate;
use inkwell::values::{IntValue, PointerValue};

use crate::error::Error;
use crate::lower::values::{IntRepr, Repr, Val};
use crate::lower::{Lowering, Slot};

impl<'ctx> Lowering<'_, 'ctx> {
	/// Decimal integers through 64 bits fit directly in the native small
	/// string. No shared text builder, scratch collection, or heap copy.
	pub(crate) fn integer_string(&mut self, value: Val<'ctx>) -> Result<Option<Val<'ctx>>, Error> {
		let Repr::Int(int) = value.repr else {
			return Ok(None);
		};
		if int.bits > 64 {
			return Ok(None);
		}
		let raw = if int.signed {
			self.builder
				.build_int_s_extend_or_bit_cast(value.int(), self.context.i64_type(), "")?
		} else {
			self.builder
				.build_int_z_extend_or_bit_cast(value.int(), self.context.i64_type(), "")?
		};
		let name = if int.signed {
			"fa_string_i64"
		} else {
			"fa_string_u64"
		};
		let text = self.call_with_pair(name, &[raw.into()], Repr::Str)?;
		Ok(Some(self.fresh(text)))
	}

	fn single_integer_template(
		&mut self,
		parts: TemplateParts<'_>,
	) -> Result<Option<Val<'ctx>>, Error> {
		let mut meaningful = parts
			.iter()
			.filter(|p| !matches!(p, TemplatePart::Text(text) if text.is_empty()));
		if let Some(TemplatePart::Expression(node)) = meaningful.next()
			&& meaningful.next().is_none()
			&& matches!(self.node_repr(node)?, Repr::Int(int) if int.bits <= 64)
		{
			let value = self.expr(node)?;
			if value.repr == Repr::Never {
				return Ok(Some(value));
			}
			return self.integer_string(value);
		}
		Ok(None)
	}

	/// Common textual/scalar templates need no shared builder. Evaluate all
	/// expressions first, then borrow their values for one exact-sized join.
	fn string_template(&mut self, parts: TemplateParts<'_>) -> Result<Option<Val<'ctx>>, Error> {
		for part in parts.iter() {
			if let TemplatePart::Expression(node) = part {
				match self.node_repr(node)? {
					Repr::Str => {}
					Repr::Int(int) if int.bits <= 64 => {}
					_ => return Ok(None),
				}
			}
		}
		let mut values = Vec::new();
		for part in parts.iter() {
			let value = match part {
				TemplatePart::Text("") => continue,
				TemplatePart::Text(text) => self.string_constant(text),
				TemplatePart::Expression(node) => {
					let value = self.expr(node)?;
					if value.repr == Repr::Never {
						return Ok(Some(value));
					}
					if value.repr == Repr::Str {
						value
					} else {
						self.integer_string(value)?.expect("a checked integer")
					}
				}
			};
			values.push(value);
		}
		let ty = self.string_type.array_type(values.len() as u32);
		let storage = self.alloca_type(ty.into(), "template_parts")?;
		let zero = self.context.i32_type().const_zero();
		for (index, value) in values.iter().enumerate() {
			let index = self.context.i32_type().const_int(index as u64, false);
			let slot = unsafe {
				self.builder
					.build_in_bounds_gep(ty, storage, &[zero, index], "")?
			};
			self.builder
				.build_store(slot, value.value.expect("a string"))?;
		}
		let count = self.usize.const_int(values.len() as u64, false);
		let result = self.call_with_pair(
			"fa_strings_join",
			&[storage.into(), count.into()],
			Repr::Str,
		)?;
		Ok(Some(self.fresh(result)))
	}

	/// Inline strings have zero padding after their bytes. Two such values
	/// compare in three words; the optional SIMPLE optimization flag is ignored.
	pub(crate) fn strings_equal(
		&mut self,
		left: Val<'ctx>,
		right: Val<'ctx>,
	) -> Result<IntValue<'ctx>, Error> {
		let a = left.value.expect("a string").into_struct_value();
		let b = right.value.expect("a string").into_struct_value();
		let ac = self.string_capacity(left)?;
		let bc = self.string_capacity(right)?;
		let both = self.builder.build_and(ac, bc, "")?;
		let flag = self
			.builder
			.build_and(both, self.string_inline_flag(), "")?;
		let inline =
			self.builder
				.build_int_compare(IntPredicate::NE, flag, self.usize.const_zero(), "")?;
		let small = self.append_block("inline_equal");
		let external = self.append_block("external_equal");
		let end = self.append_block("string_equal");
		self.builder
			.build_conditional_branch(inline, small, external)?;
		self.builder.position_at_end(small);
		let mut same = self.context.bool_type().const_int(1, false);
		for index in 0..3 {
			let av = self.builder.build_extract_value(a, index, "")?;
			let bv = self.builder.build_extract_value(b, index, "")?;
			let (av, bv) = if index == 0 {
				(
					self.builder
						.build_ptr_to_int(av.into_pointer_value(), self.usize, "")?,
					self.builder
						.build_ptr_to_int(bv.into_pointer_value(), self.usize, "")?,
				)
			} else if index == 2 {
				let mask = self.simple_flag().const_not();
				(
					self.builder.build_and(av.into_int_value(), mask, "")?,
					self.builder.build_and(bv.into_int_value(), mask, "")?,
				)
			} else {
				(av.into_int_value(), bv.into_int_value())
			};
			let equal = self
				.builder
				.build_int_compare(IntPredicate::EQ, av, bv, "")?;
			same = self.builder.build_and(same, equal, "")?;
		}
		self.builder.build_unconditional_branch(end)?;
		self.builder.position_at_end(external);
		let (ap, al) = self.string_parts(left)?;
		let (bp, bl) = self.string_parts(right)?;
		let equal = self
			.call_value(
				self.runtime.function("fa_string_equal")?,
				&[ap.into(), al.into(), bp.into(), bl.into()],
			)?
			.into_int_value();
		let equal = self.builder.build_int_compare(
			IntPredicate::NE,
			equal,
			self.context.i32_type().const_zero(),
			"",
		)?;
		let external_end = self.builder.get_insert_block().expect("a block");
		self.builder.build_unconditional_branch(end)?;
		self.builder.position_at_end(end);
		let result = self.builder.build_phi(self.context.bool_type(), "equal")?;
		result.add_incoming(&[(&same, small), (&equal, external_end)]);
		Ok(result.as_basic_value().into_int_value())
	}

	/// Append without exposing the destination header to the slow runtime
	/// call. The ordinary path updates SSA values; only growth spills a
	/// snapshot, allowing LLVM to promote a local header across the loop.
	pub(crate) fn append_string(
		&mut self,
		target: PointerValue<'ctx>,
		suffix: Val<'ctx>,
	) -> Result<(), Error> {
		let slot = Slot {
			pointer: target,
			repr: Repr::Str,
		};
		let current = self.load(slot)?;
		// The heap path can use the raw fields. Decoding the inline length
		// here would add shifts/selects to every byte of a large append loop.
		let pair = current.value.expect("a string").into_struct_value();
		let bytes = self
			.builder
			.build_extract_value(pair, 0, "")?
			.into_pointer_value();
		let length = self
			.builder
			.build_extract_value(pair, 1, "")?
			.into_int_value();
		let capacity = self.string_capacity(current)?;
		let (suffix_bytes, suffix_length) = self.string_parts(suffix)?;
		let suffix_capacity = self.string_capacity(suffix)?;
		let needed = self.builder.build_int_add(length, suffix_length, "")?;
		let mask = self.string_kind_mask();
		let kind = self
			.builder
			.build_and(capacity, self.simple_flag().const_not(), "")?;
		let simple = self.builder.build_int_compare(
			IntPredicate::SLT,
			suffix_capacity,
			self.usize.const_zero(),
			"",
		)?;
		let room = self
			.builder
			.build_int_compare(IntPredicate::ULE, needed, kind, "")?;
		let owned = self
			.builder
			.build_int_compare(IntPredicate::ULT, kind, mask, "")?;
		let fast = self.builder.build_and(simple, room, "")?;
		let fast = self.builder.build_and(fast, owned, "")?;
		let append = self.append_block("append");
		let grow = self.append_block("append_slow");
		let join = self.append_block("appended");
		let branch = self.builder.build_conditional_branch(fast, append, grow)?;
		self.likely(branch)?;
		self.builder.position_at_end(append);
		let destination = unsafe {
			self.builder
				.build_in_bounds_gep(self.context.i8_type(), bytes, &[length], "")?
		};
		if suffix_length.get_zero_extended_constant().is_some() {
			self.builder
				.build_memcpy(destination, 1, suffix_bytes, 1, suffix_length)?;
		} else {
			self.call_runtime(
				"fa_copy_bytes",
				&[
					destination.into(),
					suffix_bytes.into(),
					suffix_length.into(),
				],
			)?;
		}
		let updated = self.builder.build_insert_value(
			current.value.expect("a string").into_struct_value(),
			needed,
			1,
			"",
		)?;
		self.builder.build_store(target, updated)?;
		self.builder.build_unconditional_branch(join)?;
		self.builder.position_at_end(grow);
		let snapshot = self.alloca(Repr::Str, "append_header")?;
		self.builder
			.build_store(snapshot.pointer, current.value.expect("a string"))?;
		self.call_runtime(
			"fa_string_append",
			&[
				snapshot.pointer.into(),
				suffix_bytes.into(),
				suffix_length.into(),
				suffix_capacity.into(),
			],
		)?;
		let updated = self.load(snapshot)?;
		self.builder
			.build_store(target, updated.value.expect("a string"))?;
		self.builder.build_unconditional_branch(join)?;
		self.builder.position_at_end(join);
		Ok(())
	}

	/// The bytes and the length of a string value.
	pub(crate) fn string_parts(
		&self,
		value: Val<'ctx>,
	) -> Result<(PointerValue<'ctx>, IntValue<'ctx>), Error> {
		self.string_parts_with(&self.builder, value)
	}

	/// Decode a String in either representation. Inline bytes are a snapshot
	/// of the SSA value, so an intervening write to its original place cannot
	/// change them. Allocating in the entry block avoids growing a loop's stack.
	pub(crate) fn string_parts_with(
		&self,
		builder: &inkwell::builder::Builder<'ctx>,
		value: Val<'ctx>,
	) -> Result<(PointerValue<'ctx>, IntValue<'ctx>), Error> {
		let pair = value.value.expect("a string").into_struct_value();
		let bytes = builder
			.build_extract_value(pair, 0, "")?
			.into_pointer_value();
		let length = builder.build_extract_value(pair, 1, "")?.into_int_value();
		let capacity = builder.build_extract_value(pair, 2, "")?.into_int_value();
		let flag = builder.build_and(capacity, self.string_inline_flag(), "")?;
		let inline =
			builder.build_int_compare(IntPredicate::NE, flag, self.usize.const_zero(), "")?;
		let function = builder
			.get_insert_block()
			.expect("a block")
			.get_parent()
			.expect("a function");
		let entry = function.get_first_basic_block().expect("an entry");
		let allocator = self.context.create_builder();
		if let Some(first) = entry.get_first_instruction() {
			allocator.position_before(&first);
		} else {
			allocator.position_at_end(entry);
		}
		let snapshot = allocator.build_alloca(self.string_type, "inline_string")?;
		builder.build_store(snapshot, pair)?;
		let bytes = builder
			.build_select(inline, snapshot, bytes, "")?
			.into_pointer_value();
		let shifted = builder.build_right_shift(
			capacity,
			self.usize
				.const_int((self.usize.get_bit_width() - 8) as u64, false),
			false,
			"",
		)?;
		let inline_length = builder.build_and(shifted, self.usize.const_int(31, false), "")?;
		let length = builder
			.build_select(inline, inline_length, length, "")?
			.into_int_value();
		Ok((bytes, length))
	}

	/// The capacity word of a string value (its top bit: `SIMPLE`, one byte
	/// per character; see `runtime/fa_runtime.c`).
	pub(crate) fn string_capacity(&self, value: Val<'ctx>) -> Result<IntValue<'ctx>, Error> {
		let pair = value.value.expect("a string").into_struct_value();
		Ok(self
			.builder
			.build_extract_value(pair, 2, "")?
			.into_int_value())
	}

	/// Calls a runtime function that writes a `{ ptr, usize }` to its last
	/// argument, and loads it.
	pub(crate) fn call_with_pair(
		&mut self,
		name: &str,
		arguments: &[inkwell::values::BasicMetadataValueEnum<'ctx>],
		repr: Repr,
	) -> Result<Val<'ctx>, Error> {
		let out = self.alloca(repr, "result")?;
		let mut all = arguments.to_vec();
		all.push(out.pointer.into());
		self.call_runtime(name, &all)?;
		self.load(out)
	}

	/// `a + b` on strings.
	pub(crate) fn concat(&mut self, left: Val<'ctx>, right: Val<'ctx>) -> Result<Val<'ctx>, Error> {
		let (a, a_length) = self.string_parts(left)?;
		let (b, b_length) = self.string_parts(right)?;
		let result = self.call_with_pair(
			"fa_concat",
			&[a.into(), a_length.into(), b.into(), b_length.into()],
			Repr::Str,
		)?;
		Ok(self.fresh(result))
	}

	/// Appends the `toString` of a value to the runtime's text builder.
	pub(crate) fn append_value(&mut self, value: Val<'ctx>, node: NodeId) -> Result<(), Error> {
		match value.repr {
			Repr::Str => {
				let (bytes, length) = self.string_parts(value)?;
				self.call_runtime("fa_b_bytes", &[bytes.into(), length.into()])
			}
			Repr::Int(int) => {
				let raw = value.int();
				if int.bits <= 64 {
					let i64 = self.context.i64_type();
					let (name, wide) = match int.signed {
						true if int.bits < 64 => {
							("fa_b_i64", self.builder.build_int_s_extend(raw, i64, "")?)
						}
						true => ("fa_b_i64", raw),
						false if int.bits < 64 => {
							("fa_b_u64", self.builder.build_int_z_extend(raw, i64, "")?)
						}
						false => ("fa_b_u64", raw),
					};
					self.call_runtime(name, &[wide.into()])
				} else {
					let i64 = self.context.i64_type();
					let low = self.builder.build_int_truncate(raw, i64, "")?;
					let shifted = self.builder.build_right_shift(
						raw,
						raw.get_type().const_int(64, false),
						false,
						"",
					)?;
					let high = self.builder.build_int_truncate(shifted, i64, "")?;
					let name = if int.signed { "fa_b_i128" } else { "fa_b_u128" };
					self.call_runtime(name, &[low.into(), high.into()])
				}
			}
			Repr::Bool => {
				let wide =
					self.builder
						.build_int_z_extend(value.int(), self.context.i32_type(), "")?;
				self.call_runtime("fa_b_bool", &[wide.into()])
			}
			Repr::F64 => self.call_runtime("fa_b_f64", &[value.value.expect("a float").into()]),
			Repr::F32 => self.call_runtime("fa_b_f32", &[value.value.expect("a float").into()]),
			Repr::Percentage => {
				self.call_runtime("fa_b_percentage", &[value.value.expect("a float").into()])
			}
			Repr::Enum(nominal) => {
				let name = self.enum_name(value, nominal)?;
				self.append_value(name, node)
			}
			Repr::Union | Repr::InlineUnion(_) => {
				let value = self.union_view(value)?;
				let nested = self.context.i32_type().const_zero();
				self.call_runtime(
					"fa_format_union",
					&[value.value.expect("a box").into(), nested.into()],
				)
			}
			Repr::Object(ty) => self.append_object(value, ty, node),
			Repr::Array(item) => self.append_array(value, item, node),
			Repr::Growing(item) => {
				let view = self.growing_view(value)?;
				self.append_array(view, item, node)
			}
			Repr::Map(_) | Repr::Set(_) => self.append_table(value, node),
			Repr::Pool(item) | Repr::Columnar(item) => {
				let view = self.pool_view(value, node)?;
				self.append_array(view, item, node)
			}
			Repr::Row(_) | Repr::Watch | Repr::Task(_) => {
				Err(self.unsupported(node.index(), "printing this value"))
			}
			Repr::Link(_) | Repr::Links(_) => {
				Err(self.unsupported(node.index(), "printing a relation side"))
			}
			Repr::None => {
				let none = self.string_constant("none");
				self.append_value(none, node)
			}
			Repr::Optional(_) => self.append_optional(value, node, false),
			Repr::Function(_) => Err(self.unsupported(node.index(), "printing a function")),
			Repr::Pointer => Err(self.unsupported(node.index(), "printing a pointer")),
			Repr::Sync(_) => Err(self.unsupported(node.index(), "printing a thread-safe value")),
			Repr::Tuple(ty) => self.append_tuple(value, ty, node),
			Repr::Range => self.append_range(value, node),
			Repr::Rec(target) => {
				let unfolded = self.unfold_rec(value, target, node.index())?;
				self.append_value(unfolded, node)
			}
			Repr::Void | Repr::Never => {
				Err(self.unsupported(node.index(), "printing a value of type `Void`"))
			}
		}
	}

	/// The name of a member of an enumeration (`red`).
	pub(crate) fn enum_name(
		&mut self,
		value: Val<'ctx>,
		nominal: fa_analyzer::ids::NominalId,
	) -> Result<Val<'ctx>, Error> {
		// `red`: a table of the names of the members, by index
		let types = self.types();
		let info = types.nominal(nominal);
		let names: Vec<String> = types
			.sym_list(info.members)
			.iter()
			.map(|&member| types.syms.resolve(member).to_string())
			.collect();
		let entries: Vec<_> = names
			.iter()
			.map(|name| {
				let pointer = self.constant_bytes(name.as_bytes());
				let length = self.usize.const_int(name.len() as u64, false);
				self.string_type.const_named_struct(&[
					pointer.into(),
					length.into(),
					self.static_capacity(name).into(),
				])
			})
			.collect();
		let table_type = self.string_type.array_type(entries.len() as u32);
		let table = self.module.add_global(table_type, None, "fa.enum_names");
		table.set_initializer(&self.string_type.const_array(&entries));
		table.set_constant(true);
		table.set_linkage(inkwell::module::Linkage::Private);
		let index = value.int();
		let zero = self.context.i32_type().const_zero();
		let entry = unsafe {
			self.builder.build_in_bounds_gep(
				table_type,
				table.as_pointer_value(),
				&[zero, index],
				"",
			)?
		};
		let name = self.builder.build_load(self.string_type, entry, "")?;
		Ok(Val::new(Repr::Str, name))
	}

	/// An optional: `none`, or its value (quoted when `nested`).
	pub(crate) fn append_optional(
		&mut self,
		value: Val<'ctx>,
		node: NodeId,
		nested: bool,
	) -> Result<(), Error> {
		let Repr::Optional(_) = value.repr else {
			return self.append_value(value, node);
		};
		let (present, inner) = self.split_optional(&self.builder, value)?;
		let some = self.append_block("print_some");
		let none = self.append_block("print_none");
		let join = self.append_block("printed");
		self.builder.build_conditional_branch(present, some, none)?;
		self.builder.position_at_end(some);
		if nested {
			self.append_nested(inner, node)?;
		} else {
			self.append_value(inner, node)?;
		}
		self.builder.build_unconditional_branch(join)?;
		self.builder.position_at_end(none);
		let text = self.string_constant("none");
		self.append_value(text, node)?;
		self.builder.build_unconditional_branch(join)?;
		self.builder.position_at_end(join);
		Ok(())
	}

	/// `"text {value} text"`: every part is evaluated first, then the text is built.
	pub(crate) fn template(&mut self, parts: TemplateParts<'_>) -> Result<Val<'ctx>, Error> {
		if let Some(value) = self.single_integer_template(parts)? {
			return Ok(value);
		}
		if let Some(value) = self.string_template(parts)? {
			return Ok(value);
		}
		if !self.build_template(parts)? {
			return Ok(Val::never());
		}
		let result = self.call_with_pair("fa_b_take", &[], Repr::Str)?;
		Ok(self.fresh(result))
	}

	/// `text += "… {value} …"`: the template is built in the runtime's text
	/// builder and appended from there, without a string of its own.
	pub(crate) fn append_template(
		&mut self,
		target: PointerValue<'ctx>,
		parts: TemplateParts<'_>,
	) -> Result<Val<'ctx>, Error> {
		if let Some(value) = self.single_integer_template(parts)? {
			if value.repr == Repr::Never {
				return Ok(value);
			}
			self.append_string(target, value)?;
			return Ok(Val::void());
		}
		// A template of disjoint string locals and literals can be appended
		// directly. No global builder, rescan for ASCII, or intermediate copy.
		// Restrict this to owning bindings: `use` aliases and a read of the
		// destination itself must retain the snapshot semantics of templates.
		let mut direct = true;
		for part in parts.iter() {
			if let TemplatePart::Expression(node) = part {
				let Resolution::Local(symbol) = self.analysis.resolution(node) else {
					direct = false;
					break;
				};
				let kind = self.analysis.symbols.symbol(symbol).kind;
				if !matches!(
					kind,
					fa_analyzer::symbols::SymbolKind::Let
						| fa_analyzer::symbols::SymbolKind::Mutable
				) || self.node_repr(node)? != Repr::Str
					|| self.slot_of(symbol, node)?.pointer == target
				{
					direct = false;
					break;
				}
			}
		}
		if direct {
			for part in parts.iter() {
				let value = match part {
					TemplatePart::Text(text) => self.string_constant(text),
					TemplatePart::Expression(node) => self.expr(node)?,
				};
				self.append_string(target, value)?;
			}
			return Ok(Val::void());
		}
		if !self.build_template(parts)? {
			return Ok(Val::never());
		}
		self.call_runtime("fa_b_append_into", &[target.into()])?;
		Ok(Val::void())
	}

	/// Builds a template in the text builder; `false` when a part never ends.
	fn build_template(&mut self, parts: TemplateParts<'_>) -> Result<bool, Error> {
		let mut values = Vec::new();
		for part in parts.iter() {
			match part {
				TemplatePart::Text(text) => values.push((None, Some(text.to_string()))),
				TemplatePart::Expression(node) => {
					let value = self.expr(node)?;
					if value.repr == Repr::Never {
						return Ok(false);
					}
					values.push((Some((value, node)), None));
				}
			}
		}
		self.call_runtime("fa_b_reset", &[])?;
		for (value, text) in values {
			match (value, text) {
				(Some((value, node)), _) => self.append_value(value, node)?,
				(None, Some(text)) => {
					let text = self.string_constant(&text);
					self.append_value(text, NodeId::new(0))?;
				}
				(None, None) => {}
			}
		}
		Ok(true)
	}

	/// `console.log(values…)` (fd 1) and `console.error(values…)` (fd 2):
	/// the values separated by a space, then a new line.
	pub(crate) fn print(&mut self, fd: u64, arguments: &[NodeId]) -> Result<Val<'ctx>, Error> {
		let fd_value = self.context.i32_type().const_int(fd, false);
		// only literals: one write of a constant line
		let literals: Option<Vec<&str>> = arguments
			.iter()
			.map(|&argument| match self.view(argument) {
				fa_analyzer::syntax::View::String(text) => Some(text),
				_ => None,
			})
			.collect();
		if let Some(literals) = literals {
			let line = format!("{}\n", literals.join(" "));
			let bytes = self.constant_bytes(line.as_bytes());
			let length = self.usize.const_int(line.len() as u64, false);
			self.call_runtime("fa_out", &[fd_value.into(), bytes.into(), length.into()])?;
			return Ok(Val::void());
		}
		let mut values = Vec::new();
		for &argument in arguments {
			let value = self.expr(argument)?;
			if value.repr == Repr::Never {
				return Ok(Val::never());
			}
			values.push((value, argument));
		}
		self.call_runtime("fa_b_reset", &[])?;
		for (index, (value, node)) in values.into_iter().enumerate() {
			if index > 0 {
				let space = self.string_constant(" ");
				self.append_value(space, node)?;
			}
			self.append_value(value, node)?;
		}
		self.call_runtime("fa_b_line", &[fd_value.into()])?;
		Ok(Val::void())
	}

	/// `value.property` of a built-in type, on the evaluated value.
	pub(crate) fn builtin_property(
		&mut self,
		node: NodeId,
		value: Val<'ctx>,
		member: NodeId,
	) -> Result<Val<'ctx>, Error> {
		let Resolution::Builtin(id) = self.analysis.resolution(member) else {
			return Err(self.unsupported(node.index(), "this member access"));
		};
		match fa_analyzer::builtins::get(id).path {
			"Pointer#value" => {
				let target = self.type_of(member);
				self.pointer_value(value, target, member)
			}
			"String#length" => {
				let (bytes, length) = self.string_parts(value)?;
				let capacity = self.string_capacity(value)?;
				let graphemes = self.runtime.function("fa_string_length")?;
				let count = self
					.call_value(graphemes, &[bytes.into(), length.into(), capacity.into()])?
					.into_int_value();
				let i64 = self.context.i64_type();
				let count = self
					.builder
					.build_int_z_extend_or_bit_cast(count, i64, "")?;
				Ok(Val::new(
					Repr::Int(crate::lower::values::IntRepr::I64),
					count,
				))
			}
			"Array#length" if matches!(value.repr, Repr::Growing(_)) => {
				let array = value.value.expect("an array").into_pointer_value();
				Ok(Val::new(
					Repr::Int(crate::lower::values::IntRepr::I64),
					self.growing_length(&self.builder, array)?,
				))
			}
			"Array#length" => {
				if let fa_analyzer::syntax::View::Access { target, members } = self.view(node)
					&& members.len() == 1
					&& members.first() == Some(member)
					&& let Some(length) = self.constant_array_length(target)
				{
					return Ok(Val::new(
						Repr::Int(IntRepr::I64),
						self.context.i64_type().const_int(length as u64, false),
					));
				}
				self.array_length(value)
			}

			"Set#size" | "Map#size" => self.table_size(value),
			"Pool#size" | "Columnar#size" => self.pool_size(value),
			"RelationPool#size" => self.links_size_value(value),
			"OrderedColumnar#length" => self.array_length(value),
			"Range#start" => self.range_property(member, value, "start"),
			"Range#end" => self.range_property(member, value, "end"),
			"Range#includesEnd" => self.range_property(member, value, "includesEnd"),
			"Guard#value" => self.guard_value(value, member),
			path => Err(self.unsupported(node.index(), &format!("`{path}`"))),
		}
	}
}
