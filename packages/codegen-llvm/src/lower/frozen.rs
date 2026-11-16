//! Fields never written after construction (`design/decisions.md`, "Fields
//! never written after construction"): a method reads such a field of
//! `self` once, at its entry, and every read of `self.f` in its body uses
//! that value; a string field also gets its bytes and length there, past
//! the inline-or-heap test (`string_parts` finds them). A call of a small
//! method on `self` (`self.current()`) is expanded at the Fa level into the
//! calling method's body, its `self` standing for the caller's, so that its
//! reads use the same values: inlining by LLVM alone would reload the field
//! after every call the caller makes (the field may have changed, as far as
//! LLVM can tell).
//!
//! The fact is the program's: no store anywhere writes the field (an
//! assignment, a compound assignment, a reference a callee or a `mutable
//! use` may change, a built-in method changing it), and no whole value
//! holding an object of the type is stored but by a declaration
//! (`proofs::written_fields`). A program lowered as one module computes it
//! over its files; a build per unit reads the union of every unit's writes
//! (`facts::WRITTEN_FIELDS`).

use std::collections::BTreeSet;

use fa_analyzer::analysis::{Argument, CallInfo, Callee};
use fa_analyzer::ids::{NodeId, SymbolId, TypeId};
use fa_analyzer::symbols::Resolution;
use fa_analyzer::syntax::View;
use inkwell::values::{BasicValueEnum, FunctionValue, IntValue, PointerValue};

use super::values::{Repr, Val};
use super::{Lowering, Slot};
use crate::error::Error;

/// The largest body (in nodes) of a method expanded into its caller.
const EXPANDED_NODES: usize = 48;

/// A field of `self` read once at the entry of a method.
#[derive(Copy, Clone)]
pub(crate) struct FrozenRead<'ctx> {
	function: FunctionValue<'ctx>,
	/// The object's address (the slot of `self`).
	pointer: PointerValue<'ctx>,
	index: u32,
	value: Val<'ctx>,
}

/// The bytes and length of a string read once (see the module).
#[derive(Copy, Clone)]
pub(crate) struct FrozenParts<'ctx> {
	value: BasicValueEnum<'ctx>,
	bytes: PointerValue<'ctx>,
	length: IntValue<'ctx>,
}

/// How to expand a call of a small method of `self` (see the module).
#[derive(Clone)]
pub(crate) struct MethodPlan {
	function: NodeId,
	/// The method's `self`, then its other parameters.
	receiver: SymbolId,
	params: Vec<SymbolId>,
	/// The caller's local the receiver names.
	caller_self: SymbolId,
	body: NodeId,
	block_body: bool,
}

