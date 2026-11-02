//! Calls: functions of the file (arguments in parameter order from
//! [`CallInfo`], defaults evaluated by the callee), built-ins, and number
//! conversions with their `CONVERSION_CHECK`.

use ethnum::I256;
use fa_analyzer::analysis::{Argument, CallInfo, Callee, NodeFlags};
use fa_analyzer::ids::{NodeId, TypeId};
use fa_analyzer::syntax::View;
use inkwell::FloatPredicate;
use inkwell::values::{BasicMetadataValueEnum, BasicValue, PointerValue};

use crate::error::Error;
use crate::lower::values::{IntRepr, Repr, Scalar, Val};
use crate::lower::{FAST_CALLING_CONVENTION, Lowering};

impl<'ctx> Lowering<'_, 'ctx> {
	pub(crate) fn call(&mut self, node: NodeId) -> Result<Val<'ctx>, Error> {
		// `value?.method()`
		if let View::Call { callee, .. } = self.view(node)
			&& let View::OptionalAccess { target, members } = self.view(callee)
		{
			return self.optional_call(node, target, members);
		}
		self.call_present(node)
	}

	/// A call whose receiver, if any, is present.
	pub(crate) fn call_present(&mut self, node: NodeId) -> Result<Val<'ctx>, Error> {
		let Some(info) = self.analysis.call(node).cloned() else {
			// `x.abs()` is analyzed as a number method, without a `CallInfo`
			if let View::Call { callee, arguments } = self.view(node)
				&& arguments.is_empty()
				&& let View::Access { target, members } = self.view(callee)
				&& members.len() == 1
				&& self.tree.text(members.last().expect("a member").index()) == "abs"
			{
				return self.absolute(node, target);
			}
			return Err(self.unsupported(node.index(), "this call"));
		};
		match info.callee {
			Callee::Function(declaration) => self.direct_call(node, declaration, &info),
			Callee::Builtin(id) => {
				self.builtin_call(node, fa_analyzer::builtins::get(id).path, &info)
			}
			Callee::Conversion(target) => {
				let Some(Argument::Given(argument)) = info.arguments.first() else {
					return Err(self.unsupported(node.index(), "this conversion"));
				};
				self.conversion(node, *argument, target)
			}
			Callee::Value => {
				let View::Call { callee, .. } = self.view(node) else {
					return Err(self.unsupported(node.index(), "this call"));
				};
				// a function of another file, or of a namespace: a direct call
				if let Some(function) = self.callee_function(callee) {
					return self.external_call(node, function, &info);
				}

				let closure = self.expr(callee)?;
				if closure.repr == Repr::Never {
					return Ok(closure);
				}
				let mut arguments = Vec::new();
				for argument in &info.arguments {
					let Argument::Given(argument) = argument else {
						return Err(self.unsupported(node.index(), "these arguments"));
					};
					let value = self.expr(*argument)?;
					if value.repr == Repr::Never {
						return Ok(value);
					}
					arguments.push(value);
				}
				let ty = self.type_of(callee);
				self.call_closure(closure, ty, &arguments, node)
			}
			Callee::Member(member) => {
				let Some((nominal, index)) = self.member_of_type(member) else {
					return Err(self.unsupported(node.index(), "this member"));
				};
				let data = match info.arguments.first() {
					Some(Argument::Given(argument)) => Some(self.expr(*argument)?),
					_ => None,
				};
				if data.is_some_and(|data| data.repr == Repr::Never) {
					return Ok(Val::never());
				}
				self.build_member(nominal, index, data, node)
			}
		}
	}

	fn direct_call(
		&mut self,
		node: NodeId,
		declaration: NodeId,
		info: &CallInfo,
	) -> Result<Val<'ctx>, Error> {
		let function = self.function_in(self.file, declaration, node)?;
		self.call_function(node, function, info)
	}

	/// A direct call of a function (of this file or another), the
	/// arguments in parameter order from `info`.
	pub(crate) fn call_function(
		&mut self,
		node: NodeId,
		function: super::Function<'ctx>,
		info: &CallInfo,
	) -> Result<Val<'ctx>, Error> {
		let (value, params, ret, sig, throws) = (
			function.value,
			function.params.clone(),
			function.ret,
			function.info.sig,
			function.throws,
		);
		let by_pointer = function.by_pointer.clone();
		// the places given to parameters passed by pointer, and what to write
		// back after the call
		let mut places: rustc_hash::FxHashMap<usize, PointerValue<'ctx>> = Default::default();
		let mut write_backs = Vec::new();
		// the given arguments are evaluated in the written order (named
		// arguments may come in another order than the parameters)
		let mut given: Vec<(usize, NodeId)> = info
			.arguments
			.iter()
			.enumerate()
			.filter_map(|(position, argument)| match argument {
				Argument::Given(value) => Some((position, *value)),
				_ => None,
			})
			.collect();
		given.sort_by_key(|(_, value)| self.tree.spans[value.index()].start);
		let mut evaluated: rustc_hash::FxHashMap<usize, Val<'ctx>> = Default::default();
		let receiver_first = matches!(info.arguments.first(), Some(Argument::Receiver(_)));
		let mut receiver_value = None;
		if receiver_first && by_pointer.first() == Some(&true) {
			let (pointer, write_back) = self.argument_place(node, None, params[0].0)?;
			places.insert(0, pointer);
			write_backs.extend(write_back);
		} else if receiver_first {
			let lowered = self.receiver(node)?;
			if lowered.repr == Repr::Never {
				return Ok(Val::never());
			}
			receiver_value = Some(lowered);
		}
		for (position, value) in given {
			if by_pointer.get(position) == Some(&true) {
				let (pointer, write_back) =
					self.argument_place(node, Some(value), params[position].0)?;
				places.insert(position, pointer);
				write_backs.extend(write_back);
				continue;
			}
			let lowered = self.expr(value)?;
			if lowered.repr == Repr::Never {
				return Ok(Val::never());
			}
			evaluated.insert(position, lowered);
		}
		let mut arguments: Vec<BasicMetadataValueEnum<'ctx>> = Vec::new();
		if function.lambda {
			// a local function using locals around it: its environment
			let (captures, by_reference) = (function.captures.clone(), function.by_reference);
			let environment = self.build_environment(&captures, by_reference, node)?;
			arguments.push(environment.into());
		}
		for (position, ((repr, has_default), argument)) in
			params.into_iter().zip(&info.arguments).enumerate()
		{
			if let Some(pointer) = places.get(&position) {
				arguments.push((*pointer).into());
				continue;
			}
			match argument {
				Argument::Receiver(_) => {
					let lowered = match receiver_value.take() {
						Some(value) => value,
						None => self.receiver(node)?,
					};
					let lowered = self.convert(lowered, repr, node.index())?;
					arguments.push(lowered.value.expect("a receiver").into());
				}
				Argument::Given(argument) => {
					let lowered = evaluated[&position];
					let lowered = self.convert(lowered, repr, argument.index())?;
					arguments.push(lowered.value.expect("an argument").into());
					if has_default {
						arguments.push(self.context.bool_type().const_int(1, false).into());
					}
				}
				Argument::Default if has_default => {
					let ty = self.llvm_type(repr);
					arguments.push(super::zero_of(ty).into());
					arguments.push(self.context.bool_type().const_zero().into());
				}
				Argument::Variadic(values) => {
					let Some(array) = self.variadic_array(repr, values, node)? else {
						return Ok(Val::never());
					};
					arguments.push(array.value.expect("an array").into());
				}
				_ => return Err(self.unsupported(node.index(), "these arguments")),
			}
		}
		if let Some(sig) = sig {
			let requires = self.types().sig(sig).requires;
			let keys: Vec<TypeId> = self
				.types()
				.requirements(requires)
				.iter()
				.map(|requirement| requirement.ty)
				.collect();
			for pointer in self.requirement_arguments(node, &keys)? {
				arguments.push(pointer.into());
			}
		}
		let call = self.builder.build_call(value, &arguments, "")?;
		call.set_call_convention(FAST_CALLING_CONVENTION);
		for write_back in write_backs {
			self.write_back(write_back)?;
		}
		if throws {
			let result = call.try_as_basic_value().basic().expect("a result");
			let (result, error) = if ret.has_value() {
				let pair = result.into_struct_value();
				(
					Some(self.builder.build_extract_value(pair, 0, "")?),
					self.builder
						.build_extract_value(pair, 1, "")?
						.into_pointer_value(),
				)
			} else {
				(None, result.into_pointer_value())
			};
			self.raise_if_error(error)?;
			return Ok(match result {
				Some(result) if ret != Repr::Never => self.fresh(Val::new(ret, result)),
				_ if ret == Repr::Never => {
					self.builder.build_unreachable()?;
					self.continue_unreachable();
					Val::never()
				}
				_ => Val::void(),
			});
		}
		if ret == Repr::Never {
			self.builder.build_unreachable()?;
			self.continue_unreachable();
			return Ok(Val::never());
		}
		let Some(result) = call.try_as_basic_value().basic() else {
			return Ok(Val::void());
		};
		// the range of a ranged result type
		// (only when overflows are checked: a wrapped value may be outside)
		if let (Repr::Int(int), Some(sig), true) = (ret, sig, self.overflow_checks()) {
			let ret_type = self.types().sig(sig).ret;
			if let Some(range) = self.types().as_int(ret_type).copied()
				&& !range.is_full()
				&& range.max < (I256::ONE << (int.bits - 1 + (!int.signed) as u32)) - 1
			{
				let low = self.const_int(int.bits, range.min);
				let high = self.const_int(int.bits, range.max + 1);
				let metadata = self.context.metadata_node(&[low.into(), high.into()]);
				if let Some(instruction) = result.as_instruction_value() {
					instruction
						.set_metadata(metadata, self.context.get_kind_id("range"))
						.map_err(|error| Error::Llvm(error.to_string()))?;
				}
			}
		}
		Ok(self.fresh(Val::new(ret, result)))
	}

	fn builtin_call(
		&mut self,
		node: NodeId,
		path: &str,
		info: &CallInfo,
	) -> Result<Val<'ctx>, Error> {
		match path {
			"console.log" | "console.error" => {
				let mut values = Vec::new();
				for argument in &info.arguments {
					match argument {
						Argument::Variadic(nodes) => {
							for &value in nodes {
								if matches!(self.view(value), View::Spread(_)) {
									return Err(
										self.unsupported(value.index(), "spreading arguments")
									);
								}
								values.push(value);
							}
						}
						Argument::Given(value) => values.push(*value),
						_ => {}
					}
				}
				self.print(if path == "console.log" { 1 } else { 2 }, &values)
			}
			_ if path.starts_with("String#") => self.string_method(node, path, info),
			_ if path.starts_with("Array#") => {
				let mut arguments = Vec::new();
				for argument in &info.arguments {
					match argument {
						Argument::Given(value) => arguments.push(*value),
						Argument::Receiver(_) => {}
						Argument::Default => {}
						_ => return Err(self.unsupported(node.index(), path)),
					}
				}
				self.array_method(node, path, &arguments)
			}
			"Enum#toString" => {
				let value = self.receiver(node)?;
				match value.repr {
					Repr::Enum(nominal) => self.enum_name(value, nominal),
					_ => Ok(value),
				}
			}
			"String.compareLocale" => {
				let mut values = Vec::new();
				for argument in &info.arguments {
					if let Argument::Given(value) = argument {
						let value = self.expr(*value)?;
						if value.repr == Repr::Never {
							return Ok(value);
						}
						values.push(value);
					}
				}
				let [a, b, ..] = values.as_slice() else {
					return Err(self.unsupported(node.index(), path));
				};
				let (a, a_length) = self.string_parts(*a)?;
				let (b, b_length) = self.string_parts(*b)?;
				let compare = self.runtime.function("fa_string_compare")?;
				let order = self
					.call_value(
						compare,
						&[a.into(), a_length.into(), b.into(), b_length.into()],
					)?
					.into_int_value();
				let order = self
					.builder
					.build_int_s_extend(order, self.context.i64_type(), "")?;
				Ok(Val::new(Repr::Int(IntRepr::I64), order))
			}
			_ if path.starts_with("Pool#") => self.pool_method(node, path, info),
			_ if path.starts_with("Set") || path.starts_with("Map") => {
				self.table_method(node, path, info)
			}
			"Array.from"
				if matches!(info.arguments.first(), Some(Argument::Given(source))
					if matches!(self.node_repr(*source)?, Repr::Map(_) | Repr::Set(_))) =>
			{
				let Some(Argument::Given(source)) = info.arguments.first() else {
					unreachable!("a source");
				};
				let source = self.expr(*source)?;
				let repr = self.node_repr(node)?;
				self.array_of_table(source, repr, node)
			}
			"Array.from" => {
				let Some(values) = self.array_from_arguments(info)? else {
					return Ok(Val::never());
				};
				self.array_from(node, values)
			}
			"GrowingArray.from" => {
				let Some((source, _)) = self.array_from_arguments(info)? else {
					return Ok(Val::never());
				};
				if !matches!(source.repr, Repr::Array(_) | Repr::Growing(_)) {
					return Err(self.unsupported(node.index(), "`GrowingArray.from` of this value"));
				}
				// the source was taken: the conversion moves its items
				let source = self.fresh(source);
				let repr = self.node_repr(node)?;
				self.convert(source, repr, node.index())
			}

			_ if path.starts_with("Math.")
				|| path.starts_with("Integer#")
				|| path.starts_with("Float#")
				|| path.starts_with("Percentage#")
				|| matches!(
					path,
					"Integer.round" | "Integer.floor" | "Integer.ceil" | "Integer.truncate"
				) =>
			{
				self.number_call(node, path, info)
			}
			"Integer.parse" => {
				let Some(Argument::Given(text)) = info.arguments.first() else {
					return Err(self.unsupported(node.index(), path));
				};
				let text = self.expr(*text)?;
				if text.repr == Repr::Never {
					return Ok(text);
				}
				self.parse_integer(text)
			}
			_ => Err(self.unsupported(node.index(), &format!("`{path}`"))),
		}
	}

	/// The instance of a generic function for the type arguments of a call
	/// (monomorphization): declared on first use, lowered at the end.
	pub(crate) fn instance(&mut self, declaration: NodeId, call: NodeId) -> Result<NodeId, Error> {
		let Some(list) = self.analysis.instantiation(call) else {
			return Err(self.unsupported(call.index(), "this generic call"));
		};
		let arguments: Vec<TypeId> = self.types().list_items(list).to_vec();
		let mut reprs = Vec::new();
		for argument in &arguments {
			reprs.push(self.repr_of(*argument, call.index())?);
		}
		self.instance_of(declaration, reprs)
	}

	/// The instance of a generic function of the file for these
	/// representations of its type arguments.
	pub(crate) fn instance_of(
		&mut self,
		declaration: NodeId,
		reprs: Vec<Repr>,
	) -> Result<NodeId, Error> {
		let info = self.generic_functions[&declaration].clone();
		if let Some(key) = self.instances.get(&(declaration, reprs.clone())) {
			return Ok(*key);
		}
		let bindings: rustc_hash::FxHashMap<_, _> = info
			.generics
			.iter()
			.copied()
			.zip(reprs.iter().copied())
			.collect();
		let saved = self.bindings.replace(bindings);
		let suffix = format!(".{}", self.instances.len());
		let function = self.declare_function(declaration, info, &suffix);
		self.bindings.replace(saved);
		let function = function?;
		// a key that is no node of the tree
		let key = NodeId::new(self.tree.nodes.len() + self.instances.len() + 1);
		self.functions.insert(key, function);
		self.instances.insert((declaration, reprs), key);
		self.pending_instances.push((self.file, key));
		Ok(key)
	}

	/// The arguments of `Array.from(source, capacity = n)`.
	fn array_from_arguments(
		&mut self,
		info: &CallInfo,
	) -> Result<Option<(Val<'ctx>, Option<Val<'ctx>>)>, Error> {
		let mut source = None;
		let mut capacity = None;
		for (position, argument) in info.arguments.iter().enumerate() {
			if let Argument::Given(value) = argument {
				let lowered = self.expr(*value)?;
				if lowered.repr == Repr::Never {
					return Ok(None);
				}
				let lowered = if position == 0 {
					self.take(lowered, Some(*value))?
				} else {
					lowered
				};
				match position {
					0 => source = Some(lowered),
					_ => capacity = Some(lowered),
				}
			}
		}
		Ok(source.map(|source| (source, capacity)))
	}

	/// The array a variadic parameter collects: the values, and the items of
	/// the spread collections (`...numbers`), in order.
	fn variadic_array(
		&mut self,
		repr: Repr,
		values: &[NodeId],
		node: NodeId,
	) -> Result<Option<Val<'ctx>>, Error> {
		let Repr::Array(item_type) = repr else {
			return Err(self.unsupported(node.index(), "this variadic parameter"));
		};
		let item = self.item_repr(item_type, node)?;
		let i64 = self.context.i64_type();
		let count = i64.const_int(values.len() as u64, false);
		let array = self.new_array(item, i64.const_zero(), count)?;
		let array_value = Val::new(repr, array);
		for &value in values {
			match self.view(value) {
				View::Spread(inner) => {
					let spread = self.expr(inner)?;
					if spread.repr == Repr::Never {
						return Ok(None);
					}
					let size = inkwell::types::BasicType::size_of(&self.llvm_type(item))
						.expect("a sized item");
					let size = self.builder.build_int_cast(size, i64, "")?;
					if self.needs_drop(item) {
						// each item copied
						let source = spread.value.expect("an array").into_pointer_value();
						let (_, length) = self.array_parts(source)?;
						self.for_each_item_value(source, item, length, |this, value| {
							this.array_push(array_value, value, inner, None)
						})?;
					} else {
						self.call_runtime(
							"fa_array_append",
							&[
								array.into(),
								spread.value.expect("an array").into(),
								size.into(),
							],
						)?;
					}
				}
				_ => {
					let lowered = self.expr(value)?;
					if lowered.repr == Repr::Never {
						return Ok(None);
					}
					self.array_push(array_value, lowered, value, Some(value))?;
				}
			}
		}
		Ok(Some(self.fresh(array_value)))
	}

	/// A method of strings: the receiver, then the arguments, in order.
	fn string_method(
		&mut self,
		node: NodeId,
		path: &str,
		info: &CallInfo,
	) -> Result<Val<'ctx>, Error> {
		let receiver = self.receiver(node)?;
		if receiver.repr == Repr::Never {
			return Ok(receiver);
		}
		let mut arguments = Vec::new();
		for argument in &info.arguments {
			let value = match argument {
				Argument::Receiver(_) => continue,
				Argument::Given(value) => self.expr(*value)?,
				Argument::Default => self.string_constant(" "),
				_ => return Err(self.unsupported(node.index(), path)),
			};
			if value.repr == Repr::Never {
				return Ok(value);
			}
			arguments.push(value);
		}
		let (bytes, length) = self.string_parts(receiver)?;
		let mut call: Vec<BasicMetadataValueEnum<'ctx>> = vec![bytes.into(), length.into()];
		let i32 = self.context.i32_type();
		let i64 = self.context.i64_type();
		let flag =
			|value: u64| -> BasicMetadataValueEnum<'ctx> { i32.const_int(value, false).into() };
		let string_argument = |this: &Self,
		                       index: usize,
		                       call: &mut Vec<BasicMetadataValueEnum<'ctx>>|
		 -> Result<(), Error> {
			let (bytes, length) = this.string_parts(arguments[index])?;
			call.push(bytes.into());
			call.push(length.into());
			Ok(())
		};
		let boolean = |this: &Self,
		               name: &str,
		               call: &[BasicMetadataValueEnum<'ctx>]|
		 -> Result<Val<'ctx>, Error> {
			let function = this.runtime.function(name)?;
			let result = this.call_value(function, call)?.into_int_value();
			let truth = this.builder.build_int_compare(
				inkwell::IntPredicate::NE,
				result,
				result.get_type().const_zero(),
				"",
			)?;
			Ok(Val::new(Repr::Bool, truth))
		};
		match path {
			"String#isEmpty" => {
				let empty = self.builder.build_int_compare(
					inkwell::IntPredicate::EQ,
					length,
					length.get_type().const_zero(),
					"",
				)?;
				Ok(Val::new(Repr::Bool, empty))
			}
			"String#toString" => Ok(receiver),
			"String#includes" | "String#startsWith" | "String#endsWith" => {
				string_argument(self, 0, &mut call)?;
				let name = match path {
					"String#includes" => "fa_string_includes",
					"String#startsWith" => "fa_string_starts_with",
					_ => "fa_string_ends_with",
				};
				boolean(self, name, &call)
			}
			"String#indexOf" | "String#lastIndexOf" => {
				string_argument(self, 0, &mut call)?;
				call.push(flag((path == "String#lastIndexOf") as u64));
				let function = self.runtime.function("fa_string_index_of")?;
				let index = self.call_value(function, &call)?.into_int_value();
				let present = self.builder.build_int_compare(
					inkwell::IntPredicate::SGE,
					index,
					i64.const_zero(),
					"",
				)?;
				let repr = Repr::Optional(Scalar::Int(IntRepr::I64));
				let ty = self.llvm_type(repr).into_struct_type();
				let pair = self
					.builder
					.build_insert_value(ty.get_undef(), present, 0, "")?;
				let zero_if_absent =
					self.builder
						.build_select(present, index, i64.const_zero(), "")?;
				let pair = self
					.builder
					.build_insert_value(pair, zero_if_absent, 1, "")?;
				Ok(Val::new(repr, pair.into_struct_value()))
			}
			"String#toUpperCase" | "String#toLowerCase" => {
				call.push(flag((path == "String#toUpperCase") as u64));
				let result = self.call_with_pair("fa_string_case", &call, Repr::Str)?;
				Ok(self.fresh(result))
			}
			"String#trim" | "String#trimStart" | "String#trimEnd" => {
				call.push(flag(match path {
					"String#trimStart" => 1,
					"String#trimEnd" => 2,
					_ => 3,
				}));
				let result = self.call_with_pair("fa_string_trim", &call, Repr::Str)?;
				Ok(self.fresh(result))
			}
			"String#repeat" => {
				let count = self.convert(arguments[0], Repr::Int(IntRepr::I64), node.index())?;
				call.push(count.int().into());
				let result = self.call_with_pair("fa_string_repeat", &call, Repr::Str)?;
				Ok(self.fresh(result))
			}
			"String#padStart" | "String#padEnd" => {
				let width = self.convert(arguments[0], Repr::Int(IntRepr::I64), node.index())?;
				call.push(width.int().into());
				string_argument(self, 1, &mut call)?;
				call.push(flag((path == "String#padStart") as u64));
				let result = self.call_with_pair("fa_string_pad", &call, Repr::Str)?;
				Ok(self.fresh(result))
			}
			"String#replace" | "String#replaceAll" => {
				string_argument(self, 0, &mut call)?;
				string_argument(self, 1, &mut call)?;
				call.push(flag((path == "String#replaceAll") as u64));
				let result = self.call_with_pair("fa_string_replace", &call, Repr::Str)?;
				Ok(self.fresh(result))
			}
			"String#split" | "String#characters" | "String#codePoints" => {
				let name = match path {
					"String#split" => {
						string_argument(self, 0, &mut call)?;
						"fa_string_split"
					}
					"String#characters" => "fa_string_characters",
					_ => "fa_string_code_points",
				};
				let function = self.runtime.function(name)?;
				let array = self.call_value(function, &call)?;
				Ok(self.fresh(Val::new(self.node_repr(node)?, array)))
			}
			"String#get" => {
				let index = self.convert(arguments[0], Repr::Int(IntRepr::I64), node.index())?;
				call.push(index.int().into());
				let out = self.alloca(Repr::Str, "character")?;
				call.push(out.pointer.into());
				let get = self.runtime.function("fa_string_get")?;
				let found = self.call_value(get, &call)?.into_int_value();
				let present = self.builder.build_int_compare(
					inkwell::IntPredicate::NE,
					found,
					found.get_type().const_zero(),
					"",
				)?;
				let character = self.load(out)?;
				let repr = Repr::Optional(Scalar::Str);
				let ty = self.llvm_type(repr).into_struct_type();
				let pair = self
					.builder
					.build_insert_value(ty.get_undef(), present, 0, "")?;
				let zero = self.string_type.const_zero();
				let character = self.builder.build_select(
					present,
					character.value.expect("a string"),
					zero.into(),
					"",
				)?;
				let pair = self.builder.build_insert_value(pair, character, 1, "")?;
				Ok(self.fresh(Val::new(repr, pair.into_struct_value())))
			}
			_ => Err(self.unsupported(node.index(), &format!("`{path}`"))),
		}
	}

	/// `Int8(x)`, `Integer(x)`, `Float(x)`, `Float32(x)`.
	fn conversion(
		&mut self,
		node: NodeId,
		argument: NodeId,
		target: TypeId,
	) -> Result<Val<'ctx>, Error> {
		let value = self.expr(argument)?;
		if value.repr == Repr::Never {
			return Ok(value);
		}
		let to = self.repr_of(target, node.index())?;
		let checked = self.flags(node).contains(NodeFlags::CONVERSION_CHECK);
		match (value.repr, to) {
			(Repr::Int(from), Repr::Int(int)) => {
				if checked {
					let Some(range) = self.types().as_int(target).copied() else {
						return Err(self.unsupported(node.index(), "this conversion"));
					};
					let name = self.session.display_type(target);
					let site = self.site(&name, node);
					let checked_value =
						self.sink_check_with(value, range.min, range.max, |this, raw| {
							let arguments = this.halves_for_error(raw, from.signed, site)?;
							this.raise_runtime("fa_error_conversion", &arguments)
						})?;
					return self.convert(checked_value, to, node.index());
				}
				let _ = int;
				self.convert(value, to, node.index())
			}
			(Repr::F64 | Repr::F32, Repr::Int(int)) => {
				let float = value.value.expect("a float").into_float_value();
				let wide = if value.repr == Repr::F32 {
					self.builder
						.build_float_ext(float, self.context.f64_type(), "")?
				} else {
					float
				};
				if checked {
					let range = self.types().as_int(target).copied().unwrap_or(
						fa_analyzer::types::IntType::full(fa_analyzer::types::IntStorage::Default),
					);
					let f64 = self.context.f64_type();
					// trunc(x) fits iff min - 1 < x < max + 1; NaN fails both
					let below = (range.min - 1).as_f64();
					let above = (range.max + 1).as_f64();
					let low_ok = if I256::from((range.min - 1).as_f64() as i128) == range.min - 1 {
						self.builder.build_float_compare(
							FloatPredicate::OGT,
							wide,
							f64.const_float(below),
							"",
						)?
					} else {
						self.builder.build_float_compare(
							FloatPredicate::OGE,
							wide,
							f64.const_float(range.min.as_f64()),
							"",
						)?
					};
					let high_ok = self.builder.build_float_compare(
						FloatPredicate::OLT,
						wide,
						f64.const_float(above),
						"",
					)?;
					let fits = self.builder.build_and(low_ok, high_ok, "")?;
					let failed = self.builder.build_not(fits, "")?;
					let name = self.session.display_type(target);
					let site = self.site(&name, node);
					self.guard(failed, |this| {
						this.call_noreturn("fa_float_overflow", &[wide.into(), site.into()])
					})?;
				}
				let ty = self.int_type(int.bits);
				let result = if int.signed {
					self.builder.build_float_to_signed_int(wide, ty, "")?
				} else {
					self.builder.build_float_to_unsigned_int(wide, ty, "")?
				};
				Ok(Val::new(to, result))
			}
			(Repr::Percentage, Repr::F64 | Repr::F32) => {
				let as_double = Val {
					repr: Repr::F64,
					value: value.value,
				};
				self.convert(as_double, to, node.index())
			}
			_ => self.convert(value, to, node.index()),
		}
	}

	/// The `(low, high, signed, site)` arguments of the runtime's error and
	/// stop functions for an integer value.
	fn halves_for_error(
		&self,
		raw: inkwell::values::IntValue<'ctx>,
		signed: bool,
		site: inkwell::values::PointerValue<'ctx>,
	) -> Result<Vec<BasicMetadataValueEnum<'ctx>>, Error> {
		let i128 = self.context.i128_type();
		let i64 = self.context.i64_type();
		let wide = if raw.get_type().get_bit_width() < 128 {
			if signed {
				self.builder.build_int_s_extend(raw, i128, "")?
			} else {
				self.builder.build_int_z_extend(raw, i128, "")?
			}
		} else {
			raw
		};
		let low = self.builder.build_int_truncate(wide, i64, "")?;
		let shifted = self
			.builder
			.build_right_shift(wide, i128.const_int(64, false), false, "")?;
		let high = self.builder.build_int_truncate(shifted, i64, "")?;
		let signed = self.context.i32_type().const_int(signed as u64, false);
		Ok(vec![low.into(), high.into(), signed.into(), site.into()])
	}

	/// Checks that an integer value is within `[min, max]`, calling `fail`
	/// (which ends with a `noreturn` call) in a cold block otherwise.
	fn sink_check_with(
		&mut self,
		value: Val<'ctx>,
		min: I256,
		max: I256,
		fail: impl FnOnce(&mut Self, inkwell::values::IntValue<'ctx>) -> Result<(), Error>,
	) -> Result<Val<'ctx>, Error> {
		let Repr::Int(from) = value.repr else {
			return Ok(value);
		};
		let raw = value.int();
		let (lowest, highest) = if from.signed {
			(
				-(I256::ONE << (from.bits - 1)),
				(I256::ONE << (from.bits - 1)) - 1,
			)
		} else {
			(I256::ZERO, (I256::ONE << from.bits) - 1)
		};
		let mut failed = None;
		if min > lowest {
			let predicate = if from.signed {
				inkwell::IntPredicate::SLT
			} else {
				inkwell::IntPredicate::ULT
			};
			failed = Some(self.builder.build_int_compare(
				predicate,
				raw,
				self.const_int(from.bits, min),
				"",
			)?);
		}
		if max < highest {
			let predicate = if from.signed {
				inkwell::IntPredicate::SGT
			} else {
				inkwell::IntPredicate::UGT
			};
			let above = self.builder.build_int_compare(
				predicate,
				raw,
				self.const_int(from.bits, max),
				"",
			)?;
			failed = Some(match failed {
				Some(below) => self.builder.build_or(below, above, "")?,
				None => above,
			});
		}
		if let Some(failed) = failed {
			self.guard(failed, |this| fail(this, raw))?;
		}
		let _ = IntRepr::I64;
		Ok(value)
	}
}
