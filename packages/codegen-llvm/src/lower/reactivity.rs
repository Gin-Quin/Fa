//! Reactivity (advanced brief §11, `advanced/reactivity.md`): `reactive`,
//! `derived` and `watch` over the runtime's sources and observers
//! (`fa_runtime.c`, "reactivity"), as the JavaScript backend does over
//! `runtime/reactive.js`.
//!
//! Values stay plain; only the places the analysis lists are instrumented
//! (`Analysis::reactive_reads`, `reactive_writes`, `reactive_args`,
//! `flush_after`, `watches`).
//!
//! - A `reactive` value has a source (`FaSource *`) in a slot of its own: a
//!   global at the top level of the module, an alloca in a function. A
//!   `reactive` field has a source kept by the runtime per object address.
//! - A read is preceded by `fa_track(source)`; a write is followed by
//!   `fa_notify(source)` (for a flat value, only when the value changed; a
//!   new map key also notifies the size).
//! - `derived` and `watch` bodies are functions of the observer, run by the
//!   runtime: they read the locals around them through an environment
//!   holding pointers to them, or copies of the `mutable` ones, made when the
//!   observer is declared. A derived value keeps its value in a block of
//!   the observer; reading it is `fa_derived_read`.
//! - After a statement of the outermost running code that may write a
//!   reactive place, `fa_reactive_flush()` runs the pending watch blocks.
//! - A `watch` outside the top level of the module is disposed at the end of
//!   its block (a cleanup of the block); `subscription.stop()` disposes it.
//! - A function given reactive places (D24) is one function: its callers set
//!   the sources of its parameters (`fa_argument_source_set`, null for a
//!   plain argument) right before the call, and it reads them as it starts.
//! - Sources and observers live until the end of the program
//!   (`fa_reactive_release`).

use fa_analyzer::analysis::{Callee, ReactivePlace, ReactiveRoot, ReactiveStep};
use fa_analyzer::ids::{NodeId, SymbolId};
use fa_analyzer::symbols::{Resolution, SymbolKind};
use fa_analyzer::syntax::View;
use fa_analyzer::types::{CollKind, TypeKind};
use inkwell::IntPredicate;
use inkwell::module::Linkage;
use inkwell::types::{BasicType, BasicTypeEnum};
use inkwell::values::{BasicValue, FunctionValue, IntValue, PointerValue};

use crate::error::Error;
use crate::lower::values::{IntRepr, Repr, Val};
use crate::lower::{Lowering, Slot};

/// How a derived value or a watch block sees a local declared around it.
#[derive(Copy, Clone)]
enum Capture {
	/// A pointer to the local's slot.
	Reference(SymbolId, Repr),
	/// A copy of its value made when the observer is declared (a `mutable`
	/// local: later writes don't reach the observer).
	Copy(SymbolId, Repr),
	/// A pointer to the slot holding the local's source (a `reactive`
	/// value), or its derived node.
	Source(SymbolId),
	Derived(SymbolId),
	Param(SymbolId),
}

