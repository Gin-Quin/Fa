//! Non-escaping strings use a buffer belonging to their function frame.
//! The ordinary three-word header carries a non-owning, writable tag. Every
//! destination has its own buffer: moving an owning header must never move a
//! frame pointer to a different local, including another loop iteration.

use fa_analyzer::analysis::Callee;
use fa_analyzer::ids::{NodeId, SymbolId};
use fa_analyzer::symbols::{Resolution, SymbolKind};
use fa_analyzer::syntax::{self, TemplatePart, View};
use inkwell::values::PointerValue;
use rustc_hash::FxHashSet;

use crate::error::Error;
use crate::lower::values::{Repr, Val};
use crate::lower::{Lowering, Slot};

#[derive(Copy, Clone)]
pub(super) struct FrameString<'ctx> {
	pub(super) buffer: PointerValue<'ctx>,
	pub(super) slot: PointerValue<'ctx>,
	pub(super) mutable: bool,
}

impl<'ctx> Lowering<'_, 'ctx> {
	/// A syntax literal has permanent bytes. Do not use constant propagation
	/// here: a name with known contents may still own a reusable frame buffer.
	fn literal_string_length(&self, node: NodeId) -> Option<usize> {
		match self.view(node) {
			View::String(text) => Some(text.len()),
			View::Group(inner) => self.literal_string_length(inner),
			View::Template { parts } => parts.iter().try_fold(0usize, |size, part| match part {
				TemplatePart::Text(text) => size.checked_add(text.len()),
				TemplatePart::Expression(_) => None,
			}),
			_ => None,
		}
	}

	/// This placement bound is deliberately narrow. Every written value must
	/// be a small literal, and an alias or an unknown mutator invalidates it.
	/// Its result never removes a runtime length or capacity check.
	fn find_small_mutable_strings(&self) -> FxHashSet<SymbolId> {
		let inline = (self.usize.get_bit_width() / 8 * 3 - 1) as usize;
		let mut small = FxHashSet::default();
		let mut parents = vec![None; self.tree.nodes.len()];
		for index in 0..self.tree.nodes.len() {
			let node = NodeId::new(index);
			for child in syntax::children(self.tree, node) {
				parents[child.index()] = Some(node);
			}
			if let View::Declaration {
				value: Some(value), ..
			} = self.view(node)
				&& let Some(symbol) = self.analysis.symbols.symbol_of_decl(node)
				&& self.analysis.symbols.symbol(symbol).kind == SymbolKind::Mutable
				&& self
					.literal_string_length(value)
					.is_some_and(|size| size <= inline)
			{
				small.insert(symbol);
			}
		}
		for index in 0..self.tree.nodes.len() {
			let node = NodeId::new(index);
			let Resolution::Local(symbol) = self.analysis.resolution(node) else {
				continue;
			};
			if !small.contains(&symbol) || !matches!(self.view(node), View::Name(_)) {
				continue;
			}
			let mut child = node;
			while let Some(parent) = parents[child.index()] {
				let safe = match self.view(parent) {
					View::Group(_) => {
						child = parent;
						continue;
					}
					View::Assignment { target, value, .. } if target == child => value
						.and_then(|value| self.literal_string_length(value))
						.is_some_and(|size| size <= inline),
					View::CompoundAssignment { target, .. } if target == child => false,
					View::Use { .. } => false,
					View::Access { .. } | View::OptionalAccess { .. } => {
						child = parent;
						continue;
					}
					View::Call { .. } => {
						match self.analysis.calls.get(&parent).map(|info| info.callee) {
							Some(Callee::Builtin(id)) => {
								!fa_analyzer::builtins::effects::changes_receiver(
									fa_analyzer::builtins::get(id).path,
								)
							}
							Some(Callee::Conversion(_)) => true,
							_ => false,
						}
					}
					_ => true,
				};
				if !safe {
					small.remove(&symbol);
				}
				break;
			}
		}
		small
	}

	pub(super) fn frame_string_declaration(
		&mut self,
		node: NodeId,
		symbol: SymbolId,
		value_node: NodeId,
	) -> Result<Option<Val<'ctx>>, Error> {
		let kind = self.analysis.symbols.symbol(symbol).kind;
		if !matches!(kind, SymbolKind::Let | SymbolKind::Mutable)
			|| !self.analysis.is_non_escaping(symbol)
			|| self.shared.contains_key(&symbol)
			|| self.repr_of(self.analysis.symbols.symbol(symbol).ty, node.index())? != Repr::Str
		{
			return Ok(None);
		}
		if kind == SymbolKind::Let && self.literal_string_length(value_node).is_some() {
			return Ok(None);
		}
		if kind == SymbolKind::Mutable {
			if !self.small_mutable_strings.contains_key(&self.file) {
				let small = self.find_small_mutable_strings();
				self.small_mutable_strings.insert(self.file, small);
			}
			if self.small_mutable_strings[&self.file].contains(&symbol) {
				return Ok(None);
			}
		}
		let slot = self.slot_of(symbol, node)?;
		let buffer =
			self.alloca_type(self.context.i8_type().array_type(64).into(), "string_frame")?;
		let local = FrameString {
			buffer,
			slot: slot.pointer,
			mutable: kind == SymbolKind::Mutable,
		};
		let key = (self.file, symbol);
		self.frame_mut().frame_strings.insert(key, local);
		// A declaration in a loop reuses its alloca after the preceding value
		// was dropped. Its old header may contain a freed heap pointer.
		self.builder
			.build_store(slot.pointer, self.llvm_type(Repr::Str).const_zero())?;
		if let View::Template { parts } = self.view(value_node)
			&& let Some(value) = self.string_template(parts, Some(slot), Some(local))?
		{
			if value.repr == Repr::Never {
				return Ok(Some(value));
			}
			self.own_local(symbol, slot);
			return Ok(Some(Val::void()));
		}
		let value = self.expr(value_node)?;
		if value.repr == Repr::Never {
			return Ok(Some(value));
		}
		self.assign_frame_string(local, slot, value, value_node)?;
		self.own_local(symbol, slot);
		Ok(Some(Val::void()))
	}

	pub(super) fn frame_string_assignment(
		&mut self,
		target: NodeId,
		value_node: NodeId,
	) -> Result<Option<Val<'ctx>>, Error> {
		let Resolution::Local(symbol) = self.analysis.resolution(target) else {
			return Ok(None);
		};
		let Some(local) = self
			.frame()
			.frame_strings
			.get(&(self.file, symbol))
			.copied()
		else {
			return Ok(None);
		};
		let slot = self.place(target)?;
		if slot.pointer != local.slot || slot.repr != Repr::Str {
			return Ok(None);
		}
		// Clearing a string retains its previous explicit reservation.
		if self
			.constant_text(value_node)
			.is_some_and(|text| text.is_empty())
		{
			return Ok(None);
		}
		let value = self.expr(value_node)?;
		if value.repr == Repr::Never {
			return Ok(Some(value));
		}
		self.assign_frame_string(local, slot, value, value_node)?;
		Ok(Some(Val::void()))
	}

	fn assign_frame_string(
		&mut self,
		local: FrameString<'ctx>,
		slot: Slot<'ctx>,
		value: Val<'ctx>,
		node: NodeId,
	) -> Result<(), Error> {
		// A deferred compile-time branch may still have no runtime value.
		// Keep the ordinary conversion's Unsupported diagnostic instead of
		// trying to snapshot an erased initializer as a String header.
		let value = self.convert(value, Repr::Str, node.index())?;
		let consume = self.unregister(value) || self.take_local(value, node)?.is_some();
		// Snapshot the SSA header after a move cleared its original slot.
		// Never pass a cached address of that cleared slot to the runtime.
		let source = self.string_snapshot(value)?;
		let int = self.context.i32_type();
		self.call_runtime(
			"fa_string_assign_local",
			&[
				source.pointer.into(),
				local.buffer.into(),
				slot.pointer.into(),
				int.const_int(local.mutable as u64, false).into(),
				int.const_int(consume as u64, false).into(),
			],
		)?;
		Ok(())
	}

	pub(super) fn string_snapshot(&self, value: Val<'ctx>) -> Result<Slot<'ctx>, Error> {
		let slot = self.alloca(Repr::Str, "string_snapshot")?;
		self.builder
			.build_store(slot.pointer, value.value.expect("a string"))?;
		Ok(slot)
	}

	/// The ordinary ownership path must materialize a frame value before its
	/// header can be stored in a container, returned or moved to another local.
	pub(super) fn escape_frame_string(&self, value: Val<'ctx>) -> Result<Val<'ctx>, Error> {
		if value.repr != Repr::Str || self.frame().frame_strings.is_empty() {
			return Ok(value);
		}
		let source = self.string_snapshot(value)?;
		let out = self.alloca(Repr::Str, "escaped_string")?;
		self.call_runtime(
			"fa_string_escape",
			&[source.pointer.into(), out.pointer.into()],
		)?;
		self.load(out)
	}
}
