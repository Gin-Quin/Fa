//! Header snapshots for counted loops over stable GrowingArray owners.
//!
//! A private owner's complete use set cannot give its header to another
//! function or closure. Within a loop we accept only indexing, length and
//! direct pushes. Without pushes all lookup facts are invariant; with pushes
//! only the mutable cursor/end/length are cached, with write-through updates
//! so errors and length observations keep their original behavior. A cold
//! segment advance refreshes cursor/end and leaves length unchanged.

use fa_analyzer::ids::{NodeId, SymbolId};
use fa_analyzer::symbols::{Resolution, SymbolKind};
use fa_analyzer::syntax::View;
use inkwell::values::{IntValue, PointerValue};
use rustc_hash::FxHashSet;

use super::values::Repr;
use super::{Lowering, Slot};
use crate::error::Error;

#[derive(Clone, Copy)]
pub(super) enum GrowingLoop<'ctx> {
	Fixed {
		length: IntValue<'ctx>,
		k: IntValue<'ctx>,
		directory: PointerValue<'ctx>,
		first: PointerValue<'ctx>,
		contiguous: IntValue<'ctx>,
	},
	Push {
		length: Slot<'ctx>,
		cursor: PointerValue<'ctx>,
		end: PointerValue<'ctx>,
	},
}

impl<'ctx> Lowering<'_, 'ctx> {
	pub(super) fn snapshot_growing(
		&self,
		array: PointerValue<'ctx>,
	) -> Result<GrowingLoop<'ctx>, Error> {
		Ok(GrowingLoop::Fixed {
			length: self.growing_length(&self.builder, array)?,
			k: self.growing_k(&self.builder, array)?,
			directory: self.growing_directory(&self.builder, array)?,
			first: self.growing_first_items(&self.builder, array)?,
			contiguous: self.growing_single_segment(array)?,
		})
	}
	/// The root's direct operation. Method values are rejected: only a
	/// direct builtin call can have a known effect on this header.
	fn growing_use(&self, read: NodeId) -> Option<&'static str> {
		let parents = self.parents();
		let mut at = read;
		loop {
			let parent = *parents.get(at.index())?;
			if parent == usize::MAX {
				return None;
			}
			match self.view(NodeId::new(parent)) {
				View::Group(_) => at = NodeId::new(parent),
				View::Index { target, .. } if target == at => return Some("index"),
				View::For { iterable, .. } if iterable == at => return Some("iterate"),
				View::Assignment { target, .. } if target == at => return Some("replace"),
				View::Access { target, members } if target == at && members.len() == 1 => {
					let Resolution::Builtin(id) = self.analysis.resolution(members.first()?) else {
						return None;
					};
					let path = fa_analyzer::builtins::get(id).path;
					if path == "Array#length" {
						return Some("length");
					}
					let call = *parents.get(parent)?;
					if call == usize::MAX
						|| !matches!(self.view(NodeId::new(call)), View::Call { callee, .. } if callee.index() == parent)
					{
						return None;
					}
					return match path {
						"Array#push" => Some("push"),
						"Array#reserve" | "Collection#reserve" => Some("reserve"),
						_ => None,
					};
				}
				_ => return None,
			}
		}
	}

	/// No other function, closure, reference or exported file can reach
	/// this owner. Element references do not expose the collection header.
	fn private_growing_loop_owner(&self, symbol: SymbolId) -> bool {
		let binding = self.analysis.symbols.symbol(symbol);
		if !matches!(binding.kind, SymbolKind::Let | SymbolKind::Mutable)
			|| !self.other_files.is_empty()
			|| !self.analysis.foreign_args.is_empty()
			|| self.references.contains_key(&symbol)
			|| self.shared.contains_key(&symbol)
			|| self.reactive_sources.contains_key(&symbol)
		{
			return false;
		}
		let parents = self.parents();
		let home = self.body_of(binding.decl_node.index(), &parents);
		let mut at = binding.decl_node.index();
		while let Some(&parent) = parents.get(at) {
			if parent == usize::MAX {
				break;
			}
			match self.view(NodeId::new(parent)) {
				View::ExportValue { .. } => return false,
				View::Block { .. } | View::Module { .. } => break,
				_ => at = parent,
			}
		}
		self.uses_of(symbol).into_iter().all(|read| {
			read == binding.decl_node
				|| (self.body_of(read.index(), &parents) == home
					&& self.growing_use(read).is_some())
		})
	}

	pub(super) fn hoist_growing(&mut self, body: NodeId) -> Result<Vec<SymbolId>, Error> {
		let mut hoisted = Vec::new();
		if self.tasks.asynchronous {
			return Ok(hoisted);
		}
		let mut reads = Vec::new();
		let mut stack = vec![body.index()];
		while let Some(at) = stack.pop() {
			let node = NodeId::new(at);
			if matches!(self.view(node), View::Name(_)) {
				reads.push(node);
			}
			self.tree.nodes[at].for_each_child(|child| stack.push(child));
		}
		let symbols: FxHashSet<_> = reads
			.iter()
			.filter_map(|&read| match self.analysis.resolution(read) {
				Resolution::Local(symbol) => Some(symbol),
				_ => None,
			})
			.collect();
		for symbol in symbols {
			let Some(&slot) = self.slots.get(&symbol) else {
				continue;
			};
			if !matches!(slot.repr, Repr::Growing(_))
				|| self.hoisted_arrays.contains_key(&symbol)
				|| !self.private_growing_loop_owner(symbol)
			{
				continue;
			}
			let mut pushes = false;
			let safe = reads
				.iter()
				.filter(|&&read| self.analysis.resolution(read) == Resolution::Local(symbol))
				.all(|&read| match self.growing_use(read) {
					Some("index" | "length" | "iterate") => true,
					Some("push") => {
						pushes = true;
						true
					}
					_ => false,
				});
			if !safe {
				continue;
			}
			let array = self
				.load(slot)?
				.value
				.expect("a growing array")
				.into_pointer_value();
			let facts = if pushes {
				let length = self.growing_length(&self.builder, array)?;
				let cached_length =
					self.alloca(Repr::Int(super::values::IntRepr::I64), "push_length")?;
				self.builder.build_store(cached_length.pointer, length)?;
				let cursor = self.pointer_alloca("push_cursor")?;
				let end = self.pointer_alloca("push_end")?;
				for (field, cache) in [(3, cursor), (4, end)] {
					let (_, value) = self.growing_field(array, field, "push_initial")?;
					self.builder.build_store(cache, value)?;
				}
				GrowingLoop::Push {
					length: cached_length,
					cursor,
					end,
				}
			} else {
				self.snapshot_growing(array)?
			};
			self.hoisted_arrays.insert(symbol, array);
			self.growing_loops.insert(array, facts);
			hoisted.push(symbol);
		}
		Ok(hoisted)
	}

	pub(super) fn unhoist_growing(&mut self, symbols: Vec<SymbolId>) {
		for symbol in symbols {
			if let Some(array) = self.hoisted_arrays.remove(&symbol) {
				self.growing_loops.remove(&array);
			}
		}
	}
}
