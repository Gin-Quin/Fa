//! Evaluation-time sources of callable data. Reads retain both their source
//! and projection, independently of later assignments to the original place.

use rustc_hash::{FxHashMap, FxHashSet};
use smallvec::smallvec;

use crate::analysis::{Callee, FieldValue};
use crate::check::Walker;
use crate::flow::{CallableBindings, Identity, Step};
use crate::ids::{NodeId, PlaceId, Sym, SymbolId, TypeId};
use crate::symbols::{Resolution, SymbolKind};
use crate::syntax::View;
use crate::types::{FieldFlags, TypeKind};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
enum Projection {
	Field(Sym),
	Index(i64),
	Item,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ReadSource {
	value: Option<NodeId>,
	path: Vec<Projection>,
}

#[derive(Default)]
pub(super) struct CallableFacts {
	pub functions: Vec<NodeId>,
	pub parameters: Vec<SymbolId>,
	pub unknown: bool,
}

pub(super) type CallableReads = rustc_hash::FxHashMap<NodeId, Vec<ReadSource>>;

enum SearchStep {
	Enter(NodeId, Vec<Projection>),
	Leave(NodeId, Vec<Projection>),
}

struct TargetSearch {
	pending: Vec<SearchStep>,
	active: FxHashMap<NodeId, Vec<Vec<Projection>>>,
	seen: FxHashSet<(NodeId, Vec<Projection>)>,
	unknown: bool,
}

impl TargetSearch {
	fn new(value: NodeId) -> Self {
		Self {
			pending: vec![SearchStep::Enter(value, Vec::new())],
			active: FxHashMap::default(),
			seen: FxHashSet::default(),
			unknown: false,
		}
	}

	fn push(&mut self, (node, path): (NodeId, Vec<Projection>)) {
		self.pending.push(SearchStep::Enter(node, path));
	}

	fn extend(&mut self, values: impl IntoIterator<Item = (NodeId, Vec<Projection>)>) {
		for value in values {
			self.push(value);
		}
	}

	fn pop(&mut self) -> Option<(NodeId, Vec<Projection>)> {
		while let Some(step) = self.pending.pop() {
			match step {
				SearchStep::Leave(node, path) => {
					if let Some(paths) = self.active.get_mut(&node) {
						paths.retain(|known| known != &path);
						if paths.is_empty() {
							self.active.remove(&node);
						}
					}
				}
				SearchStep::Enter(node, path) => {
					if self.active.get(&node).is_some_and(|paths| {
						paths
							.iter()
							.any(|known| known == &path || path.len() > known.len())
					}) {
						self.unknown = true;
						continue;
					}
					if !self.seen.insert((node, path.clone())) {
						continue;
					}
					self.active.entry(node).or_default().push(path.clone());
					self.pending.push(SearchStep::Leave(node, path.clone()));
					return Some((node, path));
				}
			}
		}
		None
	}
}

impl Walker<'_> {
	pub(super) fn callable_slot_type(&self, ty: TypeId) -> bool {
		if ty.is_unset() {
			return false;
		}
		match self.s.types.kind(ty) {
			TypeKind::Function(_) | TypeKind::Param(_) => return true,
			TypeKind::Object(_)
			| TypeKind::Collection(..)
			| TypeKind::CArray(..)
			| TypeKind::Tuple(_)
			| TypeKind::Union(_)
			| TypeKind::Rec(_)
			| TypeKind::NamedUnion(..)
			| TypeKind::Member(..) => {}
			_ => return false,
		}
		let types = &self.s.types;
		let mut pending = vec![ty];
		let mut seen = FxHashSet::default();
		while let Some(ty) = pending.pop() {
			if ty.is_unset() || !seen.insert(ty) {
				continue;
			}
			match types.kind(ty) {
				TypeKind::Function(_) | TypeKind::Param(_) => return true,
				TypeKind::Object(shape) => pending.extend(
					types
						.shape_fields(shape)
						.iter()
						.filter(|field| {
							!field
								.flags
								.intersects(FieldFlags::METHOD | FieldFlags::RELATION)
						})
						.map(|field| field.ty),
				),
				TypeKind::Collection(_, item, key) => pending.extend([item, key]),
				TypeKind::CArray(item, _) => pending.push(item),
				TypeKind::Tuple(list) | TypeKind::Union(list) => {
					pending.extend(types.list_items(list))
				}
				TypeKind::Rec(_) => pending.extend(types.rec_target(ty)),
				TypeKind::Member(nominal, index) => pending.push(types.payload(nominal, index)),
				TypeKind::NamedUnion(nominal, mask) => {
					let count = types.sym_list(types.nominal(nominal).members).len() as u16;
					for index in 0..count {
						if types.mask_has(mask, index) {
							pending.push(types.payload(nominal, index));
						}
					}
				}
				_ => {}
			}
		}
		false
	}

