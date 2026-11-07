//! Reuse negative inline-map probes until an index mutation invalidates them.
//! Private function slots carry path-sensitive validity; no map header changes.

use std::cell::{OnceCell, RefCell};

use fa_analyzer::analysis::{Argument, CallInfo, Callee};
use fa_analyzer::ids::{FileId, NodeId, SymbolId, TypeId};
use fa_analyzer::symbols::{Resolution, SymbolKind};
use fa_analyzer::syntax::{BindingKind, View};
use fa_analyzer::types::{CollKind, PathRoot, TypeKind};
use inkwell::IntPredicate;
use inkwell::values::{FunctionValue, InstructionOpcode, IntValue, PointerValue};
use rustc_hash::{FxHashMap, FxHashSet};

use super::Lowering;
use super::tables::Table;
use super::values::{Repr, Val};
use crate::error::Error;

#[derive(Clone, Copy)]
struct Probe<'ctx> {
	valid: PointerValue<'ctx>,
	table: PointerValue<'ctx>,
	key: PointerValue<'ctx>,
	hash: PointerValue<'ctx>,
	position: PointerValue<'ctx>,
}

#[derive(Default)]
pub(crate) struct Probes<'ctx> {
	closed: OnceCell<bool>,
	entries: RefCell<FxHashMap<FunctionValue<'ctx>, Probe<'ctx>>>,
	identities: RefCell<FxHashMap<FunctionValue<'ctx>, (FileId, SymbolId, SymbolId, NodeId)>>,
}

