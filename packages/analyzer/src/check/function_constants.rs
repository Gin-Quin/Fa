//! Reifies evaluated factory results with typed captures and stable callable
//! templates. The VM decides the values; the provenance supplies their exact
//! written types and method selections without a second body analysis.

use std::sync::Arc;

use fa_interpreter::Value;
use fa_interpreter::ir::FnKey;
use fa_interpreter::value::Closure;
use rustc_hash::FxHashSet;

use crate::check::Walker;
use crate::constants::{ConstantFunction, ConstantKind, ConstantValue};
use crate::ids::{FileId, NodeId, TypeId};
use crate::provenance::{OriginKind, Projection, ValueOrigin};
use crate::types::TypeKind;

impl Walker<'_> {
	pub(super) fn finish_function_constants(&mut self) {
		let mut values: Vec<_> = self
			.a
			.known_functions
			.iter()
			.map(|(&node, value)| (node, value.clone()))
			.collect();
		values.sort_unstable_by_key(|(node, _)| *node);
		for (node, value) in values {
			let Some(origin) = self.a.value_origins.get(&node).cloned() else {
				continue;
			};
			if let Some(value) = self.reify_function_value(&value, self.a.type_of(node), origin) {
				self.a.function_constants.insert(node, value);
			} else {
				let diagnostic = self.diagnostic(
					&crate::diagnostics::codes::analyzer::AN001,
					node,
					"the evaluated function's exact template or captured type could not be recovered",
				);
				self.report(diagnostic);
			}
		}
	}

	fn reify_function_value(
		&mut self,
		value: &Value,
		ty: TypeId,
		origin: Arc<ValueOrigin>,
	) -> Option<ConstantValue> {
		if !value.has_function() {
			return Some(ConstantValue {
				ty,
				kind: ConstantKind::Data(value.clone()),
			});
		}
		let kind = match value {
			Value::Function(function) => {
				ConstantKind::Function(Arc::new(self.reify_function(function, origin, ty)?))
			}
			Value::Array(values) | Value::Tuple(values) | Value::Set(values) => {
				let mut result = Vec::with_capacity(values.len());
				for (index, value) in values.iter().enumerate() {
					let (step, item) = match self.s.types.kind(self.s.types.unfold(ty)) {
						TypeKind::Tuple(items) => (
							Projection::Index(index as u32),
							*self.s.types.list_items(items).get(index)?,
						),
						TypeKind::Collection(_, item, _) | TypeKind::CArray(item, _) => {
							(Projection::Item, item)
						}
						_ => return None,
					};
					let origin = self.constant_projection(Arc::clone(&origin), step, item);
					result.push(self.reify_function_value(value, item, origin)?);
				}
				match value {
					Value::Array(_) => ConstantKind::Array(result),
					Value::Tuple(_) => ConstantKind::Tuple(result),
					_ => ConstantKind::Set(result),
				}
			}
			Value::Object(object) => {
				let mut result = Vec::with_capacity(object.fields.len());
				for (name, value) in &object.fields {
					let name = self.s.types.syms.intern(name);
					let field = self
						.s
						.types
						.fields_of(ty)
						.iter()
						.find(|field| field.name == name)?
						.ty;
					let origin = self.constant_projection(
						Arc::clone(&origin),
						Projection::Field(name),
						field,
					);
					result.push((name, self.reify_function_value(value, field, origin)?));
				}
				ConstantKind::Object(result)
			}
			Value::Map(entries) => {
				let TypeKind::Collection(_, item, key) = self.s.types.kind(self.s.types.unfold(ty))
				else {
					return None;
				};
				let mut result = Vec::with_capacity(entries.len());
				for (k, v) in entries.iter() {
					let key_origin =
						self.constant_projection(Arc::clone(&origin), Projection::Item, key);
					let item_origin =
						self.constant_projection(Arc::clone(&origin), Projection::Item, item);
					result.push((
						self.reify_function_value(k, key, key_origin)?,
						self.reify_function_value(v, item, item_origin)?,
					));
				}
				ConstantKind::Map(result)
			}
			Value::Member(member) => {
				let payload = match self.s.types.kind(self.s.types.unfold(ty)) {
					TypeKind::Member(nominal, index) => self.s.types.payload(nominal, index),
					TypeKind::NamedUnion(nominal, mask) => {
						let index = (0..self
							.s
							.types
							.sym_list(self.s.types.nominal(nominal).members)
							.len() as u16)
							.find(|&index| {
								self.s.types.mask_has(mask, index)
									&& crate::comptime::lower::member_name(
										&self.s.types,
										nominal,
										index,
									) == *member.name
							})?;
						self.s.types.payload(nominal, index)
					}
					_ => return None,
				};
				let data = self.reify_function_value(member.data.as_ref()?, payload, origin)?;
				ConstantKind::Member {
					name: Arc::clone(&member.name),
					kind: member.kind,
					data: Box::new(data),
				}
			}
			_ => return None,
		};
		Some(ConstantValue { ty, kind })
	}

	fn constant_projection(
		&self,
		value: Arc<ValueOrigin>,
		step: Projection,
		ty: TypeId,
	) -> Arc<ValueOrigin> {
		Arc::new(ValueOrigin {
			ty,
			kind: OriginKind::Project { value, step },
		})
	}

	fn matches_function_instance(
		&self,
		origin: &ValueOrigin,
		context: &crate::comptime::instances::Context,
	) -> bool {
		let mut merger = self.s.types.merger(Some(self.s.module_id(self.file)));
		let origin = origin.merged(&mut merger);
		let OriginKind::Function {
			bindings,
			capabilities,
			..
		} = &origin.kind
		else {
			return false;
		};
		for (param, ty) in bindings {
			if let Some((_, actual)) = context.bindings.iter().find(|entry| {
				crate::comptime::instances::parameter_key(&self.s.types, entry.0)
					== crate::comptime::instances::parameter_key(&self.s.types, *param)
			}) && actual != ty
			{
				return false;
			}
		}
		for (param, origin) in &capabilities.origins {
			if let Some((_, actual)) = context.capabilities.origins.iter().find(|entry| {
				crate::comptime::instances::parameter_key(&self.s.types, entry.0)
					== crate::comptime::instances::parameter_key(&self.s.types, *param)
			}) && actual != origin
			{
				return false;
			}
		}
		for (param, method) in &capabilities.methods {
			if let Some((_, actual)) = context.capabilities.methods.iter().find(|entry| {
				crate::comptime::instances::parameter_key(&self.s.types, entry.0)
					== crate::comptime::instances::parameter_key(&self.s.types, *param)
					&& entry.1.name == method.name
			}) {
				let mut method = method.clone();
				method.parameter = actual.parameter;
				if *actual != method {
					return false;
				}
			}
		}
		true
	}

	fn reify_function(
		&mut self,
		value: &Closure,
		origin: Arc<ValueOrigin>,
		expected: TypeId,
	) -> Option<ConstantFunction> {
		let key = value
			.partials
			.last()
			.map_or(value.function, |partial| partial.site);
		let instance = self
			.s
			.comptime
			.instances
			.lock()
			.expect("the compile-time instance registry")
			.get(key)
			.cloned();
		let FnKey(file, node) = instance.as_ref().map_or(key, |instance| instance.source);
		let (file, node) = (FileId(file), NodeId(node));
		let module = self.s.module_id(file);
		let template = if file == self.file {
			self.a.callable_keys.get(&node).copied()
		} else {
			self.s.analysis(file).callable_keys.get(&node).copied()
		}?;
		let candidates = self.resolve_value_origin(origin, &mut FxHashSet::default());
		let mut matched = candidates.iter().filter(|origin| {
			matches!(origin.kind, OriginKind::Function { declaration, .. } if declaration == template)
				&& instance.as_ref().is_none_or(|instance| {
					self.matches_function_instance(origin, &instance.context)
				})
		});
		let selected = Arc::clone(matched.next()?);
		let OriginKind::Function {
			bindings,
			capabilities,
			captures: sources,
			defaults,
			..
		} = &selected.kind
		else {
			return None;
		};
		// A data-dependent generic branch can select different methods for
		// the same source template. Never choose an arbitrary equal layout.
		if matched.any(|candidate| matches!(&candidate.kind, OriginKind::Function { bindings: other, capabilities: caps, .. } if other != bindings || caps != capabilities)) { return None; }
		let captures = if let Some(partial) = value.partials.last() {
			if sources.len() != partial.arguments.len() + 1 {
				return None;
			}
			let mut target = value.clone();
			target.partials.pop();
			let mut values = vec![self.reify_function_value(
				&Value::Function(Arc::new(target)),
				sources[0].ty,
				Arc::clone(&sources[0]),
			)?];
			for (source, value) in sources[1..].iter().zip(&partial.arguments) {
				values.push(self.reify_function_value(value, source.ty, Arc::clone(source))?);
			}
			values
		} else {
			if sources.len() != value.captured.len() {
				return None;
			}
			let mut values = Vec::with_capacity(sources.len());
			for (source, value) in sources.iter().zip(&value.captured) {
				values.push(self.reify_function_value(value, source.ty, Arc::clone(source))?);
			}
			values
		};
		let analysis = if file == self.file {
			&self.a
		} else {
			self.s.analysis(file)
		};
		let mut parameters = Vec::new();
		for (&owner, info) in &analysis.functions {
			if info.declaration != Some(owner) && info.lambda != owner {
				continue;
			}
			let owner = info.declaration.unwrap_or(info.lambda);
			let type_generics = analysis
				.type_decls
				.values()
				.find(|decl| {
					decl.functions
						.iter()
						.any(|&symbol| analysis.symbols.symbol(symbol).decl_node == owner)
				})
				.map_or(&[][..], |decl| decl.generics.as_slice());
			for (index, &parameter) in type_generics.iter().chain(&info.generics).enumerate() {
				parameters.push((parameter, owner, index as u16));
			}
		}
		// Parallel consumers may hold the source interface without its full
		// body analysis. Its exported generic contracts retain the exact
		// owner/order, including implicit and derived binders.
		if let Some(interface) = self.s.interface(module) {
			for (&owner, function) in &interface.generic_functions {
				for (index, &parameter) in self
					.s
					.types
					.list_items(function.generics)
					.iter()
					.enumerate()
				{
					if let TypeKind::Param(parameter) = self.s.types.kind(parameter) {
						parameters.push((parameter, owner, index as u16));
					}
				}
			}
		}
		parameters.sort_unstable();
		parameters.dedup_by_key(|entry| entry.0);
		let defaults = defaults.clone();
		// The origin can acquire an alpha conversion while it passes through
		// a forall parameter. Apply it to the signature as well as to the
		// template bindings before restoring the quantified binders below.
		let substitution = crate::types::Bindings {
			params: bindings.iter().map(|entry| entry.0).collect(),
			types: bindings.iter().map(|entry| entry.1).collect(),
			..Default::default()
		};
		let mut ty = self.s.types.substitute(selected.ty, &substitution);
		let mut bindings = bindings.clone();
		let mut capabilities = Arc::clone(capabilities);
		if let TypeKind::Function(expected_sig) = self.s.types.kind(expected) {
			let expected_sig = self.s.types.sig(expected_sig);
			if !self.s.types.list_items(expected_sig.generics).is_empty()
				&& let TypeKind::Function(actual_sig) = self.s.types.kind(ty)
			{
				let actual_sig = self.s.types.sig(actual_sig);
				if self.s.types.list_items(actual_sig.generics).is_empty() {
					// A forall conversion alpha-renames its own binders; it does
					// not instantiate them. Preserve the actual effects/error ABI.
					ty = self.s.types.function(crate::types::FnSig {
						generics: expected_sig.generics,
						..actual_sig
					});
				}
				let own = self.own_generics(expected);
				let renamed: Vec<_> = bindings.iter().filter_map(|&(param, ty)| {
					matches!(self.s.types.kind(ty), TypeKind::Param(target) if own.contains(&target)).then_some(param)
				}).collect();
				bindings.retain(|(param, _)| !renamed.contains(param));
				let caps = Arc::make_mut(&mut capabilities);
				caps.origins.retain(|(param, _)| !renamed.contains(param));
				caps.methods.retain(|(param, _)| !renamed.contains(param));
			}
		}
		Some(ConstantFunction {
			parameters,
			defaults,
			ty,
			template,
			module,
			node,
			bindings,
			capabilities,
			captures,
		})
	}
}

