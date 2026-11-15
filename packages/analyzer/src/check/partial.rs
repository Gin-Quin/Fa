//! Named partial application. Construction captures the function and bound
//! values once; the residual signature has no effect on captured parameters.

use crate::analysis::PartialApplication;
use crate::check::Walker;
use crate::diagnostics::codes::types::{FN072, FN100, FN101, FN102, FN103, FN104};
use crate::diagnostics::codes::values::E_TYPE_1;
use crate::ids::{NodeId, TypeId};
use crate::syntax::{NodeList, View};
use crate::types::{Bindings, Candidates, FnSig, PathRoot, PathStep, TypeKind};

impl Walker<'_> {
	pub(super) fn check_partial(
		&mut self,
		node: NodeId,
		function: NodeId,
		arguments: NodeList<'_>,
	) -> TypeId {
		let mut original_type = self.check(function, None);
		self.escape(function, node);
		let Some(signature) = self.signature_of(original_type) else {
			for argument in arguments.iter() {
				let value = match self.view(argument) {
					View::Assignment {
						value: Some(value), ..
					} => value,
					_ => argument,
				};
				self.check(value, Some(TypeId::UNKNOWN));
			}
			return self.memo(node, |walker| {
				if !walker.s.types.is_recovering(original_type) {
					let message = format!(
						"`with` needs a function, not `{}`",
						walker.display(original_type)
					);
					let diagnostic = walker.diagnostic(&E_TYPE_1, function, message);
					walker.report(diagnostic);
				}
				TypeId::UNKNOWN
			});
		};
		let params = self.s.types.params(signature.params).to_vec();
		let names = self
			.visible_parameter_names(function)
			.unwrap_or_else(|| vec![None; params.len()]);
		let mut defaults = self.visible_parameter_defaults(function);
		defaults.resize(params.len(), false);
		let mut bound = Vec::new();
		for argument in arguments.iter() {
			let View::Assignment {
				target,
				annotation: None,
				value: Some(value),
			} = self.view(argument)
			else {
				self.check(argument, None);
				let diagnostic = self.diagnostic(
					&FN100,
					argument,
					"`with` binds parameters by name: write `name = value`",
				);
				self.report(diagnostic);
				continue;
			};
			let View::Name(name) = self.view(target) else {
				self.check(value, None);
				let diagnostic = self.diagnostic(
					&FN100,
					target,
					"a bound parameter needs its name: write `name = value`",
				);
				self.report(diagnostic);
				continue;
			};
			let text = self.tree.name(name).to_string();
			let name = self.s.types.syms.intern(&text);
			let Some(index) = names.iter().position(|known| *known == Some(name)) else {
				self.check(value, None);
				let message = format!("the visible signature has no parameter named `{text}`");
				let diagnostic = self.diagnostic(&FN101, target, message);
				self.report(diagnostic);
				continue;
			};
			if bound.iter().any(|(known, _)| *known == index as u16) {
				self.check(value, None);
				let diagnostic =
					self.diagnostic(&FN102, target, format!("`{text}` is already bound"));
				self.report(diagnostic);
				continue;
			}
			if params[index].variadic {
				let diagnostic = self.diagnostic(
					&FN104,
					target,
					format!("the variadic parameter `{text}` can't be bound"),
				);
				self.report(diagnostic);
			}
			let effects = self.s.types.effects(signature.effects);
			let mutated = self
				.s
				.types
				.effects(signature.effects)
				.0
				.iter()
				.chain(effects.1)
				.any(|path| match self.s.types.path_data(*path).0.root {
					PathRoot::Param(own) => own as usize == index,
					PathRoot::SelfParam => index == 0,
					_ => false,
				});
			if mutated {
				let diagnostic = self.diagnostic(
					&FN103,
					target,
					format!(
						"`{text}` is mutated by the function: pass it when calling the residual function"
					),
				);
				self.report(diagnostic);
			}
			bound.push((index as u16, value));
		}
		let mut generics = self.callee_generics(function, crate::analysis::Callee::Value);
		for param in self
			.own_generics(original_type)
			.into_iter()
			.chain(self.free_params(original_type))
		{
			if !generics.contains(&param) {
				generics.push(param);
			}
		}
		let mut bindings = Bindings::new(&generics);
		if !generics.is_empty() {
			let own: Vec<_> = generics
				.iter()
				.map(|&param| self.s.types.intern(TypeKind::Param(param)))
				.collect();
			original_type = self.generic_function_type(original_type, &own);
			self.set_type(function, original_type);
		}
		let mut candidates = Candidates::default();
		let mut actuals = Vec::new();
		for &(index, value) in &bound {
			let expected = params[index as usize].ty;
			let actual = if self.has_free(expected, &generics) {
				self.check(value, None)
			} else {
				self.check(value, Some(expected))
			};
			self.s
				.types
				.unify(expected, actual, &bindings, &mut candidates);
			actuals.push(actual);
		}
		let mut problems = Vec::new();
		self.resolve_candidates(&mut bindings, &candidates, &mut problems);
		for &(index, value) in &bound {
			self.infer_written_origin(params[index as usize].ty, value, &mut bindings);
		}
		self.check_constraints(&mut bindings, &mut problems, true);
		for (code, message) in problems {
			let diagnostic = self.diagnostic(code, node, message);
			self.report(diagnostic);
		}
		let mut residual_generics = Vec::new();
		for (index, &param) in generics.iter().enumerate() {
			if bindings.types[index].is_unset() {
				let ty = self.s.types.intern(TypeKind::Param(param));
				bindings.types[index] = ty;
				residual_generics.push(ty);
			}
		}
		let generic_arguments = self.s.types.list(&bindings.types);
		if !bindings.params.is_empty() {
			self.record_instantiation(node, &bindings);
			// These arguments specialize the captured target, not the residual
			// function value. Keep that plan below without marking the newly
			// constructed forall value as an application of its own binders.
			self.a.instantiations.remove(&node);
		}
		for (&(index, value), actual) in bound.iter().zip(actuals) {
			let expected = self
				.s
				.types
				.substitute(params[index as usize].ty, &bindings);
			self.sink(value, actual, expected);
			self.escape(value, node);
			// A partial is an escaping, repeatable function. Its captured
			// arguments cannot acquire unique ownership through substitution,
			// even when an ordinary last use could otherwise transfer them.
			if self
				.s
				.types
				.flags(actual)
				.contains(crate::types::TypeFlags::HAS_PARAM)
			{
				self.defer_copy_condition(
					value,
					crate::analysis::CopyRequirement {
						subject: actual,
						explicit: true,
						task_transfer: false,
						task_only: false,
						resource_gate: None,
					},
				);
			}
			if self.s.types.is_move_only(actual) {
				let diagnostic = self.diagnostic(
					&FN072,
					value,
					format!(
						"a `{}` can't be copied into a function's bound arguments",
						self.display(actual)
					),
				);
				self.report(diagnostic);
			} else {
				self.note_copy_site(value);
			}
		}
		let remaining: Vec<_> = (0..params.len() as u16)
			.filter(|index| !bound.iter().any(|(known, _)| known == index))
			.collect();
		let parameters: Vec<_> = remaining
			.iter()
			.map(|&index| crate::types::ParamSig {
				ty: self
					.s
					.types
					.substitute(params[index as usize].ty, &bindings),
				..params[index as usize]
			})
			.collect();
		let effects = self.partial_effects(signature.effects, &remaining, &bound);
		let params = self.s.types.param_list(&parameters);
		let ret = self.s.types.substitute(signature.ret, &bindings);
		let throws = self.s.types.substitute(signature.throws, &bindings);
		let requirements: Vec<_> = self
			.s
			.types
			.requirements(signature.requires)
			.to_vec()
			.into_iter()
			.map(|requirement| crate::types::Requirement {
				ty: self.s.types.substitute(requirement.ty, &bindings),
				..requirement
			})
			.collect();
		let requires = self.s.types.req_list(&requirements);
		let residual_list = self.s.types.list(&residual_generics);
		let mut stored = 0;
		for (position, &index) in remaining.iter().enumerate().take(32) {
			if index < 32 && signature.stored & (1 << index) != 0 {
				stored |= 1 << position;
			}
		}
		let ty = self.s.types.function(FnSig {
			params,
			ret,
			throws,
			requires,
			effects,
			generics: residual_list,
			stored,
			..signature
		});
		let mut bound_names = self
			.named_signature(function)
			.and_then(|names| self.resolved_signature_names(names))
			.and_then(|names| match &*names {
				crate::signatures::SignatureNames::Function { bound, .. } => bound.clone(),
				_ => None,
			})
			.unwrap_or_default();
		for &(index, _) in &bound {
			if let Some(name) = names[index as usize]
				&& !bound_names.contains(&name)
			{
				bound_names.push(name);
			}
		}
		self.a.partial_applications.insert(
			node,
			PartialApplication {
				function,
				original_type,
				defaults,
				bound,
				bound_names,
				names: remaining
					.iter()
					.map(|&index| names.get(index as usize).copied().flatten())
					.collect(),
				remaining,
				generic_arguments,
				generic_parameters: generics,
			},
		);
		self.memo(node, |_| ty)
	}

	fn partial_effects(
		&mut self,
		effects: crate::ids::EffectSetId,
		remaining: &[u16],
		bound: &[(u16, NodeId)],
	) -> crate::ids::EffectSetId {
		let effects = self.s.types.effects(effects);
		let lists = [effects.0.to_vec(), effects.1.to_vec()];
		let mut remapped = [Vec::new(), Vec::new()];
		for (source, target) in lists.into_iter().zip(&mut remapped) {
			for path in source {
				let (path, steps) = self.s.types.path_data(path);
				let mut steps = steps.to_vec();
				let root = match path.root {
					PathRoot::Param(index) => {
						match remaining.iter().position(|known| *known == index) {
							Some(index) => PathRoot::Param(index as u16),
							None => continue,
						}
					}
					PathRoot::SelfParam if bound.iter().any(|(index, _)| *index == 0) => continue,
					root => root,
				};
				for step in &mut steps {
					*step = match *step {
						PathStep::IndexParam(index) | PathStep::IndexFrom(index) => {
							match remaining.iter().position(|known| *known == index) {
								Some(index) => {
									if matches!(step, PathStep::IndexFrom(_)) {
										PathStep::IndexFrom(index as u16)
									} else {
										PathStep::IndexParam(index as u16)
									}
								}
								None => PathStep::All,
							}
						}
						step => step,
					};
				}
				target.push(self.s.types.path(root, &steps));
			}
		}
		self.s.types.effect_set(&remapped[0], &remapped[1])
	}
}

