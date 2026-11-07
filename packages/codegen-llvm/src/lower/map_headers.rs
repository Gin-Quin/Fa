//! A private inline map may keep its header in the current function frame.
//! The entry storage stays ordinary and is released by the normal map drop.

use fa_analyzer::analysis::{Argument, Callee};
use fa_analyzer::ids::{FileId, NodeId, SymbolId};
use fa_analyzer::symbols::{Resolution, SymbolKind};
use fa_analyzer::syntax::View;
use inkwell::values::{FunctionValue, IntValue, PointerValue};
use rustc_hash::FxHashMap;

use super::Lowering;
use super::tables::Table;
use super::values::Repr;
use crate::error::Error;

pub(crate) type Headers<'ctx> =
	FxHashMap<(FileId, NodeId, FunctionValue<'ctx>), PointerValue<'ctx>>;

impl<'ctx> Lowering<'_, 'ctx> {
	fn private_map_header_uses(&self, declaration: NodeId, symbol: SymbolId) -> bool {
		let parents = self.parents();
		let home = self.body_of(declaration.index(), &parents);
		let mut ancestor = declaration.index();
		while let Some(&parent) = parents.get(ancestor) {
			if parent == usize::MAX {
				break;
			}
			match self.view(NodeId::new(parent)) {
				View::ExportValue { .. } => return false,
				View::Block { .. } | View::Module { .. } => break,
				_ => ancestor = parent,
			}
		}
		for read in self.uses_of(symbol) {
			if read == declaration {
				continue;
			}
			if self.body_of(read.index(), &parents) != home {
				return false;
			}
			let mut current = read;
			loop {
				let Some(&parent) = parents.get(current.index()) else {
					return false;
				};
				if parent == usize::MAX {
					return false;
				}
				match self.view(NodeId::new(parent)) {
					View::Group(_) => current = NodeId::new(parent),
					View::Index { target, .. } if target == current => break,
					View::Assignment { target, .. } if target == current => break,
					View::For { iterable, .. } if iterable == current => break,
					View::Access { target, members } if target == current && members.len() == 1 => {
						let Some(member) = members.first() else {
							return false;
						};
						let Resolution::Builtin(id) = self.analysis.resolution(member) else {
							return false;
						};
						let path = fa_analyzer::builtins::get(id).path;
						if matches!(path, "Map#size" | "Map#capacity" | "Collection#capacity") {
							break;
						}
						if !matches!(
							path,
							"Map#get"
								| "Map#includes" | "Map#getOrInsert"
								| "Map#delete" | "Map#take"
								| "Map#clear" | "Map#reserve"
								| "Collection#reserve"
						) {
							return false;
						}
						let mut access = parent;
						loop {
							let Some(&above) = parents.get(access) else {
								return false;
							};
							if above == usize::MAX {
								return false;
							}
							match self.view(NodeId::new(above)) {
								View::Group(_) => access = above,
								View::Call { callee, .. } if callee.index() == access => break,
								_ => return false,
							}
						}
						break;
					}
					_ => return false,
				}
			}
		}
		true
	}

	pub(super) fn prepare_local_map_header(
		&mut self,
		declaration: NodeId,
		symbol: SymbolId,
		mut value: NodeId,
	) -> Result<(), Error> {
		let Some(owner) = self.frame.as_ref().map(|frame| frame.function) else {
			return Ok(());
		};
		if self
			.builder
			.get_insert_block()
			.and_then(|block| block.get_parent())
			!= Some(owner)
		{
			return Ok(());
		}
		if !self.map_probe_program_is_closed()
			|| !self.bindings.borrow().is_empty()
			|| self.shared.contains_key(&symbol)
			|| self.referenced_locals().contains(&symbol)
			|| !matches!(
				self.analysis.symbols.symbol(symbol).kind,
				SymbolKind::Let | SymbolKind::Mutable
			) || !self.private_map_header_uses(declaration, symbol)
		{
			return Ok(());
		}
		let repr = self.repr_of(self.analysis.symbols.symbol(symbol).ty, declaration.index())?;
		if !matches!(repr, Repr::Map(_)) {
			return Ok(());
		}
		let table = self.table_of(repr)?;
		if !table.inline || !matches!(table.key, Repr::Int(int) if int.bits == 32) {
			return Ok(());
		}
		while let View::Group(inner) = self.view(value) {
			value = inner;
		}
		let fresh = match self.view(value) {
			View::Array { .. } => true,
			View::Call { .. } => self.analysis.call(value).is_some_and(|info| {
				matches!(info.callee, Callee::Builtin(id) if fa_analyzer::builtins::get(id).path == "Map.from")
					&& matches!(info.arguments.first(), Some(Argument::Given(source)) if matches!(self.view(*source), View::Array { .. }))
			}),
			_ => false,
		};
		if !fresh {
			return Ok(());
		}
		let Some(header_type) = self.context.get_struct_type("struct.FaTable") else {
			return Ok(());
		};
		let pointer = self.alloca_type(header_type.into(), "local_map_header")?;
		self.local_map_headers
			.insert((self.file, value, self.frame().function), pointer);
		Ok(())
	}

	pub(super) fn table_new_at(
		&mut self,
		node: NodeId,
		table: Table<'ctx>,
		capacity: IntValue<'ctx>,
	) -> Result<PointerValue<'ctx>, Error> {
		let Some(owner) = self.frame.as_ref().map(|frame| frame.function) else {
			return self.table_new(table, capacity);
		};
		if self
			.builder
			.get_insert_block()
			.and_then(|block| block.get_parent())
			!= Some(owner)
		{
			return self.table_new(table, capacity);
		}
		if table.inline
			&& matches!(table.key, Repr::Int(int) if int.bits == 32)
			&& let Some(&header) = self.local_map_headers.get(&(self.file, node, owner))
		{
			let pointer = self.call_value(
				self.runtime.function("fa_table_new_local")?,
				&[
					header.into(),
					capacity.into(),
					self.entry_size(table).into(),
				],
			)?;
			return Ok(pointer.into_pointer_value());
		}
		self.table_new(table, capacity)
	}
}
