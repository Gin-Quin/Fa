//! A huge top level split into several functions.
//!
//! The top level of the entry file runs in `main`; a generated or very long
//! file makes it one function of hundreds of thousands of instructions,
//! which LLVM takes gigabytes to compile (and which `compile.rs` leaves
//! unoptimized). Above [`SPLIT_ABOVE`] bytes of source, its statements are
//! lowered into functions of about [`PART_BYTES`] bytes each, which `main`
//! calls in order: every function stays small, so the memory of the
//! compilation no longer grows with the size of the top level.
//!
//! - A top-level local read by a later part lives in a global (as the
//!   locals functions read); the others stay in their part.
//! - What the globals own is dropped where `main` drops it (the cleanups
//!   move to `main`); what a part's own locals own, at its end, under the
//!   rule of the drops at exit.
//! - The top level isn't split when a part couldn't hand over what it
//!   started: reactivity, `provide`, asynchronous mode.

use fa_analyzer::ids::{NodeId, SymbolId};
use fa_analyzer::symbols::{Resolution, SymbolKind};
use fa_analyzer::syntax::View;
use inkwell::module::Linkage;
use rustc_hash::FxHashMap;

use super::Lowering;
use super::ownership::Cleanup;
use super::values::Repr;
use crate::error::Error;

/// The source bytes of a top level above which it is split.
const SPLIT_ABOVE: usize = 64 * 1024;

/// The source bytes of a part (at least: a part ends after a statement).
const PART_BYTES: usize = 16 * 1024;

impl<'ctx> Lowering<'_, 'ctx> {
	/// Lowers the statements of the top level, in parts when it is huge.
	pub(crate) fn top_level_statements(&mut self, statements: &[NodeId]) -> Result<(), Error> {
		let parts = self.top_level_parts(statements);
		if parts.len() <= 1 {
			for &statement in statements {
				self.top_level_statement(statement)?;
			}
			return Ok(());
		}
		self.share_across_parts(&parts);
		let void = self.context.void_type().fn_type(&[], false);
		for part in parts {
			if self.terminated() {
				break;
			}
			let function = self
				.module
				.add_function("fa.top_level_part", void, Some(Linkage::Internal));
			// kept apart: the point is that each function stays small
			self.runtime.add_attributes(function, &["nounwind", "noinline"]);
			self.builder.build_call(function, &[], "")?;
			let outer = self.frame.take();
			let block = self.builder.get_insert_block().expect("a block");
			let exit_code = self.exit_code;
			self.begin_function(function, Repr::Void, None);
			for &statement in part {
				self.top_level_statement(statement)?;
			}
			// what the part's own locals own is dropped at its end; what
			// the globals own goes to `main`
			let entries = self.cleanups.take();
			let (globals, locals): (Vec<_>, Vec<_>) = entries.into_iter().partition(
				|entry| matches!(entry, Cleanup::Local(slot) if slot.pointer.is_const()),
			);
			if !self.terminated() {
				self.cleanups.replace(locals);
				if self.leak_check {
					self.drop_to(0)?;
				} else {
					self.drop_at_exit()?;
				}
				self.cleanups.borrow_mut().clear();
			}
			if !self.terminated() {
				self.builder.build_return(None)?;
			}
			self.end_function()?;
			self.frame = outer;
			self.exit_code = exit_code;
			self.builder.position_at_end(block);
			self.cleanups.borrow_mut().extend(globals);
		}
		Ok(())
	}

	/// One statement of the top level, its temporaries dropped.
	fn top_level_statement(&mut self, statement: NodeId) -> Result<(), Error> {
		let mark = self.mark();
		self.statement(statement)?;
		self.end_statement(mark, None)?;
		self.flush_after(statement)
	}

	/// The parts of a top level: one, unless it is huge and can be split.
	fn top_level_parts<'s>(&self, statements: &'s [NodeId]) -> Vec<&'s [NodeId]> {
		let size = |statement: &NodeId| {
			let span = self.tree.spans[statement.index()];
			span.end.saturating_sub(span.start)
		};
		let total: usize = statements.iter().map(size).sum();
		let splittable = total > SPLIT_ABOVE
			&& !self.tasks.asynchronous
			&& self.analysis.watches.is_empty()
			&& !(0..self.tree.nodes.len()).any(|at| {
				matches!(
					self.view(NodeId::new(at)),
					View::Provide { .. } | View::Watch(_) | View::Use { mutable: true, .. }
				)
			}) && !self
			.analysis
			.symbols
			.symbols()
			.iter()
			.any(|symbol| matches!(symbol.kind, SymbolKind::Reactive));
		if !splittable {
			return vec![statements];
		}
		let mut parts = Vec::new();
		let mut start = 0;
		let mut bytes = 0;
		for (index, statement) in statements.iter().enumerate() {
			bytes += size(statement);
			if bytes >= PART_BYTES {
				parts.push(&statements[start..=index]);
				start = index + 1;
				bytes = 0;
			}
		}
		if start < statements.len() {
			parts.push(&statements[start..]);
		}
		parts
	}

	/// Makes the top-level locals that a later part reads globals.
	fn share_across_parts(&mut self, parts: &[&[NodeId]]) {
		let mut first: FxHashMap<SymbolId, usize> = FxHashMap::default();
		let mut shared = Vec::new();
		for (part, statements) in parts.iter().enumerate() {
			for &statement in *statements {
				let mut stack = vec![statement.index()];
				while let Some(at) = stack.pop() {
					if let Resolution::Local(symbol) = self.analysis.resolution(NodeId::new(at)) {
						let data = self.analysis.symbols.symbol(symbol);
						let variable = !matches!(
							data.kind,
							SymbolKind::Function | SymbolKind::Type | SymbolKind::Namespace
						);
						if variable && *first.entry(symbol).or_insert(part) != part {
							shared.push(symbol);
						}
					}
					self.tree.nodes[at].for_each_child(|child| stack.push(child));
				}
			}
		}
		for symbol in shared {
			let scope = self.analysis.symbols.symbol(symbol).scope;
			if self.owner_of_scope(scope).is_none() {
				self.shared.insert(symbol, ());
			}
		}
	}
}
