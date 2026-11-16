//! Whether a callee can keep a callable argument beyond the call. These
//! summaries are separate from parameter consumption and callback effects.

use rustc_hash::{FxHashMap, FxHashSet};

use crate::analysis::{Argument, Callee, FieldValue};
use crate::check::Walker;
use crate::ids::{NodeId, SymbolId};
use crate::symbols::{Resolution, SymbolKind};
use crate::syntax::View;

/// A local parameter dependency, an external summary, or an unknown callee.
#[derive(Default)]
struct Retention {
	parameters: Vec<SymbolId>,
	keeps: bool,
	unknown: bool,
}

impl Retention {
	fn unknown() -> Self {
		Self {
			keeps: true,
			unknown: true,
			parameters: Vec::new(),
		}
	}
}

impl Walker<'_> {
	/// The single retention query. Function types will eventually carry
	/// `invalidates parameter`; until then an unknown origin may keep it.
	fn may_callee_keep_argument(&self, call: NodeId, index: usize) -> Retention {
		let Some(info) = self.a.calls.get(&call) else {
			return Retention::unknown();
		};
		let mut retention = Retention::default();
		let functions = match info.callee {
			Callee::Function(function) => vec![function],
			Callee::Value => {
				let View::Call { callee, .. } = self.view(call) else {
					return Retention::unknown();
				};
				if let Some(keeps) = self.external_argument_retention(callee, index) {
					retention.keeps = keeps;
					return retention;
				}
				let facts = self.callable_source_facts(callee);
				retention.unknown = facts.unknown || facts.functions.is_empty();
				retention.keeps = retention.unknown;
				facts.functions
			}
			Callee::Builtin(builtin) => {
				let path = crate::builtins::get(builtin).path;
				retention.keeps = crate::builtins::effects::stores_arguments(path)
					|| matches!(path, "Channel#send" | "WorkerPool#submit");
				return retention;
			}
			Callee::Member(_) => {
				retention.keeps = true;
				return retention;
			}
			Callee::Conversion(_) => return retention,
		};
		for function in functions {
			let Some(param) = self
				.a
				.function(function)
				.and_then(|info| info.params.get(index))
			else {
				retention.keeps = true;
				retention.unknown = true;
				continue;
			};
			retention.parameters.push(param.symbol);
		}
		retention
	}

	fn external_argument_retention(&self, mut callee: NodeId, index: usize) -> Option<bool> {
		loop {
			match self.view(callee) {
				View::Group(inner) => callee = inner,
				View::Call { callee: inner, .. }
					if self
						.a
						.has_flags(callee, crate::analysis::NodeFlags::GENERIC_APPLICATION) =>
				{
					callee = inner
				}
				View::Access { members, .. } | View::OptionalAccess { members, .. } => {
					callee = members.last()?
				}
				_ => break,
			}
		}
		let Resolution::External(key) = self.a.resolution(callee) else {
			return None;
		};
		let interface = self.s.interface(self.s.types.decl_key_data(key).module)?;
		let declaration = interface.keys.get(&key)?.node;
		Some(interface.parameters.get(&declaration)?.get(index)?.kept)
	}

	/// Solve local forwarding dependencies once, then publish per-parameter
	/// booleans. A cycle that only borrows retains no parameter.
	pub(super) fn finish_parameter_retention(&mut self) {
		let mut origins = FxHashMap::default();
		let mut kept = FxHashSet::default();
		for (value, owner) in self.retention_stores() {
			kept.extend(
				self.retention_origins(value, &mut origins)
					.into_iter()
					.filter(|&parameter| {
						owner.is_none_or(|owner| {
							self.ownership_body(self.a.symbols.symbol(parameter).decl_node) == owner
						})
					}),
			);
		}
		let mut dependencies: FxHashMap<SymbolId, Vec<SymbolId>> = FxHashMap::default();
		for info in self.a.functions.values() {
			for param in &info.params {
				if let Some(default) = param.default {
					dependencies
						.entry(param.symbol)
						.or_default()
						.extend(self.retention_origins(default, &mut origins));
				}
			}
		}
		for info in self.a.starts.values() {
			for &capture in &info.captures {
				if matches!(
					self.a.symbols.symbol(capture).kind,
					SymbolKind::Parameter { .. }
				) {
					kept.insert(capture);
				}
			}
		}
		for (&call, info) in &self.a.calls {
			for (index, argument) in info.arguments.iter().enumerate() {
				let retention = self.may_callee_keep_argument(call, index);
				let values = match argument {
					Argument::Given(value) => vec![*value],
					Argument::Variadic(values) => values.clone(),
					Argument::Default => match info.callee {
						Callee::Function(function) => self
							.a
							.function(function)
							.and_then(|info| info.params.get(index))
							.and_then(|param| param.default)
							.into_iter()
							.collect(),
						_ => Vec::new(),
					},
					_ => Vec::new(),
				};
				let sources: Vec<_> = values
					.into_iter()
					.flat_map(|value| self.retention_origins(value, &mut origins))
					.collect();
				if retention.keeps {
					kept.extend(sources.iter().copied());
				}
				for parameter in retention.parameters {
					dependencies.entry(parameter).or_default().extend(&sources);
				}
			}
		}
		let mut pending: Vec<_> = kept.iter().copied().collect();
		while let Some(parameter) = pending.pop() {
			for &source in dependencies.get(&parameter).into_iter().flatten() {
				if kept.insert(source) {
					pending.push(source);
				}
			}
		}
		self.a.kept_parameters = kept;
	}

	fn retention_stores(&self) -> Vec<(NodeId, Option<NodeId>)> {
		let mut values: Vec<_> = self
			.a
			.objects
			.values()
			.flat_map(|object| {
				object.fields.iter().filter_map(|(_, value)| match value {
					FieldValue::Given(value) | FieldValue::Default(value) => Some((*value, None)),
					_ => None,
				})
			})
			.collect();
		for index in 0..self.tree.nodes.len() {
			let node = NodeId::new(index);
			if self
				.a
				.has_flags(node, crate::analysis::NodeFlags::TYPE_EXPR)
			{
				continue;
			}
			match self.view(node) {
				View::Array { items } | View::Tuple { items } => {
					values.extend(items.iter().map(|item| (item, None)))
				}
				View::Return(Some(value)) => values.push((value, Some(self.ownership_body(node)))),
				View::ExportValue { value, .. } | View::Throw(value) => values.push((value, None)),
				View::Assignment {
					target,
					value: Some(value),
					..
				} if self.retention_target_outlives_call(target, node) => values.push((value, None)),
				_ => {}
			}
		}
		values.extend(
			self.a
				.functions
				.values()
				.map(|info| (info.body, Some(info.lambda))),
		);
		values
	}

	fn retention_target_outlives_call(&self, target: NodeId, store: NodeId) -> bool {
		if !matches!(self.view(target), View::Name(_)) {
			return true;
		}
		match self.a.resolution(target) {
			Resolution::Local(symbol) => {
				self.ownership_body(self.a.symbols.symbol(symbol).decl_node)
					!= self.ownership_body(store)
			}
			Resolution::External(_) => true,
			_ => false,
		}
	}

	fn retention_origins(
		&self,
		value: NodeId,
		cache: &mut FxHashMap<NodeId, Vec<SymbolId>>,
	) -> Vec<SymbolId> {
		// Function block bodies are checked directly, without an expression
		// type-table entry. Inspect their result paths instead of dropping
		// them as if they had a known scalar type.
		let ty = self.a.type_of(value);
		if !ty.is_unset() && !self.callable_slot_type(ty) {
			return Vec::new();
		}
		if let Some(known) = cache.get(&value) {
			return known.clone();
		}
		let mut parameters = FxHashSet::default();
		let facts = self.callable_source_facts(value);
		parameters.extend(facts.parameters);
		let mut pending = facts.functions;
		let mut seen = FxHashSet::default();
		while let Some(function) = pending.pop() {
			if !seen.insert(function) {
				continue;
			}
			{
				let Some(info) = self.a.function(function) else {
					continue;
				};
				for &capture in &info.captures {
					let source = self
						.aliases
						.get(&capture)
						.and_then(|&place| self.place_symbol(place))
						.unwrap_or(capture);
					if self.callable_slot_type(self.a.symbols.symbol(source).ty)
						&& matches!(
							self.a.symbols.symbol(source).kind,
							SymbolKind::Parameter { .. } | SymbolKind::SelfParameter
						) {
						parameters.insert(source);
					}
					let facts = self.callable_capture_facts(info.lambda, capture);
					parameters.extend(facts.parameters);
					pending.extend(facts.functions);
				}
			}
		}
		let mut parameters: Vec<_> = parameters.into_iter().collect();
		parameters.sort_unstable();
		cache.insert(value, parameters.clone());
		parameters
	}
}