#[cfg(test)]
mod tests {
	use super::super::test_support::{analyzed, codes};
	use crate::constants::ConstantKind;

	#[test]
	fn named_factories_are_evaluated_and_keep_typed_captures() {
		let (session, file) = analyzed(
			"function makeAdd = (value: Integer) => (other: Integer) => value + other\nfunction add3 = makeAdd(3)\nfunction add7 = makeAdd(add3(4))\nconsole.log(add3(1), add7(2))\n",
		);
		let analysis = session.analysis(file);
		assert_eq!(analysis.function_constants.len(), 2);
		let mut values = analysis
			.function_constants
			.values()
			.map(|value| {
				let ConstantKind::Function(function) = &value.kind else {
					panic!("a compiled closure");
				};
				assert_eq!(function.captures.len(), 1);
				let ConstantKind::Data(fa_interpreter::Value::Integer(value)) =
					function.captures[0].kind
				else {
					panic!("an integer capture");
				};
				value.as_i128()
			})
			.collect::<Vec<_>>();
		values.sort_unstable();
		assert_eq!(values, [3, 7]);
	}

	#[test]
	fn runtime_factory_arguments_need_let_and_impure_factories_are_not_folded() {
		let prefix = "function makeAdd = (value: Integer) => (other: Integer) => value + other\n";
		assert_eq!(
			codes(&format!(
				"{prefix}function invalid = (value: Integer) => do {{ function named = makeAdd(value)\nnamed }}\n"
			)),
			["FN110"]
		);
		analyzed(&format!(
			"{prefix}function valid = (value: Integer) => do {{ let runtime = makeAdd(value)\nruntime }}\n"
		));
		assert_eq!(
			codes(
				"function noisy = (value: Integer) => do { console.log(value)\n(other: Integer) => value + other }\nfunction invalid = noisy(3)\n"
			),
			["FN110"]
		);
		assert_eq!(
			codes(
				"function add = (first: Integer, second: Integer) => first + second\nfunction invalid = (value: Integer) => do { function named = add with (first = value)\nnamed }\n"
			),
			["FN110"]
		);
	}

