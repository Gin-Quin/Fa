//! Function values: lambdas, and functions used as values.
//!
//! A function value is a pair `{ ptr function, ptr environment }`. A lambda
//! is an `internal fastcc` function whose first parameter is its
//! environment: a block holding a copy of the locals of the enclosing
//! functions it reads, made when the lambda is evaluated, after the drop
//! and the copy of the environment (the function value owns it). A function
//! declaration used as a value goes through a thunk that ignores the
//! environment. Calling a function value is an indirect call, with the
//! signature of the value's own type.

use fa_analyzer::ids::{NodeId, SymbolId, TypeId};
use fa_analyzer::symbols::{Resolution, SymbolKind};
use fa_analyzer::types::TypeKind;
use inkwell::IntPredicate;
use inkwell::module::Linkage;
use inkwell::types::{BasicMetadataTypeEnum, BasicType, BasicTypeEnum, FunctionType, StructType};
use inkwell::values::{BasicMetadataValueEnum, FunctionValue, PointerValue};

use crate::error::Error;
use crate::lower::values::{Repr, Val};
use crate::lower::{FAST_CALLING_CONVENTION, Lowering, Slot};

impl<'ctx> Lowering<'_, 'ctx> {
	/// `{ ptr function, ptr environment }`.
	pub(crate) fn closure_type(&self) -> StructType<'ctx> {
		let pointer = self.pointer_type();
		self.context
			.struct_type(&[pointer.into(), pointer.into()], false)
	}

	/// The locals of enclosing functions that a lambda reads.
	pub(crate) fn find_captures(&mut self, lambda: NodeId) -> Vec<SymbolId> {
		if let Some(info) = self.analysis.function(lambda) {
			return info.captures.clone();
		}
		let span = self.tree.spans[lambda.index()];
		let mut captures = Vec::new();
		let mut stack = vec![lambda.index()];
		while let Some(node) = stack.pop() {
			if self.analysis.has_flags(
				NodeId::new(node),
				fa_analyzer::analysis::NodeFlags::TYPE_EXPR,
			) {
				continue;
			}
			if let Resolution::Local(symbol) = self.analysis.resolution(NodeId::new(node)) {
				let data = self.analysis.symbols.symbol(symbol);
				let variable = !matches!(
					data.kind,
					SymbolKind::Function
						| SymbolKind::Type | SymbolKind::TypeParameter
						| SymbolKind::Namespace
				);
				// declared outside the lambda, in a function (top-level locals are globals)
				let declared = self.tree.spans[data.decl_node.index()];
				let outside = declared.start < span.start || declared.start >= span.end;
				if variable
					&& outside && self.owner_of_scope(data.scope).is_some()
					&& !captures.contains(&symbol)
				{
					captures.push(symbol);
				}
			}
			self.tree.nodes[node].for_each_child(|child| stack.push(child));
		}
		captures.sort();
		captures
	}

	/// The struct of an environment: its drop and its copy, then the
	/// captured values (or pointers to them).
	fn environment_type(
		&self,
		captures: &[SymbolId],
		by_reference: bool,
	) -> Result<StructType<'ctx>, Error> {
		let mut fields: Vec<BasicTypeEnum<'ctx>> = vec![
			self.pointer_type().into(),
			self.pointer_type().into(),
			self.context.i64_type().into(),
		];
		if by_reference {
			fields.extend(
				captures
					.iter()
					.map(|_| BasicTypeEnum::from(self.pointer_type())),
			);
			return Ok(self.context.struct_type(&fields, false));
		}
		for &symbol in captures {
			let data = self.analysis.symbols.symbol(symbol);
			let repr = self.repr_of(data.ty, data.decl_node.index())?;
			fields.push(self.llvm_type(repr));
		}
		Ok(self.context.struct_type(&fields, false))
	}

	/// Loads the captured values of a lambda from its environment into slots
	/// of its own (returns the slots they replace, to restore after).
	pub(crate) fn bind_captures(
		&mut self,
		environment: PointerValue<'ctx>,
		captures: &[SymbolId],
		by_reference: bool,
		ty: TypeId,
	) -> Result<Vec<(SymbolId, Option<Slot<'ctx>>)>, Error> {
		let structure = self.environment_type(captures, by_reference)?;
		if !by_reference && self.in_coroutine() {
			let value = self.make_closure_of(self.pointer_type().const_null(), environment, ty)?;
			let owned = self.emit_copy(value)?;
			let slot = self.alloca(owned.repr, "capture_environment")?;
			self.builder
				.build_store(slot.pointer, owned.value.expect("a closure"))?;
			self.cleanups
				.borrow_mut()
				.push(super::ownership::Cleanup::Local(slot));
		}
		let mut saved = Vec::new();
		for (index, &symbol) in captures.iter().enumerate() {
			let data = self.analysis.symbols.symbol(symbol);
			let repr = self.repr_of(data.ty, data.decl_node.index())?;
			let pointer =
				self.builder
					.build_struct_gep(structure, environment, index as u32 + 3, "")?;
			if by_reference {
				// the variable itself
				let pointer = self
					.builder
					.build_load(self.pointer_type(), pointer, "")?
					.into_pointer_value();
				saved.push((symbol, self.slots.insert(symbol, Slot { pointer, repr })));
				continue;
			}
			let value = self.builder.build_load(self.llvm_type(repr), pointer, "")?;
			let slot = self.alloca(repr, "captured")?;
			self.builder.build_store(slot.pointer, value)?;
			saved.push((symbol, self.slots.insert(symbol, slot)));
		}
		Ok(saved)
	}

	/// The instance of a lambda of a generic function (or of a generic
	/// lambda, whose own parameters are `own`): declared for the bindings of
	/// the instance being lowered, lowered at the end. Its key in
	/// `instances` is the lambda and the representations of every bound
	/// parameter, in the order of their ids.
	pub(crate) fn lambda_instance(&mut self, node: NodeId, own: &[Repr]) -> Result<NodeId, Error> {
		let Some(info) = self.generic_functions.get(&node).cloned() else {
			return Err(self.unsupported(node.index(), "this lambda"));
		};
		let mut bindings = self.bindings.borrow().clone();
		for (&param, &repr) in info.generics.iter().zip(own) {
			bindings.insert(param, repr);
		}
		let mut entries: Vec<(fa_analyzer::ids::ParamId, Repr)> = bindings
			.iter()
			.map(|(&param, &repr)| (param, repr))
			.collect();
		entries.sort_by_key(|(param, _)| *param);
		let reprs: Vec<Repr> = entries.iter().map(|(_, repr)| *repr).collect();
		let mut values: Vec<_> = entries
			.iter()
			.map(|(param, _)| {
				self.type_bindings
					.borrow()
					.get(param)
					.copied()
					.unwrap_or(TypeId::UNKNOWN)
			})
			.collect();
		values.extend(self.sorted_value_bindings());
		let mut methods: Vec<_> = self
			.method_bindings
			.borrow()
			.iter()
			.filter_map(|(param, method)| {
				entries
					.iter()
					.position(|(bound, _)| bound == param)
					.map(|index| fa_analyzer::analysis::GenericMethod {
						parameter: index as u16,
						..method.clone()
					})
			})
			.collect();
		methods.sort_by_key(|method| (method.parameter, method.name));
		if let Some(key) =
			self.instances
				.get(&(node, reprs.clone(), values.clone(), methods.clone()))
		{
			return Ok(*key);
		}
		let saved = self.bindings.replace(bindings);
		let suffix = format!(".{}", self.instances.len());
		let function = self.declare_function(node, info, &suffix);
		self.bindings.replace(saved);
		let function = function?;
		// a key that is no node of the tree
		let key = NodeId::new(self.tree.nodes.len() + self.instances.len() + 1);
		self.functions.insert(key, function);
		self.instances.insert((node, reprs, values, methods), key);
		self.pending_instances.push((self.file, key));
		Ok(key)
	}

	/// A lambda evaluated: its function and a new environment.
	pub(crate) fn lambda_value(&mut self, node: NodeId) -> Result<Val<'ctx>, Error> {
		// a generic lambda (`generic_values.rs`)
		if self
			.generic_functions
			.get(&node)
			.is_some_and(|info| !info.generics.is_empty() && info.declaration.is_none())
		{
			return self.generic_lambda_value(node);
		}
		let key = if self.functions.contains_key(&node) {
			node
		} else {
			self.lambda_instance(node, &[])?
		};
		let Some(function) = self.functions.get(&key).cloned() else {
			return Err(self.unsupported(node.index(), "this lambda"));
		};
		let captures = function.captures.clone();
		let thunk = self.needs_value_thunk(&function);
		let value = if thunk {
			self.thunk_of(&function)?
		} else {
			function.value
		};
		let by_reference = function.by_reference;
		let environment = self.build_environment(&captures, by_reference, node)?;
		// an async lambda: its async instance too (tasks.rs)
		let entry =
			self.async_value_entry(&function, value.as_global_value().as_pointer_value(), thunk)?;
		let closure = self.make_closure(entry, environment, node)?;
		// a closure on the heap owns its environment
		let closure = if by_reference {
			closure
		} else {
			self.fresh(closure)
		};
		self.function_with_defaults(closure, &function)
	}

	/// The environment of a closure: null without captures, pointers to the
	/// captured variables in the frame when it doesn't escape, copies of
	/// them in a heap block otherwise.
	pub(crate) fn build_environment(
		&mut self,
		captures: &[SymbolId],
		by_reference: bool,
		node: NodeId,
	) -> Result<PointerValue<'ctx>, Error> {
		Ok(if captures.is_empty() {
			self.pointer_type().const_null()
		} else if by_reference {
			let structure = self.environment_type(captures, true)?;
			let pointer = self.alloca_type(structure.into(), "environment")?;
			let (drop, copy) = self.shared_environment_functions();
			for (index, function) in [drop, copy].into_iter().enumerate() {
				let field = self
					.builder
					.build_struct_gep(structure, pointer, index as u32, "")?;
				self.builder
					.build_store(field, function.as_global_value().as_pointer_value())?;
			}
			for (index, &symbol) in captures.iter().enumerate() {
				let slot = self.slot_of(symbol, node)?;
				let field =
					self.builder
						.build_struct_gep(structure, pointer, index as u32 + 3, "")?;
				self.builder.build_store(field, slot.pointer)?;
			}
			pointer
		} else {
			let structure = self.environment_type(captures, false)?;
			let pointer = self.allocate(structure.size_of().expect("a sized environment"))?;
			let (drop, copy) = self.environment_functions(structure, captures)?;
			for (index, function) in [drop, copy].into_iter().enumerate() {
				let field = self
					.builder
					.build_struct_gep(structure, pointer, index as u32, "")?;
				self.builder
					.build_store(field, function.as_global_value().as_pointer_value())?;
			}
			let count = self.builder.build_struct_gep(structure, pointer, 2, "")?;
			self.builder
				.build_store(count, self.context.i64_type().const_int(1, false))?;
			for (index, &symbol) in captures.iter().enumerate() {
				let slot = self.slot_of(symbol, node)?;
				let loaded = self.load(slot)?;
				// the environment holds copies
				let loaded = self.emit_copy(loaded)?;
				let field =
					self.builder
						.build_struct_gep(structure, pointer, index as u32 + 3, "")?;
				self.builder
					.build_store(field, loaded.value.expect("a value"))?;
			}
			pointer
		})
	}

	/// The drop and the copy of an environment in a frame: nothing to free,
	/// and a copy shares it (the closure doesn't outlive the frame).
	fn shared_environment_functions(&self) -> (FunctionValue<'ctx>, FunctionValue<'ctx>) {
		let pointer = self.pointer_type();
		let drop = self
			.module
			.get_function("fa.keep_environment")
			.unwrap_or_else(|| {
				let function = self.module.add_function(
					"fa.keep_environment",
					self.context.void_type().fn_type(&[pointer.into()], false),
					Some(Linkage::Internal),
				);
				let builder = self.context.create_builder();
				builder.position_at_end(self.context.append_basic_block(function, "entry"));
				let _ = builder.build_return(None);
				function
			});
		let copy = self
			.module
			.get_function("fa.share_environment")
			.unwrap_or_else(|| {
				let function = self.module.add_function(
					"fa.share_environment",
					pointer.fn_type(&[pointer.into()], false),
					Some(Linkage::Internal),
				);
				let builder = self.context.create_builder();
				builder.position_at_end(self.context.append_basic_block(function, "entry"));
				let environment = function.get_nth_param(0).expect("an environment");
				let _ = builder.build_return(Some(&environment));
				function
			});
		(drop, copy)
	}

	/// The drop and the copy of an environment of this layout.
	fn environment_functions(
		&self,
		structure: StructType<'ctx>,
		captures: &[SymbolId],
	) -> Result<(FunctionValue<'ctx>, FunctionValue<'ctx>), Error> {
		let mut reprs = Vec::new();
		for &symbol in captures {
			let data = self.analysis.symbols.symbol(symbol);
			reprs.push(self.repr_of(data.ty, data.decl_node.index())?);
		}
		let pointer = self.pointer_type();
		let size = structure.size_of().expect("a sized environment");
		let align = self.usize.const_int(16, false);

		let drop = self.module.add_function(
			"fa.drop_environment",
			self.context.void_type().fn_type(&[pointer.into()], false),
			Some(Linkage::Internal),
		);
		self.runtime.add_attributes(drop, &["nounwind"]);
		let builder = self.context.create_builder();
		builder.position_at_end(self.context.append_basic_block(drop, "entry"));
		let environment = drop
			.get_nth_param(0)
			.expect("an environment")
			.into_pointer_value();
		let last = builder
			.build_call(
				self.runtime.function("fa_environment_release")?,
				&[environment.into()],
				"",
			)?
			.try_as_basic_value()
			.basic()
			.expect("a last-owner flag")
			.into_int_value();
		let last =
			builder.build_int_compare(IntPredicate::NE, last, last.get_type().const_zero(), "")?;
		let release = self.context.append_basic_block(drop, "release");
		let kept = self.context.append_basic_block(drop, "kept");
		builder.build_conditional_branch(last, release, kept)?;
		builder.position_at_end(kept);
		builder.build_return(None)?;
		builder.position_at_end(release);
		for (index, repr) in reprs.iter().enumerate().rev() {
			if self.needs_drop(*repr) {
				let field =
					builder.build_struct_gep(structure, environment, index as u32 + 3, "")?;
				let loaded = builder.build_load(self.llvm_type(*repr), field, "")?;
				builder.build_call(self.drop_function(*repr)?, &[loaded.into()], "")?;
			}
		}
		let size_value = builder.build_int_cast(size, self.usize, "")?;
		builder.build_call(
			self.runtime.free()?,
			&[environment.into(), size_value.into(), align.into()],
			"",
		)?;
		builder.build_return(None)?;

		let copy = self.module.add_function(
			"fa.copy_environment",
			pointer.fn_type(&[pointer.into()], false),
			Some(Linkage::Internal),
		);
		self.runtime.add_attributes(copy, &["nounwind"]);
		builder.position_at_end(self.context.append_basic_block(copy, "entry"));
		let environment = copy
			.get_nth_param(0)
			.expect("an environment")
			.into_pointer_value();
		let retained = builder
			.build_call(
				self.runtime.function("fa_environment_retain")?,
				&[environment.into()],
				"",
			)?
			.try_as_basic_value()
			.basic()
			.expect("an environment");
		builder.build_return(Some(&retained))?;
		Ok((drop, copy))
	}

	fn make_closure(
		&self,
		function: PointerValue<'ctx>,
		environment: PointerValue<'ctx>,
		node: NodeId,
	) -> Result<Val<'ctx>, Error> {
		self.make_closure_of(function, environment, self.type_of(node))
	}

	/// A closure of a function of type `ty`: the type its entry is called
	/// with (a value given where more errors are allowed is converted:
	/// `adapt_function_errors`).
	pub(crate) fn make_closure_of(
		&self,
		function: PointerValue<'ctx>,
		environment: PointerValue<'ctx>,
		ty: TypeId,
	) -> Result<Val<'ctx>, Error> {
		let closure = self.closure_type();
		let pair = self
			.builder
			.build_insert_value(closure.get_undef(), function, 0, "")?;
		let pair = self.builder.build_insert_value(pair, environment, 1, "")?;
		Ok(Val::new(Repr::Function(ty), pair.into_struct_value()))
	}

	/// The type of the entry of a function value: the function's own type
	/// (its errors, not those of the type it is given to).
	fn own_function_type(&self, function: &super::Function<'ctx>, node: NodeId) -> TypeId {
		let own = function.info.ty;
		if own.is_unset() || matches!(self.types().kind(own), TypeKind::Unknown) {
			self.type_of(node)
		} else {
			own
		}
	}

	/// A function declaration used as a value: a thunk with an environment.
	pub(crate) fn function_value(
		&mut self,
		declaration: NodeId,
		node: NodeId,
	) -> Result<Val<'ctx>, Error> {
		// `function f = g then h` (compose.rs)
		if let fa_analyzer::syntax::View::Function { value, .. } = self.view(declaration)
			&& self.analysis.compositions.contains_key(&value)
		{
			return self.composition_value(value);
		}
		if let fa_analyzer::syntax::View::Function { value, .. } = self.view(declaration)
			&& self.analysis.partial_applications.contains_key(&value)
		{
			return self.partial_value(value);
		}
		let declaration = if self
			.generic_functions
			.get(&declaration)
			.is_some_and(|info| info.generics.is_empty())
		{
			self.lambda_instance(declaration, &[])?
		} else {
			declaration
		};
		let Some(function) = self.functions.get(&declaration).cloned() else {
			// a generic function used as a generic function value (`generic_values.rs`)
			if self.generic_functions.contains_key(&declaration) {
				// given where an instance is expected (`names.map(identity)`)
				if self.analysis.instantiation(node).is_some()
					&& !self.is_generic_function_type(self.type_of(node))
				{
					let key = self.instance(declaration, node)?;
					let function = self.functions[&key].clone();
					let thunk = self.thunk_of(&function)?;
					// the type of the instance
					let list = self.analysis.instantiation(node).expect("an instantiation");
					let arguments: Vec<TypeId> = self
						.types()
						.list_items(list)
						.iter()
						.map(|&argument| self.concrete_type(argument))
						.collect();
					let generics = self.generic_functions[&declaration].generics.clone();
					let ty = self.substitute_params(self.type_of(node), &generics, &arguments);
					let closure = self.make_closure_of(
						thunk.as_global_value().as_pointer_value(),
						self.pointer_type().const_null(),
						ty,
					)?;
					return self.function_with_defaults(closure, &function);
				}
				return self.generic_declaration_value(declaration, node);
			}
			return Err(self.unsupported(node.index(), "this function value"));
		};
		if function.lambda {
			// a local function using locals around it: its own environment
			let (target, captures, by_reference) = (
				function.value,
				function.captures.clone(),
				function.by_reference,
			);
			let environment = self.build_environment(&captures, by_reference, node)?;
			let thunk = self.needs_value_thunk(&function);
			let target = if thunk {
				self.thunk_of(&function)?
			} else {
				target
			};
			let entry = self.async_value_entry(
				&function,
				target.as_global_value().as_pointer_value(),
				thunk,
			)?;
			let ty = self.own_function_type(&function, node);
			let closure = self.make_closure_of(entry, environment, ty)?;
			let closure = if by_reference {
				closure
			} else {
				self.fresh(closure)
			};
			let closure = self.function_with_defaults(closure, &function)?;
			// given where more errors are allowed: adapted (`adapt_function_errors`)
			return self.convert(closure, Repr::Function(self.type_of(node)), node.index());
		}
		let thunk = self.thunk_of(&function)?;
		// an async function: its async instance too (tasks.rs)
		let entry =
			self.async_value_entry(&function, thunk.as_global_value().as_pointer_value(), true)?;
		let ty = self.own_function_type(&function, node);
		let closure = self.make_closure_of(entry, self.pointer_type().const_null(), ty)?;
		let closure = self.function_with_defaults(closure, &function)?;
		// given where more errors are allowed: adapted (`adapt_function_errors`)
		self.convert(closure, Repr::Function(self.type_of(node)), node.index())
	}

	/// An explicitly applied function from another unit, with a concrete
	/// function type at this node and no captured environment.
	pub(crate) fn applied_external_function_value(
		&mut self,
		function: &super::Function<'ctx>,
		node: NodeId,
	) -> Result<Val<'ctx>, Error> {
		let thunk = self.thunk_of(function)?;
		let entry =
			self.async_value_entry(function, thunk.as_global_value().as_pointer_value(), true)?;
		let closure =
			self.make_closure_of(entry, self.pointer_type().const_null(), self.type_of(node))?;
		self.function_with_defaults(closure, function)
	}

	pub(super) fn external_declaration_value(
		&mut self,
		function: &super::Function<'ctx>,
		node: NodeId,
	) -> Result<Val<'ctx>, Error> {
		let thunk = self.thunk_of(function)?;
		let entry =
			self.async_value_entry(function, thunk.as_global_value().as_pointer_value(), true)?;
		let own = self.own_function_type(function, node);
		let closure = self.make_closure_of(entry, self.pointer_type().const_null(), own)?;
		let closure = self.function_with_defaults(closure, function)?;
		self.convert(closure, Repr::Function(self.type_of(node)), node.index())
	}

	/// Whether the raw source entry needs the function-value convention.
	pub(super) fn needs_value_thunk(&self, function: &super::Function<'ctx>) -> bool {
		!function.lambda
			|| function
				.params
				.iter()
				.enumerate()
				.any(|(index, &(repr, default))| {
					default || function.by_pointer[index] != self.value_borrows_parameter(repr)
				})
	}

	/// The thunk of a function declaration used as a value: the indirect
	/// call convention, whose environment it ignores.
	pub(crate) fn thunk_of(
		&mut self,
		function: &super::Function<'ctx>,
	) -> Result<FunctionValue<'ctx>, Error> {
		self.forwarding_thunk(function, function.value)
	}

	pub(super) fn default_forwarding_thunk(
		&self,
		function: &super::Function<'ctx>,
		target: FunctionValue<'ctx>,
	) -> Result<FunctionValue<'ctx>, Error> {
		self.forwarding_thunk_flags(function, target, true)
	}

	pub(super) fn forwarding_thunk(
		&self,
		function: &super::Function<'ctx>,
		target: FunctionValue<'ctx>,
	) -> Result<FunctionValue<'ctx>, Error> {
		self.forwarding_thunk_flags(function, target, false)
	}

	fn forwarding_thunk_flags(
		&self,
		function: &super::Function<'ctx>,
		target: FunctionValue<'ctx>,
		defaults: bool,
	) -> Result<FunctionValue<'ctx>, Error> {
		let name = format!(
			"{}.{}",
			target.get_name().to_string_lossy(),
			if defaults { "defaults" } else { "thunk" }
		);
		if let Some(thunk) = self.module.get_function(&name) {
			return Ok(thunk);
		}
		let target_type = target.get_type();
		let target_parameters = target_type.get_param_types();
		let params = function.params.clone();
		let by_pointer = function.by_pointer.clone();
		// the indirect ABI (`indirect_type`): every argument, the
		// aggregates borrowed (`value_borrows_parameter`), no default flags;
		// then the target's other parameters (requirements)
		let mut parameters: Vec<BasicMetadataTypeEnum<'ctx>> = vec![self.pointer_type().into()];
		let mut at = usize::from(function.lambda);
		for (repr, has_default) in &params {
			parameters.push(if self.value_borrows_parameter(*repr) {
				self.pointer_type().into()
			} else {
				self.llvm_type(*repr).into()
			});
			at += 1 + usize::from(*has_default);
		}
		let count = params.iter().filter(|(_, default)| *default).count();
		if defaults {
			parameters.extend(std::iter::repeat_n(
				BasicMetadataTypeEnum::from(self.context.bool_type()),
				count,
			));
		}
		parameters.extend(target_parameters.iter().skip(at).copied());
		let thunk_type = match target_type.get_return_type() {
			Some(ret) => ret.fn_type(&parameters, false),
			None => self.context.void_type().fn_type(&parameters, false),
		};
		let thunk = self
			.module
			.add_function(&name, thunk_type, Some(Linkage::Internal));
		thunk.set_call_conventions(FAST_CALLING_CONVENTION);
		self.runtime.add_attributes(thunk, &["nounwind"]);
		let builder = self.context.create_builder();
		builder.position_at_end(self.context.append_basic_block(thunk, "entry"));
		let mut arguments: Vec<BasicMetadataValueEnum<'ctx>> = Vec::new();
		if function.lambda {
			let environment = thunk
				.get_first_param()
				.expect("an environment")
				.into_pointer_value();
			let environment = if count != 0 {
				self.default_environment(&builder, environment)?
			} else {
				environment
			};
			arguments.push(environment.into());
		}
		let mut given = thunk.get_param_iter().skip(1).take(params.len());
		let mut flags = thunk.get_param_iter().skip(params.len() + 1);
		for (index, (repr, has_default)) in params.iter().enumerate() {
			let argument = given.next().expect("an argument");
			let borrowed = self.value_borrows_parameter(*repr);
			let argument = match (borrowed, by_pointer[index]) {
				(true, false) => builder.build_load(
					self.llvm_type(*repr),
					argument.into_pointer_value(),
					"argument",
				)?,
				(false, true) => {
					let slot = builder.build_alloca(self.llvm_type(*repr), "argument")?;
					builder.build_store(slot, argument)?;
					slot.into()
				}
				_ => argument,
			};
			arguments.push(argument.into());
			if *has_default {
				arguments.push(if defaults {
					flags.next().expect("a presence flag").into()
				} else {
					self.context.bool_type().const_int(1, false).into()
				});
			}
		}
		arguments.extend(
			thunk
				.get_param_iter()
				.skip(1 + params.len() + if defaults { count } else { 0 })
				.map(BasicMetadataValueEnum::from),
		);
		let call = builder.build_call(target, &arguments, "")?;
		call.set_call_convention(FAST_CALLING_CONVENTION);
		match call.try_as_basic_value().basic() {
			Some(result) => builder.build_return(Some(&result))?,
			None => builder.build_return(None)?,
		};
		Ok(thunk)
	}

	/// The IR signature of a call through a function value of type `ty`
	/// (without the environment): the parameter and result representations.
	pub(crate) fn value_signature(
		&self,
		ty: TypeId,
		node: NodeId,
	) -> Result<(Vec<Repr>, Repr, bool), Error> {
		let TypeKind::Function(sig) = self.types().kind(ty) else {
			return Err(self.unsupported(node.index(), "calling this value"));
		};
		let sig = self.types().sig(sig);
		let mut params = Vec::new();
		for param in self.types().params(sig.params) {
			params
				.push(self.repr_of(self.global_type(self.concrete_type(param.ty)), node.index())?);
		}
		let ret = self.repr_of(self.global_type(self.concrete_type(sig.ret)), node.index())?;
		Ok((params, ret, self.concrete_type(sig.throws) != TypeId::NEVER))
	}

	pub(crate) fn indirect_type(
		&self,
		params: &[Repr],
		ret: Repr,
		throws: bool,
	) -> FunctionType<'ctx> {
		self.indirect_type_with_requirements(params, ret, throws, 0)
	}

	pub(crate) fn indirect_type_with_requirements(
		&self,
		params: &[Repr],
		ret: Repr,
		throws: bool,
		requirements: usize,
	) -> FunctionType<'ctx> {
		let mut types: Vec<BasicMetadataTypeEnum<'ctx>> = vec![self.pointer_type().into()];
		types.extend(params.iter().map(|repr| {
			if self.value_borrows_parameter(*repr) {
				self.pointer_type().into()
			} else {
				BasicMetadataTypeEnum::from(self.llvm_type(*repr))
			}
		}));
		types.extend(std::iter::repeat_n(
			BasicMetadataTypeEnum::from(self.pointer_type()),
			requirements,
		));
		match (ret.has_value(), throws) {
			(true, false) => self.llvm_type(ret).fn_type(&types, false),
			(false, false) => self.context.void_type().fn_type(&types, false),
			(true, true) => self
				.context
				.struct_type(&[self.llvm_type(ret), self.pointer_type().into()], false)
				.fn_type(&types, false),
			(false, true) => self.pointer_type().fn_type(&types, false),
		}
	}

	pub(super) fn value_requirement_keys(&self, ty: TypeId) -> Vec<TypeId> {
		match self.types().kind(ty) {
			TypeKind::Function(signature) => self
				.types()
				.requirements(self.types().sig(signature).requires)
				.iter()
				.map(|requirement| self.concrete_type(requirement.ty))
				.collect(),
			_ => Vec::new(),
		}
	}

	/// Collection callbacks borrow their original element slots. A recursive
	/// parameter can use that address directly instead of allocating a copy.
	pub(super) fn call_collection_callback(
		&mut self,
		callback: Val<'ctx>,
		node: NodeId,
		before: &[Val<'ctx>],
		items: &[Slot<'ctx>],
		after: &[Val<'ctx>],
	) -> Result<Val<'ctx>, Error> {
		// the type of the value given (an instance of a generic lambda, `generic_values.rs`)
		let ty = match callback.repr {
			Repr::Function(ty)
				if self.is_generic_function_type(self.type_of(node))
					|| self.value_signature(self.type_of(node), node).is_err() =>
			{
				ty
			}
			_ => self.type_of(node),
		};
		let (params, _, _) = self.value_signature(ty, node)?;
		let mark = self.mark();
		let mut arguments = before.to_vec();
		let mut places = vec![None; before.len()];
		for &slot in items {
			let expected = params.get(arguments.len()).copied().unwrap_or(slot.repr);
			// an item of the parameter's representation: lent in place
			places.push(
				(slot.repr == expected && self.value_borrows_parameter(expected))
					.then_some(slot.pointer),
			);
			arguments.push(self.collection_item_view(slot, expected, node.index())?);
		}
		arguments.extend_from_slice(after);
		let result = self.call_closure_lending(callback, ty, &arguments, &places, node)?;
		self.end_scope(mark, Some(result))?;
		Ok(result)
	}

	/// Whether a callback given to a built-in may throw (its type's errors):
	/// the built-in then throws them (`design/decisions.md`).
	pub(crate) fn callback_may_throw(&self, node: NodeId) -> bool {
		self.value_signature(self.type_of(node), node)
			.is_ok_and(|(_, _, throws)| throws)
	}

	/// A callback call of a built-in's loop whose error must restore what
	/// the loop changed first (`fixup`: drops what was built, closes a gap),
	/// then leaves like any error (`raise`). Without errors, just `call`.
	pub(crate) fn callback_with_fixup(
		&mut self,
		callback_node: NodeId,
		call: impl FnOnce(&mut Self) -> Result<Val<'ctx>, Error>,
		fixup: impl FnOnce(&mut Self) -> Result<(), Error>,
	) -> Result<Val<'ctx>, Error> {
		if !self.callback_may_throw(callback_node) {
			return call(self);
		}
		let handler = self.append_block("callback_failed");
		let error_slot = self.alloca(Repr::Union, "callback_error")?;
		let mark = self.mark();
		self.frame_mut()
			.catches
			.push((handler, error_slot.pointer, mark));
		let result = call(self);
		self.frame_mut().catches.pop();
		let result = result?;
		let resume = self.builder.get_insert_block();
		self.builder.position_at_end(handler);
		fixup(self)?;
		let error = self
			.builder
			.build_load(self.pointer_type(), error_slot.pointer, "")?
			.into_pointer_value();
		self.raise(error)?;
		if let Some(resume) = resume {
			self.builder.position_at_end(resume);
		}
		Ok(result)
	}

	/// Calls a function value of type `ty` with lowered arguments (as many as
	/// its type declares: extra ones, like the `index` of a callback, are left out).
	pub(crate) fn call_closure(
		&mut self,
		closure: Val<'ctx>,
		ty: TypeId,
		arguments: &[Val<'ctx>],
		node: NodeId,
	) -> Result<Val<'ctx>, Error> {
		self.call_closure_lending(closure, ty, arguments, &[], node)
	}

	/// `call_closure` where some arguments are places (`places`, by
	/// position) that a borrowed parameter may see directly.
	pub(crate) fn call_closure_lending(
		&mut self,
		closure: Val<'ctx>,
		ty: TypeId,
		arguments: &[Val<'ctx>],
		places: &[Option<PointerValue<'ctx>>],
		node: NodeId,
	) -> Result<Val<'ctx>, Error> {
		// a value of a generic function type: its instance for the call (`generic_values.rs`)
		let (closure, ty) = if self.is_generic_function_type(ty) {
			let instance = self.generic_instance_value(closure, node)?;
			let Repr::Function(instance_type) = instance.repr else {
				return Err(self.unsupported(node.index(), "calling this generic function value"));
			};
			(instance, instance_type)
		} else {
			(closure, ty)
		};
		// Select the generic instance before reading the async signature:
		// its result and parameters may still contain the forall binders.
		let keys = self.value_requirement_keys(ty);
		let requirements = self.requirement_arguments(node, &keys)?;
		if let Some(value) =
			self.async_value_call(closure, ty, arguments, places, &requirements, node)?
		{
			return Ok(value);
		}
		let (mut params, ret, throws) = self.value_signature(ty, node)?;
		// a lambda or a function may declare fewer parameters than its type
		// (the `index` of a callback): it gets the ones it declares
		if let Some(arity) = self.declared_arity(node) {
			params.truncate(arity);
		}
		let pair = closure.value.expect("a function value").into_struct_value();
		let function = self
			.builder
			.build_extract_value(pair, 0, "")?
			.into_pointer_value();
		let environment = self.builder.build_extract_value(pair, 1, "")?;
		let mut lowered: Vec<BasicMetadataValueEnum<'ctx>> = vec![environment.into()];
		// Argument conversions can allocate (an inline collection item passed
		// as a recursive edge). Their temporaries belong to this call, which
		// may itself be inside a conditional callback branch.
		let mark = self.mark();
		lowered.extend(self.lower_value_arguments(&params, arguments, places, node)?);
		let flags = self.call_default_flags(node);
		let count = flags.len();
		lowered.extend(flags.into_iter().map(BasicMetadataValueEnum::from));
		lowered.extend(
			requirements
				.iter()
				.copied()
				.map(BasicMetadataValueEnum::from),
		);
		let fn_type =
			self.indirect_type_with_defaults(&params, ret, throws, count, requirements.len());
		let call = self
			.builder
			.build_indirect_call(fn_type, function, &lowered, "")?;
		call.set_call_convention(FAST_CALLING_CONVENTION);
		let result = self.call_result(call.try_as_basic_value().basic(), ret, throws)?;
		self.end_scope(mark, Some(result))?;
		Ok(result)
	}

	/// Function-value arguments have one convention for synchronous calls,
	/// awaits and starts. Changed aggregates preserve the caller's place.
	pub(super) fn lower_value_arguments(
		&mut self,
		params: &[Repr],
		arguments: &[Val<'ctx>],
		places: &[Option<PointerValue<'ctx>>],
		node: NodeId,
	) -> Result<Vec<BasicMetadataValueEnum<'ctx>>, Error> {
		let mut lowered = Vec::with_capacity(params.len());
		for (position, (repr, argument)) in params.iter().zip(arguments).enumerate() {
			if self.value_borrows_parameter(*repr) {
				// a borrowed parameter: the place itself, or the value in a
				// slot of the caller for the call
				let pointer = match places.get(position).copied().flatten() {
					Some(pointer) => pointer,
					None => {
						let argument = self.convert(*argument, *repr, node.index())?;
						let slot = self.alloca(*repr, "argument")?;
						self.builder
							.build_store(slot.pointer, argument.value.expect("an argument"))?;
						slot.pointer
					}
				};
				lowered.push(pointer.into());
				continue;
			}
			let argument = self.convert(*argument, *repr, node.index())?;
			lowered.push(argument.value.expect("an argument").into());
		}
		Ok(lowered)
	}

	/// The value of a call: raises its error, if it throws.
	pub(crate) fn call_result(
		&mut self,
		result: Option<inkwell::values::BasicValueEnum<'ctx>>,
		ret: Repr,
		throws: bool,
	) -> Result<Val<'ctx>, Error> {
		if throws {
			let result = result.expect("a result");
			let (value, error) = if ret.has_value() {
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
			return Ok(match value {
				Some(value) if ret != Repr::Never => self.fresh(Val::new(ret, value)),
				_ => Val::void(),
			});
		}
		if ret == Repr::Never {
			self.builder.build_unreachable()?;
			self.continue_unreachable();
			return Ok(Val::never());
		}
		Ok(match result {
			Some(result) => self.fresh(Val::new(ret, result)),
			None => Val::void(),
		})
	}

	/// The number of parameters a lambda or a named function declares.
	pub(crate) fn declared_arity(&self, node: NodeId) -> Option<usize> {
		let mut node = node;
		while let fa_analyzer::syntax::View::Group(inner) = self.view(node) {
			node = inner;
		}
		if let Some(function) = self.functions.get(&node) {
			return Some(function.info.params.len());
		}
		if let Resolution::Local(symbol) = self.analysis.resolution(node) {
			let declaration = self.analysis.symbols.symbol(symbol).decl_node;
			return self
				.functions
				.get(&declaration)
				.map(|function| function.info.params.len());
		}
		None
	}

	/// The function value of a name: a lambda's, or a declaration's thunk.
	pub(crate) fn function_of_symbol(
		&mut self,
		symbol: SymbolId,
		node: NodeId,
	) -> Result<Val<'ctx>, Error> {
		let declaration = self.analysis.symbols.symbol(symbol).decl_node;
		if let Some(&source) = self.analysis.static_function_aliases.get(&declaration) {
			return self.expr(source);
		}
		if let fa_analyzer::syntax::View::Function { value, .. } = self.view(declaration)
			&& self.analysis.function_constants.contains_key(&value)
		{
			return self.expr(value);
		}
		if self.analysis.computed_functions.contains(&declaration) {
			let slot = self.slot_of(symbol, node)?;
			return self.load(slot);
		}
		if !self.analysis.functions.contains_key(&declaration)
			&& let fa_analyzer::syntax::View::Function { value, .. } = self.view(declaration)
		{
			return self.expr(value);
		}
		self.function_value(declaration, node)
	}
}
