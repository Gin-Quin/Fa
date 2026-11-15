//! Copies and moves (advanced brief §6.2): the last use of a local that is
//! a copy (`let b = a`, `x = a`, `return a`, `items.push(a)`) can move the
//! value instead, since nothing reads it afterwards. The backends read the
//! `MOVE` flag of the read: native code skips the copy and the drop of the
//! source, JavaScript skips the copy (D15).
//!
//! The walk follows the evaluation order, so the last use recorded for a
//! local is its last use. A use counts only at the loop and function depth
//! of the local's declaration: inside a deeper loop the value is read again
//! by the next iteration, inside a closure it may be read later. A use
//! through a `use` alias, and a call of a local function that captures the
//! local, are uses of it.

use crate::analysis::NodeFlags;
use crate::check::Walker;
use crate::ids::{NodeId, SymbolId, TypeId};
use crate::symbols::Resolution;
use crate::symbols::SymbolKind;
use crate::syntax::View;
use crate::types::TypeKind;

/// The last use of a local seen so far.
#[derive(Copy, Clone, Debug)]
pub(crate) struct LastUse {
	/// The read.
	pub node: NodeId,
	/// At the loop and function depth of the declaration.
	pub movable: bool,
}

impl Walker<'_> {
	/// Records the loop and function depth of a local's declaration.
	pub(crate) fn note_declaration(&mut self, symbol: SymbolId) {
		let depth = (
			self.body.loops.len() as u32,
			self.body
				.functions
				.last()
				.map_or(u32::MAX, |function| function.node.0),
		);
		self.declaration_depths.insert(symbol, depth);
		self.last_uses.remove(&symbol);
	}

	/// Records a use of a local (a read or a write of its name, or through
	/// an alias of a place below it).
	pub(crate) fn note_use(&mut self, n: NodeId, symbol: SymbolId) {
		let movable = self
			.declaration_depths
			.get(&symbol)
			.is_some_and(|&(loops, functions)| {
				loops == self.body.loops.len() as u32
					&& functions
						== self
							.body
							.functions
							.last()
							.map_or(u32::MAX, |function| function.node.0)
			});
		let last = LastUse { node: n, movable };
		self.last_uses.insert(symbol, last);
		let uses = self.move_uses.entry(symbol).or_default();
		if let Some(known) = uses.iter_mut().find(|known| known.node == n) {
			known.movable |= movable;
		} else {
			uses.push(last);
		}
	}

	/// A value node in a copy position: the value of a `let`/`mutable`, of
	/// an assignment, of a `return`, an item stored by `push`/`add`.
	pub(crate) fn note_copy_site(&mut self, value: NodeId) {
		let value = self.strip_groups(value);
		self.escape(value, value);
		self.note_resource_move(value);
		self.record_copy_sources(value);
	}

	/// A conditional stores the selected branch, so each borrowed branch is
	/// a copy site. A fresh `copy` expression has no borrowed source here.
	fn record_copy_sources(&mut self, value: NodeId) {
		let value = self.strip_groups(value);
		let branches: Vec<_> = match self.view(value) {
			View::Do(inner) | View::Try(inner) => vec![inner],
			View::If {
				then_body,
				else_body,
				..
			} => std::iter::once(then_body).chain(else_body).collect(),
			View::When { arms, .. } => arms.iter().map(|arm| arm.value).collect(),
			View::Catch {
				expression,
				handler,
				..
			} => vec![expression, handler],
			View::Fallback { operands } => operands.iter().collect(),
			View::Block { statements } => statements.last().into_iter().collect(),
			_ => {
				self.copy_sites.insert(value);
				return;
			}
		};
		for branch in branches {
			self.record_copy_sources(branch);
		}
	}

	/// The use of a name node: a local itself, or the place an alias designates.
	pub(crate) fn note_name_use(&mut self, n: NodeId, symbol: SymbolId) {
		let kind = self.a.symbols.symbol(symbol).kind;
		match kind {
			SymbolKind::Let | SymbolKind::Mutable | SymbolKind::Parameter { .. } => {
				self.note_use(n, symbol);
			}
			SymbolKind::Use | SymbolKind::MutableUse | SymbolKind::LoopVariable { copy: false } => {
				// every place it may designate here (a `reuse` on one branch only)
				let mut places: Vec<crate::ids::PlaceId> = self
					.body
					.flow
					.ref_state(symbol)
					.map(|state| self.facts.refs.get(state).places.to_vec())
					.unwrap_or_default();
				places.extend(self.aliases.get(&symbol).copied());
				for place in places {
					if let Some(root) = self.place_symbol(place)
						&& root != symbol
					{
						// a read through the alias is not a copy of the root
						self.note_use(n, root);
					}
				}
			}
			SymbolKind::Function => {
				// a local function reads what it captures when it runs
				if let Some(declaration) = self.direct_function(symbol)
					&& let Some(info) = self.a.functions.get(&declaration)
				{
					// used as a value: it may run at any later point
					let value = self.callee_name_node != Some(n);
					for capture in info.captures.clone() {
						// Calling a local function from a closure needs the values
						// captured by that function in the closure's environment too.
						if self.is_captured(capture)
							&& let Some(cx) = self.body.functions.last_mut()
							&& !cx.captures.contains(&capture)
						{
							cx.captures.push(capture);
						}
						self.note_use(n, capture);
						if value {
							self.pinned_captures.insert(capture);
						}
					}
				}
			}
			_ => {}
		}
	}

	/// At the end of the walk: each last read in a copy position moves its
	/// owned source, including reads in mutually exclusive branches.
	pub(crate) fn finish_moves(&mut self) {
		let sites: Vec<_> = self.copy_sites.iter().copied().collect();
		for value in sites {
			let Some((symbol, read)) = self.copy_source(value) else {
				continue;
			};
			if !matches!(
				self.a.symbols.symbol(symbol).kind,
				SymbolKind::Let | SymbolKind::Mutable
			) || !(self.source_is_last_use(symbol, read, true)
				|| self.source_is_replaced_after(symbol, read, value))
				|| !self.source_is_owned(value)
			{
				continue;
			}
			let ty = self.a.type_of(value);
			if self.s.types.is_resource(ty)
				&& self
					.a
					.coercion(value)
					.is_some_and(|target| !self.s.types.is_resource(target))
			{
				// Copying a structural projection doesn't transfer its owner:
				// the resource's destructor still observes the original fields.
				continue;
			}
			if self
				.s
				.types
				.flags(ty)
				.contains(crate::types::TypeFlags::HAS_PARAM)
			{
				self.a.collection_moves.insert(value, ty);
				continue;
			}
			if self.s.types.contains_shared_handle(ty)
				&& !self.s.types.requires_explicit_copy(ty)
				&& !self.s.types.is_move_only(ty)
			{
				continue;
			}
			if self.owns_memory(ty)
				|| self
					.s
					.types
					.flags(ty)
					.contains(crate::types::TypeFlags::HAS_PARAM)
			{
				self.set_flag(value, NodeFlags::MOVE);
			}
		}
	}

	/// A direct local or its fields. Indexed items remain owned by their
	/// collection, so they require a copy unless explicitly taken out.
	pub(super) fn copy_source(&self, value: NodeId) -> Option<(SymbolId, NodeId)> {
		match self.view(self.strip_groups(value)) {
			View::Name(_) => match self.a.resolution(self.strip_groups(value)) {
				Resolution::Local(symbol) => Some((symbol, self.strip_groups(value))),
				_ => None,
			},
			View::Access { target, .. } => self.copy_source(target),
			_ => None,
		}
	}

	/// A resource's destructor needs its fields; they can't be moved out.
	pub(super) fn source_is_owned(&self, value: NodeId) -> bool {
		match self.view(self.strip_groups(value)) {
			View::Name(_) => true,
			View::Access { target, members } => {
				!self.s.types.is_resource(self.a.type_of(target))
					&& !matches!(
						self.s.types.kind(self.a.type_of(target)),
						TypeKind::Pointer(_) | TypeKind::Sync(..)
					) && members
					.iter()
					.all(|member| matches!(self.a.resolution(member), Resolution::Field(..)))
					&& members
						.iter()
						.take(members.len().saturating_sub(1))
						.all(|member| !self.s.types.is_resource(self.a.type_of(member)))
					&& self.source_is_owned(target)
			}
			_ => false,
		}
	}

	pub(super) fn source_is_last_use(&self, symbol: SymbolId, read: NodeId, movable: bool) -> bool {
		if self.pinned_captures.contains(&symbol) {
			return false;
		}
		let Some(uses) = self.move_uses.get(&symbol) else {
			return false;
		};
		let Some(own) = uses.iter().find(|use_| use_.node == read) else {
			return false;
		};
		let exit = self.passive_return_of(symbol, read);
		if movable && !own.movable && exit.is_none() {
			return false;
		}
		let start = self.tree.spans[read.index()].start;
		!uses.iter().any(|use_| {
			let at = self.tree.spans[use_.node.index()].start;
			use_.node != read
				&& at > start
				&& exit.is_none_or(|exit| at < self.tree.spans[exit.index()].end)
				&& !self.exclusive_reads(read, use_.node)
		})
	}

	/// A pure projection returned directly commits the function's exit.
	/// Code after it cannot read this owner, including another loop round.
	/// A returned assembly or call may still fail and enter a handler, and
	/// references/captures keep the conservative path of passive returns.
	fn passive_return_of(&self, symbol: SymbolId, read: NodeId) -> Option<NodeId> {
		let mut value = read;
		let exit = loop {
			let parent = self.parent(value)?;
			match self.view(parent) {
				View::Group(inner) if inner == value => value = parent,
				View::Access { target, members }
					if target == value
						&& members.iter().all(|member| {
							matches!(self.a.resolution(member), Resolution::Field(..))
						}) =>
				{
					value = parent
				}
				View::Return(Some(inner)) if inner == value => break parent,
				_ => return None,
			}
		};
		let mut parent = self.parent(exit);
		while let Some(node) = parent {
			if matches!(self.view(node), View::Lambda { .. }) {
				if !self
					.declaration_depths
					.get(&symbol)
					.is_some_and(|&(_, function)| function == node.0)
					|| self
						.aliases
						.values()
						.any(|&place| self.place_symbol(place) == Some(symbol))
					|| self
						.a
						.functions
						.values()
						.any(|info| info.captures.contains(&symbol))
				{
					return None;
				}
				return Some(exit);
			}
			parent = self.parent(node);
		}
		None
	}

	/// An owned local can be transferred into the data value replacing it,
	/// even when the assignment runs on every loop iteration. Later reads
	/// see the replacement. Keep the old slot intact if building the RHS can
	/// fail or run code: a handler or callback could still observe that slot.
	fn source_is_replaced_after(&self, symbol: SymbolId, read: NodeId, value: NodeId) -> bool {
		if self.strip_groups(value) != read || self.pinned_captures.contains(&symbol) {
			return false;
		}
		let ty = self.a.type_of(value);
		if self.s.types.contains_shared_handle(ty)
			|| self
				.s
				.types
				.flags(ty)
				.contains(crate::types::TypeFlags::HAS_PARAM)
			|| self.s.types.is_move_only(ty)
		{
			return false;
		}
		let mut node = read;
		let (assignment, rhs) = loop {
			let Some(parent) = self.parent(node) else {
				return false;
			};
			match self.view(parent) {
				View::Lambda { .. } | View::Function { .. } | View::Start(_) | View::Watch(_) => {
					return false;
				}
				View::Assignment {
					target,
					value: Some(rhs),
					..
				} if self.strip_groups(rhs) == self.strip_groups(node)
					&& matches!(self.view(self.strip_groups(target)), View::Name(_))
					&& self.a.resolution(self.strip_groups(target))
						== Resolution::Local(symbol) =>
				{
					break (parent, rhs);
				}
				_ => node = parent,
			}
		};
		let mut owner = Some(assignment);
		let mut function = u32::MAX;
		while let Some(node) = owner {
			if matches!(self.view(node), View::Lambda { .. }) {
				function = node.0;
				break;
			}
			owner = self.parent(node);
		}
		if !self
			.declaration_depths
			.get(&symbol)
			.is_some_and(|depth| depth.1 == function)
		{
			return false;
		}
		let start = self.tree.spans[read.index()].start;
		let end = self.tree.spans[assignment.index()].end;
		if self.move_uses.get(&symbol).is_some_and(|uses| {
			uses.iter().any(|use_| {
				use_.node != read
					&& self.tree.spans[use_.node.index()].start > start
					&& self.tree.spans[use_.node.index()].start < end
					&& !self.exclusive_reads(read, use_.node)
			})
		}) {
			return false;
		}
		self.replacement_is_data(rhs)
	}

	fn replacement_is_data(&self, rhs: NodeId) -> bool {
		let mut pending = vec![rhs];
		while let Some(node) = pending.pop() {
			if self.a.has_flags(node, NodeFlags::TYPE_EXPR) {
				continue;
			}
			if self
				.a
				.flags_of(node)
				.intersects(NodeFlags::CAN_FAIL | NodeFlags::SINK_CHECK)
			{
				return false;
			}
			match self.view(node) {
				View::Name(_)
				| View::Integer(_)
				| View::Boolean(_)
				| View::None
				| View::String(_)
				| View::Group(_)
				| View::Access { .. }
				| View::ImplicitMember(_)
				| View::Object { .. }
				| View::TypedObject { .. }
				| View::Tuple { .. }
				| View::Array { .. } => {}
				View::Assignment { .. }
					if self.parent(node).is_some_and(|parent| {
						matches!(
							self.view(parent),
							View::Object { .. } | View::TypedObject { .. }
						)
					}) => {}
				_ => return false,
			}
			pending.extend(crate::syntax::children(&self.tree, node));
		}
		true
	}

	pub(super) fn exclusive_reads(&self, first: NodeId, second: NodeId) -> bool {
		let chain = |mut node| {
			let mut nodes = vec![node];
			while let Some(parent) = self.parent(node) {
				nodes.push(parent);
				node = parent;
			}
			nodes
		};
		let first = chain(first);
		let second = chain(second);
		let Some((index, &common)) = first
			.iter()
			.enumerate()
			.find(|(_, node)| second.contains(node))
		else {
			return false;
		};
		let Some(other) = second.iter().position(|&node| node == common) else {
			return false;
		};
		if index == 0 || other == 0 {
			return false;
		}
		let a = first[index - 1];
		let b = second[other - 1];
		if a == b {
			return false;
		}
		match self.view(common) {
			View::If {
				then_body,
				else_body: Some(else_body),
				..
			} => [then_body, else_body].contains(&a) && [then_body, else_body].contains(&b),
			View::When { arms, .. } => {
				arms.iter().any(|arm| arm.value == a) && arms.iter().any(|arm| arm.value == b)
			}
			_ => false,
		}
	}

	/// Whether copying a value of this type copies memory (a collection, a
	/// string, an object or a union holding one): a move saves that.
	pub(super) fn owns_memory(&self, ty: TypeId) -> bool {
		match self.s.types.kind(ty) {
			TypeKind::Collection(..) | TypeKind::String | TypeKind::Tuple(_) => true,
			// an object: a JavaScript object, or fields that may own memory
			TypeKind::Object(_) => true,
			TypeKind::Union(list) => self
				.s
				.types
				.list_items(list)
				.iter()
				.any(|&member| self.owns_memory(member)),
			TypeKind::NamedUnion(..) | TypeKind::Member(..) => true,
			_ => false,
		}
	}
}

