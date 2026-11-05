//! Destination passing for string results appended to a string:
//! `out += "…{quoted(name)}…"` or `out += quoted(name)` with a function that
//! builds its result in an accumulator,
//!
//! ```fa
//! function quoted = (text: String): String => {
//!     mutable result = "\""
//!     for character in text { … result += character }
//!     result + "\""
//! }
//! ```
//!
//! is lowered at the call as its body with the accumulator standing for the
//! destination: its initial value and every `result += …` append to `out`,
//! and so do the operands after it in the final expression. No string of its
//! own is built, copied, then freed.
//!
//! The function qualifies when nothing but the result can tell the
//! difference (`accumulator_plan`): a function of this file, neither
//! generic, a lambda, asynchronous nor throwing, with no effects, whose
//! block body starts with `mutable result = …` (a string), ends with `result`
//! or `result + …`, uses `result` only as the target of `+=` in between, and
//! has no `return`, `throw`, `try`, closure, task, reference nor call of a
//! function that isn't a built-in. Its parameters are read only (no effect),
//! and the caller checks that no argument reads the destination, so the
//! appends can't be seen by the body.

use fa_analyzer::analysis::{Argument, Callee};
use fa_analyzer::ids::{NodeId, SymbolId, TypeId};
use fa_analyzer::symbols::{Resolution, SymbolKind};
use fa_analyzer::syntax::{ArithOp, BindingKind, View};
use inkwell::values::PointerValue;

use crate::error::Error;
use crate::lower::values::{Repr, Val};
use crate::lower::{Lowering, Slot};

/// How to expand a function into a destination (see the module).
#[derive(Clone)]
pub(crate) struct AccumulatorPlan {
	/// The function.
	function: NodeId,
	/// The accumulator local.
	accumulator: SymbolId,
	/// Its initial value.
	initial: NodeId,
	/// The statements between its declaration and the final expression.
	middle: Vec<NodeId>,
	/// The operands appended after it by the final expression.
	suffix: Vec<NodeId>,
	/// The parameters, in order.
	params: Vec<SymbolId>,
}

