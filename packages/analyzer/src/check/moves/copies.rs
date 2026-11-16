//! Diagnostics for concrete owned stores and explicit copies.

use rustc_hash::FxHashMap;

use crate::analysis::NodeFlags;
use crate::check::Walker;
use crate::diagnostics::codes::advanced::{MEM002, MEM003, MEM004};
use crate::diagnostics::{Fix, Location, SourceSpan, TextEdit};
use crate::ids::NodeId;
use crate::symbols::Resolution;
use crate::syntax::View;
use crate::types::{Ownership, TypeKind};

impl Walker<'_> {
	pub(super) fn finish_copy_checks(&mut self, conflicts: &FxHashMap<NodeId, NodeId>) {
		for index in 0..self.tree.nodes.len() {
			let site = NodeId::new(index);
			if matches!(self.view(site), View::Copy(_))
				&& self.s.types.is_unique(self.a.type_of(site))
			{
				let ty = self.display(self.a.type_of(site));
				let diagnostic = self.diagnostic(&MEM003, site, format!("`{ty}` is unique or owns a unique value and cannot be copied; move its owner, or create a new instance instead"));
				self.report(diagnostic);
			}
		}
		let mut sites: Vec<_> = self.copy_sites.iter().copied().collect();
		sites.sort_unstable();
		for value in sites {
			if !self.copy_reads_place(value) || self.ownership_unreachable(value) {
				continue;
			}
			let ty = self
				.a
				.coercion(value)
				.unwrap_or_else(|| self.a.type_of(value));
			let category = self.s.types.ownership(ty);
			if !matches!(category, Ownership::Structure | Ownership::Unique) {
				continue;
			}
			if self.a.has_flags(value, NodeFlags::MOVE) || self.a.moved_reads.contains(&value) {
				continue;
			}
			if self.output.iter().any(|diagnostic| {
				diagnostic.code.id == MEM002.id && diagnostic.location.span == self.span(value)
			}) {
				continue;
			}
			if category == Ownership::Unique {
				let text = self.tree.text(value.index());
				let diagnostic = self.diagnostic(&MEM002, value, format!("`{text}` is borrowed here and cannot transfer a unique value; move its owner in the owning scope, remove an element with `take`/`pop`, or create a new instance"));
				self.report(diagnostic);
			} else {
				self.report_structure_copy(value, conflicts.get(&value).copied());
			}
		}
	}

	fn ownership_unreachable(&self, mut node: NodeId) -> bool {
		loop {
			if self.a.has_flags(node, NodeFlags::UNREACHABLE) {
				return true;
			}
			if matches!(self.view(node), View::Lambda { .. }) {
				return false;
			}
			let Some(parent) = self.parent(node) else {
				return false;
			};
			node = parent;
		}
	}

	fn copy_reads_place(&self, node: NodeId) -> bool {
		match self.view(node) {
			View::Name(_) => matches!(
				self.a.resolution(node),
				Resolution::Local(_) | Resolution::External(_)
			),
			View::Access { target, .. } | View::OptionalAccess { target, .. } => {
				// Preserve existing transfers from fresh, destructor-free
				// owners of unique fields. Structure-field moves land next.
				!(self.s.types.is_unique(self.a.type_of(node)) && self.fresh_unique_owner(target))
			}
			View::Index { index, .. } => {
				!matches!(self.s.types.kind(self.a.type_of(index)), TypeKind::Range)
			}
			View::Call { .. } => crate::lower::copies::element_view(&self.tree, &self.a, node)
				.is_some_and(|(_, collection)| !collection),
			_ => false,
		}
	}

	fn fresh_unique_owner(&self, node: NodeId) -> bool {
		let node = self.strip_groups(node);
		if self.s.types.has_destroy_boundary(self.a.type_of(node))
			|| matches!(
				self.s.types.kind(self.a.type_of(node)),
				TypeKind::Ref(..) | TypeKind::Pointer(_) | TypeKind::Sync(..)
			) {
			return false;
		}
		match self.view(node) {
			View::Object { .. } | View::TypedObject { .. } | View::Tuple { .. } => true,
			View::Call { .. } => {
				crate::lower::copies::element_view(&self.tree, &self.a, node).is_none()
			}
			View::Group(inner) => self.fresh_unique_owner(inner),
			_ => false,
		}
	}

	fn report_structure_copy(&mut self, value: NodeId, later: Option<NodeId>) {
		let ty = self.display(
			self.a
				.coercion(value)
				.unwrap_or_else(|| self.a.type_of(value)),
		);
		let text = self.tree.text(value.index());
		let reason = later.map_or_else(
			|| "its source is borrowed or this field cannot transfer ownership here".to_string(),
			|node| format!("its source is used again on line {}", self.line_of(node)),
		);
		let mut diagnostic = self.diagnostic(&MEM004, value, format!("storing `{text}` would silently copy the structure `{ty}`: {reason}; write `copy {text}`, borrow with `use`, or move the later use before this store"));
		if let Some(node) = later {
			diagnostic = diagnostic.with_related(
				Location {
					file: self.file,
					span: self.span(node),
				},
				"this later use keeps the source alive",
			);
		}
		let start = self.span(value).start;
		diagnostic = diagnostic.with_fix(Fix {
			title: format!("Make an independent copy of `{text}`"),
			edits: vec![TextEdit {
				location: Location {
					file: self.file,
					span: SourceSpan { start, end: start },
				},
				replacement: "copy ".to_string(),
			}],
		});
		self.report(diagnostic);
	}
}

