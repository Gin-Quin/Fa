//! Compile-time closures use ordinary AOT entries and owned environments.
//! Composite constants delegate to the existing typed constant emitter.

use std::sync::Arc;

use fa_analyzer::constants::{ConstantFunction, ConstantKind, ConstantValue};
use fa_analyzer::ids::NodeId;
use fa_analyzer::types::TypeKind;
use fa_interpreter::Value;
use fa_interpreter::value::{Closure, Member, Object};
use inkwell::module::Linkage;
use inkwell::types::BasicTypeEnum;
use inkwell::values::PointerValue;
use rustc_hash::FxHashMap;

use crate::error::Error;
use crate::lower::Lowering;
use crate::lower::generic_values::ProducerKind;
use crate::lower::values::Val;

impl<'ctx> Lowering<'_, 'ctx> {
	pub(super) fn compiled_function_constant(
		&mut self,
		constant: &ConstantValue,
		node: NodeId,
	) -> Result<Val<'ctx>, Error> {
		let mut functions = FxHashMap::default();
		let value = self.marshal_function_constant(constant, &mut functions);
		self.constant_value_with_functions(
			&value,
			self.concrete_type(constant.ty),
			node,
			&functions,
		)
	}

	fn marshal_function_constant(
		&self,
		constant: &ConstantValue,
		functions: &mut FxHashMap<usize, Arc<ConstantFunction>>,
	) -> Value {
		match &constant.kind {
			ConstantKind::Data(value) => value.clone(),
			ConstantKind::Function(function) => {
				let value = Arc::new(Closure {
					function: fa_interpreter::ir::FnKey(0, 0),
					captured: Vec::new(),
					bound: Vec::new(),
					partials: Vec::new(),
				});
				functions.insert(Arc::as_ptr(&value) as usize, Arc::clone(function));
				Value::Function(value)
			}
			ConstantKind::Array(values)
			| ConstantKind::Tuple(values)
			| ConstantKind::Set(values) => {
				let values = Arc::new(
					values
						.iter()
						.map(|value| self.marshal_function_constant(value, functions))
						.collect(),
				);
				match constant.kind {
					ConstantKind::Array(_) => Value::Array(values),
					ConstantKind::Tuple(_) => Value::Tuple(values),
					_ => Value::Set(values),
				}
			}
			ConstantKind::Map(entries) => Value::Map(Arc::new(
				entries
					.iter()
					.map(|(key, value)| {
						(
							self.marshal_function_constant(key, functions),
							self.marshal_function_constant(value, functions),
						)
					})
					.collect(),
			)),
			ConstantKind::Object(fields) => Value::Object(Arc::new(Object {
				fields: fields
					.iter()
					.map(|(name, value)| {
						(
							Arc::from(self.types().syms.resolve(*name)),
							self.marshal_function_constant(value, functions),
						)
					})
					.collect(),
			})),
			ConstantKind::Member { name, kind, data } => Value::Member(Arc::new(Member {
				name: Arc::clone(name),
				kind: *kind,
				data: Some(self.marshal_function_constant(data, functions)),
			})),
		}
	}

	pub(super) fn constant_function(
		&mut self,
		constant: &ConstantFunction,
		node: NodeId,
	) -> Result<Val<'ctx>, Error> {
		let mut bindings = FxHashMap::default();
		let mut types = FxHashMap::default();
		for &(param, ty) in &constant.bindings {
			if matches!(self.types().kind(ty), TypeKind::Param(same) if same == param) {
				continue;
			}
			let ty = self.concrete_type(ty);
			bindings.insert(param, self.repr_of(ty, node.index())?);
			types.insert(param, ty);
		}
		let saved = self.bindings.replace(bindings);
		let saved_types = self.type_bindings.replace(types);
		let saved_methods = self
			.method_bindings
			.replace(constant.capabilities.methods.clone());
		let result = self.constant_function_bound(constant, node);
		self.bindings.replace(saved);
		self.type_bindings.replace(saved_types);
		self.method_bindings.replace(saved_methods);
		result
	}

	fn constant_function_bound(
		&mut self,
		constant: &ConstantFunction,
		node: NodeId,
	) -> Result<Val<'ctx>, Error> {
		let current = self.file;
		let file = self.session.file_of(constant.module).ok_or_else(|| {
			self.unsupported(node.index(), "the source module of this constant function")
		})?;
		let remote = file != current && !self.other_files.contains_key(&file);
		if !remote {
			self.switch_file(file);
		}
		let saved_values = self
			.value_bindings
			.replace(self.value_bindings_for(&constant.bindings));
		let result = self.constant_function_here(constant, node, remote);
		self.value_bindings.replace(saved_values);
		if !remote {
			self.switch_file(current);
		}
		result
	}

	fn constant_function_here(
		&mut self,
		constant: &ConstantFunction,
		node: NodeId,
		remote: bool,
	) -> Result<Val<'ctx>, Error> {
		let mut values = Vec::with_capacity(constant.captures.len());
		for capture in &constant.captures {
			values.push(self.compiled_function_constant(capture, node)?);
		}
		let reprs: Vec<_> = values.iter().map(|value| value.repr).collect();
		let ty = self.concrete_type(constant.ty);
		let partial = self
			.analysis
			.partial_applications
			.get(&constant.node)
			.cloned();
		let mut fields: Vec<BasicTypeEnum<'ctx>> = vec![
			self.pointer_type().into(),
			self.pointer_type().into(),
			self.context.i64_type().into(),
		];
		fields.extend(reprs.iter().map(|&repr| self.llvm_type(repr)));
		let layout = self.context.struct_type(&fields, false);
		let entry = if remote {
			self.remote_constant_entry(constant, ty, node, false)?
		} else if self.is_generic_function_type(ty) {
			self.generic_table_of(
				ty,
				constant.node,
				if partial.is_some() {
					ProducerKind::Partial
				} else {
					ProducerKind::Lambda
				},
			)?
		} else if let Some(partial) = &partial {
			self.partial_entry(constant.node, partial, layout, &reprs)?
		} else {
			let key = if self.generic_functions.contains_key(&constant.node) {
				self.lambda_instance(constant.node, &[])?
			} else {
				constant.node
			};
			let function =
				self.functions.get(&key).cloned().ok_or_else(|| {
					self.unsupported(node.index(), "this constant function template")
				})?;
			let thunk = !function.lambda || function.params.iter().any(|(_, default)| *default);
			let target = if thunk {
				self.thunk_of(&function)?
			} else {
				function.value
			};
			self.async_value_entry(
				&function,
				target.as_global_value().as_pointer_value(),
				thunk,
			)?
		};
		let environment = if values.is_empty() {
			self.pointer_type().const_null()
		} else {
			let mut arguments: Vec<_> = constant
				.bindings
				.iter()
				.map(|&(param, ty)| {
					let info = self.types().param(param);
					(
						self.types().decl_hash(info.owner),
						info.index,
						self.types().stable_hash(self.concrete_type(ty)),
					)
				})
				.collect();
			arguments.sort_unstable();
			let key = format!(
				"constant.{:x}.{:x}.{arguments:x?}",
				self.types().decl_hash(constant.template),
				constant.capabilities.stable_hash(self.types())
			);
			let (drop, copy) = self.partial_environment_functions(&key, layout, &reprs)?;
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
			for (index, value) in values.into_iter().enumerate() {
				self.unregister(value);
				let field =
					self.builder
						.build_struct_gep(layout, environment, index as u32 + 3, "")?;
				self.builder
					.build_store(field, value.value.expect("a captured value"))?;
			}
			environment
		};
		let value = self.make_closure_of(entry, environment, ty)?;
		if !constant.defaults.iter().any(|&default| default) {
			return Ok(value);
		}
		let entry = if remote {
			self.remote_constant_entry(constant, ty, node, true)?
		} else if self.is_generic_function_type(ty) {
			self.generic_table_of(
				ty,
				constant.node,
				if partial.is_some() {
					ProducerKind::DefaultPartial
				} else {
					ProducerKind::DefaultLambda
				},
			)?
		} else if let Some(partial) = &partial {
			self.partial_entry_with_defaults(constant.node, partial, layout, &reprs, true)?
		} else {
			let key = if self.generic_functions.contains_key(&constant.node) {
				self.lambda_instance(constant.node, &[])?
			} else {
				constant.node
			};
			let function = self.functions[&key].clone();
			self.function_default_entry(&function)?
		};
		let value = self.fresh(value);
		let value = self.with_default_entry(value, entry)?;
		// The typed constant emitter registers its complete result once.
		self.unregister(value);
		Ok(value)
	}
	/// A template in another unit is emitted by that unit's ordinary
	/// instance wave. A non-generic closure uses the same protocol with one
	/// empty instance, so no source AST or captured-layout reconstruction is
	/// needed in its consumer.
	fn remote_constant_entry(
		&mut self,
		constant: &ConstantFunction,
		ty: fa_analyzer::ids::TypeId,
		node: NodeId,
		defaults: bool,
	) -> Result<PointerValue<'ctx>, Error> {
		let mut bindings = Vec::new();
		for &(param, bound) in &constant.bindings {
			if matches!(self.types().kind(bound), TypeKind::Param(same) if same == param) {
				continue;
			}
			let (_, owner, index) = constant
				.parameters
				.iter()
				.find(|entry| entry.0 == param)
				.ok_or_else(|| {
					self.unsupported(node.index(), "the source binder of this constant template")
				})?;
			bindings.push((
				owner.0,
				u32::from(*index),
				self.global_type(self.concrete_type(bound)),
			));
		}
		bindings.sort_unstable_by_key(|&(owner, index, _)| (owner, index));
		let mut methods = Vec::new();
		for (param, method) in &constant.capabilities.methods {
			let (_, owner, index) = constant
				.parameters
				.iter()
				.find(|entry| entry.0 == *param)
				.ok_or_else(|| {
					self.unsupported(node.index(), "the method binder of this constant template")
				})?;
			let mut merger = self.types().merger(Some(self.session.module_id(self.file)));
			methods.push((owner.0, u32::from(*index), method.merged(&mut merger)));
		}
		methods.sort_unstable_by_key(|(owner, index, method)| (*owner, *index, method.name));
		let file = self
			.session
			.file_of(constant.module)
			.expect("an evaluated template's file");
		let module = self.session.module(file).to_string();
		let mut hashes: Vec<_> = bindings
			.iter()
			.map(|&(owner, index, bound)| (owner, index, self.types().stable_hash(bound)))
			.collect();
		hashes.sort_unstable();
		let table = format!(
			"fa.gv.table.{module}.{}.constant.{hashes:x?}.{:x}{}",
			constant.node.index(),
			constant.capabilities.stable_hash(self.types()),
			if defaults { ".defaults" } else { "" }
		);
		let global = self.module.get_global(&table).unwrap_or_else(|| {
			let global = self.module.add_global(self.context.i8_type(), None, &table);
			global.set_linkage(Linkage::External);
			global.set_constant(true);
			global.set_visibility(inkwell::GlobalVisibility::Hidden);
			global
		});
		let producer = crate::split::GenericProducer {
			table,
			class: self.class_key(ty),
			module,
			node: constant.node.0,
			lambda: true,
			defaults,
			adapter: None,
			asynchronous: self.tasks.asynchronous
				&& matches!(self.types().kind(ty), TypeKind::Function(sig) if self.types().sig(sig).is_async),
			bindings,
			methods,
		};
		if !self.generic_needs_mut().producers.contains(&producer) {
			self.generic_needs_mut().producers.push(producer);
		}
		if self.is_generic_function_type(ty) {
			return Ok(global.as_pointer_value());
		}
		let index = self.instance_index(ty, Vec::new(), &[], &[], node)?;
		let slot = unsafe {
			self.builder.build_in_bounds_gep(
				self.pointer_type(),
				global.as_pointer_value(),
				&[index],
				"",
			)?
		};
		Ok(self
			.builder
			.build_load(self.pointer_type(), slot, "")?
			.into_pointer_value())
	}
}