impl<'ctx> Lowering<'_, 'ctx> {
	/// Whether a program uses reactivity: its sources and observers are
	/// released at its end.
	pub(crate) fn program_uses_reactivity(&self) -> bool {
		let uses =
			|analysis: &fa_analyzer::analysis::Analysis| {
				!analysis.watches.is_empty()
					|| !analysis.reactive_writes.is_empty()
					|| !analysis.reactive_reads.is_empty()
					|| analysis.symbols.symbols().iter().any(|symbol| {
						matches!(symbol.kind, SymbolKind::Reactive | SymbolKind::Derived)
					})
			};
		uses(self.analysis) || self.other_files.values().any(|state| uses(state.analysis))
	}

	/// A call of a reactivity function of the runtime returning a pointer.
	fn reactive_call(
		&self,
		name: &str,
		arguments: &[inkwell::values::BasicMetadataValueEnum<'ctx>],
	) -> Result<PointerValue<'ctx>, Error> {
		Ok(self
			.call_value(self.runtime.function(name)?, arguments)?
			.into_pointer_value())
	}

	// ── sources ─────────────────────────────────────────────────────────

	/// The slot holding the source of a `reactive` value (a global at the
	/// top level, created on first use).
	fn source_slot(&mut self, symbol: SymbolId) -> Result<PointerValue<'ctx>, Error> {
		if let Some(&slot) = self.reactive_sources.get(&symbol) {
			return Ok(slot);
		}
		let data = self.analysis.symbols.symbol(symbol);
		let slot = if self.owner_of_scope(data.scope).is_none() {
			let global = self
				.module
				.add_global(self.pointer_type(), None, "fa.source");
			global.set_linkage(Linkage::Internal);
			global.set_initializer(&self.pointer_type().const_null());
			global.as_pointer_value()
		} else {
			let slot = self.alloca_type(self.pointer_type().into(), "source")?;
			self.builder
				.build_store(slot, self.pointer_type().const_null())?;
			slot
		};
		self.reactive_sources.insert(symbol, slot);
		Ok(slot)
	}

	/// `reactive x = value`: a mutable value and its new source.
	pub(crate) fn reactive_declaration(
		&mut self,
		node: NodeId,
		value: Option<NodeId>,
	) -> Result<Val<'ctx>, Error> {
		let result = self.declaration(node, value)?;
		if result.repr == Repr::Never {
			return Ok(result);
		}
		let Some(symbol) = self.analysis.symbols.symbol_of_decl(node) else {
			return Ok(result);
		};
		// a new source at each declaration (a loop declares a new value)
		let slot = self.source_slot(symbol)?;
		let source = self.reactive_call("fa_source_new", &[])?;
		self.builder.build_store(slot, source)?;
		Ok(result)
	}

	/// The source of a reactive place, `None` when it isn't reactive here
	/// (a parameter given a plain value is a null source at run time) or
	/// can't be designated without evaluating code again: an index with
	/// effects is not followed (its container's source is coarser, still
	/// correct).
	fn place_source(&mut self, place: &ReactivePlace) -> Result<Option<PointerValue<'ctx>>, Error> {
		let i64 = self.context.i64_type();
		let mut source = match place.root {
			ReactiveRoot::Value(symbol) => {
				let slot = self.source_slot(symbol)?;
				self.builder
					.build_load(self.pointer_type(), slot, "")?
					.into_pointer_value()
			}
			ReactiveRoot::Param(symbol) => match self.param_sources.get(&symbol) {
				Some(&slot) => self
					.builder
					.build_load(self.pointer_type(), slot, "")?
					.into_pointer_value(),
				None => return Ok(None),
			},
			ReactiveRoot::Field { access, member } => {
				let (View::Access { target, members } | View::OptionalAccess { target, members }) =
					self.view(access)
				else {
					return Ok(None);
				};
				let members: Vec<NodeId> = members.iter().collect();
				let Some(&field) = members.get(member as usize) else {
					return Ok(None);
				};
				if !self.is_place(target) {
					return Ok(None);
				}
				let slot = self.place(target)?;
				let slot = self.place_members(
					slot,
					self.type_of(target),
					members[..member as usize].iter().copied(),
				)?;
				let key = match self.analysis.resolution(field) {
					// a relation side: its offset, as the runtime notifies it
					// (relations.rs, `relation_notifies`)
					Resolution::Field(_, index) => match slot.repr {
						Repr::Object(object) => self
							.types()
							.fields_of(object)
							.get(index as usize)
							.and_then(|field| self.side_source_key(object, field.name))
							.unwrap_or(u64::from(index)),
						_ => u64::from(index),
					},
					_ => 0,
				};
				self.reactive_call(
					"fa_field_source",
					&[slot.pointer.into(), i64.const_int(key, false).into()],
				)?
			}
		};
		for step in &place.steps {
			let (kind, key) = match *step {
				ReactiveStep::Field(name) => (1, i64.const_int(name.index() as u64, false)),
				ReactiveStep::Index(index) => match self.pure_key(index)? {
					Some(key) => (2, key),
					None => break,
				},
				ReactiveStep::Length | ReactiveStep::Size => (0, i64.const_zero()),
			};
			source = self.reactive_call(
				"fa_source_child",
				&[source.into(), i64.const_int(kind, false).into(), key.into()],
			)?;
		}
		Ok(Some(source))
	}

	/// The key of an element or an entry, when evaluating it again is pure
	/// (a literal or a local): an integer, or the hash of a key.
	fn pure_key(&mut self, node: NodeId) -> Result<Option<IntValue<'ctx>>, Error> {
		let mut inner = node;
		while let View::Group(group) = self.view(inner) {
			inner = group;
		}
		let pure = match self.view(inner) {
			View::Integer(_) | View::String(_) | View::Boolean(_) => true,
			View::Name(_) => matches!(self.analysis.resolution(inner), Resolution::Local(_)),
			_ => false,
		};
		if !pure {
			return Ok(None);
		}
		self.untracked += 1;
		let value = self.expr(node);
		self.untracked -= 1;
		let value = value?;
		let i64 = self.context.i64_type();
		Ok(match value.repr {
			Repr::Int(int) => Some(self.int_to(value.int(), int, IntRepr::I64)?),
			Repr::Bool | Repr::Enum(_) => {
				Some(self.builder.build_int_z_extend(value.int(), i64, "")?)
			}
			Repr::Str => {
				let hash = self.hash_function(Repr::Str)?;
				Some(
					self.call_value(hash, &[value.value.expect("a key").into()])?
						.into_int_value(),
				)
			}
			_ => None,
		})
	}

	/// `fa_track` of the source a read node reads, before it is read.
	pub(crate) fn track_read(&mut self, node: NodeId) -> Result<(), Error> {
		if self.untracked > 0 {
			return Ok(());
		}
		let read = match self.view(node) {
			// a built-in method reading its receiver: tracked on the callee
			View::Call { callee, .. } if !self.analysis.reactive_reads.contains_key(&node) => {
				callee
			}
			_ => node,
		};
		let Some(place) = self.analysis.reactive_reads.get(&read).cloned() else {
			return Ok(());
		};
		if let Some(source) = self.place_source(&place)? {
			self.call_runtime("fa_track", &[source.into()])?;
		}
		Ok(())
	}

	/// Notifies the sources a call wrote.
	pub(crate) fn notify_call(&mut self, node: NodeId) -> Result<(), Error> {
		if self.untracked > 0 || self.terminated_block() {
			return Ok(());
		}
		let Some(places) = self.analysis.reactive_writes.get(&node).cloned() else {
			return Ok(());
		};
		for place in &places {
			if let Some(source) = self.place_source(place)? {
				self.call_runtime("fa_notify", &[source.into()])?;
			}
		}
		Ok(())
	}

	/// Whether the current block already ended.
	fn terminated_block(&self) -> bool {
		self.builder
			.get_insert_block()
			.is_some_and(|block| block.get_terminator().is_some())
	}

	/// An assignment or a compound assignment (`node`, to `target`) writing
	/// reactive places: the previous value of a flat place, the write
	/// (`write`), then the notifications (only on a change of a flat value;
	/// a map entry with a new key also changes the size).
	pub(crate) fn reactive_write(
		&mut self,
		node: NodeId,
		target: NodeId,
		write: impl FnOnce(&mut Self) -> Result<Val<'ctx>, Error>,
	) -> Result<Val<'ctx>, Error> {
		let places = match self.analysis.reactive_writes.get(&node) {
			Some(places) if self.untracked == 0 => places.clone(),
			_ => return write(self),
		};
		let mut inner = target;
		while let View::Group(group) = self.view(inner) {
			inner = group;
		}
		let flat = matches!(
			self.node_repr(inner),
			Ok(Repr::Int(_)
				| Repr::Bool | Repr::F64
				| Repr::F32 | Repr::Percentage
				| Repr::Str | Repr::Enum(_))
		) && self.is_place(inner)
			&& !matches!(self.view(inner), View::Index { .. });
		// a map entry: its size before
		let mut size = None;
		if let View::Index {
			target: container, ..
		} = self.view(inner)
			&& matches!(
				self.types().kind(self.type_of(container)),
				TypeKind::Collection(CollKind::Map | CollKind::GrowingMap, ..)
			) && self.is_place(container)
			&& let Some(place) = places
				.iter()
				.find(|place| matches!(place.steps.last(), Some(ReactiveStep::Index(_))))
		{
			let mut place = place.clone();
			place.steps.pop();
			place.steps.push(ReactiveStep::Size);
			let slot = self.place(container)?;
			let map = self.load(slot)?;
			let before = self.table_size(map)?.int();
			size = Some((slot, before, place));
		}
		let previous = if flat {
			let slot = self.place(inner)?;
			let value = self.load(slot)?;
			// a string keeps its bytes for the comparison
			let value = self.emit_copy(value)?;
			Some((slot, value))
		} else {
			None
		};
		let result = write(self)?;
		if result.repr == Repr::Never || self.terminated_block() {
			return Ok(result);
		}
		let notify = self.append_block("notify");
		let done = self.append_block("notified");
		match previous {
			Some((slot, old)) => {
				let current = self.load(slot)?;
				let same = self.values_equal_any(old, current, node)?;
				self.emit_drop(old)?;
				self.builder.build_conditional_branch(same, done, notify)?;
			}
			None => {
				self.builder.build_unconditional_branch(notify)?;
			}
		}
		self.builder.position_at_end(notify);
		for place in &places {
			if let Some(source) = self.place_source(place)? {
				self.call_runtime("fa_notify", &[source.into()])?;
			}
		}
		self.builder.build_unconditional_branch(done)?;
		self.builder.position_at_end(done);
		if let Some((slot, before, place)) = size {
			let map = self.load(slot)?;
			let after = self.table_size(map)?.int();
			let changed = self
				.builder
				.build_int_compare(IntPredicate::NE, before, after, "")?;
			let notify = self.append_block("notify_size");
			let done = self.append_block("notified_size");
			self.builder
				.build_conditional_branch(changed, notify, done)?;
			self.builder.position_at_end(notify);
			if let Some(source) = self.place_source(&place)? {
				self.call_runtime("fa_notify", &[source.into()])?;
			}
			self.builder.build_unconditional_branch(done)?;
			self.builder.position_at_end(done);
		}
		Ok(result)
	}

	/// `fa_reactive_flush()` after a statement of the outermost running code
	/// that may write a reactive place.
	pub(crate) fn flush_after(&mut self, statement: NodeId) -> Result<(), Error> {
		if self.analysis.flush_after.contains(&statement) && !self.terminated_block() {
			self.call_runtime("fa_reactive_flush", &[])?;
		}
		Ok(())
	}

	// ── the parameters given reactive places (D24) ──────────────────────

	/// The functions of this file that callers may give reactive places.
	fn reactive_callee(&self, declaration: NodeId) -> bool {
		self.analysis.reactive_args.keys().any(|call| {
			self.analysis
				.call(*call)
				.is_some_and(|info| info.callee == Callee::Function(declaration))
		})
	}

	/// Before a call of a function that may receive reactive places: the
	/// source of each argument's place (null for a plain one).
	pub(crate) fn pass_argument_sources(
		&mut self,
		call: NodeId,
		declaration: NodeId,
		count: usize,
	) -> Result<(), Error> {
		if !self.reactive_callee(declaration) {
			return Ok(());
		}
		let places = self
			.analysis
			.reactive_args
			.get(&call)
			.cloned()
			.unwrap_or_default();
		let i32 = self.context.i32_type();
		for index in 0..count.min(16) {
			let source = match places.get(index).cloned().flatten() {
				Some(place) => self.place_source(&place)?,
				None => None,
			};
			let source = source.unwrap_or_else(|| self.pointer_type().const_null());
			self.call_runtime(
				"fa_argument_source_set",
				&[i32.const_int(index as u64, false).into(), source.into()],
			)?;
		}
		Ok(())
	}

	/// As a function that may receive reactive places starts: the sources
	/// of its parameters, from its caller.
	pub(crate) fn receive_argument_sources(
		&mut self,
		declaration: NodeId,
		params: &[SymbolId],
	) -> Result<(), Error> {
		// the slots of another function's locals are not this one's
		self.param_sources.clear();
		self.reactive_sources.retain(|_, slot| is_global(*slot));
		self.derived_nodes.retain(|_, slot| is_global(*slot));
		if !self.reactive_callee(declaration) {
			return Ok(());
		}
		let i32 = self.context.i32_type();
		for (index, &param) in params.iter().enumerate().take(16) {
			let source = self.reactive_call(
				"fa_argument_source_get",
				&[i32.const_int(index as u64, false).into()],
			)?;
			let slot = self.alloca_type(self.pointer_type().into(), "parameter_source")?;
			self.builder.build_store(slot, source)?;
			self.param_sources.insert(param, slot);
		}
		Ok(())
	}

	// ── derived values and watch blocks ─────────────────────────────────

	/// The locals declared around a derived value or a watch block (node
	/// `node`, body `body`) that it reads, and how.
	fn observer_captures(&mut self, node: NodeId, body: NodeId) -> Result<Vec<Capture>, Error> {
		let span = self.tree.spans[body.index()];
		// the values whose reactive fields it reads: referred to, not copied
		let mut referred = Vec::new();
		for (&read, place) in &self.analysis.reactive_reads {
			let ReactiveRoot::Field { access, .. } = place.root else {
				continue;
			};
			let at = self.tree.spans[read.index()];
			if at.start < span.start || at.end > span.end {
				continue;
			}
			let mut current = access;
			loop {
				match self.view(current) {
					View::Access { target, .. }
					| View::OptionalAccess { target, .. }
					| View::Index { target, .. } => current = target,
					View::Group(inner) => current = inner,
					_ => break,
				}
			}
			if let Resolution::Local(symbol) = self.analysis.resolution(current) {
				referred.push(symbol);
			}
		}
		let mut found: Vec<SymbolId> = Vec::new();
		let mut stack = vec![body.index()];
		while let Some(index) = stack.pop() {
			if let Resolution::Local(symbol) = self.analysis.resolution(NodeId::new(index)) {
				let data = self.analysis.symbols.symbol(symbol);
				let declared = self.tree.spans[data.decl_node.index()];
				let outside = declared.end <= span.start || declared.start >= span.end;
				let value = !matches!(
					data.kind,
					SymbolKind::Function
						| SymbolKind::Type | SymbolKind::Namespace
						| SymbolKind::Requirement { .. }
				);
				if outside && value && !found.contains(&symbol) {
					found.push(symbol);
				}
			}
			self.tree.nodes[index].for_each_child(|child| stack.push(child));
		}
		let _ = node;
		found.sort();
		let mut captures = Vec::new();
		for symbol in found {
			let data = self.analysis.symbols.symbol(symbol);
			let kind = data.kind;
			// module values in globals are read directly
			let global =
				self.global_slots.contains_key(&symbol) || self.shared.contains_key(&symbol);
			if kind == SymbolKind::Derived {
				if self
					.derived_nodes
					.get(&symbol)
					.is_some_and(|slot| !is_global(*slot))
				{
					captures.push(Capture::Derived(symbol));
				}
				continue;
			}
			let repr = self.repr_of(data.ty, data.decl_node.index())?;
			if !repr.has_value() {
				continue;
			}
			if !global {
				let copy = matches!(kind, SymbolKind::Mutable | SymbolKind::MutableUse)
					&& !referred.contains(&symbol);
				captures.push(if copy {
					Capture::Copy(symbol, repr)
				} else {
					Capture::Reference(symbol, repr)
				});
			}
			if kind == SymbolKind::Reactive
				&& self
					.reactive_sources
					.get(&symbol)
					.is_some_and(|slot| !is_global(*slot))
			{
				captures.push(Capture::Source(symbol));
			}
			if self.param_sources.contains_key(&symbol) {
				captures.push(Capture::Param(symbol));
			}
		}
		Ok(captures)
	}

	/// The struct of an environment of these captures.
	fn observer_environment(&self, captures: &[Capture]) -> inkwell::types::StructType<'ctx> {
		let fields: Vec<BasicTypeEnum<'ctx>> = captures
			.iter()
			.map(|capture| match capture {
				Capture::Copy(_, repr) => self.llvm_type(*repr),
				_ => self.pointer_type().into(),
			})
			.collect();
		self.context.struct_type(&fields, false)
	}

	/// The environment of an observer, made when it is declared.
	fn build_observer_environment(
		&mut self,
		captures: &[Capture],
		node: NodeId,
	) -> Result<PointerValue<'ctx>, Error> {
		if captures.is_empty() {
			return Ok(self.pointer_type().const_null());
		}
		let structure = self.observer_environment(captures);
		let pointer = self.allocate(structure.size_of().expect("a sized environment"))?;
		for (index, capture) in captures.iter().enumerate() {
			let field = self
				.builder
				.build_struct_gep(structure, pointer, index as u32, "")?;
			let value: inkwell::values::BasicValueEnum<'ctx> = match *capture {
				Capture::Reference(symbol, _) => self.slot_of(symbol, node)?.pointer.into(),
				Capture::Copy(symbol, _) => {
					let slot = self.slot_of(symbol, node)?;
					let value = self.load(slot)?;
					self.emit_copy(value)?.value.expect("a value")
				}
				Capture::Source(symbol) => self.reactive_sources[&symbol].into(),
				Capture::Derived(symbol) => self.derived_nodes[&symbol].into(),
				Capture::Param(symbol) => self.param_sources[&symbol].into(),
			};
			self.builder.build_store(field, value)?;
		}
		Ok(pointer)
	}

	/// A function of an observer (`void (FaObserver *)`), lowered now with
	/// the captures bound from its environment: `body` lowers what it does.
	fn observer_function(
		&mut self,
		name: &str,
		captures: &[Capture],
		body: impl FnOnce(&mut Self, PointerValue<'ctx>) -> Result<(), Error>,
	) -> Result<FunctionValue<'ctx>, Error> {
		let pointer = self.pointer_type();
		let function = self.module.add_function(
			name,
			self.context.void_type().fn_type(&[pointer.into()], false),
			Some(Linkage::Internal),
		);
		self.runtime.add_attributes(function, &["nounwind"]);
		// the state of the function being lowered, restored after
		let saved_block = self.builder.get_insert_block();
		let saved_frame = self.frame.take();
		let owner = saved_frame.as_ref().and_then(|frame| frame.owner);
		let saved_slots = std::mem::take(&mut self.slots);
		let saved_owned = std::mem::take(&mut self.owned_locals);
		let saved_references = std::mem::take(&mut self.references);
		let saved_sources = self.reactive_sources.clone();
		let saved_derived = self.derived_nodes.clone();
		let saved_params = std::mem::take(&mut self.param_sources);
		self.reactive_sources.retain(|_, slot| is_global(*slot));
		self.derived_nodes.retain(|_, slot| is_global(*slot));
		self.slots = saved_slots
			.iter()
			.filter(|(_, slot)| is_global(slot.pointer))
			.map(|(symbol, slot)| (*symbol, *slot))
			.collect();
		self.begin_function(function, Repr::Void, owner);
		let observer = function
			.get_nth_param(0)
			.expect("an observer")
			.into_pointer_value();
		let result = (|| -> Result<(), Error> {
			if !captures.is_empty() {
				let structure = self.observer_environment(captures);
				let environment = self.reactive_call("fa_observer_env", &[observer.into()])?;
				for (index, capture) in captures.iter().enumerate() {
					let field =
						self.builder
							.build_struct_gep(structure, environment, index as u32, "")?;
					let load = |this: &Self| -> Result<PointerValue<'ctx>, Error> {
						Ok(this
							.builder
							.build_load(this.pointer_type(), field, "")?
							.into_pointer_value())
					};
					match *capture {
						Capture::Reference(symbol, repr) => {
							let pointer = load(self)?;
							self.slots.insert(symbol, Slot { pointer, repr });
						}
						Capture::Copy(symbol, repr) => {
							self.slots.insert(
								symbol,
								Slot {
									pointer: field,
									repr,
								},
							);
						}
						Capture::Source(symbol) => {
							let pointer = load(self)?;
							self.reactive_sources.insert(symbol, pointer);
						}
						Capture::Derived(symbol) => {
							let pointer = load(self)?;
							self.derived_nodes.insert(symbol, pointer);
						}
						Capture::Param(symbol) => {
							let pointer = load(self)?;
							self.param_sources.insert(symbol, pointer);
						}
					}
				}
			}
			body(self, observer)?;
			if !self.terminated_block() {
				self.drop_to(0)?;
				self.builder.build_return(None)?;
			}
			Ok(())
		})();
		self.cleanups.borrow_mut().clear();
		let ended = self.end_function();
		self.frame = saved_frame;
		self.slots = saved_slots;
		self.owned_locals = saved_owned;
		self.references = saved_references;
		self.reactive_sources = saved_sources;
		self.derived_nodes = saved_derived;
		self.param_sources = saved_params;
		if let Some(block) = saved_block {
			self.builder.position_at_end(block);
		}
		result?;
		ended?;
		Ok(function)
	}

	/// The release of an observer: the copies of its environment, its value
	/// (a derived value) or its cleanup (a watch block).
	fn observer_release(
		&mut self,
		captures: &[Capture],
		value: Option<Repr>,
		watch: bool,
	) -> Result<FunctionValue<'ctx>, Error> {
		let pointer = self.pointer_type();
		let function = self.module.add_function(
			"fa.observer.release",
			self.context.void_type().fn_type(&[pointer.into()], false),
			Some(Linkage::Internal),
		);
		self.runtime.add_attributes(function, &["nounwind"]);
		let builder = self.context.create_builder();
		builder.position_at_end(self.context.append_basic_block(function, "entry"));
		let observer = function
			.get_nth_param(0)
			.expect("an observer")
			.into_pointer_value();
		let call = |name: &str| -> Result<PointerValue<'ctx>, Error> {
			Ok(builder
				.build_call(self.runtime.function(name)?, &[observer.into()], "")?
				.try_as_basic_value()
				.basic()
				.expect("a pointer")
				.into_pointer_value())
		};
		if let Some(repr) = value.filter(|repr| self.needs_drop(*repr)) {
			let block = call("fa_observer_value")?;
			let loaded = builder.build_load(self.llvm_type(repr), block, "")?;
			self.call_drop(&builder, repr, loaded)?;
		}
		if watch {
			let slot = call("fa_observer_cleanup")?;
			let closure_type = self.closure_type();
			let closure = builder.build_load(closure_type, slot, "")?;
			// a missing cleanup is a zero value, which a drop accepts
			let repr = Repr::Function(fa_analyzer::ids::TypeId::UNKNOWN);
			self.call_drop(&builder, repr, closure)?;
		}
		let copies: Vec<(usize, Repr)> = captures
			.iter()
			.enumerate()
			.filter_map(|(index, capture)| match capture {
				Capture::Copy(_, repr) => Some((index, *repr)),
				_ => None,
			})
			.collect();
		if !captures.is_empty() {
			let structure = self.observer_environment(captures);
			let environment = call("fa_observer_env")?;
			for (index, repr) in copies {
				let field = builder.build_struct_gep(structure, environment, index as u32, "")?;
				let loaded = builder.build_load(self.llvm_type(repr), field, "")?;
				self.call_drop(&builder, repr, loaded)?;
			}
			let size = structure.size_of().expect("a sized environment");
			let size = builder.build_int_cast(size, self.usize, "")?;
			builder.build_call(
				self.runtime.free()?,
				&[
					environment.into(),
					size.into(),
					self.usize.const_int(16, false).into(),
				],
				"",
			)?;
		}
		builder.build_return(None)?;
		Ok(function)
	}

	/// `derived name = value`: an observer computing the value when read.
	pub(crate) fn derived_declaration(
		&mut self,
		node: NodeId,
		value: Option<NodeId>,
	) -> Result<Val<'ctx>, Error> {
		let (Some(symbol), Some(value)) = (self.analysis.symbols.symbol_of_decl(node), value)
		else {
			return Err(self.unsupported(node.index(), "this derived value"));
		};
		let repr = self.repr_of(self.analysis.symbols.symbol(symbol).ty, node.index())?;
		if !repr.has_value() {
			return Err(self.unsupported(node.index(), "this derived value"));
		}
		let captures = self.observer_captures(node, value)?;
		let flat = matches!(
			repr,
			Repr::Int(_)
				| Repr::Bool | Repr::F64
				| Repr::F32 | Repr::Percentage
				| Repr::Str | Repr::Enum(_)
		);
		let compute = self.observer_function("fa.derived", &captures, |this, observer| {
			let computed = this.expr(value)?;
			if computed.repr == Repr::Never {
				return Ok(());
			}
			let computed = this.owned_as(computed, repr, Some(value))?;
			let block = this.reactive_call("fa_observer_value", &[observer.into()])?;
			let old = this.load(Slot {
				pointer: block,
				repr,
			})?;
			if flat {
				// the version changes only when the value does
				let was = this
					.call_value(
						this.runtime.function("fa_observer_computed")?,
						&[observer.into()],
					)?
					.into_int_value();
				let was = this.builder.build_int_compare(
					IntPredicate::NE,
					was,
					was.get_type().const_zero(),
					"",
				)?;
				let same = this.values_equal_any(old, computed, value)?;
				let same = this.builder.build_and(same, was, "")?;
				let changed = this.builder.build_not(same, "")?;
				let changed =
					this.builder
						.build_int_z_extend(changed, this.context.i32_type(), "")?;
				this.call_runtime("fa_observer_changed", &[observer.into(), changed.into()])?;
			}
			this.emit_drop(old)?;
			if let Some(raw) = computed.value {
				this.builder.build_store(block, raw)?;
			}
			Ok(())
		})?;
		let release = self.observer_release(&captures, Some(repr), false)?;
		let environment = self.build_observer_environment(&captures, node)?;
		let size = self.llvm_type(repr).size_of().expect("a sized value");
		let observer = self.reactive_call(
			"fa_derived_new",
			&[
				compute.as_global_value().as_pointer_value().into(),
				release.as_global_value().as_pointer_value().into(),
				environment.into(),
				size.into(),
			],
		)?;
		let slot = self.derived_slot(symbol)?;
		self.builder.build_store(slot, observer)?;
		Ok(Val::void())
	}

	/// The slot holding the node of a derived value.
	fn derived_slot(&mut self, symbol: SymbolId) -> Result<PointerValue<'ctx>, Error> {
		if let Some(&slot) = self.derived_nodes.get(&symbol) {
			return Ok(slot);
		}
		let data = self.analysis.symbols.symbol(symbol);
		let slot = if self.owner_of_scope(data.scope).is_none() {
			let global = self
				.module
				.add_global(self.pointer_type(), None, "fa.derived");
			global.set_linkage(Linkage::Internal);
			global.set_initializer(&self.pointer_type().const_null());
			global.as_pointer_value()
		} else {
			self.alloca_type(self.pointer_type().into(), "derived")?
		};
		self.derived_nodes.insert(symbol, slot);
		Ok(slot)
	}

	/// A read of a derived value: brought up to date and tracked; a view of
	/// the value it keeps.
	pub(crate) fn derived_read(
		&mut self,
		symbol: SymbolId,
		node: NodeId,
	) -> Result<Val<'ctx>, Error> {
		let repr = self.repr_of(self.analysis.symbols.symbol(symbol).ty, node.index())?;
		let slot = self.derived_slot(symbol)?;
		let observer = self
			.builder
			.build_load(self.pointer_type(), slot, "")?
			.into_pointer_value();
		let block = self.reactive_call("fa_derived_read", &[observer.into()])?;
		self.load(Slot {
			pointer: block,
			repr,
		})
	}

	/// `watch { … }`: an observer running its body now and after a value it
	/// read changed. Outside of the top level of the module, the end of the
	/// enclosing block disposes it.
	pub(crate) fn watch(&mut self, node: NodeId) -> Result<Val<'ctx>, Error> {
		let Some(info) = self.analysis.watches.get(&node).copied() else {
			return Err(self.unsupported(node.index(), "this `watch` block"));
		};
		let captures = self.observer_captures(node, info.body)?;
		let cleanup_type = info.cleanup.map(|cleanup| self.type_of(cleanup));
		let run = self.observer_function("fa.watch", &captures, |this, observer| {
			let want = info.cleanup.is_some();
			let value = this.block(info.body, want)?;
			if let (Some(cleanup), true) = (info.cleanup, value.repr.has_value()) {
				let value = this.owned_as(value, value.repr, Some(cleanup))?;
				let slot = this.reactive_call("fa_observer_cleanup", &[observer.into()])?;
				this.builder
					.build_store(slot, value.value.expect("a cleanup"))?;
			}
			Ok(())
		})?;
		let cleanup = self.observer_function("fa.watch.cleanup", &[], |this, observer| {
			let Some(ty) = cleanup_type else {
				return Ok(());
			};
			let slot = this.reactive_call("fa_observer_cleanup", &[observer.into()])?;
			let closure_type = this.closure_type();
			let closure = this
				.builder
				.build_load(closure_type, slot, "")?
				.into_struct_value();
			let function = this
				.builder
				.build_extract_value(closure, 0, "")?
				.into_pointer_value();
			let present = this.builder.build_is_not_null(function, "")?;
			let then = this.append_block("cleanup");
			let end = this.append_block("cleaned");
			this.builder.build_conditional_branch(present, then, end)?;
			this.builder.position_at_end(then);
			// the slot is emptied first: the cleanup runs once
			this.builder.build_store(slot, closure_type.const_zero())?;
			let value = Val::new(Repr::Function(ty), closure);
			let mark = this.mark();
			this.call_closure(value, ty, &[], node)?;
			this.end_statement(mark, None)?;
			this.emit_drop(value)?;
			this.builder.build_unconditional_branch(end)?;
			this.builder.position_at_end(end);
			Ok(())
		})?;
		let release = self.observer_release(&captures, None, true)?;
		let environment = self.build_observer_environment(&captures, node)?;
		let observer = self.reactive_call(
			"fa_watch_new",
			&[
				run.as_global_value().as_pointer_value().into(),
				cleanup.as_global_value().as_pointer_value().into(),
				release.as_global_value().as_pointer_value().into(),
				environment.into(),
			],
		)?;
		if !self.is_module_statement(node) {
			self.cleanups
				.borrow_mut()
				.push(super::ownership::Cleanup::Dispose(observer));
		}
		Ok(Val::new(Repr::Watch, observer))
	}

	/// Whether a `watch` (or the declaration holding it) is a statement of
	/// the top level of the module, which lives as long as the program.
	fn is_module_statement(&self, node: NodeId) -> bool {
		let parents = self.parents();
		let mut current = node.index();
		// the `watch` itself, or the declaration it is the value of
		if let Some(&parent) = parents.get(current)
			&& parent != usize::MAX
			&& matches!(
				self.view(NodeId::new(parent)),
				View::Declaration { .. } | View::Group(_)
			) {
			current = parent;
		}
		parents
			.get(current)
			.is_some_and(|&parent| parent == self.tree.root)
	}

	/// `subscription.stop()`.
	pub(crate) fn stop_subscription(&mut self, node: NodeId) -> Result<Val<'ctx>, Error> {
		let receiver = self.receiver(node)?;
		if let Some(observer) = receiver.value {
			self.call_runtime("fa_dispose", &[observer.into()])?;
		}
		Ok(Val::void())
	}

	/// The sources and observers of the program, released at its end.
	pub(crate) fn release_reactivity(&mut self) -> Result<(), Error> {
		if self.program_uses_reactivity() && !self.terminated_block() {
			self.call_runtime("fa_reactive_release", &[])?;
		}
		Ok(())
	}
}

/// Whether a pointer is a global (not an alloca of a function).
fn is_global(pointer: PointerValue<'_>) -> bool {
	pointer.as_instruction_value().is_none()
}
