//! Ownership invalidation across branches, loop back edges and failure edges.

use std::collections::VecDeque;

use rustc_hash::FxHashMap;

use super::graph::Graph;
use crate::check::Walker;
use crate::diagnostics::Location;
use crate::diagnostics::codes::advanced::MEM002;
use crate::ids::{NodeId, PlaceId};

impl Walker<'_> {
	/// Take the place snapshot while aliases and narrowing still describe
	/// this read. Later `reuse` must not rewrite an earlier observation.
	pub(crate) fn note_place_read(&mut self, node: NodeId) {
		if self.body.flow.reachable
			&& let Some(place) = self.place_of(node)
		{
			self.move_places.insert(node, vec![place]);
			if let Some(symbol) = self.place_symbol(place) {
				self.record_move_read(node, symbol);
			}
		}
	}

	pub(super) fn check_move_validity(&mut self, graph: &Graph) {
		// Several transfers of the same place share one traversal. A
		// successful replacement ends the previous value's lifetime.
		let mut moves: FxHashMap<PlaceId, Vec<NodeId>> = FxHashMap::default();
		for &node in &self.a.moved_reads {
			if graph.at.contains_key(&node)
				&& let Some(places) = self.move_places.get(&node)
			{
				for &place in places {
					moves.entry(place).or_default().push(node);
				}
			}
		}
		let mut moves: Vec<_> = moves.into_iter().collect();
		moves.sort_by_key(|&(place, _)| place);
		for (place, mut sites) in moves {
			sites.sort_unstable();
			let Some(symbol) = self.place_symbol(place) else {
				continue;
			};
			let mut pending = VecDeque::new();
			for &site in &sites {
				for &index in &graph.at[&site] {
					// The source is read before it transfers. Store commit
					// events have no reads and must not seed a second move.
					if graph.reachable[index] && graph.events[index].reads.contains(&symbol) {
						pending.extend(graph.events[index].next.iter().map(|&next| (next, site)));
					}
				}
			}
			let mut visited = vec![false; graph.events.len()];
			while let Some((index, moved)) = pending.pop_front() {
				if std::mem::replace(&mut visited[index], true) {
					continue;
				}
				let event = &graph.events[index];
				let node = event.node;
				if event.reads.contains(&symbol) && !self.access_roots.contains(&node) {
					let overlaps = self.move_places.get(&node).is_none_or(|reads| {
						reads.iter().any(|&read| {
							self.places.is_at_or_below(read, place)
								|| self.places.is_at_or_below(place, read)
						})
					});
					if overlaps {
						self.report_moved_read(node, moved);
					}
				}
				let replaced = event.defines.contains(&symbol)
					|| self
						.move_writes
						.get(&node)
						.is_some_and(|&written| self.places.is_at_or_below(place, written));
				if !replaced {
					pending.extend(event.next.iter().map(|&next| (next, moved)));
				}
			}
		}
	}

	fn report_moved_read(&mut self, node: NodeId, moved: NodeId) {
		if self.output.iter().any(|diagnostic| {
			diagnostic.code.id == MEM002.id && diagnostic.location.span == self.span(node)
		}) {
			return;
		}
		let text = self.tree.text(moved.index());
		let line = self.line_of(moved);
		let diagnostic = self.diagnostic(&MEM002, node,
			format!("`{text}` was moved on line {line} and may no longer be used here; use its new owner, or assign the moved place before using it again"))
			.with_related(Location { file: self.file, span: self.span(moved) }, "ownership transferred here");
		self.report(diagnostic);
	}
}

#[cfg(test)]
mod tests {
	use crate::check::test_support::{analyzed, codes};

	const WORK: &str = "async function work = (): Integer => 1\n";

	#[test]
	fn a_failure_does_not_resurrect_a_consumed_task() {
		let source = format!(
			"{WORK}async function run = (text: String): Integer => {{ let task = start work()\ndo {{ let held = task\nInteger.parse(text) }} catch _error => await task\n0 }}\n"
		);
		assert_eq!(codes(&source), ["MEM002"]);
	}

	#[test]
	fn awaiting_consumes_even_when_its_error_is_caught() {
		let source = format!(
			"{WORK}async function run = (task: Task<Integer>): Integer => {{ await task catch _error => 0\nawait task catch _error => 0 }}\n"
		);
		assert_eq!(codes(&source), ["MEM002"]);
	}

	#[test]
	fn a_loop_cannot_consume_the_same_task_twice() {
		let source = format!(
			"{WORK}async function run = (count: Integer): Integer => {{ let task = start work()\nfor index in 0..count {{ await task }}\n0 }}\n"
		);
		assert_eq!(codes(&source), ["MEM002"]);
	}

	#[test]
	fn replacing_a_task_restores_its_owner() {
		analyzed(&format!(
			"{WORK}async function run = (count: Integer): Integer => {{ mutable task = start work()\nfor index in 0..count {{ await task\ntask = start work() }}\nawait task }}\n"
		));
	}

	#[test]
	fn a_sibling_field_survives_a_unique_field_move() {
		analyzed(&format!(
			"{WORK}async function run = (): Integer => {{ let holder = {{ task = start work(), count = 2 }}\nlet task = holder.task\nconsole.log(holder.count)\nawait task }}\n"
		));
	}

	#[test]
	fn a_started_block_cannot_consume_an_enclosing_task() {
		let source = format!(
			"{WORK}async function run = (): Integer => {{ let task = start work()\nlet pending = start do {{ await task }}\nawait pending }}\n"
		);
		assert_eq!(codes(&source), ["MEM002"]);
	}
}
