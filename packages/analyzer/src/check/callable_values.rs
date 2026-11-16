//! Evaluation-time sources of function slots. Reads keep their incoming
//! sources, so later assignments never rewrite an already saved handle.

use rustc_hash::FxHashSet;
use smallvec::smallvec;

use crate::check::Walker;
use crate::flow::CallableSources;
use crate::ids::{NodeId, PlaceId, TypeId};
use crate::symbols::Resolution;
use crate::syntax::View;
use crate::types::TypeKind;

impl Walker<'_> {
	fn callable_slot_type(&self, ty: TypeId) -> bool {
		if ty.is_unset() {
			return false;
		}
		match self.s.types.kind(ty) {
			TypeKind::Function(_) => return true,
			TypeKind::Union(_) | TypeKind::Rec(_) => {}
			_ => return false,
		}
		let mut pending = vec![ty];
		let mut seen = FxHashSet::default();
		while let Some(ty) = pending.pop() {
			if ty.is_unset() || !seen.insert(ty) {
				continue;
			}
			match self.s.types.kind(ty) {
				TypeKind::Function(_) => return true,
				TypeKind::Union(list) => pending.extend(self.s.types.list_items(list)),
				TypeKind::Rec(_) => pending.extend(self.s.types.rec_target(ty)),
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
			self.body
				.flow
				.callable_values
				.push((place, smallvec![Some(value)]));
		}
	}

	pub(super) fn record_callable_read(&mut self, node: NodeId, ty: TypeId) {
		if !self.body.flow.reachable
			|| !self.callable_slot_type(ty)
			|| !matches!(
				self.view(node),
				View::Name(_) | View::Access { .. } | View::Index { .. }
			) {
			return;
		}
		let Some(place) = self.place_of(node) else {
			return;
		};
		let sources = self
			.body
			.flow
			.callable_values
			.iter()
			.find(|(known, _)| *known == place)
			.map_or_else(|| smallvec![None], |(_, sources)| sources.clone());
		let recorded = self.callable_reads.entry(node).or_default();
		for source in sources {
			if !recorded.contains(&source) {
				recorded.push(source);
			}
		}
		recorded.sort_unstable();
	}

	/// Concrete local bodies a function value may call. Unknown sources stay
	/// unknown; inspecting a declaration's initializer here would be stale.
	pub(crate) fn callable_targets(&self, value: NodeId) -> Vec<NodeId> {
		let mut pending = vec![value];
		let mut seen = FxHashSet::default();
		let mut targets = Vec::new();
		while let Some(value) = pending.pop() {
			if !seen.insert(value) {
				continue;
			}
			if let Some(inputs) = crate::lower::copies::result_inputs(&self.tree, value) {
				pending.extend(inputs);
				continue;
			}
			match self.view(value) {
				View::Copy(inner) => pending.push(inner),
				View::Lambda { .. } => targets.push(value),
				View::Name(_) => {
					if let Resolution::Local(symbol) = self.a.resolution(value)
						&& let Some(declaration) = self.direct_function(symbol)
						&& let Some(info) = self.a.function(declaration)
					{
						targets.push(info.lambda);
					} else {
						pending.extend(
							self.callable_reads
								.get(&value)
								.into_iter()
								.flatten()
								.flatten(),
						);
					}
				}
				View::Access { .. } | View::Index { .. } => {
					pending.extend(
						self.callable_reads
							.get(&value)
							.into_iter()
							.flatten()
							.flatten(),
					);
				}
				View::Call { callee, .. }
					if self
						.a
						.has_flags(value, crate::analysis::NodeFlags::GENERIC_APPLICATION) =>
				{
					pending.push(callee)
				}
				_ => {}
			}
		}
		targets.sort_unstable();
		targets.dedup();
		targets
	}
}

pub(super) type CallableReads = rustc_hash::FxHashMap<NodeId, CallableSources>;

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
}
