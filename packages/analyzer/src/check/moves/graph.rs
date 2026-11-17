//! Runtime control flow for ownership: source order, loop back edges and
//! lexical failure edges. Type syntax and declared function bodies do not execute.

use std::collections::VecDeque;

use rustc_hash::FxHashMap;
use smallvec::{SmallVec, smallvec};

use crate::analysis::NodeFlags;
use crate::check::Walker;
use crate::ids::{NodeId, SymbolId};
use crate::symbols::Resolution;
use crate::syntax::{self, View};

#[derive(Default)]
pub(super) struct Graph {
	pub events: Vec<Event>,
	pub at: FxHashMap<NodeId, Vec<usize>>,
	pub stores: FxHashMap<NodeId, Vec<usize>>,
	pub reachable: Vec<bool>,
	/// Entries of each lexical block activation, including loop bodies.
	blocks: FxHashMap<NodeId, SmallVec<[usize; 1]>>,
}

pub(super) struct Event {
	pub node: NodeId,
	pub reads: SmallVec<[SymbolId; 2]>,
	pub defines: SmallVec<[SymbolId; 1]>,
	pub next: SmallVec<[usize; 2]>,
	previous: SmallVec<[usize; 2]>,
}

#[derive(Clone, Copy)]
struct Exits {
	failure: usize,
	break_: Option<usize>,
	continue_: Option<usize>,
}

struct Builder<'a, 's> {
	walker: &'a Walker<'s>,
	reads: &'a FxHashMap<NodeId, Vec<SymbolId>>,
	commits: &'a FxHashMap<NodeId, Vec<NodeId>>,
	graph: Graph,
}

impl Graph {
	pub fn build(
		walker: &Walker<'_>,
		body: NodeId,
		reads: &FxHashMap<NodeId, Vec<SymbolId>>,
		commits: &FxHashMap<NodeId, Vec<NodeId>>,
	) -> Self {
		let mut builder = Builder {
			walker,
			reads,
			commits,
			graph: Self::default(),
		};
		// The function exit: uncaught failures and returns have no successor.
		builder.event(body, SmallVec::new(), SmallVec::new(), SmallVec::new());
		let entry = builder.expression(
			body,
			0,
			Exits {
				failure: 0,
				break_: None,
				continue_: None,
			},
		);
		let mut graph = builder.graph;
		for from in 0..graph.events.len() {
			for to in graph.events[from].next.clone() {
				graph.events[to].previous.push(from);
			}
		}
		graph.reachable = vec![false; graph.events.len()];
		let mut pending = vec![entry];
		while let Some(index) = pending.pop() {
			if std::mem::replace(&mut graph.reachable[index], true) {
				continue;
			}
			pending.extend(graph.events[index].next.iter().copied());
		}
		graph
	}

	/// Events reachable from this lexical block's entry without executing
	/// the declaration. Starting at the block, rather than a function's
	/// entry, also describes a fresh loop-body activation.
	pub fn before_definition(&self, block: NodeId, symbol: SymbolId) -> Option<Vec<bool>> {
		let mut before = vec![false; self.events.len()];
		let mut pending: Vec<_> = self.blocks.get(&block)?.iter().copied().collect();
		while let Some(index) = pending.pop() {
			if !self.reachable[index]
				|| before[index]
				|| self.events[index].defines.contains(&symbol)
			{
				continue;
			}
			before[index] = true;
			pending.extend(self.events[index].next.iter().copied());
		}
		Some(before)
	}

	pub fn definition_reached(&self, node: NodeId, before: &[bool]) -> Option<bool> {
		let positions = self.at.get(&node)?;
		let mut reached = false;
		for &index in positions {
			if self.reachable[index] {
				if before[index] {
					return Some(false);
				}
				reached = true;
			}
		}
		reached.then_some(true)
	}

	/// One backward traversal per stored owner. A definition stops the old
	/// value's lifetime; a loop can reach the same read on its next round.
	/// The witness is an actual conflicting read, ready for a diagnostic.
	pub fn later_uses(&self, symbol: SymbolId) -> Vec<Option<NodeId>> {
		self.later_uses_where(symbol, |_| true)
	}

	pub fn later_uses_where(
		&self,
		symbol: SymbolId,
		observes: impl Fn(NodeId) -> bool,
	) -> Vec<Option<NodeId>> {
		let mut before = vec![false; self.events.len()];
		let mut after = vec![None; self.events.len()];
		let mut pending = VecDeque::new();
		for (index, event) in self.events.iter().enumerate() {
			if self.reachable[index] && event.reads.contains(&symbol) && observes(event.node) {
				before[index] = true;
				pending.push_back((index, event.node));
			}
		}
		while let Some((index, witness)) = pending.pop_front() {
			for &previous in &self.events[index].previous {
				if !self.reachable[previous] {
					continue;
				}
				after[previous].get_or_insert(witness);
				if !before[previous] && !self.events[previous].defines.contains(&symbol) {
					before[previous] = true;
					pending.push_back((previous, witness));
				}
			}
		}
		after
	}
}