	pub(super) fn bind_callable_source(&mut self, place: PlaceId, ty: TypeId, value: NodeId) {
		self.body
			.flow
			.callable_values
			.retain(|(other, _)| !self.places.is_at_or_below(*other, place));
		if self.body.flow.reachable && self.callable_slot_type(ty) {
			self.retention_version += 1;
			self.body
				.flow
				.callable_values
				.push((place, smallvec![Some(value)]));
		}
	}

	fn callable_path(&self, place: PlaceId) -> Option<Vec<Projection>> {
		let mut path = Vec::new();
		for step in self.place_steps(place) {
			match step {
				Step::Field(name) => path.push(Projection::Field(name)),
				Step::Index(Identity::Exact(index)) => path.push(Projection::Index(index)),
				Step::Index(_) | Step::All | Step::Child => path.push(Projection::Item),
				Step::Payload(_) => {}
				_ => return None,
			}
		}
		Some(path)
	}

	pub(super) fn record_callable_call(&mut self, call: NodeId) {
		if self.a.calls.get(&call).is_some_and(|info| info.arguments.iter().any(|argument| {
			matches!(argument, crate::analysis::Argument::Given(value) if self.callable_slot_type(self.a.type_of(*value)))
		})) { self.retention_version += 1; }
		if !self.body.flow.reachable || self.body.flow.callable_values.is_empty() {
			return;
		}
		match self.callable_calls.entry(call) {
			std::collections::hash_map::Entry::Vacant(entry) => {
				entry.insert(self.body.flow.callable_values.clone());
			}
			std::collections::hash_map::Entry::Occupied(mut entry) => {
				crate::flow::join_callable_bindings(
					entry.get_mut(),
					&self.body.flow.callable_values,
				);
			}
		}
	}

	pub(super) fn record_callable_read(&mut self, node: NodeId, _ty: TypeId) {
		if matches!(self.view(node), View::Lambda { .. } | View::Function { .. }) {
			self.record_callable_captures(node);
		}
		if !self.body.flow.reachable
			|| self.body.flow.callable_values.is_empty()
			|| !matches!(
				self.view(node),
				View::Name(_) | View::Access { .. } | View::Index { .. }
			) {
			return;
		}
		let Some(place) = self.place_of(node) else {
			return;
		};
		let sources = self.callable_place_sources(place, &self.body.flow.callable_values);
		if sources.is_empty() {
			return;
		}
		let recorded = self.callable_reads.entry(node).or_default();
		for source in sources {
			if !recorded.contains(&source) {
				recorded.push(source);
			}
		}
	}