impl<'ctx> Lowering<'_, 'ctx> {
	pub(super) fn map_probe_inline_delete_supported(&self, table: Table<'ctx>) -> bool {
		table.inline
			&& matches!(table.key, Repr::Int(int) if int.bits == 32)
			&& self.map_probe_program_is_closed()
	}

	pub(super) fn map_probe_program_is_closed(&self) -> bool {
		*self.map_probes.closed.get_or_init(|| {
			if !self.whole_program
				|| self.tasks.asynchronous
				|| self.c_library.is_some()
				|| !self.other_files.is_empty()
				|| !self.analysis.externs.is_empty()
				|| !self.analysis.extern_calls.is_empty()
				|| !self.analysis.parallel_operations.is_empty()
				|| self
					.analysis
					.functions
					.values()
					.any(|info| !info.generics.is_empty())
			{
				return false;
			}
			let parents = self.parents();
			for index in 0..self.tree.nodes.len() {
				let node = NodeId::new(index);
				match self.view(node) {
					View::TypeDecl { parameters, .. } | View::ExportType { parameters, .. }
						if !parameters.is_empty() =>
					{
						return false;
					}
					View::Lambda { signature, .. } => {
						if signature.is_async
							|| !signature.generics.is_empty()
							|| !signature.requires.is_empty()
						{
							return false;
						}
						let mut current = index;
						loop {
							let Some(&parent) = parents.get(current) else {
								return false;
							};
							if parent == usize::MAX {
								return false;
							}
							match self.view(NodeId::new(parent)) {
								View::Group(_) => current = parent,
								View::Function { name, .. } => {
									if self.tree.name(name) == "destroy" {
										return false;
									}
									break;
								}
								_ => return false,
							}
						}
					}
					View::For {
						compile_time: true, ..
					}
					| View::Declaration {
						kind: BindingKind::Reactive | BindingKind::Derived,
						..
					}
					| View::Start { .. }
					| View::Watch { .. } => return false,
					_ => {}
				}
				if let Some(call) = self.analysis.call(node) {
					match call.callee {
						Callee::Value => return false,
						Callee::Builtin(id) => {
							let path = fa_analyzer::builtins::get(id).path;
							if [
								"Pointer", "CString", "Mutex", "Channel", "Worker", "Atomic",
								"Task",
							]
							.iter()
							.any(|prefix| path.starts_with(prefix))
							{
								return false;
							}
						}
						_ => {}
					}
					if call.arguments.iter().any(|argument| match argument {
						Argument::Given(value) => matches!(
							self.types().kind(self.type_of(*value)),
							TypeKind::Function(_)
						),
						_ => false,
					}) {
						return false;
					}
				}
			}
			true
		})
	}

	fn map_probe_sources(&self, table: Table<'ctx>, node: NodeId) -> Option<(SymbolId, SymbolId)> {
		if !table.inline
			|| !matches!(table.key, Repr::Int(int) if int.bits == 32)
			|| !self.map_probe_program_is_closed()
			|| !self.bindings.borrow().is_empty()
		{
			return None;
		}
		let (target, key) = match self.view(node) {
			View::Index { target, index } => (target, index),
			View::Call { .. } => {
				let target = self.method_receiver(node)?;
				let Argument::Given(key) = self.analysis.call(node)?.arguments.first()? else {
					return None;
				};
				(target, *key)
			}
			_ => return None,
		};
		let ungroup = |mut node| {
			while let View::Group(inner) = self.view(node) {
				node = inner;
			}
			node
		};
		let target = ungroup(target);
		let key = ungroup(key);
		if !matches!(self.view(target), View::Name(_)) || !matches!(self.view(key), View::Name(_)) {
			return None;
		}
		let (Resolution::Local(map), Resolution::Local(key)) = (
			self.analysis.resolution(target),
			self.analysis.resolution(key),
		) else {
			return None;
		};
		if !matches!(
			self.analysis.symbols.symbol(map).kind,
			SymbolKind::Let | SymbolKind::Mutable
		) || !matches!(
			self.analysis.symbols.symbol(key).kind,
			SymbolKind::Let | SymbolKind::Mutable
		) || self.referenced_locals().contains(&map)
			|| self.shared.contains_key(&map)
		{
			return None;
		}
		let parents = self.parents();
		let declaration = self.analysis.symbols.symbol(map).decl_node;
		let home = self.body_of(declaration.index(), &parents);
		if self
			.uses_of(map)
			.into_iter()
			.any(|read| self.body_of(read.index(), &parents) != home)
		{
			return None;
		}
		let mut current = declaration.index();
		while let Some(&parent) = parents.get(current) {
			if parent == usize::MAX {
				break;
			}
			match self.view(NodeId::new(parent)) {
				View::ExportValue { .. } => return None,
				View::Module { .. } | View::Block { .. } => break,
				_ => current = parent,
			}
		}
		Some((map, key))
	}

	pub(super) fn invalidate_all_map_probes(&self) -> Result<(), Error> {
		if !self.map_probe_program_is_closed() {
			return Ok(());
		}
		let Some(frame) = self.frame.as_ref() else {
			return Ok(());
		};
		if self
			.builder
			.get_insert_block()
			.and_then(|block| block.get_parent())
			!= Some(frame.function)
		{
			return Ok(());
		}
		// Create the shared slot even before a producer is visited: a mutation
		// preceding it in lowering order may follow it through a runtime backedge.
		let entry = self.map_probe_for_frame()?;
		self.builder
			.build_store(entry.valid, self.context.bool_type().const_zero())?;
		Ok(())
	}

	pub(super) fn invalidate_runtime_map_probes(&self, name: &str) -> Result<(), Error> {
		if matches!(
			name,
			"fa_table_append"
				| "fa_table_new_local"
				| "fa_table_remove"
				| "fa_table_release"
				| "fa_table_kill"
				| "fa_table_reserve"
				| "fa_table_clear"
				| "fa_table_free"
		) {
			self.invalidate_all_map_probes()?;
		}
		Ok(())
	}

	fn probe_type_may_hold_map(&self, ty: TypeId, seen: &mut FxHashSet<TypeId>) -> bool {
		let ty = self.concrete_type(ty);
		if !seen.insert(ty) {
			return false;
		}
		match self.types().kind(ty) {
			TypeKind::Collection(
				CollKind::Map | CollKind::GrowingMap | CollKind::Set | CollKind::GrowingSet,
				..,
			) => true,
			TypeKind::Collection(_, item, _) => self.probe_type_may_hold_map(item, seen),
			TypeKind::Object(shape) => self
				.types()
				.shape_fields(shape)
				.iter()
				.any(|field| self.probe_type_may_hold_map(field.ty, seen)),
			TypeKind::Tuple(items) | TypeKind::Union(items) => self
				.types()
				.list_items(items)
				.iter()
				.any(|&item| self.probe_type_may_hold_map(item, seen)),
			TypeKind::Never
			| TypeKind::Void
			| TypeKind::None
			| TypeKind::Boolean
			| TypeKind::BoolLit(_)
			| TypeKind::Int(_)
			| TypeKind::Float(_)
			| TypeKind::FloatLit(..)
			| TypeKind::BigInt
			| TypeKind::BigIntLit(_)
			| TypeKind::String
			| TypeKind::StringLit(_)
			| TypeKind::Character
			| TypeKind::Percentage
			| TypeKind::Enum(..)
			| TypeKind::EnumMember(..)
			| TypeKind::Range => false,
			_ => true,
		}
	}

	pub(super) fn invalidate_named_map_probes(
		&self,
		function: &super::Function<'ctx>,
		node: NodeId,
		info: &CallInfo,
	) -> Result<(), Error> {
		if !self.map_probe_program_is_closed() {
			return Ok(());
		}
		let Some(sig) = function.info.sig else {
			return self.invalidate_all_map_probes();
		};
		let (mutates, invalidates) = self.types().effects(self.types().sig(sig).effects);
		for path in mutates.iter().chain(invalidates) {
			let argument = match self.types().path_data(*path).0.root {
				PathRoot::Param(index) => info.arguments.get(index as usize),
				PathRoot::SelfParam => info.arguments.first(),
				_ => return self.invalidate_all_map_probes(),
			};
			let actual = match argument {
				Some(Argument::Given(value)) => Some(*value),
				Some(Argument::Receiver(_)) => self.method_receiver(node),
				_ => None,
			};
			if actual.is_none_or(|actual| {
				self.probe_type_may_hold_map(self.type_of(actual), &mut FxHashSet::default())
			}) {
				return self.invalidate_all_map_probes();
			}
		}
		Ok(())
	}

	fn map_probe_for_frame(&self) -> Result<Probe<'ctx>, Error> {
		let owner = self.frame().function;
		let existing = self.map_probes.entries.borrow().get(&owner).copied();
		Ok(match existing {
			Some(entry) => entry,
			None => {
				let entry = Probe {
					valid: self.alloca_type(self.context.bool_type().into(), "probe_valid")?,
					table: self.alloca_type(self.pointer_type().into(), "probe_table")?,
					key: self.alloca_type(self.context.i32_type().into(), "probe_key")?,
					hash: self.alloca_type(self.context.i64_type().into(), "probe_hash")?,
					position: self.alloca_type(self.context.i64_type().into(), "probe_position")?,
				};
				let init = self.context.create_builder();
				let block = self.frame().allocas;
				let first = block
					.get_instructions()
					.find(|instruction| instruction.get_opcode() != InstructionOpcode::Alloca);
				if let Some(first) = first {
					init.position_before(&first);
				} else {
					init.position_at_end(block);
				}
				init.build_store(entry.valid, self.context.bool_type().const_zero())?;
				init.build_store(entry.table, self.pointer_type().const_null())?;
				init.build_store(entry.key, self.context.i32_type().const_zero())?;
				init.build_store(entry.hash, self.context.i64_type().const_zero())?;
				init.build_store(entry.position, self.context.i64_type().const_zero())?;
				self.map_probes.entries.borrow_mut().insert(owner, entry);
				entry
			}
		})
	}

	pub(super) fn release_inline_map_with_probe(
		&self,
		pointer: PointerValue<'ctx>,
		table: Table<'ctx>,
		position: IntValue<'ctx>,
	) -> Result<bool, Error> {
		if !self.map_probe_inline_delete_supported(table) {
			return Ok(false);
		}
		let entry = self.map_probe_for_frame()?;
		let valid = self
			.builder
			.build_load(self.context.bool_type(), entry.valid, "")?
			.into_int_value();
		let previous_table = self
			.builder
			.build_load(self.pointer_type(), entry.table, "")?
			.into_pointer_value();
		let same = self
			.builder
			.build_int_compare(IntPredicate::EQ, pointer, previous_table, "")?;
		let applies = self.builder.build_and(valid, same, "")?;
		let i64 = self.context.i64_type();
		let previous = self
			.builder
			.build_load(i64, entry.position, "")?
			.into_int_value();
		let hash = self
			.builder
			.build_load(i64, entry.hash, "")?
			.into_int_value();
		let hint = self
			.builder
			.build_select(applies, previous, i64.const_zero(), "")?
			.into_int_value();
		let updated = self
			.call_value(
				self.runtime.function("fa_table_release_probe")?,
				&[
					pointer.into(),
					position.into(),
					self.entry_size(table).into(),
					hint.into(),
					hash.into(),
				],
			)?
			.into_int_value();
		let updated = self.builder.build_select(applies, updated, previous, "")?;
		self.builder.build_store(entry.position, updated)?;
		Ok(true)
	}

	pub(super) fn invalidate_dropped_map_probes(&self, repr: Repr) -> Result<(), Error> {
		let may_hold_map = match repr {
			Repr::Map(_) | Repr::Set(_) => true,
			Repr::Array(item)
			| Repr::Growing(item)
			| Repr::Pool(item)
			| Repr::Columnar(item)
			| Repr::Object(item)
			| Repr::Tuple(item) => self.probe_type_may_hold_map(item, &mut FxHashSet::default()),
			Repr::Optional(scalar) => return self.invalidate_dropped_map_probes(scalar.repr()),
			Repr::Union
			| Repr::InlineUnion(_)
			| Repr::Rec(_)
			| Repr::Function(_)
			| Repr::Sync(_)
			| Repr::Task(_)
			| Repr::Watch
			| Repr::Links(_) => true,
			_ => false,
		};
		if may_hold_map {
			self.invalidate_all_map_probes()?;
		}
		Ok(())
	}

	pub(super) fn reuse_map_probe(
		&mut self,
		pointer: PointerValue<'ctx>,
		table: Table<'ctx>,
		key: Val<'ctx>,
		node: NodeId,
	) -> Result<Option<(IntValue<'ctx>, IntValue<'ctx>)>, Error> {
		let Some((map, symbol)) = self.map_probe_sources(table, node) else {
			return Ok(None);
		};
		let owner = self.frame().function;
		let identity = self.map_probes.identities.borrow().get(&owner).copied();
		let producer = match identity {
			Some((file, other_map, other_key, producer)) => {
				if (file, other_map, other_key) != (self.file, map, symbol) {
					return Ok(None);
				}
				producer
			}
			None => {
				self.map_probes
					.identities
					.borrow_mut()
					.insert(owner, (self.file, map, symbol, node));
				node
			}
		};
		let entry = self.map_probe_for_frame()?;
		let result = if node == producer {
			self.table_lookup_uncached("fa_table_find", pointer, table, key, node)?
		} else {
			let valid = self
				.builder
				.build_load(self.context.bool_type(), entry.valid, "")?
				.into_int_value();
			let old_table = self
				.builder
				.build_load(self.pointer_type(), entry.table, "")?
				.into_pointer_value();
			let old_key = self
				.builder
				.build_load(self.context.i32_type(), entry.key, "")?
				.into_int_value();
			let same_table =
				self.builder
					.build_int_compare(IntPredicate::EQ, pointer, old_table, "")?;
			let same_key =
				self.builder
					.build_int_compare(IntPredicate::EQ, key.int(), old_key, "")?;
			let reuse = self.builder.build_and(
				valid,
				self.builder.build_and(same_table, same_key, "")?,
				"",
			)?;
			let cached = self.append_block("probe_cached");
			let fresh = self.append_block("probe_fresh");
			let join = self.append_block("probe_found");
			self.builder
				.build_conditional_branch(reuse, cached, fresh)?;
			self.builder.position_at_end(cached);
			let position = self
				.builder
				.build_load(self.context.i64_type(), entry.position, "")?
				.into_int_value();
			let hash = self
				.builder
				.build_load(self.context.i64_type(), entry.hash, "")?
				.into_int_value();
			self.builder.build_unconditional_branch(join)?;
			self.builder.position_at_end(fresh);
			let (new_position, new_hash) =
				self.table_lookup_uncached("fa_table_find", pointer, table, key, node)?;
			let fresh_end = self.builder.get_insert_block().expect("a block");
			self.builder.build_unconditional_branch(join)?;
			self.builder.position_at_end(join);
			let positions = self
				.builder
				.build_phi(self.context.i64_type(), "probe_position")?;
			positions.add_incoming(&[(&position, cached), (&new_position, fresh_end)]);
			let hashes = self
				.builder
				.build_phi(self.context.i64_type(), "probe_hash")?;
			hashes.add_incoming(&[(&hash, cached), (&new_hash, fresh_end)]);
			(
				positions.as_basic_value().into_int_value(),
				hashes.as_basic_value().into_int_value(),
			)
		};
		let valid = self.builder.build_int_compare(
			IntPredicate::SLT,
			result.0,
			self.context.i64_type().const_zero(),
			"",
		)?;
		self.builder.build_store(entry.valid, valid)?;
		self.builder.build_store(entry.table, pointer)?;
		self.builder.build_store(entry.key, key.int())?;
		self.builder.build_store(entry.position, result.0)?;
		self.builder.build_store(entry.hash, result.1)?;
		Ok(Some(result))
	}
}