#[cfg(test)]
mod tests {
	use super::super::test_support::{analyzed, codes};

	#[test]
	fn binds_named_arguments_from_the_visible_signature() {
		for source in [
			"function xyz = (x: Integer, y: Integer, z: Integer) => x + y + z\nlet xz = xyz with (y = 0)\nconsole.log(xz(1, 2))\n",
			"function bind = (f: (a: Integer, b: Integer) => Integer) => f with (b = 1)\nfunction add = (x: Integer, y: Integer) => x + y\nlet result = bind(add)\nconsole.log(result(2))\n",
			"type Operation = (first: Integer, second: Integer) => Integer\nfunction bind = (f: Operation) => f with (second = 1)\n",
			"type Operations = { sum: (first: Integer, second: Integer) => Integer }\nfunction bind = (ops: Operations) => ops.sum with (second = 1)\n",
		] {
			analyzed(source);
		}
	}

	#[test]
	fn binding_one_generic_leaves_the_others_generic() {
		analyzed(
			"function choose = <Left, Right>(left: Left, right: Right): Right => right\nlet residual: <Right>(right: Right) => Right = choose with (left = 1)\nconsole.log(residual(\"ok\"))\n",
		);
	}

	#[test]
	fn bound_callbacks_declare_their_errors_and_the_residual_keeps_only_escaping_errors() {
		let prefix = "type Failure = Errors { Bad }\nfunction bad = (): Integer throws Failure.Bad => throw Failure.Bad\n";
		analyzed(&format!(
			"{prefix}function absorb = (callback: () => Integer throws Failure.Bad): Integer => callback() catch _error => 0\nlet handled = absorb with (callback = bad)\nconsole.log(handled())\nfunction relay = (callback: () => Integer throws Failure.Bad): Integer throws Failure.Bad => try callback()\nlet propagated = relay with (callback = bad)\nconsole.log(propagated() catch _error => -1)\n"
		));
		assert_eq!(
			codes(&format!(
				"{prefix}function invoke = (callback: () => Integer): Integer => callback()\nlet rejected = invoke with (callback = bad)\n"
			)),
			["E-TYPE-1"]
		);
	}

