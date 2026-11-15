//! Evaluation-time sources of values with hidden callable state. The finish
//! substitutes these sparse snapshots; it never reads a mutable slot's later
//! initializer in place of the value actually copied or transferred.

use smallvec::smallvec;

use crate::check::Walker;
use crate::flow::{Identity, PlaceRoot, Step};
use crate::ids::{NodeId, PlaceId, SymbolId, TypeId};
use crate::provenance::Projection;
use crate::syntax::View;
use crate::types::TypeFlags;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ValueRead {
	pub root: PlaceRoot,
	pub path: Vec<Projection>,
	pub bindings: Vec<(Vec<Projection>, Vec<Option<NodeId>>)>,
}

impl Walker<'_> {
	pub(super) fn bind_value_source(&mut self, place: PlaceId, ty: TypeId, value: NodeId) {
		self.body
			.flow
			.callable_values
			.retain(|(other, _)| !self.places.is_at_or_below(*other, place));
		if self.value_needs_snapshot(ty) {
			self.body
				.flow
				.callable_values
				.push((place, smallvec![Some(value)]));
		}
	}

	fn value_needs_snapshot(&self, ty: TypeId) -> bool {
		self.s
			.types
			.flags(ty)
			.intersects(TypeFlags::HAS_SHARED | TypeFlags::HAS_PARAM)
			&& self.type_may_hide_captures(ty)
	}

	pub(super) fn record_value_read(&mut self, node: NodeId, ty: TypeId) {
		let function = match self.view(node) {
			View::Lambda { .. } => Some(node),
			View::Name(_) => match self.a.resolution(node) {
				crate::symbols::Resolution::Local(symbol) => self.direct_function(symbol),
				_ => None,
			},
			View::Watch(_) => Some(node),
			_ => None,
		};
		if let Some(function) = function {
			let function = self
				.a
				.function(function)
				.map_or(function, |info| info.lambda);
			for symbol in self
				.capture_lists
				.get(&function)
				.cloned()
				.unwrap_or_default()
			{
				self.record_capture_value(node, symbol);
			}
		}
		if !self.value_needs_snapshot(ty)
			|| !matches!(
				self.view(node),
				View::Name(_) | View::Access { .. } | View::Index { .. }
			) {
			return;
		}
		let Some(place) = self.place_of(node) else {
			return;
		};
		let Some(read) = self.value_read(place) else {
			return;
		};
		let reads = self.value_reads.entry(node).or_default();
		if !reads.contains(&read) {
			reads.push(read);
		}
	}

	pub(super) fn record_capture_value(&mut self, site: NodeId, symbol: SymbolId) {
		if !self.value_needs_snapshot(self.a.symbols.symbol(symbol).ty) {
			return;
		}
		let Some(place) = self.symbol_target(symbol) else {
			return;
		};
		let Some(read) = self.value_read(place) else {
			return;
		};
		let reads = self.capture_value_reads.entry((site, symbol)).or_default();
		if !reads.contains(&read) {
			reads.push(read);
		}
	}

	fn value_read(&self, place: PlaceId) -> Option<ValueRead> {
		let root = self.places.get(place).root;
		let path = self.value_projection(place)?;
		let mut bindings = Vec::new();
		for (other, sources) in &self.body.flow.callable_values {
			if self.places.get(*other).root != root {
				continue;
			}
			let Some(other) = self.value_projection(*other) else {
				continue;
			};
			if other
				.iter()
				.zip(&path)
				.all(|(a, b)| projection_matches(*a, *b))
			{
				bindings.push((other, sources.to_vec()));
			}
		}
		if bindings.is_empty() {
			return None;
		}
		bindings.sort_unstable_by_key(|(path, _)| path.len());
		Some(ValueRead {
			root,
			path,
			bindings,
		})
	}

	fn value_projection(&self, place: PlaceId) -> Option<Vec<Projection>> {
		self.place_steps(place)
			.into_iter()
			.map(|step| match step {
				Step::Field(name) => Some(Projection::Field(name)),
				Step::Index(Identity::Exact(index)) if u32::try_from(index).is_ok() => {
					Some(Projection::Index(index as u32))
				}
				Step::Index(_) | Step::All | Step::Child => Some(Projection::Item),
				_ => None,
			})
			.collect()
	}
}

pub(super) fn projection_matches(left: Projection, right: Projection) -> bool {
	left == right
		|| matches!(
			(left, right),
			(Projection::Item, Projection::Index(_)) | (Projection::Index(_), Projection::Item)
		)
}

#[cfg(test)]
mod tests {
	fn errors(statements: &str, expression: &str) -> Vec<String> {
		let mut session = crate::Session::new();
		session.set_target("native");
		let source = format!(
			"use {{ Mutex, WorkerPool }} = std.concurrency\ntype Action = () => Integer\nfunction safe = (): Integer => 0\nfunction wrap = (action: Action): Action => () => action()\nfunction makeReader = (gate: Mutex<Task<Integer>>): Action => () => if gate.tryLock() != none {{ 1 }} else {{ 0 }}\nasync function main = (branch: Boolean) => {{ let pending = start do {{ 1 }}\nmutable gate = Mutex<Task<Integer>> {{ value = pending }}\nlet hidden = makeReader(gate)\n{statements}\nmutable pool = WorkerPool<Plain> {{ count = 1 }}\nlet work = await pool.submit((_worker) => {expression})\nawait work\npool.shutdown() }}\n"
		);
		let texts = vec![
			("main.fa".to_string(), source.clone()),
			(
				"Plain.worker.fa".to_string(),
				"export type = { function number = (self): Integer => 1 }\n".to_string(),
			),
		];
		let resolver = crate::CodebaseResolver::from_texts(&texts, Some("main.fa"));
		session.set_resolver(Box::new(resolver));
		let file = session.set_source("main", std::sync::Arc::from(source));
		session.analyze_file(file);
		session
			.diagnostics(file)
			.iter()
			.filter(|diagnostic| diagnostic.severity == crate::Severity::Error)
			.map(|diagnostic| format!("{} {}", diagnostic.code.id, diagnostic.message))
			.collect()
	}

	#[test]
	fn reassigned_callables_preserve_executor_affinity() {
		for (statements, expression) in [
			("mutable action: Action = safe\naction = hidden", "action()"),
			(
				"mutable action: Action = safe\nif branch { action = hidden }",
				"action()",
			),
			(
				"mutable holder = { action = safe }\nholder.action = hidden",
				"holder.action()",
			),
			(
				"mutable holder = { action = safe }\nholder.action = hidden\nlet copied = holder",
				"copied.action()",
			),
			(
				"mutable action: Action = safe\naction = hidden\nfunction named = () => action()",
				"named()",
			),
		] {
			let found = errors(statements, expression);
			assert_eq!(found.len(), 1, "{statements}: {found:?}");
			assert!(found[0].starts_with("CONC008"), "{statements}: {found:?}");
		}
	}

	#[test]
	fn later_assignments_do_not_rewrite_copied_captures() {
		for statements in [
			"mutable action: Action = safe\nlet saved = action\naction = hidden",
			"mutable action: Action = hidden\naction = safe\nlet saved = action",
			"mutable action: Action = safe\nlet saved = wrap(action)\naction = hidden",
		] {
			let found = errors(statements, "saved()");
			assert!(found.is_empty(), "{statements}: {found:?}");
		}
	}
}
