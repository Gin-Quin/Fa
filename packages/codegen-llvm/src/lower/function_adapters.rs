//! A function coerced to another signature owns its original callable in
//! a counted environment. Its compiled entries convert arguments and results,
//! preserve mutations of structural views, and forward requirements by key.

use fa_analyzer::analysis::GenericMethod;
use fa_analyzer::ids::{FileId, NodeId, TypeId};
use fa_analyzer::types::TypeKind;
use inkwell::module::Linkage;
use inkwell::values::{BasicMetadataValueEnum, FunctionValue, GlobalValue, PointerValue};

use super::ownership::Cleanup;
use super::references::WriteBack;
use super::values::{Repr, Val};
use super::{FAST_CALLING_CONVENTION, Lowering, Slot};
use crate::error::Error;

pub(super) struct FunctionAdapter<'ctx> {
	file: FileId,
	node: NodeId,
	from: TypeId,
	to: TypeId,
	synchronous: FunctionValue<'ctx>,
	asynchronous: Option<FunctionValue<'ctx>>,
	selection: Option<GenericSelection>,
}

#[derive(Clone)]
struct GenericSelection {
	generic: TypeId,
	arguments: Vec<TypeId>,
	methods: Vec<GenericMethod>,
}

pub(super) struct GenericFunctionAdapter<'ctx> {
	file: FileId,
	node: NodeId,
	from: TypeId,
	to: TypeId,
	placeholder: GlobalValue<'ctx>,
}

