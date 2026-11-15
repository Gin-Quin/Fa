//! `with` captures its target and bound data once. The residual function
//! forwards its own generic capabilities and borrows the captured values.

use fa_analyzer::ids::{NodeId, ParamId};
use fa_analyzer::types::TypeKind;

use crate::generate::Generator;
use crate::js::{self, Expr, Prec};

impl Generator<'_> {
	pub(crate) fn partial_value(&mut self, node: NodeId) -> Expr {
		let partial = self.analysis.partial_applications[&node].clone();
		let TypeKind::Function(signature) = self.types.kind(self.type_of(node)) else {
			return self.unsupported(node, "this partial function");
		};
		let signature = self.types.sig(signature);
		let requirements = self.types.requirements(signature.requires).len();
		let residual_generics: Vec<ParamId> = self
			.types
			.list_items(signature.generics)
			.iter()
			.filter_map(|&ty| match self.types.kind(ty) {
				TypeKind::Param(param) => Some(param),
				_ => None,
			})
			.collect();
		let mut captured = Vec::new();
		let mut releases = Vec::new();
		let mut transfers = Vec::new();
		let sources =
			std::iter::once(partial.function).chain(partial.bound.iter().map(|&(_, value)| value));
		for source in sources {
			let value = self.expr(source);
			let value = self.copy_stored(source, value);
			let drop = self.storage_drop_function(self.type_of(source));
			let value = match &drop {
				Some(drop) => self.register_drop_temporary(source, value, drop.clone()),
				None => value,
			};
			let name = self.temp();
			self.out.line(format!("const {name} = {};", value.text));
			if let Some(drop) = drop {
				releases.push(format!("{drop}({name});"));
				if let Some(transfer) =
					self.take_closure_temporary(source, &Expr::primary(name.clone()))
				{
					transfers.push(transfer.text);
				}
			}
			captured.push(name);
		}
		let saved = self.hidden_names.clone();
		let mut parameters = Vec::new();
		for requirement in self.hidden_for_generics(&residual_generics) {
			let name = self.temp();
			self.hidden_names.insert(requirement, name.clone());
			parameters.push(name);
		}
		let residual = self.temp();
		parameters.push(format!("...{residual}"));
		let bindings = self.types.list_items(partial.generic_arguments).to_vec();
		let methods = self
			.analysis
			.instantiated_methods
			.get(&node)
			.cloned()
			.unwrap_or_default();
		let hidden = self.hidden_for_generics(&partial.generic_parameters);
		let (body, mut arguments) = self.capture(|g| {
			g.hidden_arguments_bound(
				node,
				&partial.generic_parameters,
				hidden,
				&bindings,
				&methods,
			)
		});
		self.hidden_names = saved;
		let TypeKind::Function(original) = self.types.kind(partial.original_type) else {
			return self.unsupported(node, "the target of this partial function");
		};
		let original_params = self.types.params(self.types.sig(original).params);
		for (index, param) in original_params.iter().enumerate() {
			if let Some(bound) = partial
				.bound
				.iter()
				.position(|(bound, _)| *bound as usize == index)
			{
				arguments.push(captured[bound + 1].clone());
			} else {
				let index = partial
					.remaining
					.iter()
					.position(|&remaining| remaining as usize == index)
					.expect("a residual parameter");
				arguments.push(if param.variadic {
					if requirements == 0 {
						format!("...{residual}.slice({index})")
					} else {
						format!("...{residual}.slice({index}, -{requirements})")
					}
				} else if partial.defaults[partial.remaining[index] as usize]
					&& self.may_be_none(param.ty)
				{
					let omitted = self.helper("faOmitted");
					format!(
						"({residual}.length - {requirements} <= {index} ? {omitted} : {residual}[{index}])"
					)
				} else {
					format!("{residual}[{index}]")
				});
			}
		}
		for index in 0..requirements {
			arguments.push(format!(
				"{residual}[{residual}.length - {}]",
				requirements - index
			));
		}
		let call = js::call(&captured[0], &arguments);
		let parameters = parameters.join(", ");
		let prefix = body.finish();
		let mut value = format!("({parameters}) => {{ {prefix}return {}; }}", call.text);
		if signature.is_async {
			let helper = self.helper("faCallValue");
			let mut asynchronous = vec!["$task".to_string(), captured[0].clone()];
			asynchronous.extend(arguments);
			let call = js::call(helper, &asynchronous);
			let asynchronous = format!(
				"async ($task, {parameters}) => {{ {prefix}return await {}; }}",
				call.text
			);
			let helper = self.helper("faAsyncValue");
			value = js::call(helper, &[value, asynchronous]).text;
		}
		releases.reverse();
		let own = self.helper("faOwnShared");
		let value = js::call(own, &[value, format!("() => {{ {} }}", releases.join(" "))]);
		let value = if transfers.is_empty() {
			value
		} else {
			Expr::new(
				format!("({}, {})", transfers.join(", "), value.text),
				Prec::Primary,
			)
		};
		self.register_function_temporary(node, value)
	}
}
