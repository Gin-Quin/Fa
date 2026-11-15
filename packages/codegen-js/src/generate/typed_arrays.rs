//! Byte arrays: a local `Array(Boolean)` filled by a loop of `push` right
//! after its declaration, and then only indexed (read and written) and
//! measured, is a `Uint8Array` (one byte per item instead of an 8-byte
//! JavaScript value: the sieve's 100,000,001 flags take 100 MB, not 800).
//!
//! ```js
//! let isPrime = new Uint8Array(Math.max(limit + 1, 0)).fill(1);
//! if (isPrime[i] === 1) { … }
//! isPrime[multiple] = false;
//! ```
//!
//! The semantics are the array's: the fill loop runs `max(end, 0)` times,
//! an item read is compared with `1` (a `Boolean` again), a written
//! `Boolean` is stored as `1` or `0` by the typed array, `length` and the
//! index checks (`faIndex`, `faIndexSet`, `at`) work on typed arrays as on
//! arrays. Any other use of the local (a method, a loop over it, a copy, an
//! argument, a capture by a nested function, a reference, a whole write)
//! keeps the plain array, so nothing outside the function sees the typed
//! array.

use std::rc::Rc;

use ethnum::I256;
use fa_analyzer::analysis::{Argument, Callee};
use fa_analyzer::builtins;
use fa_analyzer::ids::{NodeId, SymbolId};
use fa_analyzer::symbols::{Resolution, SymbolKind};
use fa_analyzer::syntax::{BindingKind, View};
use fa_analyzer::types::{CollKind, TypeKind};

use crate::generate::Generator;
use crate::js::{self, Expr, Prec};

/// A declaration and its fill loop, turned into a byte array.
pub(crate) struct ByteFill {
	symbol: SymbolId,
	/// The end of the loop's range.
	end: NodeId,
	inclusive: bool,
	/// The value pushed.
	value: bool,
	/// The capacity given to `Array.from([], capacity = …)`: a storage hint
	/// that JavaScript doesn't take, evaluated before the end of the range.
	capacity: Option<NodeId>,
}