	#[test]
	fn value_binders_and_generic_factory_branches_keep_the_executed_instance() {
		let source = "type Person = { name: String, function describe = (self): String => \"Hello {self.name}\" }\ntype Greeting = { name: String, function describe = (self): String => \"Hi {self.name}\" }\nfunction wrap = <Item: { describe: (self) => String }>(value: Item) => () => value.describe()\nfunction choose = (left: Boolean) => if left { wrap(Person { name = \"Ada\" }) } else { wrap(Greeting { name = \"Bea\" }) }\nfunction selected = choose(false)\nfunction makeOffset = <offset: Integer>() => (value: Integer) => value + offset\nfunction add5 = makeOffset<5>()\n";
		let (session, file) = crate::analyze_source(source);
		assert!(
			session.diagnostics(file).is_empty(),
			"{}",
			session.render_diagnostics(file, "test.fa")
		);
		assert_eq!(session.analysis(file).function_constants.len(), 2);
	}

	#[test]
	fn generic_function_values_and_methods_run_inside_factories() {
		analyzed(
			"type Greeting = { name: String, function describe = (self): String => \"Hi {self.name}\" }\nfunction build = <Item: { describe: (self) => String }>(value: Item) => do { let text = value.describe()\n() => text }\nfunction selected = build(Greeting { name = \"Bea\" })\ntype Identity = <Item>(value: Item) => Item\nfunction identity = <Item>(value: Item) => value\nfunction evaluate = (transform: Identity) => do { let value = transform<Integer>(13)\n(other: Integer) => value + other }\nfunction answer = evaluate(identity)\nfunction returnIdentity = (transform: Identity) => transform\nfunction returned = returnIdentity(identity)\n",
		);
	}

