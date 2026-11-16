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
				if let Some((keeps, unknown)) = self.external_argument_retention(callee, index) {
					retention.keeps = keeps;
					retention.unknown = unknown;
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

	fn external_argument_retention(
		&self,
		mut callee: NodeId,
		index: usize,
	) -> Option<(bool, bool)> {
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
		let (module, declaration) = match self.a.resolution(callee) {
			Resolution::External(key) => {
				let module = self.s.types.decl_key_data(key).module;
				(module, self.s.interface(module)?.keys.get(&key)?.node)
			}
			Resolution::Local(symbol) => self.proxy_decl(symbol)?,
			_ => return None,
		};
		let interface = self.s.interface(module)?;
		let parameter = interface.parameters.get(&declaration)?.get(index)?;
		Some((parameter.kept, parameter.kept_unknown))
	}

	/// Solve local forwarding dependencies once, then publish per-parameter
	/// booleans. A cycle that only borrows retains no parameter.
	pub(super) fn finish_parameter_retention(&mut self) {
		let version = self.retention_version;
		if self.retention_checked_version == Some(version) {
			return;
		}
		let mut origins = FxHashMap::default();
		let mut kept = FxHashSet::default();
		let mut unknown = FxHashSet::default();
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
			if !self.retention_context_ready(call) {
				continue;
			}
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
				if retention.unknown {
					unknown.extend(sources.iter().copied());
				}
				for parameter in retention.parameters {
					dependencies.entry(parameter).or_default().extend(&sources);
				}
			}
		}
		propagate_retention(&mut kept, &dependencies);
		propagate_retention(&mut unknown, &dependencies);
		self.a.kept_parameters = kept;
		self.a.unknown_kept_parameters = unknown;
		self.finish_retained_callback_escapes();
		self.retention_checked_version = Some(version);
	}

	fn finish_retained_callback_escapes(&mut self) {
		let mut values = self.retention_stores();
		// A retained default must own its callback even when its caller is
		// another file. It is created on each omitted-argument call.
		for info in self.a.functions.values() {
			for param in &info.params {
				if self.a.kept_parameters.contains(&param.symbol)
					&& let Some(default) = param.default
				{
					values.push((default, None));
				}
			}
		}
		for (value, owner) in values {
			for function in self.callable_source_facts(value).functions {
				if !self
					.a
					.function(function)
					.is_some_and(|info| info.sig.is_some())
				{
					continue;
				}
				if owner.is_none_or(|owner| self.ownership_body(function) == owner) {
					self.escape_closure(function, value);
				}
			}
		}
		let mut arguments = Vec::new();
		for (&call, info) in &self.a.calls {
			if !self.retention_context_ready(call) {
				continue;
			}
			for (index, argument) in info.arguments.iter().enumerate() {
				let retention = self.may_callee_keep_argument(call, index);
				let keeps = retention.keeps
					|| retention
						.parameters
						.iter()
						.any(|parameter| self.a.kept_parameters.contains(parameter));
				if !keeps {
					continue;
				}
				let unknown = retention.unknown
					|| retention
						.parameters
						.iter()
						.any(|parameter| self.a.unknown_kept_parameters.contains(parameter));
				match argument {
					Argument::Given(value) => arguments.push((call, *value, unknown)),
					Argument::Variadic(values) => {
						arguments.extend(values.iter().map(|&value| (call, value, unknown)))
					}
					_ => {}
				}
			}
		}
		for (call, value, unknown) in arguments {
			for function in self.callable_source_facts(value).functions {
				if !self
					.a
					.function(function)
					.is_some_and(|info| info.sig.is_some())
				{
					continue;
				}
				if unknown {
					self.unknown_callback_escapes.insert(function, call);
				}
				self.escape_closure(function, call);
			}
		}
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
				.filter(|info| info.sig.is_some())
				.map(|info| (info.body, Some(info.lambda))),
		);
		values.retain(|(value, _)| self.retention_context_ready(*value));
		values
	}

	fn retention_context_ready(&self, node: NodeId) -> bool {
		self.a
			.function(self.ownership_body(node))
			.is_none_or(|info| info.sig.is_some())
	}

	pub(super) fn retention_target_outlives_call(&self, target: NodeId, store: NodeId) -> bool {
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

fn propagate_retention(
	parameters: &mut FxHashSet<SymbolId>,
	dependencies: &FxHashMap<SymbolId, Vec<SymbolId>>,
) {
	let mut pending: Vec<_> = parameters.iter().copied().collect();
	while let Some(parameter) = pending.pop() {
		for &source in dependencies.get(&parameter).into_iter().flatten() {
			if parameters.insert(source) {
				pending.push(source);
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use crate::check::test_support::analyzed;
	use crate::symbols::SymbolKind;

	fn retained(source: &str, function: &str) -> Vec<String> {
		let (session, file) = analyzed(source);
		retained_in(&session, file, function)
	}

	fn retained_in(
		session: &crate::Session,
		file: crate::ids::FileId,
		function: &str,
	) -> Vec<String> {
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

	#[test]
	fn retention_summaries_cross_file_boundaries() {
		let files = vec![
			("library/borrow.fa".into(), "type Reader = () => Integer\nexport function = (callback: Reader): Integer => callback()\n".into()),
			("library/keep.fa".into(), "type Reader = () => Integer\nexport function = (callback: Reader): Reader => callback\n".into()),
			("library/wrappers.fa".into(), "type Reader = () => Integer\nexport namespace = {\nfunction uses = (callback: Reader): Integer => borrow(callback)\nfunction keeps = (callback: Reader): Reader => keep(callback)\n}\n".into()),
		];
		let mut session = crate::Session::new();
		session.set_resolver(Box::new(crate::CodebaseResolver::from_texts(&files, None)));
		let program = session.analyze_package(None);
		for &file in &program.files {
			let errors: Vec<_> = session
				.diagnostics(file)
				.into_iter()
				.filter(|diagnostic| diagnostic.severity == crate::Severity::Error)
				.collect();
			assert!(errors.is_empty(), "{errors:?}");
		}
		let file = session.file_of_module("library/wrappers").unwrap();
		assert!(retained_in(&session, file, "uses").is_empty());
		assert_eq!(retained_in(&session, file, "keeps"), ["callback"]);
	}

	#[test]
	fn a_local_callable_assignment_still_borrows_its_captures() {
		analyzed("type Reader = () => Integer\nfunction run = (): Integer => {\nmutable count = 0\nfunction add = (): Integer => { count += 1\ncount }\nmutable invoke: Reader = add\nlet saved = invoke\ninvoke = saved\ninvoke()\ncount\n}\n");
	}

	#[test]
	fn an_unknown_retention_diagnostic_explains_forwarded_conservatism() {
		let source = "type Reader = () => Integer\nfunction forward = (operation: (Reader) => Integer, callback: Reader): Integer => operation(callback)\nfunction run = (operation: (Reader) => Integer): Integer => {\nmutable count = 0\nforward(operation, () => { count += 1\ncount })\n}\n";
		let (session, file) = crate::analyze_source(source);
		let diagnostics = session.diagnostics(file);
		let errors: Vec<_> = diagnostics.iter().filter(|diagnostic| diagnostic.severity == crate::Severity::Error).collect();
		assert_eq!(errors.len(), 1, "{errors:?}");
		assert_eq!(errors[0].code.id, "REF021");
		assert!(errors[0].message.contains("`count`"));
		assert!(errors[0].message.contains("type doesn't say whether it keeps"));
		assert!(errors[0].message.contains("known borrowing function"));
	}

	#[test]
	fn retained_callback_facts_precede_compile_time_execution() {
		let source = include_str!("../../tests/cases/ownership-retained-callback-snapshot.fa");
		let (session, file) = analyzed(source);
		let tree = session.tree(file);
		let value = session.analysis(file).constants.iter().find(|(node, _)| tree.text(node.index()) == "@snapshot()").unwrap().1;
		assert_eq!(value.to_text(), "14");
	}
}