impl Builder<'_, '_> {
	fn event(
		&mut self,
		node: NodeId,
		reads: SmallVec<[SymbolId; 2]>,
		defines: SmallVec<[SymbolId; 1]>,
		next: SmallVec<[usize; 2]>,
	) -> usize {
		let index = self.graph.events.len();
		self.graph.events.push(Event {
			node,
			reads,
			defines,
			next,
			previous: SmallVec::new(),
		});
		self.graph.at.entry(node).or_default().push(index);
		index
	}

	fn sequence(
		&mut self,
		values: impl IntoIterator<Item = NodeId>,
		next: usize,
		exits: Exits,
	) -> usize {
		let values: Vec<_> = values.into_iter().collect();
		values
			.into_iter()
			.rev()
			.fold(next, |next, value| self.expression(value, next, exits))
	}

	fn end(
		&mut self,
		node: NodeId,
		next: usize,
		exits: Exits,
		defines: SmallVec<[SymbolId; 1]>,
	) -> usize {
		let mut tail = next;
		if let Some(stores) = self.commits.get(&node) {
			for &value in stores.iter().rev() {
				let index = self.event(value, SmallVec::new(), SmallVec::new(), smallvec![tail]);
				self.graph.stores.entry(value).or_default().push(index);
				tail = index;
			}
		}
		let reads = self
			.reads
			.get(&node)
			.map_or_else(SmallVec::new, |reads| reads.iter().copied().collect());
		let fails =
			self.walker.a.has_flags(node, NodeFlags::CAN_FAIL)
				&& matches!(
					self.walker.view(node),
					View::Call { .. }
						| View::Index { .. }
						| View::Arithmetic { .. }
						| View::Negate(_) | View::Await(_)
						| View::Assert(_) | View::Copy(_)
				);
		let checked = self.walker.a.has_flags(node, NodeFlags::SINK_CHECK);
		if reads.is_empty() && defines.is_empty() && !fails && !checked {
			return tail;
		}
		let mut next = smallvec![tail];
		if fails || checked {
			next.push(exits.failure);
		}
		self.event(node, reads, defines, next)
	}

	fn place(&mut self, node: NodeId, next: usize, exits: Exits) -> usize {
		match self.walker.view(node) {
			View::Group(inner) => self.place(inner, next, exits),
			View::Name(_) => next,
			_ => self.expression(node, next, exits),
		}
	}