impl<'ctx> Lowering<'_, 'ctx> {
	/// The plan of a function whose result can be built in a destination.
	pub(crate) fn accumulator_plan(&self, function: NodeId) -> Option<AccumulatorPlan> {
		let info = self.analysis.function(function)?;
		if !info.generics.is_empty()
			|| info.declaration.is_none()
			|| !info.captures.is_empty()
			|| !info.block_body
			|| info.throws != TypeId::NEVER
			|| info
				.params
				.iter()
				.any(|param| param.default.is_some() || param.variadic)
		{
			return None;
		}
		let sig = self.types().sig(info.sig?);
		let (mutates, invalidates) = self.types().effects(sig.effects);
		if sig.is_async || !mutates.is_empty() || !invalidates.is_empty() {
			return None;
		}
		if self.repr_of(sig.ret, function.index()).ok()? != Repr::Str {
			return None;
		}
		let View::Block { statements } = self.view(info.body) else {
			return None;
		};
		let statements: Vec<NodeId> = statements.iter().collect();
		if statements.len() < 2 {
			return None;
		}
		let first = statements[0];
		let View::Declaration {
			kind: BindingKind::Mutable,
			value: Some(initial),
			..
		} = self.view(first)
		else {
			return None;
		};
		let accumulator = self.analysis.symbols.symbol_of_decl(first)?;
		if self.analysis.symbols.symbol(accumulator).kind != SymbolKind::Mutable {
			return None;
		}
		let last = *statements.last()?;
		let suffix = match self.view(last) {
			View::Name(_) if self.analysis.resolution(last) == Resolution::Local(accumulator) => {
				Vec::new()
			}
			View::Arithmetic {
				op: ArithOp::Add,
				operands,
			} => {
				let operands: Vec<NodeId> = operands.iter().collect();
				let head = *operands.first()?;
				if !matches!(self.view(head), View::Name(_))
					|| self.analysis.resolution(head) != Resolution::Local(accumulator)
				{
					return None;
				}
				operands[1..].to_vec()
			}
			_ => return None,
		};
		// nothing that leaves early, captures, references, or calls user code
		let mut stack = vec![info.body.index()];
		while let Some(at) = stack.pop() {
			let node = NodeId::new(at);
			match self.view(node) {
				View::Return(_)
				| View::Throw(_)
				| View::Try(_)
				| View::Catch { .. }
				| View::Lambda { .. }
				| View::Function { .. }
				| View::Start { .. }
				| View::Await(_)
				| View::Use { .. }
				| View::Reuse { .. }
				| View::Provide { .. } => return None,
				View::Call { .. }
					if !matches!(
						self.analysis.call(node).map(|call| call.callee),
						Some(Callee::Builtin(_) | Callee::Conversion(_))
					) =>
				{
					return None;
				}
				_ => {}
			}
			self.tree.nodes[at].for_each_child(|child| stack.push(child));
		}
		// the accumulator is only appended to between its declaration and the end
		let middle = statements[1..statements.len() - 1].to_vec();
		for &statement in &middle {
			let mut stack = vec![statement.index()];
			while let Some(at) = stack.pop() {
				let node = NodeId::new(at);
				if self.analysis.resolution(node) == Resolution::Local(accumulator) {
					let parent = self.parents().get(at).copied().unwrap_or(usize::MAX);
					let appended = parent != usize::MAX
						&& matches!(self.view(NodeId::new(parent)),
							View::CompoundAssignment { op: ArithOp::Add, target, .. } if target == node);
					if !appended {
						return None;
					}
				}
				self.tree.nodes[at].for_each_child(|child| stack.push(child));
			}
		}
		// the initial value and the suffix don't read it either
		for &node in suffix.iter().chain(std::iter::once(&initial)) {
			let mut stack = vec![node.index()];
			while let Some(at) = stack.pop() {
				if self.analysis.resolution(NodeId::new(at)) == Resolution::Local(accumulator) {
					return None;
				}
				self.tree.nodes[at].for_each_child(|child| stack.push(child));
			}
		}
		Some(AccumulatorPlan {
			function,
			accumulator,
			initial,
			middle,
			suffix,
			params: info.params.iter().map(|param| param.symbol).collect(),
		})
	}

	/// The plan of a call that can append its result into the destination
	/// at `place` (none of its arguments reads it).
	pub(crate) fn destination_call(&self, node: NodeId, place: NodeId) -> Option<AccumulatorPlan> {
		if !matches!(self.view(node), View::Call { .. }) || self.tasks.asynchronous {
			return None;
		}
		let info = self.analysis.call(node)?;
		let Callee::Function(function) = info.callee else {
			return None;
		};
		// not a call of the function being lowered or expanded (its body
		// would expand forever)
		if self.frame().owner == Some(function) || self.expanded_functions.contains(&function) {
			return None;
		}
		if info
			.arguments
			.iter()
			.any(|argument| !matches!(argument, Argument::Given(_)))
		{
			return None;
		}
		let plan = self.accumulator_plan(function)?;
		if plan.params.len() != info.arguments.len()
			|| self.reads_root_of(node, place)
			|| self.reads_outer_values(function)
		{
			return None;
		}
		Some(plan)
	}

	/// Whether a function's body reads a value declared outside of it that
	/// may change, so may be (or alias) a destination: a module-level
	/// `mutable` (`out += quoted("x")` with `quoted` reading `out`), a
	/// reference, a reactive value; or a function of the program given to a
	/// built-in (`items.map(describe)`), whose body may read anything.
	/// Constants and types can't.
	fn reads_outer_values(&self, function: NodeId) -> bool {
		let parents = self.parents();
		let inside = |mut at: usize| loop {
			if at == function.index() {
				return true;
			}
			match parents.get(at) {
				Some(&parent) if parent != usize::MAX => at = parent,
				_ => return false,
			}
		};
		let mut stack = vec![function.index()];
		while let Some(at) = stack.pop() {
			if let Resolution::Local(symbol) = self.analysis.resolution(NodeId::new(at)) {
				let data = self.analysis.symbols.symbol(symbol);
				if !matches!(
					data.kind,
					SymbolKind::Let
						| SymbolKind::Static
						| SymbolKind::Type | SymbolKind::TypeParameter
						| SymbolKind::Namespace
				) && !inside(data.decl_node.index())
				{
					return true;
				}
			}
			self.tree.nodes[at].for_each_child(|child| stack.push(child));
		}
		false
	}

	/// Appends the result of a call (see the module) to `destination`.
	pub(crate) fn append_call_into(
		&mut self,
		destination: PointerValue<'ctx>,
		place: NodeId,
		call: NodeId,
		plan: AccumulatorPlan,
	) -> Result<Val<'ctx>, Error> {
		let info = self.analysis.call(call).expect("a call").clone();
		// the arguments, in order, then the parameters bound to them
		let mut bound = Vec::new();
		for (argument, &param) in info.arguments.iter().zip(&plan.params) {
			let Argument::Given(node) = argument else {
				unreachable!("checked by destination_call");
			};
			let value = self.expr(*node)?;
			if value.repr == Repr::Never {
				return Ok(value);
			}
			let repr = self.repr_of(self.analysis.symbols.symbol(param).ty, node.index())?;
			let value = self.convert(value, repr, node.index())?;
			let slot = self.alloca(repr, "destination_parameter")?;
			if let Some(raw) = value.value {
				self.builder.build_store(slot.pointer, raw)?;
			}
			bound.push((param, slot));
		}
		let mut saved = Vec::new();
		for (param, slot) in bound {
			saved.push((param, self.slots.insert(param, slot)));
		}
		let accumulator = Slot {
			pointer: destination,
			repr: Repr::Str,
		};
		saved.push((
			plan.accumulator,
			self.slots.insert(plan.accumulator, accumulator),
		));
		self.expanded_functions.push(plan.function);
		let result = self.expand_into(destination, place, &plan);
		self.expanded_functions.pop();
		for (symbol, previous) in saved.into_iter().rev() {
			match previous {
				Some(slot) => self.slots.insert(symbol, slot),
				None => self.slots.remove(&symbol),
			};
		}
		result
	}

	fn expand_into(
		&mut self,
		destination: PointerValue<'ctx>,
		place: NodeId,
		plan: &AccumulatorPlan,
	) -> Result<Val<'ctx>, Error> {
		let scope = self.mark();
		if self.append_part(destination, place, plan.initial)?.repr == Repr::Never {
			self.end_scope(scope, None)?;
			return Ok(Val::never());
		}
		for &statement in &plan.middle {
			let mark = self.mark();
			let value = self.expr(statement)?;
			if value.repr == Repr::Never {
				self.end_scope(scope, None)?;
				return Ok(Val::never());
			}
			self.end_statement(mark, None)?;
		}
		for &operand in &plan.suffix {
			if self.append_part(destination, place, operand)?.repr == Repr::Never {
				self.end_scope(scope, None)?;
				return Ok(Val::never());
			}
		}
		self.end_scope(scope, None)?;
		Ok(Val::void())
	}

	/// `destination += node`: a template, a call expanded in turn, or a value.
	pub(crate) fn append_part(
		&mut self,
		destination: PointerValue<'ctx>,
		place: NodeId,
		node: NodeId,
	) -> Result<Val<'ctx>, Error> {
		if let View::Template { parts } = self.view(node) {
			return self.append_template(destination, place, parts);
		}
		if let Some(plan) = self.destination_call(node, place) {
			return self.append_call_into(destination, place, node, plan);
		}
		let mark = self.mark();
		let value = self.expr(node)?;
		if value.repr == Repr::Never {
			return Ok(value);
		}
		let value = if value.repr == Repr::Str {
			value
		} else {
			match self.integer_string(value)? {
				Some(text) => text,
				None => return Err(self.unsupported(node.index(), "this appended value")),
			}
		};
		self.append_string(destination, value)?;
		self.end_statement(mark, None)?;
		Ok(Val::void())
	}
}