	fn record_callable_captures(&mut self, node: NodeId) {
		if !self.body.flow.reachable {
			return;
		}
		let Some(info) = self.a.function(node) else {
			return;
		};
		let lambda = info.lambda;
		let captures = info.captures.clone();
		for symbol in captures {
			let place = match self.aliases.get(&symbol) {
				Some(&place) => place,
				None => self.places.symbol(symbol),
			};
			let declared = self
				.place_type(place)
				.unwrap_or(self.a.symbols.symbol(symbol).ty);
			let actual = self.narrowed_place_type(place, declared);
			let key = (lambda, symbol);
			let captured = match self.capture_types.get(&key).copied() {
				Some(previous) if previous != actual => self.s.types.join(previous, actual),
				_ => actual,
			};
			if self.capture_types.insert(key, captured) != Some(captured) {
				self.retention_version += 1;
			}
			let sources = self.callable_place_sources(place, &self.body.flow.callable_values);
			if sources.is_empty() {
				continue;
			}
			let recorded = self.callable_captures.entry((lambda, symbol)).or_default();
			for source in sources {
				if !recorded.contains(&source) {
					recorded.push(source);
					self.retention_version += 1;
				}
			}
		}
	}

	pub(super) fn callable_capture_facts(&self, lambda: NodeId, symbol: SymbolId) -> CallableFacts {
		let mut search = TargetSearch::new(lambda);
		search.pending.clear();
		for source in self
			.callable_captures
			.get(&(lambda, symbol))
			.into_iter()
			.flatten()
		{
			if let Some(value) = source.value {
				search.push((value, source.path.clone()));
			} else {
				search.unknown = true;
			}
		}
		self.search_callable_targets(search)
	}

	fn callable_place_sources(&self, place: PlaceId, values: &CallableBindings) -> Vec<ReadSource> {
		let Some(path) = self.callable_path(place) else {
			return Vec::new();
		};
		let root = self.places.get(place).root;
		let mut bindings = Vec::new();
		for (known, sources) in values {
			if self.places.get(*known).root != root {
				continue;
			}
			let Some(prefix) = self.callable_path(*known) else {
				continue;
			};
			if prefix.len() <= path.len()
				&& prefix
					.iter()
					.zip(&path)
					.all(|(a, b)| projection_matches(*a, *b))
			{
				bindings.push((prefix.len(), sources));
			}
		}
		bindings.sort_unstable_by_key(|(length, _)| std::cmp::Reverse(*length));
		let mut recorded = Vec::new();
		for (length, sources) in bindings {
			for value in sources {
				let source = ReadSource {
					value: *value,
					path: path[length..].to_vec(),
				};
				if !recorded.contains(&source) {
					recorded.push(source);
				}
			}
			// Unknown replacements also retain the ancestor on another path.
			if !sources.contains(&None) {
				break;
			}
		}
		recorded
	}

	/// A borrowed capture of a callable slot is read when the outer call
	/// runs, rather than at the captured function's declaration.
	pub(crate) fn callable_targets_at_call(&self, call: NodeId, place: PlaceId) -> Vec<NodeId> {
		let Some(bindings) = self.callable_calls.get(&call) else {
			return Vec::new();
		};
		let mut search = TargetSearch::new(call);
		search.pending.clear();
		for source in self.callable_place_sources(place, bindings) {
			if let Some(value) = source.value {
				search.push((value, source.path));
			}
		}
		self.search_callable_targets(search).functions
	}

	pub(crate) fn callable_is_borrowed_capture(&mut self, value: NodeId) -> bool {
		let Some(place) = self.place_of(value) else {
			return false;
		};
		let Some(symbol) = self.place_symbol(place) else {
			return false;
		};
		self.ownership_body(self.a.symbols.symbol(symbol).decl_node) != self.ownership_body(value)
	}

	pub(crate) fn callable_borrowed_targets(&mut self, call: NodeId) -> Vec<NodeId> {
		let callee = match self.view(call) {
			View::Call { callee, .. } => callee,
			_ => return Vec::new(),
		};
		if self.callable_is_borrowed_capture(callee) {
			Vec::new()
		} else {
			self.callable_targets(callee)
		}
	}

