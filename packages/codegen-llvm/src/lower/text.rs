//! Strings and printing: literals, interpolation, concatenation, `length`,
//! `toString` through the runtime's text builder (values brief §2.8, §7).

use fa_analyzer::ids::NodeId;
use fa_analyzer::symbols::Resolution;
use fa_analyzer::syntax::{TemplatePart, TemplateParts};
use inkwell::values::{IntValue, PointerValue};

use crate::error::Error;
use crate::lower::Lowering;
use crate::lower::values::{Repr, Val};

impl<'ctx> Lowering<'_, 'ctx> {
	/// The bytes and the length of a string value.
	pub(crate) fn string_parts(
		&self,
		value: Val<'ctx>,
	) -> Result<(PointerValue<'ctx>, IntValue<'ctx>), Error> {
		let pair = value.value.expect("a string").into_struct_value();
		let bytes = self
			.builder
			.build_extract_value(pair, 0, "")?
			.into_pointer_value();
		let length = self
			.builder
			.build_extract_value(pair, 1, "")?
			.into_int_value();
		Ok((bytes, length))
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
			Repr::Union => {
				let nested = self.context.i32_type().const_zero();
				self.call_runtime(
					"fa_format_union",
					&[value.value.expect("a box").into(), nested.into()],
				)
			}
			Repr::Object(ty) => self.append_object(value, ty, node),
			Repr::Array(item) => self.append_array(value, item, node),
			Repr::None => {
				let none = self.string_constant("none");
				self.append_value(none, node)
			}
			Repr::Optional(_) => self.append_optional(value, node, false),
			Repr::Function(_) => Err(self.unsupported(node.index(), "printing a function")),
			Repr::Tuple(ty) => self.append_tuple(value, ty, node),
			Repr::Range => self.append_range(value, node),
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
				self.string_type
					.const_named_struct(&[pointer.into(), length.into(), self.usize.const_zero().into()])
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
		let Repr::Optional(scalar) = value.repr else {
			return self.append_value(value, node);
		};
		let pair = value.value.expect("an optional").into_struct_value();
		let present = self
			.builder
			.build_extract_value(pair, 0, "")?
			.into_int_value();
		let inner = self.builder.build_extract_value(pair, 1, "")?;
		let some = self.append_block("print_some");
		let none = self.append_block("print_none");
		let join = self.append_block("printed");
		self.builder.build_conditional_branch(present, some, none)?;
		self.builder.position_at_end(some);
		let inner = Val::new(scalar.repr(), inner);
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
		let mut values = Vec::new();
		for part in parts.iter() {
			match part {
				TemplatePart::Text(text) => values.push((None, Some(text.to_string()))),
				TemplatePart::Expression(node) => {
					let value = self.expr(node)?;
					if value.repr == Repr::Never {
						return Ok(Val::never());
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
		let result = self.call_with_pair("fa_b_take", &[], Repr::Str)?;
		Ok(self.fresh(result))
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
			"String#length" => {
				let (bytes, length) = self.string_parts(value)?;
				let graphemes = self.runtime.function("fa_graphemes")?;
				let count = self
					.call_value(graphemes, &[bytes.into(), length.into()])?
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
			"Array#length" => self.array_length(value),
			"Range#start" => self.range_property(member, value, "start"),
			"Range#end" => self.range_property(member, value, "end"),
			"Range#includesEnd" => self.range_property(member, value, "includesEnd"),
			path => Err(self.unsupported(node.index(), &format!("`{path}`"))),
		}
	}
}