#[cfg(test)]
mod tests {
	use crate::check::test_support::{analyzed, codes};

	#[test]
	fn a_structure_copy_has_an_applicable_fix_and_a_later_use() {
		let source = "let items = [1]\nlet duplicate = items\nconsole.log(items, duplicate)\n";
		let (session, file) = crate::analyze_source(source);
		let diagnostics = session.diagnostics(file);
		assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
		let diagnostic = &diagnostics[0];
		assert_eq!(diagnostic.code.id, "MEM004");
		assert!(diagnostic.message.contains("line 3"));
		assert_eq!(
			&source[diagnostic.related[0].0.span.start as usize
				..diagnostic.related[0].0.span.end as usize],
			"items"
		);
		assert_eq!(diagnostic.fixes.len(), 1);
		let mut fixed = source.to_string();
		for edit in diagnostic.fixes[0].edits.iter().rev() {
			assert_eq!(edit.location.file, file);
			fixed.replace_range(
				edit.location.span.start as usize..edit.location.span.end as usize,
				&edit.replacement,
			);
		}
		assert!(fixed.contains("duplicate = copy items"));
		analyzed(&fixed);
	}

	#[test]
	fn ordinary_strings_and_shared_handles_copy_as_values() {
		analyzed(
			"let name = \"ordinary\"\nlet other = name\nlet original = { name }\nlet duplicate = original\nconsole.log(name, other, original, duplicate)\n",
		);
		analyzed(
			"use { Mutex } = std.concurrency\nmutable lock = Mutex<Array<Integer>> { value = [1] }\nmutable second = lock\nconsole.log(lock, second)\n",
		);
	}

	#[test]
	fn a_structure_parameter_and_a_capture_only_borrow() {
		assert_eq!(
			codes("function wrap = (items: Array<Integer>): Array<Integer> => { items }\n"),
			["MEM004"]
		);
		analyzed("function wrap = (items: Array<Integer>) => copy items\n");
		assert_eq!(
			codes(
				"function outer = () => { let items = [1]\nfunction inner = (): Array<Integer> => { items }\ninner() }\n"
			),
			["MEM004"]
		);
	}

	#[test]
	fn partial_structure_moves_wait_for_field_support() {
		assert_eq!(
			codes(
				"let holder = { items = [1], count = 2 }\nlet items = holder.items\nconsole.log(items)\n"
			),
			["MEM004"]
		);
		analyzed(
			"let holder = { items = [1], count = 2 }\nlet items = copy holder.items\nconsole.log(holder.count, items)\n",
		);
	}

	#[test]
	fn an_indexed_owner_needs_an_explicit_copy() {
		assert_eq!(
			codes("let matrix = [[1]]\nlet row = matrix[0]\nconsole.log(matrix, row)\n"),
			["MEM004"]
		);
		analyzed("let matrix = [[1]]\nlet row = copy matrix[0]\nconsole.log(matrix, row)\n");
	}

	#[test]
	fn unique_values_cannot_be_explicitly_copied() {
		assert_eq!(
			codes("function duplicate = (task: Task<Integer>): Task<Integer> => copy task\n"),
			["MEM003"]
		);
		assert_eq!(
			codes("function duplicate = (holder: { task: Task<Integer> }) => copy holder\n"),
			["MEM003"]
		);
	}

	#[test]
	fn dependent_generic_stores_are_deferred_to_instantiation() {
		analyzed(
			"function wrap = <Item>(value: Item): Item => value\nfunction duplicate = <Item>(value: Item): Item => copy value\n",
		);
		assert_eq!(
			codes("function bad = <Item>(task: Task<Item>): Task<Item> => copy task\n"),
			["MEM003"]
		);
	}
}