#[cfg(test)]
mod tests {
	use crate::analysis::NodeFlags;
	use crate::ids::NodeId;
	use crate::syntax::View;

	fn moved_tails(source: &str) -> Vec<bool> {
		let (session, file) = super::super::test_support::analyzed(source);
		let tree = session.tree(file);
		let analysis = session.analysis(file);
		(0..tree.nodes.len())
			.filter_map(|index| {
				let node = NodeId::new(index);
				let View::Assignment {
					target,
					value: Some(value),
					..
				} = crate::syntax::view(tree, node)
				else {
					return None;
				};
				(tree.text(target.index()) == "next" && tree.text(value.index()) == "head")
					.then(|| analysis.has_flags(value, NodeFlags::MOVE))
			})
			.collect()
	}

	#[test]
	fn a_loop_can_transfer_the_previous_value_into_its_replacement() {
		let source = "type Link = { value: Integer, next: Link? }\nfunction build = (count: Integer): Link? => { mutable head: Link? = none\nfor index in 0..count { head = Link { value = index, next = head } }\nhead }\n";
		assert_eq!(moved_tails(source), [true]);
	}

	#[test]
	fn passive_returns_move_on_each_exit_including_inside_a_loop() {
		for source in [
			"function build = (early: Boolean): Array<Integer> => { mutable items = [1, 2]\nif early { return (items) }\nitems.push(3)\nitems }\n",
			"function build = (early: Boolean): String => { mutable text = \"owning string\"\nfor 0..3 { if early { return text }\ntext += \"!\" }\ntext }\n",
		] {
			let (session, file) = super::super::test_support::analyzed(source);
			let tree = session.tree(file);
			let analysis = session.analysis(file);
			let mut returned = 0;
			for index in 0..tree.nodes.len() {
				if let View::Return(Some(mut value)) = crate::syntax::view(tree, NodeId::new(index))
				{
					while let View::Group(inner) = crate::syntax::view(tree, value) {
						value = inner;
					}
					assert!(analysis.has_flags(value, NodeFlags::MOVE), "{source}");
					returned += 1;
				}
			}
			assert_eq!(returned, 1);
		}
	}

