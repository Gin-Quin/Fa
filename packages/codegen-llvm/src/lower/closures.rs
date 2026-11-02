//! Function values: lambdas, and functions used as values.
//!
//! A function value is a pair `{ ptr function, ptr environment }`. A lambda
//! is an `internal fastcc` function whose first parameter is its
//! environment: a block holding a copy of the locals of the enclosing
//! functions it reads, made when the lambda is evaluated. A function
//! declaration used as a value goes through a thunk that ignores the
//! environment. Calling a function value is an indirect call, with the
//! signature of the value's own type.

use fa_analyzer::ids::{NodeId, SymbolId, TypeId};
use fa_analyzer::symbols::{Resolution, SymbolKind};
use fa_analyzer::types::TypeKind;
use inkwell::module::Linkage;
use inkwell::types::{BasicMetadataTypeEnum, BasicType, BasicTypeEnum, FunctionType, StructType};
use inkwell::values::{BasicMetadataValueEnum, PointerValue};

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
		let span = self.tree.spans[lambda.index()];
		let mut captures = Vec::new();
		let mut stack = vec![lambda.index()];
		while let Some(node) = stack.pop() {
			if let Resolution::Local(symbol) = self.analysis.resolution(NodeId::new(node)) {
				let data = self.analysis.symbols.symbol(symbol);
				let variable = !matches!(
					data.kind,
					SymbolKind::Function | SymbolKind::Type | SymbolKind::Namespace
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

	/// The struct of an environment: the captured values.
	fn environment_type(&self, captures: &[SymbolId]) -> Result<StructType<'ctx>, Error> {
		let mut fields: Vec<BasicTypeEnum<'ctx>> = Vec::new();
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
	) -> Result<Vec<(SymbolId, Option<Slot<'ctx>>)>, Error> {
		let structure = self.environment_type(captures)?;
		let mut saved = Vec::new();
		for (index, &symbol) in captures.iter().enumerate() {
			let data = self.analysis.symbols.symbol(symbol);
			let repr = self.repr_of(data.ty, data.decl_node.index())?;
			let pointer =
				self.builder
					.build_struct_gep(structure, environment, index as u32, "")?;
			let value = self.builder.build_load(self.llvm_type(repr), pointer, "")?;
			let slot = self.alloca(repr, "captured")?;
			self.builder.build_store(slot.pointer, value)?;
			saved.push((symbol, self.slots.insert(symbol, slot)));
		}
		Ok(saved)
	}

	/// A lambda evaluated: its function and a new environment.
	pub(crate) fn lambda_value(&mut self, node: NodeId) -> Result<Val<'ctx>, Error> {
		let Some(function) = self.functions.get(&node) else {
			return Err(self.unsupported(node.index(), "this lambda"));
		};
		let (value, captures) = (function.value, function.captures.clone());
		let environment = if captures.is_empty() {
			self.pointer_type().const_null()
		} else {
			let structure = self.environment_type(&captures)?;
			let pointer = self.allocate(structure.size_of().expect("a sized environment"))?;
			for (index, &symbol) in captures.iter().enumerate() {
				let slot = self.slot_of(symbol, node)?;
				let loaded = self.load(slot)?;
				let field = self
					.builder
					.build_struct_gep(structure, pointer, index as u32, "")?;
				self.builder
					.build_store(field, loaded.value.expect("a value"))?;
			}
			pointer
		};
		self.make_closure(
			value.as_global_value().as_pointer_value(),
			environment,
			node,
		)
	}

	fn make_closure(
		&self,
		function: PointerValue<'ctx>,
		environment: PointerValue<'ctx>,
		node: NodeId,
	) -> Result<Val<'ctx>, Error> {
		let ty = self.closure_type();
		let pair = self
			.builder
			.build_insert_value(ty.get_undef(), function, 0, "")?;
		let pair = self.builder.build_insert_value(pair, environment, 1, "")?;
		Ok(Val::new(
			Repr::Function(self.type_of(node)),
			pair.into_struct_value(),
		))
	}

	/// A function declaration used as a value: a thunk with an environment.
	pub(crate) fn function_value(
		&mut self,
		declaration: NodeId,
		node: NodeId,
	) -> Result<Val<'ctx>, Error> {
		let Some(function) = self.functions.get(&declaration) else {
			return Err(self.unsupported(node.index(), "this function value"));
		};
		let target = function.value;
		let name = format!("{}.thunk", target.get_name().to_string_lossy());
		let thunk = match self.module.get_function(&name) {
			Some(thunk) => thunk,
			None => {
				let target_type = target.get_type();
				let mut parameters: Vec<BasicMetadataTypeEnum<'ctx>> =
					vec![self.pointer_type().into()];
				parameters.extend(target_type.get_param_types());
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
				let arguments: Vec<BasicMetadataValueEnum<'ctx>> =
					thunk.get_param_iter().skip(1).map(Into::into).collect();
				let call = builder.build_call(target, &arguments, "")?;
				call.set_call_convention(FAST_CALLING_CONVENTION);
				match call.try_as_basic_value().basic() {
					Some(result) => builder.build_return(Some(&result))?,
					None => builder.build_return(None)?,
				};
				thunk
			}
		};
		self.make_closure(
			thunk.as_global_value().as_pointer_value(),
			self.pointer_type().const_null(),
			node,
		)
	}

	/// The IR signature of a call through a function value of type `ty`
	/// (without the environment): the parameter and result representations.
	fn value_signature(&self, ty: TypeId, node: NodeId) -> Result<(Vec<Repr>, Repr, bool), Error> {
		let TypeKind::Function(sig) = self.types().kind(ty) else {
			return Err(self.unsupported(node.index(), "calling this value"));
		};
		let sig = self.types().sig(sig);
		let mut params = Vec::new();
		for param in self.types().params(sig.params) {
			params.push(self.repr_of(param.ty, node.index())?);
		}
		let ret = self.repr_of(sig.ret, node.index())?;
		Ok((params, ret, sig.throws != TypeId::NEVER))
	}

	fn indirect_type(&self, params: &[Repr], ret: Repr, throws: bool) -> FunctionType<'ctx> {
		let mut types: Vec<BasicMetadataTypeEnum<'ctx>> = vec![self.pointer_type().into()];
		types.extend(
			params
				.iter()
				.map(|repr| BasicMetadataTypeEnum::from(self.llvm_type(*repr))),
		);
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

	/// Calls a function value of type `ty` with lowered arguments (as many as
	/// its type declares: extra ones, like the `index` of a callback, are left out).
	pub(crate) fn call_closure(
		&mut self,
		closure: Val<'ctx>,
		ty: TypeId,
		arguments: &[Val<'ctx>],
		node: NodeId,
	) -> Result<Val<'ctx>, Error> {
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
		for (repr, argument) in params.iter().zip(arguments) {
			let argument = self.convert(*argument, *repr, node.index())?;
			lowered.push(argument.value.expect("an argument").into());
		}
		let fn_type = self.indirect_type(&params, ret, throws);
		let call = self
			.builder
			.build_indirect_call(fn_type, function, &lowered, "")?;
		call.set_call_convention(FAST_CALLING_CONVENTION);
		self.call_result(call.try_as_basic_value().basic(), ret, throws)
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
				Some(value) if ret != Repr::Never => Val::new(ret, value),
				_ => Val::void(),
			});
		}
		if ret == Repr::Never {
			self.builder.build_unreachable()?;
			self.continue_unreachable();
			return Ok(Val::never());
		}
		Ok(match result {
			Some(result) => Val::new(ret, result),
			None => Val::void(),
		})
	}

	/// The number of parameters a lambda or a named function declares.
	fn declared_arity(&self, node: NodeId) -> Option<usize> {
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
		self.function_value(declaration, node)
	}
}
