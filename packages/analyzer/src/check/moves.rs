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
			) || !self.source_is_last_use(symbol, read, true)
				|| !self.source_is_owned(value)
			{
				continue;
			}
			let ty = self.a.type_of(value);
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

	/// A resource's fields belong to its shared cell, never to this handle.
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
		if movable && !own.movable {
			return false;
		}
		let start = self.tree.spans[read.index()].start;
		!uses.iter().any(|use_| {
			use_.node != read
				&& self.tree.spans[use_.node.index()].start > start
				&& !self.exclusive_reads(read, use_.node)
		})
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
	fn owns_memory(&self, ty: TypeId) -> bool {
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