	/// Builtin item reads retain their receiver's origin, including a
	/// projection to that receiver in a longer member chain.
	fn builtin_callable_result(
		&self,
		call: NodeId,
		callee: NodeId,
		suffix: &[Projection],
	) -> Option<(NodeId, Vec<Projection>)> {
		let builtin = self.builtin_path(call, callee)?;
		if !matches!(builtin, "Array#get" | "Array#find" | "Array#pop") {
			return None;
		}
		let View::Access { target, members } = self.view(callee) else {
			return None;
		};
		let mut path = Vec::new();
		for member in members.iter().take(members.len().saturating_sub(1)) {
			let View::Name(name) = self.view(member) else {
				return None;
			};
			path.push(Projection::Field(
				self.s.types.syms.get(self.tree.name(name))?,
			));
		}
		let projection = if builtin == "Array#get" {
			let View::Call { arguments, .. } = self.view(call) else {
				return None;
			};
			let node = arguments.iter().next()?;
			let node = match self.view(node) {
				View::Assignment {
					value: Some(value), ..
				} => value,
				View::Assignment { value: None, .. } => return None,
				_ => node,
			};
			match self.view(self.strip_groups(node)) {
				View::Integer(value) => i64::try_from(value).ok().map(Projection::Index),
				View::Negate(inner) => match self.view(inner) {
					View::Integer(value) => value
						.checked_neg()
						.and_then(|value| i64::try_from(value).ok())
						.map(Projection::Index),
					_ => None,
				},
				_ => None,
			}
			.unwrap_or(Projection::Item)
		} else {
			Projection::Item
		};
		path.push(projection);
		path.extend_from_slice(suffix);
		Some((target, path))
	}

	fn callable_returns(&self, function: NodeId) -> Vec<NodeId> {
		let Some(info) = self.a.function(function) else {
			return Vec::new();
		};
		let mut values = vec![info.body];
		let mut pending = vec![info.body];
		while let Some(node) = pending.pop() {
			match self.view(node) {
				View::Return(Some(value)) => values.push(value),
				View::Lambda { .. } | View::Function { .. } => continue,
				_ => pending.extend(crate::syntax::children(&self.tree, node)),
			}
		}
		values
	}

	/// Concrete local bodies a value may call. Unknown values never fall
	/// back to a mutable declaration's initializer.
	pub(crate) fn callable_targets(&self, value: NodeId) -> Vec<NodeId> {
		self.callable_source_facts(value).functions
	}

	pub(super) fn callable_source_facts(&self, value: NodeId) -> CallableFacts {
		self.search_callable_targets(TargetSearch::new(value))
	}