#[cfg(test)]
mod tests {
	use crate::check::test_support::analyzed;
	use crate::symbols::SymbolKind;

	fn retained(source: &str, function: &str) -> Vec<String> {
		let (session, file) = analyzed(source);
		let analysis = session.analysis(file);
		let symbol = analysis
			.symbols
			.symbols()
			.iter()
			.find(|symbol| {
				symbol.kind == SymbolKind::Function
					&& session.types().syms.resolve(symbol.name) == function
			})
			.unwrap();
		analysis
			.function(symbol.decl_node)
			.unwrap()
			.params
			.iter()
			.filter(|param| analysis.kept_parameters.contains(&param.symbol))
			.map(|param| {
				session
					.types()
					.syms
					.resolve(analysis.symbols.symbol(param.symbol).name)
					.to_string()
			})
			.collect()
	}

	#[test]
	fn callback_aliases_and_known_forwarding_borrow() {
		let source = "type Reader = () => Integer\nfunction borrow = (callback: Reader): Integer => { let alias = callback\nalias() }\nfunction forward = (callback: Reader): Integer => { let operation = borrow\noperation(callback) }\n";
		assert!(retained(source, "borrow").is_empty());
		assert!(retained(source, "forward").is_empty());
	}

	#[test]
	fn a_nested_return_is_not_an_enclosing_return() {
		let source = "type Reader = () => Integer\nfunction borrow = (callback: Reader): Integer => {\nfunction local = (): Reader => callback\nlocal()()\n}\n";
		assert!(retained(source, "borrow").is_empty());
	}