	#[test]
	fn pure_factory_can_create_an_impure_function_without_executing_it() {
		analyzed(
			"function make = () => () => console.log(\"runtime\")\nfunction logger = make()\n",
		);
		assert_eq!(
			codes(
				"function execute = (work: () => Void) => do { work()\n() => 1 }\nfunction invalid = execute(() => console.log(\"forbidden\"))\n"
			),
			["FN110"]
		);
	}

	#[test]
	fn factory_return_preserves_function_quantification() {
		let (session, file) = analyzed(
			"type Identity = <Item>(value: Item) => Item\nfunction identity = <Item>(value: Item) => value\nfunction returnIdentity = (transform: Identity) => transform\nfunction returned = returnIdentity(identity)\nlet text = returned(\"hello\")\nlet number = returned(29)\n",
		);
		let constant = session
			.analysis(file)
			.function_constants
			.values()
			.next()
			.expect("the factory result");
		let ConstantKind::Function(function) = &constant.kind else {
			panic!("a function");
		};
		for ty in [constant.ty, function.ty] {
			let crate::types::TypeKind::Function(sig) = session.types().kind(ty) else {
				panic!("a signature");
			};
			let sig = session.types().sig(sig);
			let generics = session.types().list_items(sig.generics);
			assert_eq!(generics.len(), 1);
			assert_eq!(session.types().params(sig.params)[0].ty, generics[0]);
			assert_eq!(sig.ret, generics[0]);
		}
	}