impl<'ctx> Lowering<'_, 'ctx> {
	pub(crate) fn adapt_function_errors(
		&self,
		value: Val<'ctx>,
		from: TypeId,
		to: TypeId,
		node: usize,
	) -> Result<Val<'ctx>, Error> {
		let retagged = Val {
			repr: Repr::Function(to),
			value: value.value,
		};
		if value.value.is_none() {
			return Ok(retagged);
		}
		let from = self.global_type(self.concrete_type(from));
		let to = self.global_type(self.concrete_type(to));
		let node = NodeId::new(node);
		if self.is_generic_function_type(from) {
			if self.is_generic_function_type(to) {
				if self.class_key(from) == self.class_key(to) {
					return Ok(retagged);
				}
				return self.generic_function_adapter(value, from, to, node);
			}
			let Some(list) = self.analysis.instantiation(node) else {
				return Err(self.unsupported(
					node.index(),
					"a function instance without its type arguments",
				));
			};
			let given = self.types().list_items(list);
			let count = self.own_generics(from).len();
			if given.len() < count {
				return Err(self.unsupported(node.index(), "an incomplete function instance"));
			}
			let offset = given.len() - count;
			let arguments = given[offset..]
				.iter()
				.map(|&ty| self.global_type(self.concrete_type(ty)))
				.collect::<Vec<_>>();
			let mut methods = self.call_methods(node, given);
			methods.retain_mut(|method| {
				if usize::from(method.parameter) < offset {
					return false;
				}
				method.parameter -= offset as u16;
				true
			});
			let concrete = self.instantiate_generic(from, &arguments);
			let selection = GenericSelection {
				generic: from,
				arguments,
				methods,
			};
			let (synchronous, asynchronous, key) =
				self.adapter_entries(concrete, to, Some(selection), node)?;
			let entry = self.adapter_entry_pointer(synchronous, asynchronous, &key);
			return self.adapter_environment(value, from, to, entry, &key);
		}
		let (source_params, source_ret, source_throws) = self.value_signature(from, node)?;
		let (params, ret, throws) = self.value_signature(to, node)?;
		let requirements = self.value_requirement_keys(to);
		let source_requirements = self.value_requirement_keys(from);
		let lifted = self.tasks.asynchronous
			&& matches!(self.types().kind(to), TypeKind::Function(sig) if self.types().sig(sig).is_async)
			&& !matches!(self.types().kind(from), TypeKind::Function(sig) if self.types().sig(sig).is_async);
		if source_params == params
			&& source_ret == ret
			&& source_throws == throws
			&& source_requirements == requirements
			&& !lifted
		{
			return Ok(retagged);
		}
		let (synchronous, asynchronous, key) = self.adapter_entries(from, to, None, node)?;
		let entry = self.adapter_entry_pointer(synchronous, asynchronous, &key);
		self.adapter_environment(value, from, to, entry, &key)
	}

	fn adapter_entries(
		&self,
		from: TypeId,
		to: TypeId,
		selection: Option<GenericSelection>,
		node: NodeId,
	) -> Result<(FunctionValue<'ctx>, Option<FunctionValue<'ctx>>, String), Error> {
		let (params, ret, throws) = self.value_signature(to, node)?;
		let requirements = self.value_requirement_keys(to);
		let mut key = format!(
			"{:x}.{:x}",
			self.types().stable_hash(from),
			self.types().stable_hash(to)
		);
		if let Some(selection) = &selection {
			let arguments: Vec<_> = selection
				.arguments
				.iter()
				.map(|&ty| self.types().stable_hash(ty))
				.collect();
			let methods: Vec<_> = selection
				.methods
				.iter()
				.map(|method| method.stable_hash(self.types()))
				.collect();
			key.push_str(&format!(
				".generic.{:x}.{arguments:x?}.{methods:x?}",
				self.types().stable_hash(selection.generic)
			));
		}
		let name = format!("fa.adapt_function.{key}");
		let synchronous = self.module.get_function(&name).unwrap_or_else(|| {
			self.module.add_function(
				&name,
				self.indirect_type_with_requirements(&params, ret, throws, requirements.len()),
				Some(Linkage::Internal),
			)
		});
		let first = synchronous.count_basic_blocks() == 0
			&& !self
				.function_adapters
				.borrow()
				.iter()
				.any(|adapter| adapter.synchronous == synchronous);
		let asynchronous = if self.tasks.asynchronous
			&& matches!(self.types().kind(to), TypeKind::Function(sig) if self.types().sig(sig).is_async)
		{
			let name = format!("{name}.async");
			let mut parameters = synchronous.get_type().get_param_types();
			parameters.push(self.context.i32_type().into());
			Some(self.module.get_function(&name).unwrap_or_else(|| {
				self.module.add_function(
					&name,
					self.pointer_type().fn_type(&parameters, false),
					Some(Linkage::Internal),
				)
			}))
		} else {
			None
		};
		if first {
			for function in [Some(synchronous), asynchronous].into_iter().flatten() {
				function.set_call_conventions(FAST_CALLING_CONVENTION);
				self.runtime.add_attributes(function, &["nounwind"]);
			}
			self.function_adapters.borrow_mut().push(FunctionAdapter {
				file: self.file,
				node,
				from,
				to,
				synchronous,
				asynchronous,
				selection,
			});
		}
		Ok((synchronous, asynchronous, key))
	}

	fn adapter_entry_pointer(
		&self,
		synchronous: FunctionValue<'ctx>,
		asynchronous: Option<FunctionValue<'ctx>>,
		key: &str,
	) -> PointerValue<'ctx> {
		let mut entry = synchronous.as_global_value().as_pointer_value();
		if let Some(asynchronous) = asynchronous {
			let pointer = self.pointer_type();
			let name = format!("fa.adapt_function.{key}.entries");
			let table = self.module.get_global(&name).unwrap_or_else(|| {
				let ty = self
					.context
					.struct_type(&[pointer.into(), pointer.into()], false);
				let table = self.module.add_global(ty, None, &name);
				table.set_initializer(&ty.const_named_struct(&[
					entry.into(),
					asynchronous.as_global_value().as_pointer_value().into(),
				]));
				table.set_constant(true);
				table.set_linkage(Linkage::Private);
				table.set_alignment(16);
				table
			});
			// SAFETY: one byte into the two-pointer entry table.
			entry = unsafe {
				table
					.as_pointer_value()
					.const_gep(self.context.i8_type(), &[self.usize.const_int(1, false)])
			};
		}
		entry
	}

	fn adapter_environment(
		&self,
		value: Val<'ctx>,
		from: TypeId,
		to: TypeId,
		entry: PointerValue<'ctx>,
		key: &str,
	) -> Result<Val<'ctx>, Error> {
		let reprs = [Repr::Function(from)];
		let layout = self.partial_layout(&reprs);
		let (drop, copy) =
			self.partial_environment_functions(&format!("adapter.{key}"), layout, &reprs)?;
		let source = self.own(value, None)?;
		let environment = self.allocate(layout.size_of().expect("a sized environment"))?;
		for (index, function) in [drop, copy].into_iter().enumerate() {
			let field = self
				.builder
				.build_struct_gep(layout, environment, index as u32, "")?;
			self.builder
				.build_store(field, function.as_global_value().as_pointer_value())?;
		}
		let count = self.builder.build_struct_gep(layout, environment, 2, "")?;
		self.builder
			.build_store(count, self.context.i64_type().const_int(1, false))?;
		let field = self.builder.build_struct_gep(layout, environment, 3, "")?;
		self.builder
			.build_store(field, source.value.expect("a function"))?;
		self.unregister(source);
		Ok(self.fresh(self.make_closure_of(entry, environment, to)?))
	}

	fn generic_function_adapter(
		&self,
		value: Val<'ctx>,
		from: TypeId,
		to: TypeId,
		node: NodeId,
	) -> Result<Val<'ctx>, Error> {
		let key = format!(
			"{:x}.{:x}",
			self.types().stable_hash(from),
			self.types().stable_hash(to)
		);
		let name = format!("fa.generic_adapter.{key}");
		let placeholder = self.module.get_global(&name).unwrap_or_else(|| {
			let placeholder = self.module.add_global(self.context.i8_type(), None, &name);
			placeholder.set_linkage(Linkage::Internal);
			self.generic_function_adapters
				.borrow_mut()
				.push(GenericFunctionAdapter {
					file: self.file,
					node,
					from,
					to,
					placeholder,
				});
			placeholder
		});
		self.adapter_environment(
			value,
			from,
			to,
			placeholder.as_pointer_value(),
			&format!("generic.{key}"),
		)
	}

	pub(super) fn generic_adapter_entry(
		&mut self,
		node: NodeId,
		from: TypeId,
		to: TypeId,
		arguments: &[TypeId],
		methods: &[GenericMethod],
	) -> Result<PointerValue<'ctx>, Error> {
		let from_instance = self.instantiate_generic(from, arguments);
		let to_instance = self.instantiate_generic(to, arguments);
		let selection = GenericSelection {
			generic: from,
			arguments: arguments.to_vec(),
			methods: methods.to_vec(),
		};
		let (synchronous, asynchronous, _) =
			self.adapter_entries(from_instance, to_instance, Some(selection), node)?;
		let sync = synchronous.as_global_value().as_pointer_value();
		Ok(match asynchronous {
			Some(asynchronous) => {
				self.async_entries(sync, asynchronous.as_global_value().as_pointer_value())
			}
			None => sync,
		})
	}

	pub(super) fn lower_next_function_adapter(&mut self) -> Result<bool, Error> {
		let generic = self.generic_function_adapters.borrow_mut().pop();
		if let Some(generic) = generic {
			self.switch_file(generic.file);
			let table = self.generic_table_of(
				generic.to,
				generic.node,
				super::generic_values::ProducerKind::Adapter {
					from: generic.from,
					to: generic.to,
				},
			)?;
			generic
				.placeholder
				.as_pointer_value()
				.replace_all_uses_with(table);
			// SAFETY: every use now refers to the producer table.
			unsafe {
				generic.placeholder.delete();
			}
			return Ok(true);
		}
		let request = self.function_adapters.borrow_mut().pop();
		let Some(request) = request else {
			return Ok(false);
		};
		self.switch_file(request.file);
		self.define_function_adapter(&request, false)?;
		if request.asynchronous.is_some() {
			self.define_function_adapter(&request, true)?;
		}
		Ok(true)
	}

	fn define_function_adapter(
		&mut self,
		adapter: &FunctionAdapter<'ctx>,
		asynchronous: bool,
	) -> Result<(), Error> {
		let function = if asynchronous {
			adapter.asynchronous.expect("an async entry")
		} else {
			adapter.synchronous
		};
		let (params, ret, throws) = self.value_signature(adapter.to, adapter.node)?;
		let (source_params, source_ret, source_throws) =
			self.value_signature(adapter.from, adapter.node)?;
		let requirements = self.value_requirement_keys(adapter.to);
		let source_requirements = self.value_requirement_keys(adapter.from);
		let changed = self.changed_parameters(adapter.from);
		self.nested_function(function, ret, |this| {
			this.frame_mut().owner = Some(adapter.node);
			this.frame_mut().throws = throws;
			let environment = function
				.get_first_param()
				.expect("an environment")
				.into_pointer_value();
			if asynchronous {
				let mode = function.get_last_param().expect("a mode").into_int_value();
				this.begin_coroutine(ret, throws, mode, true, false, "a function adapter".into())?;
				let owner = this.make_closure_of(
					this.pointer_type().const_null(),
					environment,
					adapter.to,
				)?;
				let owned = this.emit_copy(owner)?;
				let slot = this.alloca(owned.repr, "adapter_environment")?;
				this.store(slot, owned, adapter.node)?;
				this.cleanups.borrow_mut().push(Cleanup::Local(slot));
			}
			let captured = adapter
				.selection
				.as_ref()
				.map_or(adapter.from, |selection| selection.generic);
			let layout = this.partial_layout(&[Repr::Function(captured)]);
			let field = this.builder.build_struct_gep(layout, environment, 3, "")?;
			let source = Val::new(
				Repr::Function(adapter.from),
				this.builder.build_load(this.closure_type(), field, "")?,
			);
			let source = if let Some(selection) = &adapter.selection {
				let reprs = selection
					.arguments
					.iter()
					.map(|&ty| this.repr_of(ty, adapter.node.index()))
					.collect::<Result<Vec<_>, _>>()?;
				let index = this.instance_index(
					selection.generic,
					reprs,
					&selection.arguments,
					&selection.methods,
					adapter.node,
				)?;
				let pair = source
					.value
					.expect("a generic function")
					.into_struct_value();
				let table = this
					.builder
					.build_extract_value(pair, 0, "")?
					.into_pointer_value();
				let environment = this
					.builder
					.build_extract_value(pair, 1, "")?
					.into_pointer_value();
				// SAFETY: every producer defines the class instance we request.
				let slot = unsafe {
					this.builder
						.build_in_bounds_gep(this.pointer_type(), table, &[index], "")?
				};
				let entry = this
					.builder
					.build_load(this.pointer_type(), slot, "")?
					.into_pointer_value();
				this.make_closure_of(entry, environment, adapter.from)?
			} else {
				source
			};
			let TypeKind::Function(signature) = this.types().kind(adapter.to) else {
				unreachable!()
			};
			let parameter_types = this
				.types()
				.params(this.types().sig(signature).params)
				.to_vec();
			let TypeKind::Function(signature) = this.types().kind(adapter.from) else {
				unreachable!()
			};
			let source_types = this
				.types()
				.params(this.types().sig(signature).params)
				.to_vec();
			let mut places = Vec::new();
			let mut values = Vec::new();
			let mut borrowed = Vec::new();
			for (index, &repr) in params.iter().enumerate() {
				let argument = function
					.get_nth_param(index as u32 + 1)
					.expect("an argument");
				let slot = if this.value_borrows_parameter(repr) {
					Slot {
						pointer: argument.into_pointer_value(),
						repr,
					}
				} else {
					let slot = this.alloca(repr, "adapter_parameter")?;
					this.builder.build_store(slot.pointer, argument)?;
					slot
				};
				let value = this.load(slot)?;
				let consumed = this.types().takes_task_parameter(parameter_types[index].ty);
				if consumed {
					this.cleanups.borrow_mut().push(Cleanup::Local(slot));
				} else {
					borrowed.push((value, false));
				}
				places.push(slot);
				values.push(value);
			}
			if asynchronous {
				this.prepare_value_parameters(&borrowed)?;
			}
			let mut arguments = Vec::new();
			let mut argument_places = Vec::new();
			for (index, &repr) in source_params.iter().enumerate() {
				let place = places[index];
				let consumed = this.types().takes_task_parameter(source_types[index].ty);
				let value = if consumed {
					this.forget_local(place);
					let value = this.fresh(values[index]);
					this.prepare_owned_projection(value, repr, adapter.node)?
				} else {
					this.convert(values[index], repr, adapter.node.index())?
				};
				let pointer = if this.value_borrows_parameter(repr) {
					if place.repr == repr || this.same_layout(place.repr, repr) {
						Some(place.pointer)
					} else {
						let view = this.alloca(repr, "adapter_view")?;
						this.store(view, value, adapter.node)?;
						if changed.contains(&index) {
							this.cleanups
								.borrow_mut()
								.push(Cleanup::WriteBack(WriteBack::Fields { place, view }));
						}
						Some(view.pointer)
					}
				} else {
					None
				};
				arguments.push(value);
				argument_places.push(pointer);
			}
			let requirements: Vec<_> = requirements
				.iter()
				.enumerate()
				.map(|(index, &ty)| {
					(
						function
							.get_nth_param(params.len() as u32 + 1 + index as u32)
							.expect("a requirement")
							.into_pointer_value(),
						ty,
					)
				})
				.collect();
			let source_requirements = source_requirements
				.iter()
				.map(|key| {
					requirements
						.iter()
						.find(|(_, ty)| ty == key)
						.map(|(pointer, _)| *pointer)
						.ok_or_else(|| {
							this.unsupported(
								adapter.node.index(),
								"an adapter requirement absent from its contract",
							)
						})
				})
				.collect::<Result<Vec<_>, _>>()?;
			if asynchronous {
				this.suspend_value_body()?;
			}
			for (index, value) in arguments.iter_mut().enumerate() {
				if this.types().takes_task_parameter(source_types[index].ty) {
					*value = this.take(*value, None)?;
				}
			}
			let result = if asynchronous {
				// Every source parameter is supplied: there are no default
				// expressions to execute before the wrapper's first turn.
				let child = this.create_wrapped_frame(
					source,
					adapter.from,
					(&arguments, &argument_places),
					&[],
					&source_requirements,
					adapter.node,
				)?;
				this.await_wrapped_frame(child, source_ret, source_throws, adapter.node)?
			} else {
				this.call_adapter_source(
					source,
					adapter.from,
					(&arguments, &argument_places),
					&source_requirements,
					adapter.node,
				)?
			};
			let result = if source_ret != ret && ret.has_value() {
				this.prepare_owned_projection(result, ret, adapter.node)?
			} else {
				result
			};
			this.build_return(result, adapter.node)?;
			this.cleanups.borrow_mut().clear();
			if asynchronous {
				this.end_coroutine()?;
			}
			Ok(())
		})
	}

	fn call_adapter_source(
		&mut self,
		source: Val<'ctx>,
		ty: TypeId,
		arguments: (&[Val<'ctx>], &[Option<PointerValue<'ctx>>]),
		requirements: &[PointerValue<'ctx>],
		node: NodeId,
	) -> Result<Val<'ctx>, Error> {
		let (params, ret, throws) = self.value_signature(ty, node)?;
		let pair = source.value.expect("a function").into_struct_value();
		let mut entry = self
			.builder
			.build_extract_value(pair, 0, "")?
			.into_pointer_value();
		if matches!(self.types().kind(ty), TypeKind::Function(sig) if self.types().sig(sig).is_async)
		{
			entry = self.synchronous_entry(self.frame().function, &self.builder, entry)?;
		}
		let mut lowered: Vec<BasicMetadataValueEnum<'ctx>> =
			vec![self.builder.build_extract_value(pair, 1, "")?.into()];
		lowered.extend(self.lower_value_arguments(&params, arguments.0, arguments.1, node)?);
		lowered.extend(
			requirements
				.iter()
				.copied()
				.map(BasicMetadataValueEnum::from),
		);
		let signature =
			self.indirect_type_with_requirements(&params, ret, throws, requirements.len());
		let call = self
			.builder
			.build_indirect_call(signature, entry, &lowered, "")?;
		call.set_call_convention(FAST_CALLING_CONVENTION);
		self.call_result(call.try_as_basic_value().basic(), ret, throws)
	}
}