impl Generator<'_> {
	/// `mutable flags: Array(Boolean) = []` (or `Array.from([], …)`)
	/// followed by `for 0..n { flags.push(true) }`: a byte array, when the
	/// rest of the function only indexes and measures the local.
	pub(crate) fn byte_fill(&mut self, declaration: NodeId, next: NodeId) -> Option<ByteFill> {
		let View::Declaration {
			kind: BindingKind::Mutable,
			value: Some(value),
			..
		} = self.view(declaration)
		else {
			return None;
		};
		let symbol = self.analysis.symbols.symbol_of_decl(declaration)?;
		if self.analysis.symbols.symbol(symbol).kind != SymbolKind::Mutable
			|| self.is_module_value(symbol)
		{
			return None;
		}
		let TypeKind::Collection(CollKind::Array, item, _) =
			self.types.kind(self.analysis.symbols.symbol(symbol).ty)
		else {
			return None;
		};
		if !matches!(self.types.kind(item), TypeKind::Boolean) || !self.is_empty_array(value) {
			return None;
		}
		// `for 0..end { flags.push(<literal>) }`
		let View::For {
			pattern: None,
			index: None,
			iterable,
			body,
			parallel: None,
			compile_time: false,
		} = self.view(next)
		else {
			return None;
		};
		let View::Range {
			start: Some(start),
			end: Some(end),
			inclusive,
		} = self.view(self.strip_groups(iterable))
		else {
			return None;
		};
		if !matches!(self.view(start), View::Integer(zero) if zero == I256::ZERO) {
			return None;
		}
		let View::Block { statements } = self.view(body) else {
			return None;
		};
		let [push] = statements.iter().collect::<Vec<_>>()[..] else {
			return None;
		};
		let pushed = self.pushed_literal(push, symbol)?;
		if !self.only_indexed(symbol, declaration, next) {
			return None;
		}
		let capacity = self
			.analysis
			.call(self.strip_groups(value))
			.and_then(|call| match call.arguments.get(1) {
				Some(Argument::Given(capacity)) => Some(*capacity),
				_ => None,
			});
		Some(ByteFill {
			symbol,
			end,
			inclusive,
			value: pushed,
			capacity,
		})
	}

	/// Writes the declaration of a byte array (its fill loop included).
	/// The declared value is never evaluated: `[]`, or `Array.from([], …)`,
	/// whose capacity only keeps its effects (`collection_from`).
	pub(crate) fn byte_array_declaration(&mut self, fill: ByteFill) {
		if let Some(capacity) = fill.capacity {
			let hint = self.expr(capacity);
			if !hint.pure {
				self.out.line(format!("void ({});", hint.text));
			}
		}
		let end = self.expr(fill.end);
		let count = if fill.inclusive {
			let one = Expr::constant("1", Prec::Primary);
			js::binary(&end, "+", &one, Prec::Additive)
		} else {
			end
		};
		let minimum = I256::from(if fill.inclusive { -1 } else { 0 });
		let proven = self
			.interval_of(fill.end)
			.is_some_and(|interval| interval.lo >= minimum);
		let length = if proven {
			count.text
		} else {
			format!("Math.max({}, 0)", count.text)
		};
		let name = self.declare(fill.symbol);
		let fill_call = if fill.value { ".fill(1)" } else { "" };
		self.out
			.line(format!("let {name} = new Uint8Array({length}){fill_call};"));
		self.byte_arrays.insert(fill.symbol);
	}

	/// Whether a node reads a byte array local (an item read is then
	/// compared with `1`).
	pub(crate) fn is_byte_array(&self, node: NodeId) -> bool {
		let node = self.strip_groups(node);
		matches!(self.view(node), View::Name(_))
			&& matches!(
				self.analysis.resolution(node),
				Resolution::Local(symbol) if self.byte_arrays.contains(&symbol)
			)
	}

	/// An item read from a byte array: a `Boolean` again.
	pub(crate) fn byte_item(&self, read: Expr) -> Expr {
		let one = Expr::constant("1", Prec::Primary);
		js::binary(&read, "===", &one, Prec::Equality)
	}

	/// `[]`, or `Array.from([], capacity = …)`.
	fn is_empty_array(&self, value: NodeId) -> bool {
		let value = self.strip_groups(value);
		match self.view(value) {
			View::Array { items } => items.is_empty(),
			View::Call { .. } => {
				let Some(call) = self.analysis.call(value) else {
					return false;
				};
				let Callee::Builtin(builtin) = call.callee else {
					return false;
				};
				builtins::get(builtin).path == "Array.from"
					&& matches!(
						call.arguments.first(),
						Some(Argument::Given(source))
							if matches!(self.view(self.strip_groups(*source)), View::Array { items } if items.is_empty())
					)
			}
			_ => false,
		}
	}

	/// The Boolean literal of `flags.push(true)`.
	fn pushed_literal(&self, push: NodeId, symbol: SymbolId) -> Option<bool> {
		let View::Call { callee, arguments } = self.view(push) else {
			return None;
		};
		let call = self.analysis.call(push)?;
		let Callee::Builtin(builtin) = call.callee else {
			return None;
		};
		if builtins::get(builtin).path != "Array#push" {
			return None;
		}
		let View::Access { target, .. } = self.view(callee) else {
			return None;
		};
		if self.analysis.resolution(self.strip_groups(target)) != Resolution::Local(symbol) {
			return None;
		}
		let [argument] = arguments.iter().collect::<Vec<_>>()[..] else {
			return None;
		};
		match self.view(argument) {
			View::Boolean(value) => Some(value),
			_ => None,
		}
	}

	/// Whether every use of the local, other than its fill loop, is an item
	/// read, an item write (`flags[i] = …`) or `flags.length`, in the
	/// function that declares it.
	fn only_indexed(&mut self, symbol: SymbolId, declaration: NodeId, fill: NodeId) -> bool {
		let fill_span = self.tree.spans[fill.index()];
		let home = self.innermost_function(declaration);
		let parents = self.parents();
		for index in 0..self.tree.nodes.len() {
			let node = NodeId::new(index);
			if self.analysis.resolution(node) != Resolution::Local(symbol)
				|| !matches!(self.view(node), View::Name(_))
			{
				continue;
			}
			let span = self.tree.spans[index];
			if fill_span.start <= span.start && span.end <= fill_span.end {
				continue;
			}
			if self.innermost_function(node) != home {
				return false;
			}
			let Some(parent) = parents[index] else {
				return false;
			};
			let allowed = match self.view(parent) {
				View::Index { target, .. } if target == node => match parents[parent.index()] {
					Some(grand) => match self.view(grand) {
						View::Assignment { target, value, .. } => {
							target == parent || value == Some(parent)
						}
						View::CompoundAssignment { .. }
						| View::UpdateFields { .. }
						| View::Use { .. }
						| View::Reuse { .. }
						| View::Provide { .. } => false,
						View::Index { target, .. } => target != parent,
						View::Access { target, .. } | View::OptionalAccess { target, .. } => {
							target != parent
						}
						_ => true,
					},
					None => false,
				},
				View::Access { target, members } if target == node => {
					members.len() == 1
						&& members
							.first()
							.is_some_and(|member| self.text(member) == "length")
				}
				_ => false,
			};
			if !allowed {
				return false;
			}
		}
		true
	}

	/// The innermost function or lambda around a node (`None`: the
	/// top-level statements).
	fn innermost_function(&self, node: NodeId) -> Option<NodeId> {
		let span = self.tree.spans[node.index()];
		self.analysis
			.functions
			.values()
			.filter(|info| {
				let outer = self.tree.spans[info.lambda.index()];
				outer.start <= span.start && span.end <= outer.end
			})
			.min_by_key(|info| {
				let outer = self.tree.spans[info.lambda.index()];
				outer.end - outer.start
			})
			.map(|info| info.lambda)
	}

	/// The parent of each node (computed once per file).
	pub(crate) fn parents(&mut self) -> Rc<Vec<Option<NodeId>>> {
		if let Some(parents) = &self.parents {
			return parents.clone();
		}
		let mut parents = vec![None; self.tree.nodes.len()];
		for index in 0..self.tree.nodes.len() {
			for child in fa_analyzer::syntax::children(self.tree, NodeId::new(index)) {
				if let Some(slot) = parents.get_mut(child.index()) {
					*slot = Some(NodeId::new(index));
				}
			}
		}
		let parents = Rc::new(parents);
		self.parents = Some(parents.clone());
		parents
	}
}