	#[test]
	fn explicit_function_application_is_a_compile_time_function_value() {
		analyzed(
			"function identity = <Item>(value: Item) => value\nfunction applied = identity<Integer>\nlet number = applied(17)\n",
		);
	}

	#[test]
	fn factory_function_defaults_stay_in_the_portable_value_summary() {
		let (session, file) = analyzed(
			"function make = (bias: Integer) => (value: Integer = bias) => value + bias\nfunction plus = make(5)\nconsole.log(plus(), plus(value = 8))\n",
		);
		let analysis = session.analysis(file);
		let constant = analysis
			.function_constants
			.values()
			.next()
			.expect("a factory result");
		let ConstantKind::Function(function) = &constant.kind else {
			panic!("a function");
		};
		assert_eq!(function.defaults, [true]);
		assert!(analysis.callables.values().any(|summary| {
			matches!(&summary.result.kind, crate::provenance::OriginKind::Function { defaults, .. } if defaults == &[true])
		}));
	}

	#[test]
	fn factory_errors_are_compile_time_errors_and_success_needs_no_try() {
		let prefix = "type Failure = Errors { Bad }\ntype Operation = (value: Integer) => Integer\nfunction maybe = (fail: Boolean): Operation throws Failure.Bad => if fail { throw Failure.Bad } else { (value: Integer) => value }\n";
		analyzed(&format!(
			"{prefix}function good = maybe(false)\nconsole.log(good(1))\n"
		));
		assert_eq!(
			codes(&format!("{prefix}function bad = maybe(true)\n")),
			["META005"]
		);
	}
}
