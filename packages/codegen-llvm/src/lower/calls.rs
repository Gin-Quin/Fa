//! Calls: functions of the file (arguments in parameter order from
//! [`CallInfo`], defaults evaluated by the callee), built-ins, and number
//! conversions with their `CONVERSION_CHECK`.

use ethnum::I256;
use fa_analyzer::analysis::{Argument, CallInfo, Callee, GenericMethod, NodeFlags};
use fa_analyzer::ids::{FileId, NodeId, SymbolId, TypeId};
use fa_analyzer::symbols::SymbolKind;
use fa_analyzer::syntax::View;
use inkwell::FloatPredicate;
use inkwell::values::{BasicMetadataValueEnum, BasicValue, PointerValue};

use crate::error::Error;
use crate::lower::Lowering;
use crate::lower::values::{IntRepr, Repr, Scalar, Val};

impl<'ctx> Lowering<'_, 'ctx> {
	pub(crate) fn call(&mut self, node: NodeId) -> Result<Val<'ctx>, Error> {
		// `pick(value, { foo })`: the value's fields the node's type keeps
		if let Some(&value) = self.analysis.forwards.get(&node) {
			let source = self.expr(value)?;
			if source.repr == Repr::Never {
				return Ok(source);
			}
			let picked = self.picked_copy(source, self.node_repr(node)?, node)?;
			return Ok(self.fresh(picked));
		}
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
		// a C function (foreign.rs)
		if let Some(call) = self.analysis.extern_calls.get(&node).cloned() {
			return self.foreign_call(node, &call);
		}
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
			// `transform<Integer>`: a generic function value's instance
			// (`generic_values.rs`, through `expr`)
			if let View::Call { callee, .. } = self.view(node)
				&& self
					.analysis
					.flags_of(node)
					.contains(fa_analyzer::analysis::NodeFlags::GENERIC_APPLICATION)
				&& self.is_generic_function_type(self.type_of(callee))
			{
				return self.expr(callee);
			}
			// `repeatText<3>` given as a value: the instance's function
			if let View::Call { callee, .. } = self.view(node)
				&& self
					.analysis
					.flags_of(node)
					.contains(fa_analyzer::analysis::NodeFlags::GENERIC_APPLICATION)
				&& let View::Name(_) = self.view(callee)
				&& let fa_analyzer::symbols::Resolution::Local(symbol) =
					self.analysis.resolution(callee)
				&& self.analysis.symbols.symbol(symbol).kind == SymbolKind::Function
			{
				let declaration = self.analysis.symbols.symbol(symbol).decl_node;
				if self.generic_functions.contains_key(&declaration) {
					return self.function_value(declaration, node);
				}
			}
			if let View::Call { callee, .. } = self.view(node)
				&& self
					.flags(node)
					.contains(fa_analyzer::analysis::NodeFlags::GENERIC_APPLICATION)
				&& let Some(reached) = self.callee_function(callee)
			{
				let function = self.reached_function(reached, node)?;
				return self.applied_external_function_value(&function, node);
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
				// `item.describe()` in an instance, the method required by the
				// constraint of `item`'s type parameter: the method of the type
				// the instance is made for
				if let Some(method) = self.constraint_method(callee) {
					let function = self.selected_method(method, node)?;
					let mut arguments = vec![Argument::Receiver(callee)];
					arguments.extend(info.arguments.iter().cloned());
					let info = CallInfo {
						callee: Callee::Value,
						arguments,
					};
					return self.call_function(node, function, &info);
				}

				let closure = self.expr(callee)?;
				if closure.repr == Repr::Never {
					return Ok(closure);
				}
				let ty = match closure.repr {
					Repr::Function(ty) => ty,
					_ => self.type_of(callee),
				};
				// the parameters the function type says it changes: the
				// caller's places, not copies (ft-1)
				let changed = self.changed_parameters(ty);
				let mut arguments = Vec::new();
				let mut places = Vec::new();
				for (position, argument) in info.arguments.iter().enumerate() {
					let Argument::Given(argument) = argument else {
						return Err(self.unsupported(node.index(), "these arguments"));
					};
					if changed.contains(&position) && self.is_place(*argument) {
						let place = self.place(*argument)?;
						if self.borrows_parameter(place.repr) {
							arguments.push(Val::new(place.repr, self.zero_value(place.repr)));
							places.push(Some(place.pointer));
							continue;
						}
					}
					let value = self.expr(*argument)?;
					if value.repr == Repr::Never {
						return Ok(value);
					}
					// a task moves into the callee, which owns it
					let value = self.give_task(value, *argument)?;
					arguments.push(value);
					places.push(None);
				}
				let catches = self
					.analysis
					.has_flags(node, NodeFlags::CALLBACK_CALL)
					.then(|| std::mem::take(&mut self.frame_mut().catches));
				let result = self.call_closure_lending(closure, ty, &arguments, &places, node);
				if let Some(catches) = catches {
					self.frame_mut().catches = catches;
				}
				result
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
		// `self.current()`: a small method expanded into the method being
		// lowered (`frozen.rs`)
		if let Some(plan) = self.method_expansion(info) {
			return self.expand_method(node, info, plan);
		}
		let function = self.function_in(self.file, declaration, node)?;
		self.call_function(node, function, info)
	}

	/// A call of `target` (a function's body) neither awaited nor started:
	/// a blocking call of an `async` function that starts tasks raises the
	/// blocking depth around it (`blocking_starter`).
	fn plain_call(
		&mut self,
		function: &super::Function<'ctx>,
		node: NodeId,
		info: &CallInfo,
		target: inkwell::values::FunctionValue<'ctx>,
		arguments: &[BasicMetadataValueEnum<'ctx>],
	) -> Result<Option<inkwell::values::BasicValueEnum<'ctx>>, Error> {
		let blocking = self.blocking_starter(node, function);
		if blocking {
			self.add_blocking_depth(1)?;
		}
		let raw = match self.invariant_remainder_call(function, node, info, arguments)? {
			Some(result) => Some(result),
			None => self.recursive_base_call(function, target, arguments)?,
		};
		if blocking {
			self.add_blocking_depth(-1)?;
		}
		Ok(raw)
	}

	/// A direct call of a function (of this file or another), the
	/// arguments in parameter order from `info`.
	pub(crate) fn call_function(
		&mut self,
		node: NodeId,
		function: super::Function<'ctx>,
		info: &CallInfo,
	) -> Result<Val<'ctx>, Error> {
		let callbacks = self.call_callback_types(node, info);
		let function = if callbacks.is_empty() {
			function
		} else {
			self.callback_instance(function, callbacks)?
		};
		let (value, params, ret, sig, throws) = (
			if self.frame().arena {
				function.arena_body.unwrap_or(function.value)
			} else {
				function.value
			},
			function.params.clone(),
			function.ret,
			function.info.sig,
			function.throws,
		);
		// an instance's result typed with its type parameters: the type of
		// the call (`twice(7)`: `Array(Integer)`, not `Array(Value)`)
		let ret = match ret {
			Repr::Object(ty)
			| Repr::Array(ty)
			| Repr::Function(ty)
			| Repr::Tuple(ty)
			| Repr::Rec(ty)
			| Repr::Map(ty)
			| Repr::Set(ty)
			| Repr::Growing(ty)
			| Repr::Pool(ty)
			| Repr::Sync(ty)
			| Repr::Task(ty)
				if !function.bindings.is_empty()
					&& self
						.types()
						.flags(ty)
						.contains(fa_analyzer::types::TypeFlags::HAS_PARAM) =>
			{
				self.node_repr(node).unwrap_or(ret)
			}
			_ => ret,
		};
		// a method of a worker of an asynchronous program: a call on its thread (tasks.rs)
		if let Some(value) = self.worker_call(node, &function, info)? {
			return Ok(value);
		}
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
			let receiver = self.method_receiver(node);
			let repr = self.concrete_param_repr(&function, params[0].0, receiver)?;
			let (pointer, write_back) =
				self.argument_place(node, None, repr, function.borrowed[0])?;
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
				let repr = self.concrete_param_repr(&function, params[position].0, Some(value))?;
				let (pointer, write_back) =
					self.argument_place(node, Some(value), repr, function.borrowed[position])?;
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
				if has_default {
					arguments.push(self.context.bool_type().const_int(1, false).into());
				}
				continue;
			}
			match argument {
				Argument::Receiver(_) => {
					let lowered = match receiver_value.take() {
						Some(value) => value,
						None => self.receiver(node)?,
					};
					let lowered = self.convert_argument(&function, lowered, repr, node.index())?;
					arguments.push(lowered.value.expect("a receiver").into());
				}
				Argument::Given(argument) => {
					let lowered = evaluated[&position];
					// a task moves into the callee, which owns it
					let lowered = self.give_task(lowered, *argument)?;
					let lowered =
						self.convert_argument(&function, lowered, repr, argument.index())?;
					arguments.push(lowered.value.expect("an argument").into());
					if has_default {
						arguments.push(self.context.bool_type().const_int(1, false).into());
					}
				}
				Argument::Default if has_default && function.borrowed[position] => {
					arguments.push(self.pointer_type().const_null().into());
					arguments.push(self.context.bool_type().const_zero().into());
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
		let return_slot = if function.return_slot {
			let slot = self.call_return_slot(node, ret)?;
			arguments.insert(usize::from(function.lambda), slot.pointer.into());
			Some(slot)
		} else {
			None
		};
		if function.return_collection {
			let offer = self.collection_call_buffer(node)?;
			let (pointer, bytes) = offer
				.map(|o| (o.pointer, o.bytes))
				.unwrap_or((self.pointer_type().const_null(), self.usize.const_zero()));
			arguments.insert(usize::from(function.lambda), pointer.into());
			arguments.insert(usize::from(function.lambda) + 1, bytes.into());
		}
		// the sources of the reactive places given (D24), right before the call
		self.pass_argument_sources(node, function.declaration, function.info.params.len())?;
		self.invalidate_named_map_probes(&function, node, info)?;
		// an awaited or started call: its async instance (tasks.rs)
		let raw = match self.task_call(node, &function, &arguments)? {
			Some(super::tasks::TaskCall::Task(task)) => {
				for write_back in write_backs {
					self.write_back(write_back)?;
				}
				return Ok(task);
			}
			Some(super::tasks::TaskCall::Result(raw)) => raw,
			None => self.plain_call(&function, node, info, value, &arguments)?,
		};
		for write_back in write_backs {
			self.write_back(write_back)?;
		}
		if return_slot.is_some() {
			return self.slot_call_result(raw, ret, throws, return_slot);
		}
		if throws {
			let result = raw.expect("a result");
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
		let Some(result) = raw else {
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

	/// The positions of the parameters a function type changes (`mutates`,
	/// `invalidates`).
	pub(crate) fn changed_parameters(&self, ty: TypeId) -> Vec<usize> {
		let fa_analyzer::types::TypeKind::Function(sig) = self.types().kind(ty) else {
			return Vec::new();
		};
		let (mutates, invalidates) = self.types().effects(self.types().sig(sig).effects);
		mutates
			.iter()
			.chain(invalidates)
			.filter_map(|path| match self.types().path_data(*path).0.root {
				fa_analyzer::types::PathRoot::Param(index) => Some(index as usize),
				fa_analyzer::types::PathRoot::SelfParam => Some(0),
				_ => None,
			})
			.collect()
	}

	/// A placeholder value of a representation (a lent argument's).
	fn zero_value(&self, repr: Repr) -> inkwell::values::BasicValueEnum<'ctx> {
		super::zero_of(self.llvm_type(repr))
	}

	/// An argument converted to the representation of its parameter: an
	/// instance's parameters are typed with its type parameters, so the
	/// conversion sees its bindings too.
	/// `item.describe` where `item`'s type is a type parameter whose
	/// constraint requires the method `describe`: the declaration of that
	/// method in the type the instance being lowered is made for (of this
	/// file).
	fn constraint_method(&self, callee: NodeId) -> Option<GenericMethod> {
		use fa_analyzer::types::{FieldFlags, TypeKind};
		let View::Access { target, members } = self.view(callee) else {
			return None;
		};
		let member = members.last()?;
		let fa_analyzer::symbols::Resolution::Field(shape, index) =
			self.analysis.resolution(member)
		else {
			return None;
		};
		let field = *self.types().shape_fields(shape).get(index as usize)?;
		if !field.flags.contains(FieldFlags::METHOD) {
			return None;
		}
		let receiver = members
			.len()
			.checked_sub(2)
			.and_then(|index| members.get(index))
			.unwrap_or(target);
		let TypeKind::Param(param) = self.types().kind(self.type_of(receiver)) else {
			return None;
		};
		self.method_bindings
			.borrow()
			.iter()
			.find(|(bound, method)| {
				*bound == param
					&& self.types().syms.resolve(method.name)
						== self.types().syms.resolve(field.name)
			})
			.map(|(_, method)| method.clone())
	}

	fn selected_method(
		&mut self,
		method: GenericMethod,
		call: NodeId,
	) -> Result<super::Function<'ctx>, Error> {
		let method = self.concrete_method(method);
		if !self.is_local_key(method.function) {
			return self.remote_function_with_arguments(
				method.function,
				call,
				Some(method.arguments),
				Some(&method.methods),
			);
		}
		let (module, keyed) = self
			.session
			.keyed(method.function)
			.ok_or_else(|| self.unsupported(call.index(), "this selected method"))?;
		let file = self
			.session
			.file_of(module)
			.ok_or_else(|| self.unsupported(call.index(), "this selected method"))?;
		let arguments = self.types().list_items(method.arguments).to_vec();
		if arguments.is_empty() {
			return self.function_in(file, keyed.node, call);
		}
		let current = self.file;
		self.switch_file(file);
		let result = (|| {
			let generics = self
				.generic_functions
				.get(&keyed.node)
				.ok_or_else(|| self.unsupported(call.index(), "this selected generic method"))?
				.generics
				.clone();
			let reprs = arguments
				.iter()
				.map(|&ty| self.repr_of(ty, call.index()))
				.collect::<Result<Vec<_>, _>>()?;
			let values = generics
				.iter()
				.zip(&arguments)
				.filter_map(|(&param, &ty)| self.types().param(param).value.then_some((param, ty)))
				.collect();
			let instance = self.instance_here(
				keyed.node,
				&generics,
				&arguments,
				reprs,
				values,
				method.methods.as_ref().clone(),
			)?;
			Ok(self.functions[&instance].clone())
		})();
		self.switch_file(current);
		result
	}

	/// The representation of a parameter of an instance typed with its type
	/// parameters (`self: Pair<Left, Right>`), as the caller sees it: the
	/// representation of the argument (`argument`) given to it.
	fn concrete_param_repr(
		&self,
		function: &super::Function<'ctx>,
		repr: Repr,
		argument: Option<NodeId>,
	) -> Result<Repr, Error> {
		match repr {
			Repr::Object(ty)
			| Repr::Array(ty)
			| Repr::Function(ty)
			| Repr::Tuple(ty)
			| Repr::Map(ty)
			| Repr::Set(ty)
			| Repr::Growing(ty)
			| Repr::Pool(ty)
				if !function.bindings.is_empty()
					&& self
						.types()
						.flags(ty)
						.contains(fa_analyzer::types::TypeFlags::HAS_PARAM) =>
			{
				match argument {
					Some(argument) => self.node_repr(argument),
					None => Ok(repr),
				}
			}
			_ => Ok(repr),
		}
	}

	/// The receiver of a method call (`pair` in `pair.swap()`, `a.b` in
	/// `a.b.c()`).
	pub(super) fn method_receiver(&self, call: NodeId) -> Option<NodeId> {
		let View::Call { callee, .. } = self.view(call) else {
			return None;
		};
		let View::Access { target, members } = self.view(callee) else {
			return None;
		};
		match members.len() {
			0 | 1 => Some(target),
			count => members.get(count - 2),
		}
	}

	fn convert_argument(
		&mut self,
		function: &super::Function<'ctx>,
		value: Val<'ctx>,
		repr: Repr,
		at: usize,
	) -> Result<Val<'ctx>, Error> {
		if function.bindings.is_empty() {
			return self.convert(value, repr, at);
		}
		// under the caller's bindings first (what calls that compiled before
		// keep), under the callee's when a type parameter is unbound
		let first = self.convert(value, repr, at);
		if first.is_ok() {
			return first;
		}
		let mut merged = self.bindings.borrow().clone();
		merged.extend(
			function
				.bindings
				.iter()
				.map(|(param, bound)| (*param, *bound)),
		);
		let saved = self.bindings.replace(merged);
		let mut types = self.type_bindings.borrow().clone();
		types.extend(function.types.iter().map(|(&param, &ty)| (param, ty)));
		let saved_types = self.type_bindings.replace(types);
		let converted = self.convert(value, repr, at);
		self.type_bindings.replace(saved_types);
		self.bindings.replace(saved);
		converted
	}

	fn builtin_call(
		&mut self,
		node: NodeId,
		path: &str,
		info: &CallInfo,
	) -> Result<Val<'ctx>, Error> {
		// `numbers.parallel().map(f)` (parallel.rs)
		if let Some(operation) = self.analysis.parallel_operations.get(&node).copied() {
			return self.parallel_operation(node, operation, info);
		}
		if path == "Array#filter"
			&& let Some(split) = self.nonempty_split_filter(node)
		{
			let info = self.analysis.call(split).expect("a split call").clone();
			return self.string_method(split, "String#split", &info, true);
		}
		match path {
			// `collection.parallel()`: the collection, which the operation reads
			"Collection#parallel" => self.receiver(node),
			// tasks (tasks.rs)
			"Task.yield" | "Task.all" | "Task.any" | "Task.settled" | "Task#cancel"
			| "std.time.sleep" => self.task_builtin(node, path, info),
			"Subscription#stop" => self.stop_subscription(node),
			// std.time, std.fs, std.net (io.rs)
			"std.time.monotonic" | "std.time.now" | "File.read" | "File.readBytes"
			| "File.write" | "TcpListener#accept" | "TcpListener#close" | "TcpStream.connect"
			| "TcpStream#read" | "TcpStream#write" | "TcpStream#close" => self.io_builtin(node, path, info),
			// std.json (json.rs)
			"Json.parse" | "Json.stringify" => self.json_builtin(node, path, info),
			// capacity (capacity.rs)
			_ if path.ends_with("#reserve") => self.reserve_call(node, info),
			"String.from" => self.string_from(node, info),
			"console.log" | "console.error" => {
				let mut values = Vec::new();
				for argument in &info.arguments {
					match argument {
						Argument::Variadic(nodes) => {
							for &value in nodes {
								values.push(value);
							}
						}
						Argument::Given(value) => values.push(*value),
						_ => {}
					}
				}
				self.print(if path == "console.log" { 1 } else { 2 }, &values)
			}
			_ if path.starts_with("String#") => self.string_method(node, path, info, false),
			"Pointer.allocate" => self.pointer_allocate(node, info),
			_ if path.starts_with("Pointer#") || path.starts_with("CString#") => {
				self.pointer_method(node, path, info)
			}
			// a worker's state lives as long as its handle (parallel.rs); in
			// asynchronous mode its thread stops (tasks.rs)
			"Worker#stop" => self.stop_worker(node),
			// pools of workers (tasks.rs)
			"WorkerPool#submit" => self.pool_submit(node, info, false),
			"WorkerPool#shutdown" => self.pool_shutdown(node),
			// thread-safe values (parallel.rs)
			_ if path.starts_with("AtomicInteger#")
				|| path.starts_with("AtomicBoolean#")
				|| path.starts_with("Mutex#")
				|| path.starts_with("Channel#") =>
			{
				self.sync_method(node, path, info)
			}
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
			"Fields#toStringArray" => self.fields_to_string_array(node),
			"Enum#toString" => {
				let value = self.receiver(node)?;
				match value.repr {
					Repr::Enum(nominal) => self.enum_name(value, nominal),
					_ => Ok(value),
				}
			}
			"Error#toString" => {
				let value = self.receiver(node)?;
				let source = self.method_receiver(node).unwrap_or(node);
				self.call_runtime("fa_b_reset", &[])?;
				self.append_value(value, source)?;
				let text = self.call_with_pair("fa_b_take", &[], Repr::Str)?;
				Ok(self.fresh(text))
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
			_ if path.starts_with("Pool#") || path.starts_with("Columnar#") => {
				self.pool_method(node, path, info)
			}
			_ if path.starts_with("RelationPool#") => self.relation_pool_method(node, path, info),
			_ if path.starts_with("OrderedColumnar#") => {
				let method = path.trim_start_matches("OrderedColumnar#");
				let method = if method == "add" { "push" } else { method };
				let mut arguments = Vec::new();
				for argument in &info.arguments {
					match argument {
						Argument::Given(value) => arguments.push(*value),
						Argument::Receiver(_) | Argument::Default => {}
						_ => return Err(self.unsupported(node.index(), path)),
					}
				}
				self.array_method(node, &format!("Array#{method}"), &arguments)
			}
			_ if path.starts_with("Iterator#") => self.iterator_method(node, path, info),
			_ if path.starts_with("Set")
				|| path.starts_with("Map")
				|| path.starts_with("GrowingSet")
				|| path.starts_with("GrowingMap") =>
			{
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
				let result = self.array_of_table(source, repr, node)?;
				let capacity = match info.arguments.get(1) {
					Some(Argument::Given(hint)) => {
						let hint = self.expr(*hint)?;
						if hint.repr == Repr::Never {
							return Ok(hint);
						}
						Some(hint)
					}
					_ => None,
				};
				let result = self.take(result, None)?;
				self.array_from(node, (result, capacity))
			}
			"Pool.from" | "GrowingPool.from" | "Columnar.from" | "GrowingColumnar.from"
				if matches!(self.node_repr(node)?, Repr::Pool(_) | Repr::Columnar(_)) =>
			{
				self.pool_from(node, info)
			}
			"Array.from" | "OrderedColumnar.from" => {
				if path == "Array.from"
					&& let Some(result) = self.array_from_copy_into(node, info)?
				{
					return Ok(result);
				}
				let Some(values) = self.array_from_arguments(info)? else {
					return Ok(Val::never());
				};
				self.array_from(node, values)
			}
			"GrowingArray.from" => {
				let Some((source, capacity)) = self.array_from_arguments(info)? else {
					return Ok(Val::never());
				};
				if !matches!(source.repr, Repr::Array(_) | Repr::Growing(_)) {
					return Err(self.unsupported(node.index(), "`GrowingArray.from` of this value"));
				}
				// Convert through an owned flat array so the first segment uses the requested size.
				let repr = self.node_repr(node)?;
				let Repr::Growing(item_type) = repr else {
					unreachable!("a growing array");
				};
				let source = self.fresh(source);
				let flat = self.convert(source, Repr::Array(item_type), node.index())?;
				let flat = self.take(flat, None)?;
				let item = self.item_repr(item_type, node)?;
				let capacity = match capacity {
					Some(capacity) => self
						.convert(capacity, Repr::Int(IntRepr::I64), node.index())?
						.int(),
					None => self.context.i64_type().const_zero(),
				};
				let result = self
					.call_value(
						self.runtime.function("fa_segmented_from_array_capacity")?,
						&[
							flat.value.expect("an array").into(),
							self.item_size_const(item).into(),
							self.segment_k(item).into(),
							capacity.into(),
						],
					)?
					.into_pointer_value();
				Ok(self.fresh(Val::new(repr, result)))
			}

			_ if path.starts_with("Math.")
				|| path.starts_with("Integer#")
				|| path.starts_with("Float#")
				|| path.starts_with("Percentage#")
				|| path.starts_with("BigInt#")
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
	/// The type arguments of a call of a generic function: the call's, or
	/// those written before it (`first<Integer>(items)`, `sum(Float)([1])`).
	pub(crate) fn call_instantiation(&self, call: NodeId) -> Option<fa_analyzer::ids::ListId> {
		self.analysis
			.instantiation(call)
			.or_else(|| match self.view(call) {
				View::Call { callee, .. } => self.analysis.instantiation(callee),
				_ => None,
			})
	}

	/// The instance of a generic function of the current file for the type
	/// arguments of `call` (a node of the current file).
	pub(crate) fn instance(&mut self, declaration: NodeId, call: NodeId) -> Result<NodeId, Error> {
		self.instance_in(self.file, declaration, call)
	}

	/// The instance of the generic function `declaration` of `file` for the
	/// type arguments of `call`, a node of the current file. The arguments
	/// are read here, the instance is declared in its file.
	pub(crate) fn instance_in(
		&mut self,
		file: FileId,
		declaration: NodeId,
		call: NodeId,
	) -> Result<NodeId, Error> {
		let generics = if file == self.file {
			self.generic_functions
				.get(&declaration)
				.map(|info| info.generics.clone())
		} else {
			self.other_files
				.get(&file)
				.and_then(|state| state.generic_functions.get(&declaration))
				.map(|info| info.generics.clone())
		};
		let Some(generics) = generics else {
			return Err(self.unsupported(call.index(), "calling this function"));
		};
		let owner = self.call_method_owner(call).cloned();
		// a function of a generic type: preserve its written arguments,
		// including parameters not present in the receiver's data layout.
		let type_arguments = match self.generic_type_methods.get(&(file, declaration)).cloned() {
			Some((params, declared)) => match owner
				.as_ref()
				.map(|owner| self.types().list_items(owner.arguments).to_vec())
				.filter(|arguments| arguments.len() == params.len())
				.or_else(|| self.receiver_type_arguments(call, &params, declared))
			{
				Some(arguments) => arguments,
				None => {
					return Err(self.unsupported(
						call.index(),
						"this function of a generic type without a receiver",
					));
				}
			},
			None => Vec::new(),
		};
		let own = generics.len() - type_arguments.len();
		let owner_count = type_arguments.len();
		let list = match self.call_instantiation(call) {
			Some(list) => self.types().list_items(list).to_vec(),
			None if own == 0 => Vec::new(),
			None => return Err(self.unsupported(call.index(), "this generic call")),
		};
		let own_arguments: Vec<_> = list.iter().rev().take(own).rev().copied().collect();
		let mut methods = self.call_methods(call, &own_arguments);
		for method in &mut methods {
			method.parameter += owner_count as u16;
		}
		methods.extend(self.forwarded_methods(&type_arguments));
		if let Some(owner) = owner {
			methods.extend(owner.methods.iter().cloned());
		}
		let methods = methods
			.into_iter()
			.map(|method| self.concrete_method(method))
			.collect();
		let given: Vec<TypeId> = type_arguments.into_iter().chain(own_arguments).collect();
		// the compile-time value parameters (`repeatText<2>`): bound to the
		// literal types, an instance per value
		let mut values = Vec::new();
		for (&param, &argument) in generics.iter().zip(&given) {
			if !self.types().param(param).value {
				continue;
			}
			let Some(literal) = self.value_argument(argument) else {
				return Err(self.unsupported(call.index(), "this compile-time value argument"));
			};
			values.push((param, literal));
		}
		// the type arguments, as the types the instance being lowered (if
		// any) stands for (`units.rs`)
		let arguments: Vec<TypeId> = given
			.into_iter()
			.map(|argument| self.concrete_type(argument))
			.collect();
		let mut reprs = Vec::new();
		for argument in &arguments {
			reprs.push(self.repr_of(*argument, call.index())?);
		}
		let current = self.file;
		self.switch_file(file);
		let instance =
			self.instance_here(declaration, &generics, &arguments, reprs, values, methods);
		self.switch_file(current);
		instance
	}

	/// The instance of a generic function of the current file, its type
	/// arguments and the literals of its value parameters known.
	pub(crate) fn instance_here(
		&mut self,
		declaration: NodeId,
		generics: &[fa_analyzer::ids::ParamId],
		arguments: &[TypeId],
		reprs: Vec<Repr>,
		values: Vec<(fa_analyzer::ids::ParamId, TypeId)>,
		methods: Vec<GenericMethod>,
	) -> Result<NodeId, Error> {
		self.note_instance_types(generics, arguments, &reprs);
		let mut types = self.type_bindings.borrow().clone();
		types.extend(generics.iter().copied().zip(arguments.iter().copied()));
		let saved_types = self.type_bindings.replace(types);
		let saved_methods = self.method_bindings.replace(
			methods
				.iter()
				.filter_map(|method| {
					generics
						.get(method.parameter as usize)
						.map(|&param| (param, method.clone()))
				})
				.collect(),
		);
		if values.is_empty() {
			let result = self.instance_with_values(declaration, reprs, arguments.to_vec(), methods);
			self.method_bindings.replace(saved_methods);
			self.type_bindings.replace(saved_types);
			return result;
		}
		let mut value_bindings = rustc_hash::FxHashMap::default();
		for &(param, literal) in &values {
			let Some(symbol) = self.value_parameter_symbol(declaration, param) else {
				return Err(
					self.unsupported(declaration.index(), "this compile-time value parameter")
				);
			};
			value_bindings.insert(symbol, literal);
		}
		let saved = self.value_bindings.replace(value_bindings);
		let instance = self.instance_with_values(declaration, reprs, arguments.to_vec(), methods);
		self.value_bindings.replace(saved);
		self.method_bindings.replace(saved_methods);
		self.type_bindings.replace(saved_types);
		instance
	}

	/// Methods are selected by the analyzer; a call forwarding an outer
	/// parameter forwards the selection made for that parameter's instance.
	pub(crate) fn call_methods(&self, call: NodeId, arguments: &[TypeId]) -> Vec<GenericMethod> {
		let node = if self.analysis.instantiated_methods.contains_key(&call) {
			call
		} else if let View::Call { callee, .. } = self.view(call) {
			callee
		} else {
			call
		};
		let mut methods = self
			.analysis
			.instantiated_methods
			.get(&node)
			.cloned()
			.unwrap_or_default();
		methods.extend(self.forwarded_methods(arguments));
		methods.sort_by(|left, right| {
			left.parameter.cmp(&right.parameter).then_with(|| {
				self.types()
					.syms
					.resolve(left.name)
					.cmp(self.types().syms.resolve(right.name))
			})
		});
		methods.dedup();
		methods
			.into_iter()
			.map(|method| self.concrete_method(method))
			.collect()
	}

	/// A selection made while checking a generic body may still refer to
	/// that body's parameters. Preserve its exact owner arguments and method
	/// dependencies before switching to the file declaring the selected method.
	fn concrete_method(&self, method: GenericMethod) -> GenericMethod {
		let arguments = self.types().list_items(method.arguments).to_vec();
		let mut dependencies = method.methods.as_ref().clone();
		dependencies.extend(self.forwarded_methods(&arguments));
		let dependencies: Vec<_> = dependencies
			.into_iter()
			.map(|method| self.concrete_method(method))
			.collect();
		let arguments: Vec<_> = arguments
			.into_iter()
			.map(|argument| self.concrete_type(argument))
			.collect();
		let module = self.session.module_id(self.file);
		let mut merger = self.types().merger(Some(module));
		let arguments: Vec<_> = arguments
			.iter()
			.map(|&argument| merger.ty(argument))
			.collect();
		drop(merger);
		let mut scratch = fa_analyzer::types::TypeStore::view(self.types().global());
		let list = scratch.list(&arguments);
		let mut method = method;
		method.arguments = scratch.merger(Some(module)).list(list);
		method.methods = std::sync::Arc::new(dependencies);
		method
	}

	pub(crate) fn forwarded_methods(&self, arguments: &[TypeId]) -> Vec<GenericMethod> {
		let mut methods = Vec::new();
		for (index, &ty) in arguments.iter().enumerate() {
			let fa_analyzer::types::TypeKind::Param(param) = self.types().kind(ty) else {
				continue;
			};
			methods.extend(
				self.method_bindings
					.borrow()
					.iter()
					.filter(|(bound, _)| *bound == param)
					.map(|(_, method)| GenericMethod {
						parameter: index as u16,
						..method.clone()
					}),
			);
		}
		methods.sort_by(|left, right| {
			left.parameter.cmp(&right.parameter).then_with(|| {
				self.types()
					.syms
					.resolve(left.name)
					.cmp(self.types().syms.resolve(right.name))
			})
		});
		methods.dedup();
		methods
	}

	pub(crate) fn call_method_owner(
		&self,
		call: NodeId,
	) -> Option<&fa_analyzer::analysis::WrittenType> {
		let View::Call { mut callee, .. } = self.view(call) else {
			return None;
		};
		loop {
			while let View::Group(inner) = self.view(callee) {
				callee = inner;
			}
			match self.view(callee) {
				View::Call { callee: inner, .. }
					if self
						.analysis
						.has_flags(callee, NodeFlags::GENERIC_APPLICATION) =>
				{
					callee = inner
				}
				View::Access { members, .. } | View::OptionalAccess { members, .. } => {
					return self.analysis.method_instantiations.get(&members.last()?);
				}
				_ => return None,
			}
		}
	}

	/// The literal type a compile-time value argument stands for: itself, or
	/// the binding of the enclosing instance's own value parameter.
	fn value_argument(&self, argument: TypeId) -> Option<TypeId> {
		use fa_analyzer::types::TypeKind;
		let argument = self.concrete_type(argument);
		let types = self.types();
		match types.kind(argument) {
			TypeKind::Param(param) => {
				let name = types.param(param).name;
				let bindings = self.value_bindings.borrow();
				bindings
					.iter()
					.find(|(symbol, _)| self.analysis.symbols.symbol(**symbol).name == name)
					.map(|(_, literal)| *literal)
			}
			TypeKind::Int(_) | TypeKind::StringLit(_) | TypeKind::BoolLit(_)
				if types.is_literal(argument) =>
			{
				Some(argument)
			}
			// a key set: its `Fields` type and its bits (fields.rs)
			TypeKind::Tuple(_) if self.key_set_bits(argument).is_some() => Some(argument),
			_ => None,
		}
	}

	/// The `Static` symbol a value parameter's name resolves to in the body
	/// of a generic function declaration.
	fn value_parameter_symbol(
		&self,
		declaration: NodeId,
		param: fa_analyzer::ids::ParamId,
	) -> Option<SymbolId> {
		let name = self.types().param(param).name;
		let owner = self
			.analysis
			.function_owners
			.get(&declaration)
			.copied()
			.filter(|owner| {
				self.analysis
					.type_decls
					.get(owner)
					.is_some_and(|info| info.generics.contains(&param))
			})
			.unwrap_or(declaration);
		let span = self.tree.spans[owner.index()];
		self.analysis
			.symbols
			.symbols()
			.iter()
			.position(|symbol| {
				symbol.kind == SymbolKind::Static && symbol.name == name && {
					let declared = self.tree.spans[symbol.decl_node.index()];
					span.start <= declared.start && declared.end <= span.end
				}
			})
			.map(SymbolId::new)
	}

	/// The current value bindings, in a stable order (a part of the key of
	/// the instances declared under them).
	pub(crate) fn sorted_value_bindings(&self) -> Vec<TypeId> {
		let mut entries: Vec<(SymbolId, TypeId)> = self
			.value_bindings
			.borrow()
			.iter()
			.map(|(&symbol, &literal)| (symbol, literal))
			.collect();
		entries.sort_by_key(|(symbol, _)| *symbol);
		entries.into_iter().map(|(_, literal)| literal).collect()
	}

	/// The value of a compile-time value parameter in an instance: its
	/// literal, of the parameter's declared type.
	pub(crate) fn value_parameter(
		&mut self,
		symbol: SymbolId,
		literal: TypeId,
		node: NodeId,
	) -> Result<Val<'ctx>, Error> {
		use fa_analyzer::types::TypeKind;
		use fa_interpreter::Value;
		let types = self.types();
		let value = match types.kind(literal) {
			TypeKind::Int(_) => types
				.as_int(literal)
				.filter(|int| int.min == int.max)
				.map(|int| Value::Integer(int.min)),
			TypeKind::StringLit(text) => Some(Value::string(types.syms.resolve(text))),
			TypeKind::BoolLit(value) => Some(Value::Boolean(value)),
			_ => None,
		};
		let Some(value) = value else {
			return Err(self.unsupported(node.index(), "this compile-time value parameter"));
		};
		let declared = self.analysis.symbols.symbol(symbol).ty;
		let built = self.constant_value(&value, declared, node)?;
		Ok(if self.needs_drop(built.repr) {
			self.fresh(built)
		} else {
			built
		})
	}

	/// The arguments of the generic type of a call's receiver
	/// (`stack.push(1)` with `stack: Stack<Integer>`): the types `params`
	/// stand for in the receiver's type, `declared` substituted.
	fn receiver_type_arguments(
		&self,
		call: NodeId,
		params: &[fa_analyzer::ids::ParamId],
		declared: TypeId,
	) -> Option<Vec<TypeId>> {
		let View::Call { callee, .. } = self.view(call) else {
			return None;
		};
		let View::Access { target, .. } = self.view(callee) else {
			return None;
		};
		let receiver = self.concrete_type(self.type_of(target));
		let mut found = rustc_hash::FxHashMap::default();
		self.match_params(declared, receiver, &mut found, 0);
		params
			.iter()
			.map(|param| found.get(param).copied())
			.collect()
	}

	/// The instance of a generic function of the file for these
	/// representations of its type arguments and literal types of its value
	/// arguments (bound in `value_bindings` by the caller).
	fn instance_with_values(
		&mut self,
		declaration: NodeId,
		reprs: Vec<Repr>,
		values: Vec<TypeId>,
		methods: Vec<GenericMethod>,
	) -> Result<NodeId, Error> {
		let info = self.generic_functions[&declaration].clone();
		let key = (declaration, reprs, values, methods);
		if let Some(instance) = self.instances.get(&key) {
			return Ok(*instance);
		}
		let bindings: rustc_hash::FxHashMap<_, _> = info
			.generics
			.iter()
			.copied()
			.zip(key.1.iter().copied())
			.collect();
		let saved = self.bindings.replace(bindings);
		let suffix = format!(".{}", self.instances.len());
		let function = self.declare_function(declaration, info, &suffix);
		self.bindings.replace(saved);
		let function = function?;
		// a key that is no node of the tree
		let instance = NodeId::new(
			self.tree.nodes.len() + self.instances.len() + self.callback_instances.len() + 1,
		);
		self.functions.insert(instance, function);
		self.instances.insert(key, instance);
		self.pending_instances.push((self.file, instance));
		Ok(instance)
	}

	/// The parameter positions of throwing callbacks, after receiver binding.
	pub(crate) fn call_callback_types(
		&self,
		node: NodeId,
		info: &CallInfo,
	) -> Vec<(usize, TypeId)> {
		let Some(callbacks) = self.analysis.callback_types.get(&node) else {
			return Vec::new();
		};
		info.arguments
			.iter()
			.enumerate()
			.filter_map(|(index, argument)| {
				let Argument::Given(node) = argument else {
					return None;
				};
				callbacks
					.iter()
					.find(|(given, _)| given == node)
					.map(|(_, ty)| (index, self.concrete_type(*ty)))
			})
			.collect()
	}

	/// Reuses the ordinary instance's bindings; no error ABI is added to
	/// the instance called with nonthrowing callbacks.
	pub(crate) fn callback_instance(
		&mut self,
		base: super::Function<'ctx>,
		callbacks: Vec<(usize, TypeId)>,
	) -> Result<super::Function<'ctx>, Error> {
		let key = (base.value, callbacks);
		if let Some(&(file, instance)) = self.callback_instances.get(&key) {
			let current = self.file;
			self.switch_file(file);
			let function = self.functions[&instance].clone();
			self.switch_file(current);
			return Ok(function);
		}
		// A remote declaration already has the specialized signature; its
		// defining unit creates the body through an instance request.
		let Some((file, _)) = self.function_key(base.value) else {
			return Ok(base);
		};
		let current = self.file;
		self.switch_file(file);
		let saved_bindings = self.bindings.replace(base.bindings.clone());
		let saved_types = self.type_bindings.replace(base.types.clone());
		let saved_values = self.value_bindings.replace(base.values.clone());
		let saved_methods = self.method_bindings.replace(base.methods.clone());
		let suffix = format!(".callback{}", self.callback_instances.len());
		let result =
			self.declare_function_with_callbacks(base.declaration, base.info, &suffix, &key.1);
		self.bindings.replace(saved_bindings);
		self.type_bindings.replace(saved_types);
		self.value_bindings.replace(saved_values);
		self.method_bindings.replace(saved_methods);
		let result = result.inspect(|function| {
			let instance = NodeId::new(
				self.tree.nodes.len() + self.instances.len() + self.callback_instances.len() + 1,
			);
			self.functions.insert(instance, function.clone());
			self.callback_instances.insert(key, (file, instance));
			self.pending_instances.push((file, instance));
		});
		self.switch_file(current);
		result
	}

	/// The arguments of `Array.from(source, capacity = n)`.
	pub(crate) fn array_from_arguments(
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
					// Keep ownership registered while later arguments run:
					// a throwing capacity expression must drop this source.
					let owned = self.take(lowered, Some(*value))?;
					self.fresh(owned)
				} else {
					lowered
				};
				match position {
					0 => source = Some(lowered),
					_ => capacity = Some(lowered),
				}
			}
		}
		match source {
			Some(source) => Ok(Some((self.take(source, None)?, capacity))),
			None => Ok(None),
		}
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
					// a set or a map spreads its items: an array of them first
					let spread = match spread.repr {
						Repr::Set(_) | Repr::Map(_) => self.array_of_table(spread, repr, inner)?,
						_ => spread,
					};
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
		split_nonempty: bool,
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
		// the capacity word (its `SIMPLE` flag) for the functions that count characters
		if matches!(
			path,
			"String#includes"
				| "String#startsWith"
				| "String#endsWith"
				| "String#indexOf"
				| "String#lastIndexOf"
				| "String#replace"
				| "String#replaceAll"
				| "String#split"
				| "String#characters"
				| "String#codePoints"
				| "String#get"
		) {
			call.push(self.string_capacity(receiver)?.into());
		}
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
				if let Some(result) = self.alphabet_index(node, info, arguments[0])? {
					return Ok(result);
				}
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
				call.push(self.string_capacity(receiver)?.into());
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
						match (self.splits_into_views(node), split_nonempty) {
							(true, true) => "fa_string_split_nonempty_views",
							(false, true) => "fa_string_split_nonempty",
							// the array in the caller's storage when it offers some
							(views, false) => match self.collection_call_buffer(node)? {
								Some(destination) => {
									call.push(destination.pointer.into());
									call.push(destination.bytes.into());
									if views {
										"fa_string_split_views_into"
									} else {
										"fa_string_split_into"
									}
								}
								None if views => "fa_string_split_views",
								None => "fa_string_split",
							},
						}
					}
					"String#characters" => "fa_string_characters",
					_ => "fa_string_code_points",
				};
				let function = self.runtime.function(name)?;
				let invocation = self.builder.build_call(function, &call, "")?;
				if name.ends_with("_into") && self.recursive_collection_frame() {
					// The split helper has bounded scanning/construction scratch.
					// Keep that scratch in one transient helper frame, rather than
					// embedding it in every live recursive invocation.
					invocation.add_attribute(
						inkwell::attributes::AttributeLoc::Function,
						self.runtime.attribute("noinline"),
					);
				}
				let array = invocation
					.try_as_basic_value()
					.basic()
					.expect("an array result");
				Ok(self.fresh(Val::new(self.node_repr(node)?, array)))
			}
			"String#bytes" => {
				// a copy of the UTF-8 bytes
				let byte = Repr::Int(IntRepr {
					bits: 8,
					signed: false,
				});
				let count = self.byte_length(receiver)?;
				let array = self.new_array(byte, count, count)?;
				let (items, _) = self.array_parts(array)?;
				self.builder.build_memcpy(items, 1, bytes, 1, length)?;
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

	/// Searching a constant consecutive ASCII alphabet for one Character is
	/// a subtraction and a range test, rather than a loop over the alphabet.
	fn alphabet_index(
		&mut self,
		node: NodeId,
		info: &CallInfo,
		search: Val<'ctx>,
	) -> Result<Option<Val<'ctx>>, Error> {
		let Some(Argument::Given(argument)) = info
			.arguments
			.iter()
			.find(|arg| matches!(arg, Argument::Given(_)))
		else {
			return Ok(None);
		};
		if self.type_of(*argument) != TypeId::CHARACTER {
			return Ok(None);
		}
		let View::Call { callee, .. } = self.view(node) else {
			return Ok(None);
		};
		let View::Access { target, members } = self.view(callee) else {
			return Ok(None);
		};
		if members.len() != 1 {
			return Ok(None);
		}
		let Some(text) = self.constant_text(target) else {
			return Ok(None);
		};
		let alphabet = text.as_bytes();
		if alphabet.len() < 2
			|| alphabet.iter().any(|&c| c >= 128 || c == b'\r')
			|| !alphabet.windows(2).all(|pair| pair[1] == pair[0] + 1)
		{
			return Ok(None);
		}
		let (bytes, length) = self.string_parts(search)?;
		// A Character has at least one byte, including for a long grapheme.
		let byte = self
			.builder
			.build_load(self.context.i8_type(), bytes, "")?
			.into_int_value();
		let i64 = self.context.i64_type();
		let byte = self.builder.build_int_z_extend(byte, i64, "")?;
		let index =
			self.builder
				.build_int_sub(byte, i64.const_int(alphabet[0] as u64, false), "")?;
		let inside = self.builder.build_int_compare(
			inkwell::IntPredicate::ULT,
			index,
			i64.const_int(alphabet.len() as u64, false),
			"",
		)?;
		let single = self.builder.build_int_compare(
			inkwell::IntPredicate::EQ,
			length,
			self.usize.const_int(1, false),
			"",
		)?;
		let present = self.builder.build_and(inside, single, "")?;
		let index = self
			.builder
			.build_select(present, index, i64.const_zero(), "")?;
		let scalar = Scalar::Int(IntRepr::I64);
		let pair = self.pack_optional(&self.builder, scalar, present, index)?;
		Ok(Some(Val::new(Repr::Optional(scalar), pair)))
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
			(Repr::BigInt, Repr::Int(int)) => self.big_to_int(node, value, target, int, checked),
			// `Percentage(25)`: the number divided by 100
			(Repr::Int(_) | Repr::F64 | Repr::F32, Repr::Percentage) => {
				let number = self.convert(value, Repr::F64, node.index())?;
				let fraction = self.builder.build_float_div(
					number.value.expect("a float").into_float_value(),
					self.context.f64_type().const_float(100.0),
					"",
				)?;
				Ok(Val::new(Repr::Percentage, fraction))
			}
			(Repr::Percentage, Repr::F64 | Repr::F32) => {
				let as_double = Val {
					destination: None,
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