	#[test]
	fn binding_a_type_parameter_substitutes_requirement_keys() {
		analyzed(
			"type Slot<Value> = { value: Value }\nfunction combine = <Value>(first: Value, second: Value): Value requires slot: Slot<Value> => slot.value\nfunction fixed = combine with (first = 1)\nprovide Slot<Integer> = { value = 4 }\nconsole.log(fixed(2))\n",
		);
	}

	#[test]
	fn residual_declarations_keep_names_and_previously_bound_parameters() {
		let prefix = "function xyz = (x: Integer, y: Integer, z: Integer) => x + y + z\nfunction xz = xyz with (y = 0)\nfunction z = xz with (x = 1)\n";
		analyzed(&format!(
			"{prefix}console.log(xz(z = 2, x = 1), z(z = 4))\n"
		));
		assert_eq!(
			codes(&format!("{prefix}let invalid = z(3, y = 4)\n")),
			["FN102"]
		);
		assert_eq!(
			codes(&format!("{prefix}let invalid = z(3, x = 4)\n")),
			["FN102"]
		);
	}

	#[test]
	fn residual_declarations_retain_defaults_but_plain_values_erase_them() {
		let prefix = "function add = (first: Integer, second: Integer = first + 1, last: Integer? = 9) => first + second + (last ?? 0)\nfunction fixed = add with (first = 2)\nfunction chained = fixed with (last = none)\n";
		let (session, file) = analyzed(&format!(
			"{prefix}console.log(fixed(), fixed(last = none), chained())\n"
		));
		let defaults = &session.analysis(file).function_defaults;
		assert_eq!(defaults.len(), 3);
		assert!(defaults.values().any(|defaults| defaults == &[true, true]));
		assert!(defaults.values().any(|defaults| defaults == &[true]));
		let (session, file) = analyzed(
			"function first = <Item>(value: Item, times: Integer = 2) => (value, times)\nfunction fixed = first with (times = 4)\nconsole.log(first<Integer>(3), fixed<Integer>(7))\n",
		);
		assert_eq!(session.analysis(file).applied_defaults.len(), 1);
		assert_eq!(
			codes(&format!(
				"{prefix}let plain = fixed\nconsole.log(plain())\n"
			)),
			["FN024"]
		);
		assert_eq!(
			codes(
				"function add = (first: Integer, second: Integer = 1) => first + second\nlet value = add\nfunction fixed = value with (first = 2)\nconsole.log(fixed())\n"
			),
			["FN024"]
		);
	}

