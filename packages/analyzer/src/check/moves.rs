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
use crate::symbols::SymbolKind;
use crate::types::TypeKind;

mod concrete;
mod copies;
mod graph;
mod validity;

/// The last use of a local seen so far.
#[derive(Copy, Clone, Debug)]
pub(crate) struct LastUse {
	/// The read.
	pub node: NodeId,
	/// At the loop and function depth of the declaration.
	pub movable: bool,
}

impl Walker<'_> {
	pub(crate) fn record_move_read(&mut self, node: NodeId, symbol: SymbolId) {
		let reads = self.move_reads.entry(node).or_default();
		if !reads.contains(&symbol) {
			reads.push(symbol);
		}
	}

	/// Records the loop and function depth of a local's declaration.
	pub(crate) fn note_declaration(&mut self, symbol: SymbolId) {
		let depth = (
			self.body.loops.len() as u32,
			self.body.functions.len() as u32,
		);
		self.declaration_depths.insert(symbol, depth);
		self.last_uses.remove(&symbol);
	}

	/// Records a use of a local (a read or a write of its name, or through
	/// an alias of a place below it).
	pub(crate) fn note_use(&mut self, n: NodeId, symbol: SymbolId) {
		if n.0 != u32::MAX {
			self.record_move_read(n, symbol);
		}
		let Some(&(loops, functions)) = self.declaration_depths.get(&symbol) else {
			return;
		};
		let movable =
			loops == self.body.loops.len() as u32 && functions == self.body.functions.len() as u32;
		self.last_uses.insert(symbol, LastUse { node: n, movable });
	}

	/// A value node in a copy position: the value of a `let`/`mutable`, of
	/// an assignment, of a `return`, an item stored by `push`/`add`.
	pub(crate) fn note_copy_site(&mut self, value: NodeId) {
		let value = self.strip_groups(value);
		if self.is_relation_link_store(value) {
			return;
		}
		// a resource (a type with `destroy`) is moved (foreign.rs)
		self.note_resource_move(value);
		self.record_copy_sources(value);
	}

	/// A conditional stores only its selected value; a block stores its
	/// final expression. Record those actual reads, not the branch syntax.
	fn record_copy_sources(&mut self, node: NodeId) {
		let mut pending = vec![node];
		while let Some(node) = pending.pop() {
			let node = self.strip_groups(node);
			if let Some(inputs) = crate::lower::copies::result_inputs(&self.tree, node) {
				pending.extend(inputs);
			} else {
				self.copy_sites.insert(node);
			}
		}
	}

	/// The use of a name node: a local itself, or the place an alias designates.
	pub(crate) fn note_name_use(&mut self, n: NodeId, symbol: SymbolId) {
		let kind = self.a.symbols.symbol(symbol).kind;
		let places = if matches!(
			kind,
			SymbolKind::Use | SymbolKind::MutableUse | SymbolKind::LoopVariable { copy: false }
		) {
			let mut places = self
				.body
				.flow
				.ref_state(symbol)
				.map(|state| self.facts.refs.get(state).places.to_vec())
				.unwrap_or_default();
			places.extend(self.aliases.get(&symbol).copied());
			places
		} else {
			vec![self.places.symbol(symbol)]
		};
		self.move_places.insert(n, places);
		match kind {
			SymbolKind::Let | SymbolKind::Mutable => self.note_use(n, symbol),
			SymbolKind::Parameter { .. }
			| SymbolKind::SelfParameter
			| SymbolKind::CatchBinding
			| SymbolKind::PatternBinding
			| SymbolKind::Reactive
			| SymbolKind::Derived
			| SymbolKind::Requirement { .. } => self.record_move_read(n, symbol),
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
						self.record_move_read(n, root);
						self.note_use(NodeId(u32::MAX), root);
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
						if value {
							self.record_move_read(n, capture);
						}
						self.note_use(NodeId(u32::MAX), capture);
						if value {
							self.pinned_captures.insert(capture);
						}
					}
				}
			}
			_ => {}
		}
	}

	/// At the end of the walk: the `MOVE` flag on each last use that is a copy.
	pub(crate) fn finish_moves(&mut self) {
		let conflicts = self.finish_concrete_moves();
		self.finish_copy_checks(&conflicts);
		let uses: Vec<(SymbolId, LastUse)> = self
			.last_uses
			.iter()
			.map(|(&symbol, &last)| (symbol, last))
			.collect();
		for (symbol, last) in uses {
			if !last.movable
				|| last.node.0 == u32::MAX
				|| !self.copy_sites.contains(&last.node)
				|| self.pinned_captures.contains(&symbol)
			{
				continue;
			}
			let ty = self.a.symbols.symbol(symbol).ty;
			if self.s.types.is_ordinary(ty) && self.owns_memory(ty) {
				self.set_flag(last.node, NodeFlags::MOVE);
			}
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