	#[test]
	fn returned_stored_and_captured_callbacks_are_retained() {
		let source = "type Reader = () => Integer\nfunction keep = (callback: Reader): Reader => callback\nfunction forward = (callback: Reader): Reader => keep(callback)\nfunction capture = (callback: Reader): Reader => () => callback()\nfunction store = (callback: Reader): Integer => { let items = [callback]\nitems.length }\n";
		for name in ["keep", "forward", "capture", "store"] {
			assert_eq!(retained(source, name), ["callback"], "{name}");
		}
	}

	#[test]
	fn defaults_and_saved_capture_aliases_forward_retention() {
		let source = "type Reader = () => Integer\nfunction defaults = (callback: Reader, selected: Reader = callback): Reader => selected\nfunction capture = (callback: Reader): Reader => {\nlet alias = callback\n() => alias()\n}\nfunction project = (holder: { run: Reader }): Reader => holder.run\n";
		assert_eq!(retained(source, "defaults"), ["callback", "selected"]);
		assert_eq!(retained(source, "capture"), ["callback"]);
		assert_eq!(retained(source, "project"), ["holder"]);
	}

	#[test]
	fn unknown_indirect_calls_keep_their_callback_argument() {
		let source = "type Reader = () => Integer\nfunction indirect = (operation: (Reader) => Integer, callback: Reader): Integer => operation(callback)\n";
		assert_eq!(retained(source, "indirect"), ["callback"]);
	}

	#[test]
	fn borrowing_cycles_do_not_invent_retention() {
		let source = "type Reader = () => Integer\nfunction first = (callback: Reader, stop: Boolean): Integer => if stop { callback() } else { second(callback, true) }\nfunction second = (callback: Reader, stop: Boolean): Integer => first(callback, stop)\n";
		assert!(retained(source, "first").is_empty());
		assert!(retained(source, "second").is_empty());
	}

	#[test]
	fn retention_has_no_32_parameter_limit() {
		let mut parameters: Vec<_> = (0..40)
			.map(|index| format!("unused{index}: Integer"))
			.collect();
		parameters.push("callback: Reader".into());
		let source = format!(
			"type Reader = () => Integer\nfunction keep = ({}): Reader => callback\n",
			parameters.join(", ")
		);
		assert_eq!(retained(&source, "keep"), ["callback"]);
	}

	#[test]
	fn retention_is_published_and_changes_the_interface_hash() {
		let (session, file) = analyzed(
			"type Reader = () => Integer\nexport function = (callback: Reader): Reader => callback\n",
		);
		let interface = &session.source_file(file).interface;
		let mut changed = interface.clone();
		let parameter = changed
			.parameters
			.values_mut()
			.next()
			.unwrap()
			.first_mut()
			.unwrap();
		assert!(parameter.kept);
		parameter.kept = false;
		assert_ne!(
			interface.hash(session.types()),
			changed.hash(session.types())
		);
	}
}
