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
	pub(crate) fn check_structure_conversion(
		&mut self,
		value: NodeId,
		actual: crate::ids::TypeId,
		target: crate::ids::TypeId,
	) {
		let Some((source_item, target_item)) = self.s.types.structure_conversion(actual, target)
		else {
			return;
		};
		self.a.coercions.insert(value, target);
		if self.is_copy_operand_value(value) {
			return;
		}
		if self
			.s
			.types
			.store_projection(actual, target)
			.erased_owner
			.is_some()
		{
			self.report_projection_erasure(value, actual, target);
			return;
		}
		if self.output.iter().any(|diagnostic| {
			diagnostic.code.id == MEM004.id && diagnostic.location.span == self.span(value)
		}) {
			return;
		}
		let source = self.tree.text(value.index());
		let enclosing = self.copy_scope_hint(value);
		let generic = if self.view_sink {
			if let TypeKind::Collection(crate::types::CollKind::Array, item, _) =
				self.s.types.kind(target)
			{
				format!(
					"; preserve the parameter's actual item type with `<Item: {}>(items: Array<Item>)`",
					self.display(item)
				)
			} else {
				"; make the parameter generic over the actual type to avoid converting its structure".to_string()
			}
		} else {
			String::new()
		};
		let diagnostic = self.diagnostic(&MEM004, value, format!("converting `{source}` from `{}` to `{}` changes items or recursive parts from `{}` to `{}`: the items are stored differently and the conversion walks the whole value; write `copy {source}`, even at its last use{generic}{enclosing}", self.display(actual), self.display(target), self.display(source_item), self.display(target_item)))
			.with_fix(Fix { title: format!("Convert every part with `copy {source}`"), edits: vec![TextEdit {
				location: Location { file: self.file, span: SourceSpan::new(self.span(value).start, self.span(value).start) },
				replacement: "copy ".into(),
			}] });
		self.report_flow(diagnostic);
	}

	/// Only value-preserving result paths lead to a Copy operand. Copying
	/// a call's result does not authorize conversions of its arguments.
	fn is_copy_operand_value(&self, mut value: NodeId) -> bool {
		while let Some(parent) = self.parent(value) {
			if let View::Copy(operand) = self.view(parent) {
				return operand == value;
			}
			let result = crate::lower::copies::result_inputs(&self.tree, parent)
				.is_some_and(|inputs| inputs.contains(&value));
			if !result {
				return false;
			}
			value = parent;
		}
		false
	}

	fn copy_scope_hint(&self, mut value: NodeId) -> &'static str {
		while let Some(parent) = self.parent(value) {
			match self.view(parent) {
				View::Copy(_) => {
					return "; the enclosing `copy` copies its operand's completed value and does not reach this inner store or argument";
				}
				View::Lambda { .. } | View::Watch(_) | View::Test { .. } => break,
				View::Do(_)
					if self
						.parent(parent)
						.is_some_and(|node| matches!(self.view(node), View::Start(_))) =>
				{
					break;
				}
				_ => value = parent,
			}
		}
		""
	}

	/// Widening the destination cannot introduce owned data that the
	/// source does not have. Structural projection checks retained fields.
	pub(crate) fn store_ownership(&mut self, value: NodeId) -> Ownership {
		let actual = self.a.type_of(value);
		let target = self.a.coercion(value).unwrap_or(actual);
		self.s.types.store_projection(actual, target).ownership
	}

	pub(super) fn finish_copy_checks(&mut self, conflicts: &FxHashMap<NodeId, NodeId>) {
		self.check_consuming_defaults();
		for index in 0..self.tree.nodes.len() {
			let site = NodeId::new(index);
			if let View::Copy(inner) = self.view(site)
				&& (self.store_ownership(inner) == Ownership::Unique
					|| self.s.types.has_lifetime_boundary(self.a.type_of(inner)))
			{
				let ty = self.display(self.a.type_of(inner));
				let diagnostic = self.diagnostic(&MEM003, site, format!("`{ty}` is unique or owns a unique value and cannot be copied; move its owner, or create a new instance instead"));
				self.report(diagnostic);
			}
		}
		let mut sites: Vec<_> = self.copy_sites.iter().copied().collect();
		sites.sort_unstable();
		for value in sites {
			if self.ownership_unreachable(value) {
				continue;
			}
			let actual = self.a.type_of(value);
			let ty = self
				.a
				.coercion(value)
				.unwrap_or_else(|| self.a.type_of(value));
			if self
				.s
				.types
				.store_projection(actual, ty)
				.erased_owner
				.is_some()
			{
				self.report_projection_erasure(value, actual, ty);
				continue;
			}
			let projection = self.s.types.store_projection(actual, ty);
			if projection.needs_field_moves
				&& !matches!(self.view(value), View::Copy(_))
				&& !self.a.copy_inputs.contains_key(&value)
			{
				if projection.ownership == Ownership::Unique {
					let source = self.tree.text(value.index());
					let diagnostic = self.diagnostic(&MEM002, value, format!("this projection would move selected unique fields out of `{source}`; write the retained fields explicitly in the owning scope; remove a collection element with `take`/`pop` first, and keep a borrowed resource with its owner"));
					self.report(diagnostic);
				} else {
					self.report_structure_copy(value, conflicts.get(&value).copied());
				}
				continue;
			}
			if !self.copy_reads_place(value) {
				continue;
			}
			if self.output.iter().any(|diagnostic| {
				diagnostic.code.id == MEM004.id && diagnostic.location.span == self.span(value)
			}) {
				continue;
			}
			let category = self.store_ownership(value);
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

	/// A repeatedly evaluated consuming default must create its owner inside
	/// that default. An existing value cannot be consumed at every call.
	fn check_consuming_defaults(&mut self) {
		let mut defaults: Vec<_> = self
			.a
			.functions
			.values()
			.flat_map(|info| &info.params)
			.filter(|param| {
				self.s
					.types
					.parameter_can_be_consumed(self.a.symbols.symbol(param.symbol).ty)
			})
			.filter_map(|param| param.default)
			.collect();
		defaults.sort_unstable();
		defaults.dedup();
		for default in defaults {
			let span = self.span(default);
			let mut pending = vec![default];
			while let Some(value) = pending.pop() {
				if let Some(inputs) = crate::lower::copies::result_inputs(&self.tree, value) {
					pending.extend(inputs);
					continue;
				}
				if !self.copy_reads_place(value) {
					continue;
				}
				let created_here = crate::lower::copies::value_place_root(
					&self.tree, &self.a, value,
				)
				.is_some_and(|symbol| {
					let declaration = self.span(self.a.symbols.symbol(symbol).decl_node);
					span.start <= declaration.start
						&& declaration.end <= span.end
						&& self.ownership_source_owned(symbol, value)
				});
				if created_here {
					continue;
				}
				let source = self.tree.text(value.index());
				let diagnostic = self.diagnostic(&MEM002, value, format!("the default `{source}` names an existing value, but this parameter consumes its argument at every call; build a new value in the default, e.g. `start work()`, or pass the value explicitly at the call"));
				self.report(diagnostic);
			}
		}
	}

	pub(crate) fn check_function_projection_owner(
		&mut self,
		node: NodeId,
		actual: crate::ids::TypeId,
		target: crate::ids::TypeId,
	) {
		let TypeKind::Function(source) = self.s.types.kind(actual) else {
			return;
		};
		let TypeKind::Function(target) = self.s.types.kind(target) else {
			return;
		};
		let actual = self.s.types.sig(source).ret;
		let target = self.s.types.sig(target).ret;
		if self
			.s
			.types
			.store_projection(actual, target)
			.erased_owner
			.is_some()
			&& !self.output.iter().any(|diagnostic| {
				diagnostic.code.id == MEM002.id && diagnostic.location.span == self.span(node)
			}) {
			let diagnostic = self.diagnostic(&MEM002, node, format!("this function returns `{}`, whose lifetime cannot be erased by an implicit result adapter; return a written data projection from the function body, e.g. `{{ name = resource.name }}`, with `copy` on retained structure fields", self.display(actual)));
			self.report_flow(diagnostic);
		}
	}

	pub(crate) fn report_projection_erasure(
		&mut self,
		value: NodeId,
		actual: crate::ids::TypeId,
		ty: crate::ids::TypeId,
	) {
		if !self.output.iter().any(|diagnostic| {
			diagnostic.code.id == MEM002.id && diagnostic.location.span == self.span(value)
		}) {
			let source = self.tree.text(value.index());
			let fields = self
				.s
				.types
				.fields_of(ty)
				.iter()
				.filter(|field| {
					!field.flags.intersects(
						crate::types::FieldFlags::METHOD | crate::types::FieldFlags::RELATION,
					)
				})
				.map(|field| {
					let name = self.s.types.syms.resolve(field.name);
					format!("{name} = {source}.{name}")
				})
				.collect::<Vec<_>>()
				.join(", ");
			let diagnostic = self.diagnostic(&MEM002, value, format!("this conversion would erase the lifetime owner `{}`; its destructor must stay with its owner; write a data projection explicitly, e.g. `{{ {fields} }}`, adding `copy` to any retained structure field", self.display(actual)));
			self.report(diagnostic);
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
		let actual = self.a.type_of(value);
		let target = self.a.coercion(value).unwrap_or(actual);
		let ty = self.display(target);
		let text = self.tree.text(value.index()).to_string();
		let mut retained = Vec::new();
		if actual != target {
			for field in self.s.types.fields_of(target).to_vec() {
				if let Some(source) = self
					.s
					.types
					.fields_of(actual)
					.iter()
					.find(|source| source.name == field.name)
					.copied() && self.s.types.store_projection(source.ty, field.ty).ownership
					== Ownership::Structure
				{
					let name = self.s.types.syms.resolve(field.name);
					retained.push(format!(
						"`{text}.{name}` (write `{name} = copy {text}.{name}` in a projection)"
					));
				}
			}
		}
		let projection = if retained.is_empty() {
			String::new()
		} else {
			format!(
				"; retained field {} needs an independent copy",
				retained.join(", ")
			)
		};
		let reason = later.map_or_else(
			|| "its source is borrowed or this field cannot transfer ownership here".to_string(),
			|node| format!("its source is used again on line {}", self.line_of(node)),
		);
		let enclosing = self.copy_scope_hint(value);
		let mut diagnostic = self.diagnostic(&MEM004, value, format!("storing `{text}` would silently copy the structure `{ty}`: {reason}{projection}; write `copy {text}`, borrow with `use`, or move the later use before this store{enclosing}"));
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
	fn projections_that_discard_siblings_wait_for_field_moves() {
		let source = "type Box = { items: Array<Integer>, count: Integer }\nfunction inspect = (value: Integer): Integer => {\nlet box = Box { items = [value], count = 7 }\nlet stored: { items: Array<Integer> } = box\nstored.items.length\n}\n";
		let (session, file) = crate::analyze_source(source);
		let diagnostics = session.diagnostics(file);
		assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
		assert_eq!(diagnostics[0].code.id, "MEM004");
		let mut fixed = source.to_string();
		for edit in diagnostics[0].fixes[0].edits.iter().rev() {
			fixed.replace_range(
				edit.location.span.start as usize..edit.location.span.end as usize,
				&edit.replacement,
			);
		}
		analyzed(&fixed);
		analyzed(
			"type Box = { items: Array<Integer> }\nfunction inspect = (value: Integer): Integer => {\nlet box = Box { items = [value] }\nlet stored: { items: Array<Integer> } = box\nstored.items.length\n}\n",
		);
	}

	#[test]
	fn a_projected_copy_copies_only_retained_nonunique_fields() {
		let source = "function inspect = (box: { items: Array<Integer>, task: Task<Integer> }): Integer => {\nlet stored: { items: Array<Integer> } = box\nstored.items.length\n}\n";
		let (session, file) = crate::analyze_source(source);
		let diagnostics = session.diagnostics(file);
		assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
		assert_eq!(diagnostics[0].code.id, "MEM004");
		let mut fixed = source.to_string();
		for edit in diagnostics[0].fixes[0].edits.iter().rev() {
			fixed.replace_range(
				edit.location.span.start as usize..edit.location.span.end as usize,
				&edit.replacement,
			);
		}
		analyzed(&fixed);
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

	#[test]
	fn projecting_ordinary_fields_does_not_copy_the_discarded_collection() {
		analyzed(
			"function label = (box: { items: Array<Integer>, name: String }): { name: String } => box\n",
		);
		assert_eq!(
			codes(
				"function selected = (box: { items: Array<Integer>, name: String }): { items: Array<Integer> } => box\n"
			),
			["MEM004"]
		);
		analyzed(
			"function selected = (box: { items: Array<Integer>, name: String }): { items: Array<Integer> } => ({ items = copy box.items })\n",
		);
	}

	#[test]
	fn storing_an_ordinary_union_member_does_not_move_its_source() {
		analyzed(
			"let name = \"ordinary\"\nlet first: String | Task<Integer> = name\nlet second: String | Task<Integer> = copy name\nconsole.log(name, first, second)\n",
		);
	}

	#[test]
	fn a_resource_requires_a_written_data_projection() {
		for (body, expected) in [
			(
				"let plain: { name: String } = held\nconsole.log(plain)",
				vec!["MEM002"],
			),
			(
				"let plain: { name: String } = copy held\nconsole.log(plain)",
				vec!["MEM003"],
			),
			(
				"let plain = { name = held.name }\nconsole.log(plain)",
				vec![],
			),
		] {
			let source = format!(
				"type Held = {{ name: String, function destroy = (self) => {{}} }}\nlet held = Held {{ name = \"kept\" }}\n{body}\n"
			);
			let mut session = crate::Session::new();
			let file = session.set_source("projection.foreign", std::sync::Arc::from(source));
			session.analyze_file(file);
			let diagnostics = session.diagnostics(file);
			assert_eq!(
				diagnostics
					.iter()
					.map(|diagnostic| diagnostic.code.id)
					.collect::<Vec<_>>(),
				expected,
				"{diagnostics:?}"
			);
			if expected == ["MEM002"] {
				assert!(
					diagnostics[0].message.contains("name = held.name"),
					"{diagnostics:?}"
				);
			}
		}
	}

	#[test]
	fn a_function_adapter_cannot_erase_a_result_destructor() {
		let source = "type Held = { name: String, function destroy = (self) => {} }\nfunction acquire = (): Held => Held { name = \"file\" }\ntype Plain = { name: String }\nlet make: () => Plain = acquire\nconsole.log(make())\n";
		let mut session = crate::Session::new();
		let file = session.set_source("adapter.foreign", std::sync::Arc::from(source));
		session.analyze_file(file);
		let diagnostics = session.diagnostics(file);
		assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
		assert_eq!(diagnostics[0].code.id, "MEM002", "{diagnostics:?}");
		assert!(diagnostics[0].message.contains("function body"));
	}

	#[test]
	fn converting_array_items_is_explicit_even_at_the_last_use() {
		for suffix in [
			"let wide: Array<Integer> = small\nconsole.log(wide)",
			"function read = (items: Array<Integer>) => console.log(items)\nread(small)",
		] {
			let source = format!("let small: Array<Int8> = [1, 2]\n{suffix}\n");
			let (session, file) = crate::analyze_source(&source);
			let diagnostics = session.diagnostics(file);
			assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
			assert_eq!(diagnostics[0].code.id, "MEM004");
			assert!(diagnostics[0].message.contains("walks the whole value"));
			let edit = &diagnostics[0].fixes[0].edits[0];
			let mut fixed = source.clone();
			fixed.insert_str(edit.location.span.start as usize, &edit.replacement);
			analyzed(&fixed);
		}
	}

	#[test]
	fn widening_primitive_ranges_and_literals_does_not_convert_items() {
		analyzed(
			"let exact: Array<1 | 2> = [1, 2]\nfunction read = (items: Array<Integer>) => console.log(items)\nread(exact)\nlet wide: Array<Integer> = exact\nconsole.log(wide)\n",
		);
		analyzed(
			"let exact: Array<\"a\" | \"b\"> = [\"a\", \"b\"]\nfunction read = (items: Array<String>) => console.log(items)\nread(exact)\nlet wide: Array<String> = exact\nconsole.log(wide)\n",
		);
		analyzed(
			"let exact: Array<\"a\"> = [\"a\"]\nlet wider: Array<\"a\" | \"b\"> = exact\nconsole.log(wider)\n",
		);
	}

	#[test]
	fn adding_optionality_or_union_alternatives_changes_item_storage() {
		for target in ["Integer?", "Integer | String"] {
			assert_eq!(
				codes(&format!(
					"let items = [1, 2]\nlet other: Array<{target}> = items\nconsole.log(other)\n"
				)),
				["MEM004"]
			);
			analyzed(&format!(
				"let items = [1, 2]\nlet other: Array<{target}> = copy items\nconsole.log(other)\n"
			));
		}
	}

	#[test]
	fn a_different_recursive_type_requires_a_converting_copy() {
		let declarations = "type Small = { value: Int8, next: Small? }\ntype Wide = { value: Integer, next: Wide? }\nlet small = Small { value = 1, next = Small { value = 2 } }\n";
		assert_eq!(
			codes(&format!(
				"{declarations}let wide: Wide = small\nconsole.log(wide.value)\n"
			)),
			["MEM004"]
		);
		analyzed(&format!(
			"{declarations}let wide: Wide = copy small\nconsole.log(wide.value)\n"
		));
	}

	#[test]
	fn copying_does_not_make_a_fallible_item_narrowing_implicit() {
		assert_eq!(
			codes("function narrow = (items: Array<Integer>): Array<Int8> => copy items\n"),
			["E-TYPE-1"]
		);
	}

	#[test]
	fn copying_a_call_result_does_not_copy_its_arguments() {
		let declarations = "function count = (items: Array<Integer>): Integer => items.length\nlet items: Array<Int8> = [1]\n";
		assert_eq!(
			codes(&format!(
				"{declarations}let length = copy count(items)\nconsole.log(length)\n"
			)),
			["MEM004"]
		);
		analyzed(&format!(
			"{declarations}let length = copy count(copy items)\nconsole.log(length)\n"
		));
	}

	#[test]
	fn copying_a_literal_result_does_not_copy_its_field_stores() {
		let declarations = "let small: Array<Int8> = [1, 2]\n";
		let source = format!(
			"{declarations}let box: {{ items: Array<Integer> }} = copy {{ items = small }}\nconsole.log(box)\n"
		);
		let (session, file) = crate::analyze_source(&source);
		let diagnostics = session.diagnostics(file);
		assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
		assert_eq!(diagnostics[0].code.id, "MEM004");
		assert!(
			diagnostics[0]
				.message
				.contains("does not reach this inner store")
		);
		analyzed(&format!(
			"{declarations}let box: {{ items: Array<Integer> }} = {{ items = copy small }}\nconsole.log(box)\n"
		));
	}
	#[test]
	fn copy_inputs_keep_each_producers_type_before_coercion() {
		let source = "function choose = (flag: Boolean, first: Array<Int8>, second: Array<Int16>): Array<Integer> => copy (if flag { first } else { second })\n";
		let (session, file) = analyzed(source);
		let tree = session.tree(file);
		let analysis = session.analysis(file);
		let mut values: Vec<_> = analysis
			.copy_inputs
			.iter()
			.filter(|(_, input)| input.produces)
			.map(|(&node, input)| {
				(
					tree.text(node.index()).to_string(),
					session.display_type(input.source),
					session.display_type(input.target),
				)
			})
			.collect();
		values.sort();
		assert_eq!(
			values,
			[
				(
					"first".into(),
					"Array<Int8>".into(),
					"Array<Integer>".into()
				),
				(
					"second".into(),
					"Array<Int16>".into(),
					"Array<Integer>".into()
				)
			]
		);
		assert!(
			analysis
				.copy_inputs
				.iter()
				.filter(|(_, input)| !input.produces)
				.all(|(_, input)| session.display_type(input.target) == "Array<Integer>")
		);
	}

	#[test]
	fn fallback_inputs_preserve_none_until_the_fallback_selects_a_value() {
		let (session, file) = analyzed(
			"function choose = (first: Array<Int8>?, last: Array<Int16>): Array<Integer> => copy (first ?? last)\n",
		);
		let analysis = session.analysis(file);
		let tree = session.tree(file);
		let first = analysis
			.copy_inputs
			.iter()
			.find(|(node, input)| input.produces && tree.text(node.index()) == "first")
			.map(|(_, input)| input)
			.unwrap();
		assert_eq!(session.display_type(first.source), "Array<Int8>?");
		assert_eq!(session.display_type(first.target), "Array<Integer>?");
		let last = analysis
			.copy_inputs
			.iter()
			.find(|(node, input)| input.produces && tree.text(node.index()) == "last")
			.map(|(_, input)| input)
			.unwrap();
		assert_eq!(session.display_type(last.target), "Array<Integer>");
	}
}