impl<'ctx> Lowering<'_, 'ctx> {
	/// The program's writes of fields (`proofs::written_fields`), when the
	/// whole program is known.
	fn written_fields(&self) -> Option<&BTreeSet<String>> {
		self.written_fields
			.get_or_init(|| {
				if self.whole_program {
					let mut files = vec![fa_analyzer::proofs::ProofFile::new(
						self.tree,
						self.analysis,
						self.types(),
					)];
					for state in self.other_files.values() {
						files.push(fa_analyzer::proofs::ProofFile::new(
							state.tree,
							state.analysis,
							self.types(),
						));
					}
					return Some(fa_analyzer::proofs::written_fields(&files));
				}
				self.program_facts
					.as_ref()
					.and_then(|facts| facts.set(crate::facts::WRITTEN_FIELDS))
					.cloned()
			})
			.as_ref()
	}

	/// Whether the field `index` of objects of `owner` is never written
	/// after construction.
	pub(crate) fn frozen_field(&self, owner: TypeId, index: u32) -> bool {
		let Some(written) = self.written_fields() else {
			return false;
		};
		let Some(field) = self.types().fields_of(owner).get(index as usize) else {
			return false;
		};
		fa_analyzer::proofs::frozen_field(written, self.types(), owner, field.name)
	}

	/// The slot of a local an access may read a field of directly: its
	/// own storage (not a reference, a requirement, a global, a reactive or
	/// shared value), holding an object.
	fn object_slot(&self, target: NodeId) -> Option<(SymbolId, Slot<'ctx>, TypeId)> {
		if !matches!(self.view(target), View::Name(_))
			|| self
				.flags(target)
				.contains(fa_analyzer::analysis::NodeFlags::NARROWED)
		{
			return None;
		}
		let Resolution::Local(symbol) = self.analysis.resolution(target) else {
			return None;
		};
		if self.references.contains_key(&symbol)
			|| self.reactive_sources.contains_key(&symbol)
			|| self.derived_nodes.contains_key(&symbol)
			|| self.shared.contains_key(&symbol)
			|| self.param_sources.contains_key(&symbol)
		{
			return None;
		}
		let slot = *self.slots.get(&symbol)?;
		let Repr::Object(ty) = slot.repr else {
			return None;
		};
		Some((symbol, slot, ty))
	}

	/// The value of `target.member` read at the entry of the method, if any.
	/// And the field's type.
	pub(crate) fn frozen_value(
		&self,
		target: NodeId,
		member: NodeId,
	) -> Option<(Val<'ctx>, TypeId)> {
		if self.frozen_reads.is_empty()
			|| self
				.flags(member)
				.contains(fa_analyzer::analysis::NodeFlags::NARROWED)
		{
			return None;
		}
		let (_, slot, owner) = self.object_slot(target)?;
		let index = self.concrete_field_index(owner, member).ok()?;
		let function = self.frame().function;
		let read = self.frozen_reads.iter().find(|read| {
			read.function == function && read.pointer == slot.pointer && read.index == index
		})?;
		Some((read.value, self.types().fields_of(owner)[index as usize].ty))
	}

	/// The bytes and length of a string read at the entry of the method.
	pub(crate) fn frozen_parts(
		&self,
		value: Val<'ctx>,
	) -> Option<(PointerValue<'ctx>, IntValue<'ctx>)> {
		let raw = value.value?;
		self.frozen_string_parts
			.iter()
			.find(|parts| parts.value == raw)
			.map(|parts| (parts.bytes, parts.length))
	}

	/// Whether a representation is a scalar: nothing to drop or to own.
	fn plain(&self, repr: Repr) -> bool {
		matches!(
			repr,
			Repr::Void
				| Repr::Bool | Repr::Int(_)
				| Repr::F64 | Repr::F32
				| Repr::Percentage
				| Repr::Enum(_)
				| Repr::None
		) || (matches!(repr, Repr::Optional(_)) && !self.needs_drop(repr))
	}

	/// The plan of a call `self.method(…)` that can be expanded into the
	/// method being lowered: a method of this file, neither generic,
	/// asynchronous, throwing nor capturing, with no requirements, scalar
	/// parameters and result, and a small body that doesn't leave early,
	/// create closures, tasks or references, nor call anything but
	/// built-ins and such methods of its own `self`.
	pub(crate) fn method_expansion(&self, info: &CallInfo) -> Option<MethodPlan> {
		if self.tasks.asynchronous {
			return None;
		}
		let Callee::Function(function) = info.callee else {
			return None;
		};
		if self.frame().owner == Some(function) || self.expanded_functions.contains(&function) {
			return None;
		}
		let Some(Argument::Receiver(callee)) = info.arguments.first() else {
			return None;
		};
		let View::Access { target, members } = self.view(*callee) else {
			return None;
		};
		if members.len() != 1
			|| info.arguments[1..]
				.iter()
				.any(|argument| !matches!(argument, Argument::Given(_)))
		{
			return None;
		}
		let (caller_self, _, _) = self.object_slot(target)?;
		// only into a method that reads fields of this object once
		let current = self.frame().function;
		let pointer = self.slots.get(&caller_self)?.pointer;
		if !self
			.frozen_reads
			.iter()
			.any(|read| read.function == current && read.pointer == pointer)
		{
			return None;
		}
		let function_info = self.analysis.function(function)?;
		if !function_info.generics.is_empty()
			|| function_info.declaration.is_none()
			|| !function_info.captures.is_empty()
			|| function_info.throws != TypeId::NEVER
			|| function_info.params.len() != info.arguments.len()
			|| function_info
				.params
				.iter()
				.any(|param| param.default.is_some() || param.variadic)
		{
			return None;
		}
		let sig = self.types().sig(function_info.sig?);
		if sig.is_async || !self.types().requirements(sig.requires).is_empty() {
			return None;
		}
		if !self.plain(self.repr_of(sig.ret, function.index()).ok()?) {
			return None;
		}
		let receiver = function_info.params.first()?.symbol;
		for param in &function_info.params[1..] {
			let ty = self.analysis.symbols.symbol(param.symbol).ty;
			if !self.plain(self.repr_of(ty, function.index()).ok()?) {
				return None;
			}
		}
		// A small nonthrowing body: locally handled errors stay inside it.
		// Nothing leaves early, captures, starts, references, or calls user
		// code but methods of its `self`.
		let mut stack = vec![function_info.body.index()];
		let mut count = 0;
		while let Some(at) = stack.pop() {
			count += 1;
			if count > EXPANDED_NODES {
				return None;
			}
			let at_node = NodeId::new(at);
			match self.view(at_node) {
				View::Return(_)
				| View::Throw(_)
				| View::Lambda { .. }
				| View::Function { .. }
				| View::Start { .. }
				| View::Await(_)
				| View::Use { .. }
				| View::Reuse { .. }
				| View::Provide { .. } => return None,
				View::Call { .. } => match self.analysis.call(at_node).map(|call| &call.callee) {
					Some(Callee::Builtin(_) | Callee::Conversion(_)) => {}
					Some(Callee::Function(_)) => {
						let call = self.analysis.call(at_node)?;
						let Some(Argument::Receiver(inner)) = call.arguments.first() else {
							return None;
						};
						let View::Access { target, .. } = self.view(*inner) else {
							return None;
						};
						if self.analysis.resolution(target) != Resolution::Local(receiver) {
							return None;
						}
					}
					_ => return None,
				},
				_ => {}
			}
			self.tree.nodes[at].for_each_child(|child| stack.push(child));
		}
		Some(MethodPlan {
			function,
			receiver,
			params: function_info.params[1..]
				.iter()
				.map(|param| param.symbol)
				.collect(),
			caller_self,
			body: function_info.body,
			block_body: function_info.block_body,
		})
	}

	/// A call of a method expanded into the caller (see the module).
	pub(crate) fn expand_method(
		&mut self,
		node: NodeId,
		info: &CallInfo,
		plan: MethodPlan,
	) -> Result<Val<'ctx>, Error> {
		// the arguments, in order, then the parameters bound to them
		let mut bound = Vec::new();
		for (argument, &param) in info.arguments[1..].iter().zip(&plan.params) {
			let Argument::Given(argument) = argument else {
				unreachable!("checked by method_expansion");
			};
			let value = self.expr(*argument)?;
			if value.repr == Repr::Never {
				return Ok(value);
			}
			let repr = self.repr_of(self.analysis.symbols.symbol(param).ty, argument.index())?;
			let value = self.convert(value, repr, argument.index())?;
			let slot = self.alloca(repr, "expanded_parameter")?;
			if let Some(raw) = value.value {
				self.builder.build_store(slot.pointer, raw)?;
			}
			bound.push((param, slot));
		}
		let receiver = *self
			.slots
			.get(&plan.caller_self)
			.expect("checked by method_expansion");
		bound.push((plan.receiver, receiver));
		let mut saved = Vec::new();
		for (param, slot) in bound {
			saved.push((param, self.slots.insert(param, slot)));
		}
		self.expanded_functions.push(plan.function);
		let result = self.expand_body(node, &plan);
		self.expanded_functions.pop();
		for (symbol, previous) in saved.into_iter().rev() {
			match previous {
				Some(slot) => self.slots.insert(symbol, slot),
				None => self.slots.remove(&symbol),
			};
		}
		result
	}

	fn expand_body(&mut self, node: NodeId, plan: &MethodPlan) -> Result<Val<'ctx>, Error> {
		let repr = self.node_repr(node)?;
		let scope = self.mark();
		let value = if plan.block_body {
			self.block(plan.body, repr.has_value())?
		} else {
			self.expr(plan.body)?
		};
		if value.repr == Repr::Never {
			self.end_scope(scope, None)?;
			return Ok(value);
		}
		let value = if repr.has_value() && value.repr.has_value() {
			self.convert(value, repr, node.index())?
		} else {
			Val::void()
		};
		self.end_scope(scope, None)?;
		Ok(value)
	}

	/// At the entry of a method: reads once the fields of `self` never
	/// written after construction that its body (with the methods expanded
	/// into it) reads more than once or in a loop.
	pub(crate) fn read_frozen_fields(
		&mut self,
		info: &fa_analyzer::analysis::FnInfo,
	) -> Result<(), Error> {
		if self.tasks.asynchronous || self.in_coroutine() || self.in_task_body() {
			return Ok(());
		}
		let Some(receiver) = info.params.first().map(|param| param.symbol) else {
			return Ok(());
		};
		let name = self.analysis.symbols.symbol(receiver).name;
		if self.types().syms.resolve(name) != "self" {
			return Ok(());
		}
		let Some(&slot) = self.slots.get(&receiver) else {
			return Ok(());
		};
		let Repr::Object(owner) = slot.repr else {
			return Ok(());
		};
		if self.written_fields().is_none() {
			return Ok(());
		}
		// the reads of each field, a read in a loop counting twice
		let mut reads: Vec<(u32, usize)> = Vec::new();
		// the method's body, then those of the methods it calls on `self`
		// (each call counts), never one already on the chain of calls
		let mut budget = 32;
		let mut pending = vec![(info.body, receiver, 1usize, vec![info.body])];
		while let Some((body, self_symbol, outer, chain)) = pending.pop() {
			let mut stack = vec![(body.index(), outer)];
			while let Some((at, weight)) = stack.pop() {
				let node = NodeId::new(at);
				let mut child_weight = weight;
				match self.view(node) {
					View::While { .. } | View::For { .. } | View::Loop(_) => child_weight = 2,
					View::Access { target, members }
						if self.analysis.resolution(target) == Resolution::Local(self_symbol) =>
					{
						if let Some(first) = members.first()
							&& let Ok(index) = self.concrete_field_index(owner, first)
						{
							match reads.iter_mut().find(|(field, _)| *field == index) {
								Some((_, count)) => *count += weight,
								None => reads.push((index, weight)),
							}
						}
					}
					View::Call { .. } => {
						if let Some(call) = self.analysis.call(node)
							&& let Callee::Function(function) = call.callee
							&& let Some(Argument::Receiver(callee)) = call.arguments.first()
							&& let View::Access { target, .. } = self.view(*callee)
							&& self.analysis.resolution(target) == Resolution::Local(self_symbol)
							&& let Some(callee_info) = self.analysis.function(function)
							&& let Some(callee_self) = callee_info.params.first()
							&& !chain.contains(&callee_info.body)
							&& chain.len() < 4 && budget > 0
						{
							budget -= 1;
							let mut inner = chain.clone();
							inner.push(callee_info.body);
							pending.push((callee_info.body, callee_self.symbol, weight, inner));
						}
					}
					_ => {}
				}
				self.tree.nodes[at].for_each_child(|child| stack.push((child, child_weight)));
			}
		}
		let function = self.frame().function;
		for (index, count) in reads {
			if count < 2 || !self.frozen_field(owner, index) {
				continue;
			}
			let repr = self.field_repr(owner, index as usize, info.body.index())?;
			if repr != Repr::Str && !self.plain(repr) || !repr.has_value() {
				continue;
			}
			let structure = self.object_type(owner);
			let pointer = self
				.builder
				.build_struct_gep(structure, slot.pointer, index, "")?;
			let value = self.load(Slot { pointer, repr })?;
			if repr == Repr::Str {
				// The whole-program field proof also fixes inline payload
				// storage for this call. Borrow it instead of snapshotting it.
				let (bytes, length) = self.string_place_parts(value, pointer)?;
				self.frozen_string_parts.push(FrozenParts {
					value: value.value.expect("a string"),
					bytes,
					length,
				});
			}
			self.frozen_reads.push(FrozenRead {
				function,
				pointer: slot.pointer,
				index,
				value,
			});
		}
		Ok(())
	}
}