	#[test]
	fn generic_bound_arguments_cannot_capture_unique_values() {
		let prefix = "function ignore = <Value>(_value: Value, number: Integer): Integer => number\nfunction bind = <Value>(value: Value) => ignore with (_value = value)\n";
		assert_eq!(
			codes(&format!(
				"{prefix}function invalid = () => {{ let task = start do {{ 1 }}\nlet residual = bind(task)\nconsole.log(residual(2)) }}\n"
			)),
			["MEM003"]
		);
		analyzed(&format!(
			"{prefix}let scalar = bind(1)\nconsole.log(scalar(2))\n"
		));
	}

	#[test]
	fn rejects_invalid_bindings() {
		let prefix = "function add = (first: Integer, second: Integer) => first + second\n";
		assert_eq!(
			codes(&format!("{prefix}let bad = add with (1)\n")),
			["FN100"]
		);
		assert_eq!(
			codes(&format!("{prefix}let bad = add with (missing = 1)\n")),
			["FN101"]
		);
		assert_eq!(
			codes(&format!(
				"{prefix}let bad = add with (second = 1, second = 2)\n"
			)),
			["FN102"]
		);
		assert_eq!(
			codes(
				"function update = (item: { value: Integer }, amount: Integer) mutates item.value => { item.value = amount }\nlet bad = update with (item = { value = 1 })\n"
			),
			["FN103"]
		);
		assert_eq!(
			codes(
				"function count = (...values: Integer) => values.length\nlet bad = count with (values = [1])\n"
			),
			["FN104"]
		);
		assert_eq!(
			codes("function bind = (f: (Integer, Integer) => Integer) => f with (first = 1)\n"),
			["FN101"]
		);
	}

	#[test]
	fn parameter_spelling_is_not_structural_type_identity() {
		let (session, file) = analyzed(
			"type First = (left: Integer, right: Integer) => Integer\ntype Second = (a: Integer, b: Integer) => Integer\nfunction first = (f: First) => f with (right = 1)\nfunction second = (f: Second) => f with (b = 1)\n",
		);
		let symbols = session.analysis(file).symbols.symbols();
		let first = symbols
			.iter()
			.find(|symbol| session.types().syms.resolve(symbol.name) == "First")
			.unwrap();
		let second = symbols
			.iter()
			.find(|symbol| session.types().syms.resolve(symbol.name) == "Second")
			.unwrap();
		assert_eq!(first.ty, second.ty);
	}

	#[test]
	fn names_of_imported_aliases_and_fields_survive_interfaces() {
		let files = vec![
			("library/types.fa".to_string(), "export namespace = { type Operation = (first: Integer, second: Integer) => Integer\ntype Operations = { next: Operations?, sum: Operation } }\n".to_string()),
			("application/main.fa".to_string(), "function bind = (f: library.types.Operation) => f with (second = 1)\nfunction bindField = (ops: library.types.Operations) => ops.sum with (second = 1)\nexport function = () => {}\n".to_string()),
		];
		let mut session = crate::Session::new();
		session.set_resolver(Box::new(crate::CodebaseResolver::from_texts(
			&files,
			Some("application/main.fa"),
		)));
		let program = session.analyze_package(Some("application/main"));
		let errors: Vec<_> = program
			.files
			.iter()
			.flat_map(|&file| session.diagnostics(file))
			.filter(|diagnostic| diagnostic.severity == crate::Severity::Error)
			.collect();
		assert!(errors.is_empty(), "{errors:?}");
	}
}