	fn search_callable_targets(&self, mut pending: TargetSearch) -> CallableFacts {
		let mut facts = CallableFacts::default();
		while let Some((value, path)) = pending.pop() {
			if self.a.type_of(value) == TypeId::NONE {
				continue;
			}
			if let Some(inputs) = crate::lower::copies::result_inputs(&self.tree, value) {
				pending.extend(inputs.into_iter().map(|input| (input, path.clone())));
				continue;
			}
			match self.view(value) {
				// A missing optional contributes no callable to `??`.
				View::None => {}
				View::Copy(inner) | View::Spread(inner) | View::Return(Some(inner)) => {
					pending.push((inner, path))
				}
				View::Lambda { .. } if path.is_empty() => facts.functions.push(value),
				View::Name(_) | View::Access { .. } | View::Index { .. } => {
					let direct = if path.is_empty() {
						if let Resolution::Local(symbol) = self.a.resolution(value) {
							self.direct_function(symbol)
								.and_then(|declaration| self.a.function(declaration))
								.map(|info| info.lambda)
						} else {
							None
						}
					} else {
						None
					};
					if let Some(function) = direct {
						facts.functions.push(function);
					} else {
						if let Some(symbol) =
							crate::lower::copies::value_place_root(&self.tree, &self.a, value)
							&& matches!(
								self.a.symbols.symbol(symbol).kind,
								SymbolKind::Parameter { .. } | SymbolKind::SelfParameter
							) {
							facts.parameters.push(symbol);
						}
						if !self.callable_reads.contains_key(&value) {
							pending.unknown = true;
						}
						for source in self.callable_reads.get(&value).into_iter().flatten() {
							if source.value.is_none() {
								pending.unknown = true;
							}
							if let Some(node) = source.value {
								let mut projection = source.path.clone();
								projection.extend_from_slice(&path);
								pending.push((node, projection));
							}
						}
					}
				}
				View::Object { .. } | View::TypedObject { .. } => {
					let Some(object) = self.a.objects.get(&value) else {
						continue;
					};
					for (name, field) in &object.fields {
						let suffix = match path.split_first() {
							None => &[][..],
							Some((Projection::Field(selected), suffix)) if selected == name => {
								suffix
							}
							_ => continue,
						};
						match *field {
							FieldValue::Given(node) | FieldValue::Default(node) => {
								pending.push((node, suffix.to_vec()))
							}
							FieldValue::Spread(node) => {
								let mut path = vec![Projection::Field(*name)];
								path.extend_from_slice(suffix);
								pending.push((node, path));
							}
							_ => {}
						}
					}
				}
				View::Tuple { items } | View::Array { items } => {
					let has_spread = items
						.iter()
						.any(|node| matches!(self.view(node), View::Spread(_)));
					let (selected, suffix) = match path.split_first() {
						Some((Projection::Index(index), suffix)) if !has_spread => {
							let index = if *index < 0 {
								items.len() as i128 + i128::from(*index)
							} else {
								i128::from(*index)
							};
							(usize::try_from(index).ok(), suffix)
						}
						Some((Projection::Index(_) | Projection::Item, suffix)) => (None, suffix),
						None => (None, &[][..]),
						_ => continue,
					};
					for (index, item) in items.iter().enumerate() {
						if selected.is_some_and(|selected| selected != index) {
							continue;
						}
						let mut projection = suffix.to_vec();
						if let View::Spread(inner) = self.view(item) {
							projection.insert(0, Projection::Item);
							pending.push((inner, projection));
						} else {
							pending.push((item, projection));
						}
					}
				}
				View::Call { callee, .. } => {
					if self
						.a
						.has_flags(value, crate::analysis::NodeFlags::GENERIC_APPLICATION)
					{
						pending.push((callee, path));
					} else if let Some(source) = self.builtin_callable_result(value, callee, &path)
					{
						pending.push(source);
					} else if let Some(info) = self.a.calls.get(&value)
						&& let Callee::Function(function) = info.callee
					{
						pending.extend(
							self.callable_returns(function)
								.into_iter()
								.map(|node| (node, path.clone())),
						);
					} else {
						pending.unknown = true;
					}
				}
				_ => pending.unknown = true,
			}
		}
		facts.functions.sort_unstable();
		facts.functions.dedup();
		facts.parameters.sort_unstable();
		facts.parameters.dedup();
		facts.unknown = pending.unknown;
		facts
	}
}

fn projection_matches(a: Projection, b: Projection) -> bool {
	a == b
		|| matches!(
			(a, b),
			(Projection::Item, Projection::Index(_)) | (Projection::Index(_), Projection::Item)
		)
}

#[cfg(test)]
mod tests {
	use crate::check::test_support::{analyzed, codes};

	fn program(statements: &str, result: &str) -> String {
		format!(
			"type Reader = () => Integer\nfunction zero = (): Integer => 0\nfunction demo = (branch: Boolean): Integer => {{\nlet items = [1]\nfunction read = (): Integer => items.length\n{statements}\n{result}\n}}\nconsole.log(demo(true))\n"
		)
	}

	#[test]
	fn indirect_borrowed_calls_keep_their_captures_alive() {
		for (statements, result) in [
			(
				"let invoke = read\nlet moved = items",
				"invoke() + moved.length",
			),
			(
				"mutable invoke: Reader = zero\nif branch { invoke = read }\nlet moved = items",
				"invoke() + moved.length",
			),
			(
				"mutable invoke: Reader = read\nlet saved = invoke\ninvoke = zero\nlet moved = items",
				"saved() + moved.length",
			),
		] {
			assert_eq!(
				codes(&program(statements, result)),
				["MEM004"],
				"{statements}"
			);
		}
	}