	#[test]
	fn returned_assemblies_keep_owners_needed_by_a_handler() {
		let source = "function build = (early: Boolean, text: String): (Array<Integer>, Integer) => { mutable items = [1, 2]\nlet result = do { if early { return (items, Integer.parse(text)) }\n([], 0) } catch _error => { console.log(items)\n([], 0) }\nresult }\n";
		let (session, file) = crate::analyze_source(source);
		assert!(
			session
				.diagnostics(file)
				.iter()
				.any(|diagnostic| diagnostic.code.id == "MEM004")
		);
		let tree = session.tree(file);
		let analysis = session.analysis(file);
		for index in 0..tree.nodes.len() {
			let node = NodeId::new(index);
			if tree.text(index) == "items"
				&& matches!(crate::syntax::view(tree, node), View::Name(_))
			{
				assert!(!analysis.has_flags(node, NodeFlags::MOVE), "{source}");
			}
		}
	}

	#[test]
	fn replacement_moves_preserve_later_reads_and_failure_paths() {
		let later_read = "type Link = { next: Link?, previous: Link? }\nfunction build = (count: Integer): Link? => { mutable head: Link? = none\nfor index in 0..count { head = Link { next = head, previous = head } }\nhead }\n";
		assert_eq!(moved_tails(later_read), [false]);
		let fails = "type Link = { next: Link?, value: Integer }\nfunction build = (count: Integer, text: String): Link? throws ParseError => { mutable head: Link? = none\nfor index in 0..count { head = Link { next = head, value = try Integer.parse(text) } }\nhead }\n";
		assert_eq!(moved_tails(fails), [false]);
	}
}