	fn expression(&mut self, node: NodeId, next: usize, exits: Exits) -> usize {
		if self
			.walker
			.a
			.flags_of(node)
			.intersects(NodeFlags::TYPE_EXPR | NodeFlags::UNREACHABLE)
		{
			return next;
		}
		let tree = self.walker.tree.clone();
		match syntax::view(&tree, node) {
			View::TypeDecl { .. } | View::ExportType { .. } => next,
			View::Function { value, .. } | View::ExportFunction { value, .. } => {
				let defines = self
					.walker
					.a
					.symbols
					.symbol_of_decl(node)
					.into_iter()
					.collect();
				let tail = self.end(node, next, exits, defines);
				self.expression(value, tail, exits)
			}
			View::Lambda { .. } | View::Watch(_) => self.end(node, next, exits, SmallVec::new()),
			View::Module { statements } | View::Block { statements } => {
				let tail = self.end(node, next, exits, SmallVec::new());
				let entry = self.sequence(statements.iter(), tail, exits);
				self.graph.blocks.entry(node).or_default().push(entry);
				entry
			}
			View::If {
				condition,
				then_body,
				else_body,
				..
			} => {
				let tail = self.end(node, next, exits, SmallVec::new());
				let a = self.expression(then_body, tail, exits);
				let b = else_body.map_or(tail, |body| self.expression(body, tail, exits));
				let branches = match self.walker.view(condition) {
					View::Boolean(true) => smallvec![a],
					View::Boolean(false) => smallvec![b],
					_ => smallvec![a, b],
				};
				let branch = self.event(node, SmallVec::new(), SmallVec::new(), branches);
				self.expression(condition, branch, exits)
			}
			View::When { subject, arms } => {
				let tail = self.end(node, next, exits, SmallVec::new());
				let mut branches = SmallVec::new();
				for arm in arms.iter() {
					let body = self.expression(arm.value, tail, exits);
					branches.push(body);
				}
				if branches.is_empty() {
					branches.push(tail);
				}
				let branch = self.event(node, SmallVec::new(), SmallVec::new(), branches);
				self.expression(subject, branch, exits)
			}
			View::Logic { operands, .. } | View::Fallback { operands } => {
				let tail = self.end(node, next, exits, SmallVec::new());
				let mut next = tail;
				for value in operands.iter().collect::<Vec<_>>().into_iter().rev() {
					let branch = self.event(
						node,
						SmallVec::new(),
						SmallVec::new(),
						smallvec![next, tail],
					);
					next = self.expression(value, branch, exits);
				}
				next
			}
			View::Catch {
				expression,
				handler,
				..
			} => {
				let tail = self.end(node, next, exits, SmallVec::new());
				let handler = self.expression(handler, tail, exits);
				self.expression(
					expression,
					tail,
					Exits {
						failure: handler,
						..exits
					},
				)
			}
			View::Return(value) => value.map_or(0, |value| self.expression(value, 0, exits)),
			View::Throw(value) => self.expression(value, exits.failure, exits),
			View::Break(value) => value.map_or(exits.break_.unwrap_or(0), |value| {
				self.expression(value, exits.break_.unwrap_or(0), exits)
			}),
			View::Continue => exits.continue_.unwrap_or(0),
			View::While { condition, body } => {
				let head = self.event(node, SmallVec::new(), SmallVec::new(), SmallVec::new());
				let body = self.expression(
					body,
					head,
					Exits {
						break_: Some(next),
						continue_: Some(head),
						..exits
					},
				);
				let branch = self.event(
					node,
					SmallVec::new(),
					SmallVec::new(),
					smallvec![body, next],
				);
				let condition = self.expression(condition, branch, exits);
				self.graph.events[head].next.push(condition);
				head
			}
			View::WhileLet { value, body, .. } => {
				let head = self.event(node, SmallVec::new(), SmallVec::new(), SmallVec::new());
				let body = self.expression(
					body,
					head,
					Exits {
						break_: Some(next),
						continue_: Some(head),
						..exits
					},
				);
				let branch = self.event(
					node,
					SmallVec::new(),
					SmallVec::new(),
					smallvec![body, next],
				);
				let condition = self.expression(value, branch, exits);
				self.graph.events[head].next.push(condition);
				head
			}
			View::For { iterable, body, .. } => {
				let head = self.event(node, SmallVec::new(), SmallVec::new(), SmallVec::new());
				let body = self.expression(
					body,
					head,
					Exits {
						break_: Some(next),
						continue_: Some(head),
						..exits
					},
				);
				self.graph.events[head].next.extend([body, next]);
				self.expression(iterable, head, exits)
			}
			View::Loop(body) => {
				let head = self.event(node, SmallVec::new(), SmallVec::new(), SmallVec::new());
				let body = self.expression(
					body,
					head,
					Exits {
						break_: Some(next),
						continue_: Some(head),
						..exits
					},
				);
				self.graph.events[head].next.push(body);
				head
			}
			View::Declaration { value, .. } => {
				let defines = self
					.walker
					.a
					.symbols
					.symbol_of_decl(node)
					.into_iter()
					.collect();
				let end = self.end(node, next, exits, defines);
				value.map_or(end, |value| self.expression(value, end, exits))
			}
			View::Assignment { target, value, .. } => {
				let defines = if matches!(self.walker.view(target), View::Name(_)) {
					match self.walker.a.resolution(target) {
						Resolution::Local(symbol) => smallvec![symbol],
						_ => SmallVec::new(),
					}
				} else {
					SmallVec::new()
				};
				let end = self.end(node, next, exits, defines);
				let value = value.map_or(end, |value| self.expression(value, end, exits));
				self.place(target, value, exits)
			}
			View::Object { items } | View::TypedObject { members: items, .. } => {
				let tail = self.end(node, next, exits, SmallVec::new());
				let values = items
					.iter()
					.filter_map(|item| match self.walker.view(item) {
						View::Assignment { value, .. } => value,
						View::Function { .. } | View::TypeDecl { .. } => None,
						_ => Some(item),
					})
					.collect::<Vec<_>>();
				self.sequence(values, tail, exits)
			}
			View::Call { callee, arguments } => {
				let end = self.end(node, next, exits, SmallVec::new());
				let arguments = arguments
					.iter()
					.filter_map(|argument| match self.walker.view(argument) {
						View::Assignment { value, .. } => value,
						_ => Some(argument),
					})
					.collect::<Vec<_>>();
				let arguments = self.sequence(arguments, end, exits);
				self.expression(callee, arguments, exits)
			}
			View::Start(inner) if matches!(self.walker.view(inner), View::Do(_)) => {
				self.end(node, next, exits, SmallVec::new())
			}
			View::Test { .. } => next,
			_ => {
				let end = self.end(node, next, exits, SmallVec::new());
				self.sequence(syntax::children(&tree, node), end, exits)
			}
		}
	}
}