	#[test]
	fn projected_handles_keep_their_saved_capture_dependencies() {
		for (statements, result) in [
			(
				"let holder = { run = read }\nlet invoke = holder.run\nlet moved = items",
				"invoke() + moved.length",
			),
			(
				"let readers = (zero, read)\nlet moved = items",
				"readers[1]() + moved.length",
			),
			(
				"let readers = [zero, read]\nlet moved = items",
				"(readers.get(1) ?? zero)() + moved.length",
			),
			(
				"mutable holder = { run = zero }\nholder.run = read\nlet saved = holder.run\nholder.run = zero\nlet moved = items",
				"saved() + moved.length",
			),
		] {
			assert_eq!(
				codes(&program(statements, result)),
				["MEM004"],
				"{statements}"
			);
		}
		analyzed(&program(
			"let snapshot = copy items\nfunction independent = (): Integer => snapshot.length\nlet holder = { run = independent }\nlet moved = items",
			"holder.run() + moved.length",
		));
	}

	#[test]
	fn recursive_callable_sources_terminate_and_keep_reachable_targets() {
		let source = format!(
			"type Chain = {{ run: Reader, next: Chain? }}\n{}",
			program(
				"mutable chain = Chain { run = zero }\nfor 0..3 { chain = Chain { run = read, next = chain } }",
				"if chain.next is not none {\nlet invoke = chain.next.run\nlet moved = items\ninvoke() + moved.length\n} else { 0 }"
			)
		);
		assert_eq!(codes(&source), ["MEM004"]);
	}

	#[test]
	fn overwriting_a_handle_does_not_rewrite_earlier_reads() {
		analyzed(&program(
			"mutable invoke: Reader = read\ninvoke = zero\nlet moved = items",
			"invoke() + moved.length",
		));
		analyzed(&program(
			"mutable invoke: Reader = read\ninvoke = zero\nlet saved = invoke\ninvoke = read\nlet moved = items",
			"saved() + moved.length",
		));
		analyzed(&program(
			"let invoke = read\nlet moved = copy items",
			"invoke() + moved.length",
		));
	}

	#[test]
	fn captured_callable_slots_are_observed_at_the_call() {
		let prefix = "type Reader = () => Integer\nfunction demo = (): Integer => {\nlet first = [1]\nlet second = [1, 2]\nfunction readFirst = (): Integer => first.length\nfunction readSecond = (): Integer => second.length\nmutable current: Reader = readFirst\nfunction invoke = (): Integer => current()\ncurrent = readSecond\n";
		assert_eq!(
			codes(&format!(
				"{prefix}let moved = second\ninvoke() + moved.length\n}}\nconsole.log(demo())\n"
			)),
			["MEM004"]
		);
		analyzed(&format!(
			"{prefix}let moved = first\ninvoke() + moved.length\n}}\nconsole.log(demo())\n"
		));
		analyzed(&format!(
			"{prefix}let moved = copy second\ninvoke() + moved.length\n}}\nconsole.log(demo())\n"
		));
	}

	#[test]
	fn a_passed_borrowed_callback_keeps_its_capture_live() {
		let prefix = "function apply = (callback: Reader): Integer => callback()\nfunction ignore = (_callback: Reader): Integer => 0\n";
		for callee in ["apply", "ignore"] {
			let source = format!(
				"{prefix}{}",
				program(
					"let invoke = read\nlet moved = items",
					&format!("{callee}(invoke) + moved.length")
				)
			);
			assert_eq!(codes(&source), ["MEM004"], "{callee}");
			analyzed(&format!(
				"{prefix}{}",
				program(
					"let invoke = read\nlet moved = copy items",
					&format!("{callee}(invoke) + moved.length")
				)
			));
			analyzed(&format!(
				"{prefix}{}",
				program(
					&format!("let invoke = read\nlet length = {callee}(invoke)\nlet moved = items"),
					"length + moved.length"
				)
			));
		}
	}
}
