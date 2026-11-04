//! Reactivity (advanced brief §11, `advanced/reactivity.md`): `reactive`
//! values and fields, `derived` values, `watch` blocks and their
//! subscriptions.
//!
//! In the walk:
//!
//! - a `reactive` value is a mutable place with no flow facts (a `watch`
//!   block may change it between two statements); a `derived` value is
//!   read-only (`REA010`) and hoisted, so that a cycle is reported as such;
//! - the value of a `derived` declaration and the body of a `watch` block
//!   are walked as closures ([`ReactiveBody`]): they copy the locals they
//!   capture (`REF021` on a write), and can't throw (`REA002`, `REA004`);
//! - `watch { … }` used as a value is a `Subscription`, move-only:
//!   `stop()` consumes it (`MEM002` on a later use).
//!
//! After the walk, [`Walker::finish_reactivity`] computes the static "may
//! read" and "may write" sets of every function, `derived` value and
//! `watch` block, over every branch and every function they may call
//! (paths from a root, fine-grained like the runtime's sources), and from
//! them: the cycles between derived values (`REA001`), their purity
//! (`REA002`, `REQ005`), a `watch` that writes what it reads (`REA003`),
//! and the tables of the backends (`Analysis::reactive_reads`,
//! `reactive_writes`, `reactive_args`, `flush_after`).

use std::sync::Arc;

use rustc_hash::{FxHashMap, FxHashSet};
use smallvec::SmallVec;

use crate::analysis::{Argument, Callee, ReactivePlace, ReactiveRoot, ReactiveStep, WatchInfo};
use crate::builtins;
use crate::builtins::array::EffectStep;
use crate::check::{BodyCx, FnCx, Walker};
use crate::diagnostics::codes::advanced::{MEM002, REA001, REA002, REA003, REA004, REA010, REQ005};
use crate::flow::{PlaceRoot, Step as FlowStep};
use crate::ids::{NodeId, ScopeId, Sym, SymbolId, TypeId};
use crate::symbols::{Resolution, ScopeKind, SymbolKind};
use crate::syntax::{self, BindingKind, View};
use crate::types::{FieldFlags, TypeFlags, TypeKind};

/// What a closure-like reactive body is.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ReactiveBody {
	/// The value of a `derived` declaration.
	Derived,
	/// The body of a `watch` block.
	Watch,
}

impl Walker<'_> {
	// ── the walk ──────────────────────────────────────────────────────────

	/// The value of the `derived` declaration `n`: computed when it is read,
	/// in a context of its own (a closure that copies the locals it uses).
	pub(super) fn check_derived_value(
		&mut self,
		n: NodeId,
		value: NodeId,
		declared: Option<TypeId>,
	) -> TypeId {
		let parent = self.body.scope;
		let scope = self
			.a
			.symbols
			.scope_for(n, Some(parent), ScopeKind::Function);
		self.reactive_closure(n, scope, ReactiveBody::Derived, |w| {
			let ty = w.check(value, declared);
			if declared.is_none() {
				w.sink_unannotated(value, ty);
			}
			ty
		})
	}

	/// `watch { … }`: a statement, or a `Subscription` when its value is used.
	pub(super) fn check_watch(&mut self, n: NodeId, body: NodeId) -> TypeId {
		let statement = self.is_statement(n);
		let parent = self.body.scope;
		let scope = self
			.a
			.symbols
			.scope_for(body, Some(parent), ScopeKind::Function);
		let mut cleanup = None;
		self.reactive_closure(n, scope, ReactiveBody::Watch, |w| {
			let (ty, last) = w.check_block_value(body, None);
			// a last expression that is a lambda is the cleanup: stored, it escapes
			if let Some(last) = last
				&& matches!(w.view(w.strip_groups(last)), View::Lambda { .. })
			{
				cleanup = Some(w.strip_groups(last));
				w.escape(last, last);
			}
			ty
		});
		self.memo(n, |w| {
			w.a.watches.insert(
				n,
				WatchInfo {
					body,
					cleanup,
					subscription: !statement,
				},
			);
			if statement {
				TypeId::VOID
			} else {
				builtins::reactivity::subscription(&mut w.s.types)
			}
		})
	}

	/// Walks `f` as the body of a closure `node` whose locals are in `scope`:
	/// its captures are copied (`REF021` on a write), errors can't escape it.
	fn reactive_closure(
		&mut self,
		node: NodeId,
		scope: ScopeId,
		kind: ReactiveBody,
		f: impl FnOnce(&mut Self) -> TypeId,
	) -> TypeId {
		let inner = BodyCx {
			scope,
			flow: self.body.flow.fork(),
			functions: Vec::new(),
			loops: Vec::new(),
			flow_buffers: std::mem::take(&mut self.body.flow_buffers),
			requirements: std::mem::take(&mut self.body.requirements),
			handlers: Vec::new(),
			inside_loop: self.body.inside_loop || !self.body.loops.is_empty(),
			provides: self.body.provides.clone(),
		};
		let saved = std::mem::replace(&mut self.body, inner);
		self.body.functions.push(FnCx {
			reactive: Some(kind),
			unsupported_at_start: self.unsupported_seen,
			..FnCx::new(node, None, TypeId::UNSET, scope)
		});
		let ty = f(self);
		let cx = self.body.functions.pop();
		let inner = std::mem::replace(&mut self.body, saved);
		self.body.flow_buffers = inner.flow_buffers;
		self.body.requirements = inner.requirements;
		if let Some(cx) = &cx {
			self.finish_captures(cx, node, None);
			// the reactive values it may write lose their facts between statements
			if kind == ReactiveBody::Watch {
				for entry in &cx.effects {
					if let Some(symbol) = self.place_symbol(entry.place)
						&& self.a.symbols.symbol(symbol).kind == SymbolKind::Reactive
					{
						self.watch_writes.insert(symbol);
					}
				}
			}
		}
		// it outlives the statement that creates it: it copies what it captures
		self.escape_closure(node, node);
		ty
	}

	/// Errors leaving a `watch` block or a `derived` value (`try`, `throw`
	/// at `n`): `REA004` / `REA002`. Whether it was one of them.
	pub(crate) fn reactive_propagate(&mut self, n: NodeId, errors: TypeId) -> bool {
		let Some(kind) = self.body.functions.last().and_then(|cx| cx.reactive) else {
			return false;
		};
		let errors = self.display(errors);
		let diagnostic = match kind {
			ReactiveBody::Watch => self.diagnostic(
				&REA004,
				n,
				format!(
					"an error can't escape a `watch` block, and this can fail with `{errors}`: handle it with `catch`"
				),
			),
			ReactiveBody::Derived => self.diagnostic(
				&REA002,
				n,
				format!(
					"a derived value must be pure, and this can fail with `{errors}`: handle it with `catch`"
				),
			),
		};
		self.report_flow(diagnostic);
		true
	}

	/// After a statement of the outermost running code (the top level, a
	/// test): the pending `watch` blocks may run, so the reactive values
	/// they may write lose their facts.
	pub(crate) fn after_outer_statement(&mut self) {
		if self.watch_writes.is_empty() || !self.body.functions.iter().all(|cx| cx.test) {
			return;
		}
		let mut symbols: Vec<SymbolId> = self.watch_writes.iter().copied().collect();
		symbols.sort();
		for symbol in symbols {
			let place = self.places.symbol(symbol);
			self.drop_place_facts(place);
		}
	}

	/// `REA010` when `symbol` is a `derived` value being written. Type part.
	pub(super) fn report_derived_write(&mut self, target: NodeId, symbol: SymbolId) -> bool {
		if self.a.symbols.symbol(symbol).kind != SymbolKind::Derived {
			return false;
		}
		let name = self
			.s
			.types
			.syms
			.resolve(self.a.symbols.symbol(symbol).name)
			.to_string();
		let diagnostic = self.diagnostic(
			&REA010,
			target,
			format!(
				"`{name}` is a derived value: it is computed from the reactive values it reads, change them instead"
			),
		);
		self.report(diagnostic);
		true
	}

	// ── subscriptions ─────────────────────────────────────────────────────

	/// `subscription.stop()` consumes the local (flow part of a call).
	pub(crate) fn note_stop_call(&mut self, n: NodeId, callee: NodeId) {
		if !self.body.flow.reachable {
			return;
		}
		let View::Access { target, members } = self.view(callee) else {
			return;
		};
		let Some(member) = members.last() else {
			return;
		};
		let Resolution::Builtin(builtin) = self.a.resolution(member) else {
			return;
		};
		// `subscription.stop()`, `worker.stop()` (concurrency.rs)
		if !matches!(
			builtins::get(builtin).path,
			"Subscription#stop"
				| "Worker#stop"
				| "TcpListener#close"
				| "TcpStream#close"
				| "WorkerPool#shutdown"
		) {
			return;
		}
		let target = self.strip_groups(target);
		if let Resolution::Local(symbol) = self.a.resolution(target)
			&& !self
				.body
				.flow
				.consumed
				.iter()
				.any(|&(other, _)| other == symbol)
		{
			self.body.flow.consumed.push((symbol, n));
		}
	}

	/// A declaration gives its symbol a new value: it is not consumed.
	pub(crate) fn reset_consumed(&mut self, symbol: SymbolId) {
		self.body
			.flow
			.consumed
			.retain(|(other, _)| *other != symbol);
	}

	/// `MEM002`: the read of a consumed local (flow part of a name).
	pub(crate) fn check_consumed(&mut self, n: NodeId) {
		if !self.body.flow.reachable {
			return;
		}
		let Resolution::Local(symbol) = self.a.resolution(n) else {
			return;
		};
		// a local a field was moved out of, used as a whole (foreign.rs)
		if !self.access_roots.contains(&n)
			&& let Some(at) = self.moved_field_of(symbol)
		{
			let name = self.tree.text(n.index()).to_string();
			let line = self.line_of(at);
			let message = format!(
				"`{}` was moved out on line {line}: assign it again before using `{name}` as a whole",
				self.tree.text(at.index())
			);
			let diagnostic = self.diagnostic(&MEM002, n, message);
			self.report_flow(diagnostic);
			return;
		}
		if self.body.flow.consumed.is_empty() {
			return;
		}
		let Some(&(_, at)) = self
			.body
			.flow
			.consumed
			.iter()
			.find(|&&(other, _)| other == symbol)
		else {
			return;
		};
		let name = self.tree.text(n.index()).to_string();
		let line = self.line_of(at);
		let worker = matches!(
			self.s.types.kind(self.a.symbols.symbol(symbol).ty),
			crate::types::TypeKind::Sync(crate::types::SyncKind::Worker, _)
		);
		let task = matches!(
			self.s.types.kind(self.a.symbols.symbol(symbol).ty),
			crate::types::TypeKind::Task(..)
		);
		let message = if task && matches!(self.view(at), View::Await(_)) {
			format!(
				"`{name}` was awaited on line {line}, which consumed it: a task gives its result once"
			)
		} else if task {
			format!(
				"`{name}` was moved on line {line}: a task can't be copied, and the moved task can't be used afterwards"
			)
		} else if matches!(self.view(at), View::Call { .. }) {
			format!(
				"`{name}` was stopped on line {line}, which consumed it: it can't be used afterwards"
			)
		} else if worker {
			format!(
				"`{name}` was moved on line {line}: a worker's handle can't be copied, and the moved handle can't be used afterwards"
			)
		} else if self.s.types.is_resource(self.a.symbols.symbol(symbol).ty) {
			format!(
				"`{name}` was moved on line {line}: its type has `destroy`, so it can't be copied, and the moved value can't be used afterwards"
			)
		} else {
			format!(
				"`{name}` was moved on line {line}: it holds a task, a worker's handle or a resource, so it can't be copied, and the moved value can't be used afterwards"
			)
		};
		let diagnostic = self.diagnostic(&MEM002, n, message);
		self.report_flow(diagnostic);
	}

	// ── after the walk ────────────────────────────────────────────────────

	/// The static sets and the backend tables (see the module documentation).
	pub(crate) fn finish_reactivity(&mut self) {
		let tree = Arc::clone(&self.tree);
		let root = NodeId::new(tree.root);
		// a file without reactivity needs nothing (found by its scan, scan.rs)
		if !self.s.source_file(self.file).scan.has_reactivity {
			return;
		}
		let mut pass = Pass {
			summaries: FxHashMap::default(),
			in_progress: FxHashSet::default(),
			derived: Vec::new(),
			watches: Vec::new(),
			edges: Vec::new(),
		};

		// every function (their bodies are instrumented once)
		let mut functions: Vec<NodeId> = self
			.a
			.functions
			.iter()
			.filter(|(key, info)| info.declaration == Some(**key))
			.map(|(key, _)| *key)
			.collect();
		functions.sort();
		for function in functions {
			pass.summary(self, function);
		}
		// the outermost running code: statement by statement
		let View::Module { statements } = syntax::view(&tree, root) else {
			return;
		};
		let entry = self.entry_function(statements.iter());
		for statement in statements.iter() {
			self.outer_statement(&mut pass, statement, entry);
		}
		// the derived values and the watch blocks met
		// (a derived value or a watch block may declare others)
		let (mut derived, mut watches) = (0, 0);
		while derived < pass.derived.len() || watches < pass.watches.len() {
			if derived < pass.derived.len() {
				let (declaration, value, function) = pass.derived[derived];
				self.finish_derived(&mut pass, declaration, value, function);
				derived += 1;
			} else {
				let (watch, function) = pass.watches[watches];
				self.finish_watch(&mut pass, watch, function);
				watches += 1;
			}
		}
		self.report_derived_cycles(&pass);
	}

	/// The entry function of a program file: `export function = …`.
	fn entry_function(&self, mut statements: impl Iterator<Item = NodeId>) -> Option<NodeId> {
		if self.s.role(self.file) != crate::modules::FileRole::Program {
			return None;
		}
		statements.find(|&statement| matches!(self.view(statement), View::ExportFunction { .. }))
	}

	/// A statement of the outermost running code: its writes are flushed
	/// after it. A test's body and the entry function's are made of such
	/// statements.
	fn outer_statement(&mut self, pass: &mut Pass, statement: NodeId, entry: Option<NodeId>) {
		match self.view(statement) {
			View::Test { body, .. } => {
				if let View::Block { statements } = self.view(body) {
					let statements: Vec<NodeId> = statements.iter().collect();
					for inner in statements {
						self.outer_statement(pass, inner, None);
					}
				}
				return;
			}
			View::Function { .. } | View::TypeDecl { .. } => return,
			View::ExportFunction { .. } if Some(statement) != entry => return,
			View::ExportFunction { .. } => {
				let body = self.a.function(statement).map(|info| info.body);
				if let Some(body) = body
					&& let View::Block { statements } = self.view(body)
				{
					let statements: Vec<NodeId> = statements.iter().collect();
					for inner in statements {
						self.outer_statement(pass, inner, None);
					}
				}
				return;
			}
			_ => {}
		}
		let mut effects = Effects::default();
		let cx = VisitCx {
			function: None,
			tracked: false,
		};
		pass.visit(self, statement, cx, &mut effects);
		if effects.writes_reactive || effects.creates_watch {
			self.a.flush_after.insert(statement);
		}
	}

	/// A `derived` value: its purity (`REA002`, `REQ005`) and what it reads.
	fn finish_derived(
		&mut self,
		pass: &mut Pass,
		declaration: NodeId,
		value: NodeId,
		function: Option<NodeId>,
	) {
		let mut effects = Effects::default();
		let cx = VisitCx {
			function,
			tracked: true,
		};
		pass.visit(self, value, cx, &mut effects);
		let span = self.span(value);
		let local = |w: &Self, symbol: SymbolId| {
			let declared = w.span(w.a.symbols.symbol(symbol).decl_node);
			span.start <= declared.start && declared.end <= span.end
		};
		let name = match self.view(declaration) {
			View::Declaration { name, .. } => name,
			_ => return,
		};
		let at = syntax::name_span(name);
		if let Some(node) = effects.requires {
			let line = self.line_of(node);
			let diagnostic = self.diagnostic_at(
				&REQ005,
				at,
				format!(
					"a derived value can't use requirements (line {line}): give the value as a reactive value instead"
				),
			);
			self.report(diagnostic);
		} else {
			// an outside effect, or a write of something it didn't declare
			let impure = effects.impure.clone().or_else(|| {
				effects
					.writes
					.iter()
					.find_map(|(path, node)| match path.root {
						SRoot::Symbol(symbol) if !local(self, symbol) => {
							let name = self.symbol_name(symbol);
							Some((*node, format!("it changes `{name}`")))
						}
						SRoot::Param(_) | SRoot::Temp | SRoot::Symbol(_) => None,
					})
			});
			if let Some((node, what)) = impure {
				let line = self.line_of(node);
				let diagnostic = self.diagnostic_at(
					&REA002,
					at,
					format!(
						"a derived value must be pure, but {what} (line {line}): put side effects in a `watch` block"
					),
				);
				self.report(diagnostic);
			}
		}
		// the derived values it reads (REA001)
		let symbol = self.a.symbols.symbol_of_decl(declaration);
		if let Some(symbol) = symbol {
			let reads: Vec<SymbolId> = effects
				.reads
				.iter()
				.filter_map(|path| match path.root {
					SRoot::Symbol(read)
						if self.a.symbols.symbol(read).kind == SymbolKind::Derived =>
					{
						Some(read)
					}
					_ => None,
				})
				.collect();
			pass.edges_push(symbol, declaration, reads);
		}
	}

	/// `REA001`: a cycle between derived values, reported once, on its first
	/// declaration.
	fn report_derived_cycles(&mut self, pass: &Pass) {
		let edges: FxHashMap<SymbolId, &Vec<SymbolId>> = pass
			.edges
			.iter()
			.map(|(symbol, _, reads)| (*symbol, reads))
			.collect();
		let mut reported: FxHashSet<SymbolId> = FxHashSet::default();
		let mut ordered: Vec<(NodeId, SymbolId)> = pass
			.edges
			.iter()
			.map(|(symbol, declaration, _)| (*declaration, *symbol))
			.collect();
		ordered.sort();
		for (declaration, symbol) in ordered {
			if reported.contains(&symbol) {
				continue;
			}
			// a path from `symbol` back to itself
			let Some(cycle) = find_cycle(symbol, &edges) else {
				continue;
			};
			reported.extend(cycle.iter().copied());
			let names: Vec<String> = cycle
				.iter()
				.map(|&member| format!("`{}`", self.symbol_name(member)))
				.collect();
			let message = match names.len() {
				1 => format!(
					"cyclic dependency: {} reads itself, in some branch or through a function it calls",
					names[0]
				),
				_ => format!(
					"cyclic dependency between {}: a derived value can't depend on itself, even in a branch that doesn't run",
					join_names(&names)
				),
			};
			let name = match self.view(declaration) {
				View::Declaration { name, .. } => syntax::name_span(name),
				_ => self.span(declaration),
			};
			let diagnostic = self.diagnostic_at(&REA001, name, message);
			self.report(diagnostic);
		}
	}

	/// A `watch` block: `REA003` when it may write a reactive place it may read.
	fn finish_watch(&mut self, pass: &mut Pass, watch: NodeId, function: Option<NodeId>) {
		let Some(info) = self.a.watches.get(&watch).copied() else {
			return;
		};
		let mut effects = Effects::default();
		let tracked = VisitCx {
			function,
			tracked: true,
		};
		let untracked = VisitCx {
			function,
			tracked: false,
		};
		let View::Block { statements } = self.view(info.body) else {
			return;
		};
		let statements: Vec<NodeId> = statements.iter().collect();
		for statement in statements {
			if Some(self.strip_groups(statement)) == info.cleanup {
				// the cleanup runs untracked, before the next run
				let mut ignored = Effects::default();
				pass.visit(self, statement, untracked, &mut ignored);
				continue;
			}
			pass.visit(self, statement, tracked, &mut effects);
		}
		let span = self.span(info.body);
		for (written, node) in &effects.writes {
			if !self.path_is_reactive(written) {
				continue;
			}
			// a local of the block itself
			if let SRoot::Symbol(symbol) = written.root {
				let declared = self.span(self.a.symbols.symbol(symbol).decl_node);
				if span.start <= declared.start && declared.end <= span.end {
					continue;
				}
			}
			if effects
				.reads
				.iter()
				.any(|read| self.path_is_reactive(read) && paths_overlap(written, read))
			{
				let name = self.path_text(written);
				let diagnostic = self.diagnostic(
					&REA003,
					*node,
					format!(
						"this `watch` block writes `{name}`, which it reads: it would run forever. Compute the value with `derived` instead"
					),
				);
				self.report(diagnostic);
				break;
			}
		}
	}

	/// Whether a static path designates a reactive place (or a derived value).
	fn path_is_reactive(&self, path: &SPath) -> bool {
		let root = match path.root {
			SRoot::Symbol(symbol) => matches!(
				self.a.symbols.symbol(symbol).kind,
				SymbolKind::Reactive | SymbolKind::Derived
			),
			SRoot::Param(_) | SRoot::Temp => false,
		};
		root || path
			.steps
			.iter()
			.any(|step| matches!(step, SStep::Field(_, true)))
	}

	/// A path as written, for messages.
	fn path_text(&self, path: &SPath) -> String {
		let mut text = match path.root {
			SRoot::Symbol(symbol) | SRoot::Param(symbol) => self.symbol_name(symbol),
			SRoot::Temp => "a value".to_string(),
		};
		for step in &path.steps {
			match step {
				SStep::Field(name, _) => {
					text.push('.');
					text.push_str(self.s.types.syms.resolve(*name));
				}
				SStep::Index(Some(index)) => text.push_str(&format!("[{index}]")),
				SStep::Index(None) => text.push_str("[…]"),
				SStep::Length => text.push_str(".length"),
				SStep::Size => text.push_str(".size"),
			}
		}
		text
	}

	/// The name of a symbol.
	fn symbol_name(&self, symbol: SymbolId) -> String {
		self.s
			.types
			.syms
			.resolve(self.a.symbols.symbol(symbol).name)
			.to_string()
	}
}

/// `a`, `a and b`, `a, b and c`.
fn join_names(names: &[String]) -> String {
	match names {
		[] => String::new(),
		[one] => one.clone(),
		[rest @ .., last] => format!("{} and {last}", rest.join(", ")),
	}
}

/// A cycle through `start` in the graph of derived values, in order.
fn find_cycle(
	start: SymbolId,
	edges: &FxHashMap<SymbolId, &Vec<SymbolId>>,
) -> Option<Vec<SymbolId>> {
	// depth-first search keeping the path
	let mut path: Vec<(SymbolId, usize)> = vec![(start, 0)];
	let mut visited: FxHashSet<SymbolId> = FxHashSet::default();
	visited.insert(start);
	while let Some(&mut (node, ref mut next)) = path.last_mut() {
		let targets = edges
			.get(&node)
			.map_or(&[][..], |targets| targets.as_slice());
		if *next >= targets.len() {
			path.pop();
			continue;
		}
		let target = targets[*next];
		*next += 1;
		if target == start {
			return Some(path.iter().map(|&(node, _)| node).collect());
		}
		if visited.insert(target) {
			path.push((target, 0));
		}
	}
	None
}

// ── static paths ──────────────────────────────────────────────────────────

/// The root of a static path.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
enum SRoot {
	/// A local or module-level value (or a derived value, read whole).
	Symbol(SymbolId),
	/// A parameter of the function being summarized.
	Param(SymbolId),
	/// A temporary (a call result): it overlaps nothing.
	Temp,
}

/// A step of a static path.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
enum SStep {
	/// A field, and whether it is `reactive`.
	Field(Sym, bool),
	/// An element: its index when it is a literal.
	Index(Option<i128>),
	/// `.length`.
	Length,
	/// `.size`.
	Size,
}

/// A path a code may read or write.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct SPath {
	root: SRoot,
	steps: SmallVec<[SStep; 4]>,
}

/// Whether two paths may designate overlapping places: one is a prefix of
/// the other, step by step (two literal indices that differ, two fields,
/// a length and an element are distinct).
fn paths_overlap(a: &SPath, b: &SPath) -> bool {
	if a.root != b.root || a.root == SRoot::Temp {
		return false;
	}
	for (x, y) in a.steps.iter().zip(&b.steps) {
		let distinct = match (x, y) {
			(SStep::Field(a, _), SStep::Field(b, _)) => a != b,
			(SStep::Index(Some(a)), SStep::Index(Some(b))) => a != b,
			(SStep::Index(_), SStep::Index(_)) => false,
			(SStep::Length, SStep::Length) | (SStep::Size, SStep::Size) => false,
			_ => true,
		};
		if distinct {
			return false;
		}
	}
	true
}

/// What a code may do, for reactivity.
#[derive(Clone, Debug, Default)]
struct Effects {
	/// The paths it may read (reactive ones, parameters, derived values).
	reads: Vec<SPath>,
	/// The paths it may write, with the write.
	writes: Vec<(SPath, NodeId)>,
	/// The first outside effect it may have (`console`, a module-level
	/// `mutable` read), for `REA002`.
	impure: Option<(NodeId, String)>,
	/// The first use of a requirement (`REQ005`).
	requires: Option<NodeId>,
	/// It may write a reactive place: a flush follows.
	writes_reactive: bool,
	/// It may create a `watch` block (which runs at once).
	creates_watch: bool,
}

impl Effects {
	fn read(&mut self, path: SPath) {
		if !self.reads.contains(&path) {
			self.reads.push(path);
		}
	}

	fn write(&mut self, path: SPath, node: NodeId) {
		if !self.writes.iter().any(|(other, _)| *other == path) {
			self.writes.push((path, node));
		}
	}

	fn impure(&mut self, node: NodeId, what: impl Into<String>) {
		if self.impure.is_none() {
			self.impure = Some((node, what.into()));
		}
	}
}

/// The summary of a function: its effects outside of its locals.
#[derive(Clone, Debug, Default)]
struct Summary {
	effects: Effects,
	/// Its parameters, in `FnInfo::params` order.
	params: Vec<SymbolId>,
}

/// Where a node is visited.
#[derive(Copy, Clone, Debug)]
struct VisitCx {
	/// The function whose parameters are [`ReactiveRoot::Param`] roots.
	function: Option<NodeId>,
	/// The code may run while a derived value or a watch block is computed:
	/// its reads are tracked.
	tracked: bool,
}

/// A place expression, followed from its root.
#[derive(Clone, Debug)]
struct Chain {
	/// The static path.
	path: SPath,
	/// The innermost reactive root and the steps below it.
	place: Option<ReactivePlace>,
	/// The outer reactive roots (a reactive field of a reactive value): a
	/// write notifies them too.
	outer: Vec<ReactivePlace>,
	/// The nodes evaluated along the way: indices, a temporary target.
	values: Vec<NodeId>,
}

/// The state of the pass.
struct Pass {
	summaries: FxHashMap<NodeId, Summary>,
	in_progress: FxHashSet<NodeId>,
	/// The derived declarations met, with their value and the function
	/// they are declared in.
	derived: Vec<(NodeId, NodeId, Option<NodeId>)>,
	/// The watch blocks met, with the function they are declared in.
	watches: Vec<(NodeId, Option<NodeId>)>,
	/// The derived values each derived value reads: `(symbol, declaration, reads)`.
	edges: Vec<(SymbolId, NodeId, Vec<SymbolId>)>,
}

impl Pass {
	fn edges_push(&mut self, symbol: SymbolId, declaration: NodeId, reads: Vec<SymbolId>) {
		self.edges.push((symbol, declaration, reads));
	}

	/// The summary of a function (by its declaration node): its body is
	/// visited once (and instrumented), with its parameters as roots.
	fn summary(&mut self, w: &mut Walker<'_>, function: NodeId) -> Summary {
		if let Some(summary) = self.summaries.get(&function) {
			return summary.clone();
		}
		let Some(info) = w.a.function(function).cloned() else {
			return Summary::default();
		};
		if !self.in_progress.insert(function) {
			// recursion: what is known so far
			return Summary {
				params: info.params.iter().map(|param| param.symbol).collect(),
				..Summary::default()
			};
		}
		let cx = VisitCx {
			function: Some(function),
			tracked: true,
		};
		let mut effects = Effects::default();
		for param in &info.params {
			if let Some(default) = param.default {
				self.visit(w, default, cx, &mut effects);
			}
		}
		self.visit(w, info.body, cx, &mut effects);
		// what stays inside: its own locals
		let span = w.span(info.lambda);
		let outside = |w: &Walker<'_>, path: &SPath| match path.root {
			SRoot::Symbol(symbol) => {
				let declared = w.span(w.a.symbols.symbol(symbol).decl_node);
				!(span.start <= declared.start && declared.end <= span.end)
			}
			SRoot::Param(_) => true,
			SRoot::Temp => false,
		};
		effects.reads.retain(|path| outside(w, path));
		effects.writes.retain(|(path, _)| outside(w, path));
		self.in_progress.remove(&function);
		let summary = Summary {
			effects,
			params: info.params.iter().map(|param| param.symbol).collect(),
		};
		self.summaries.insert(function, summary.clone());
		summary
	}

	/// Visits a node: its reads and writes go to `effects`, and to the
	/// tables of the backends.
	fn visit(&mut self, w: &mut Walker<'_>, n: NodeId, cx: VisitCx, effects: &mut Effects) {
		let tree = Arc::clone(&w.tree);
		match syntax::view(&tree, n) {
			View::Name(_) | View::Access { .. } | View::Index { .. } | View::Group(_) => {
				if let Some(chain) = self.chain(w, n, cx) {
					self.read(w, n, &chain, cx, effects);
					for value in chain.values.clone() {
						self.visit(w, value, cx, effects);
					}
					return;
				}
				// a name of a function used as a value: what it may do
				if let View::Name(_) = syntax::view(&tree, n)
					&& let Resolution::Local(symbol) = w.a.resolution(n)
					&& let Some(declaration) = w.direct_function(symbol)
				{
					let summary = self.summary(w, declaration);
					merge_outside(effects, &summary.effects);
					return;
				}
				if let View::Name(_) = syntax::view(&tree, n)
					&& let Resolution::Builtin(builtin) = w.a.resolution(n)
					&& let Some(what) = builtins::std_library::impurity(builtins::get(builtin).path)
				{
					effects.impure(n, format!("it uses {what}"));
				}
			}
			View::Call { callee, .. } => return self.call(w, n, callee, cx, effects),
			View::Assignment {
				target,
				value: Some(value),
				..
			} => {
				self.visit(w, value, cx, effects);
				self.write_target(w, n, target, cx, effects, false);
				return;
			}
			View::CompoundAssignment { target, value, .. } => {
				self.visit(w, value, cx, effects);
				self.write_target(w, n, target, cx, effects, true);
				return;
			}
			View::Declaration {
				kind: BindingKind::Derived,
				value,
				..
			} => {
				// computed in a context of its own
				if let Some(value) = value
					&& !self
						.derived
						.iter()
						.any(|&(declaration, ..)| declaration == n)
				{
					self.derived.push((n, value, cx.function));
				}
				return;
			}
			View::Watch(_) => {
				// it runs at once, in a context of its own
				effects.creates_watch = true;
				if !self.watches.iter().any(|&(watch, _)| watch == n) {
					self.watches.push((n, cx.function));
				}
				return;
			}
			// a local function: visited where it is called
			View::Function { .. } | View::TypeDecl { .. } => return,
			View::Provide { .. } => effects.impure(n, "it provides a requirement"),
			_ => {}
		}
		for child in syntax::children(&tree, n) {
			self.visit(w, child, cx, effects);
		}
	}

	/// A read of the place expression `n`.
	fn read(
		&mut self,
		w: &mut Walker<'_>,
		n: NodeId,
		chain: &Chain,
		cx: VisitCx,
		effects: &mut Effects,
	) {
		if let SRoot::Symbol(symbol) = chain.path.root {
			let data = w.a.symbols.symbol(symbol);
			let module = matches!(
				w.a.symbols.scope(data.scope).kind,
				ScopeKind::Module | ScopeKind::Namespace
			);
			match data.kind {
				// a reactive field of a module-level object is a reactive place
				SymbolKind::Mutable if module && !w.path_is_reactive(&chain.path) => {
					let name = w.symbol_name(symbol);
					effects.impure(n, format!("it reads the module-level mutable `{name}`"));
				}
				SymbolKind::Requirement { .. } if effects.requires.is_none() => {
					effects.requires = Some(n);
				}
				_ => {}
			}
		}
		if w.path_is_reactive(&chain.path) || matches!(chain.path.root, SRoot::Param(_)) {
			effects.read(chain.path.clone());
		}
		if cx.tracked
			&& let Some(place) = &chain.place
		{
			w.a.reactive_reads.insert(n, place.clone());
		}
	}

	/// The write of an assignment target (`compound`: read too).
	fn write_target(
		&mut self,
		w: &mut Walker<'_>,
		n: NodeId,
		target: NodeId,
		cx: VisitCx,
		effects: &mut Effects,
		compound: bool,
	) {
		let Some(chain) = self.chain(w, target, cx) else {
			self.visit(w, target, cx, effects);
			return;
		};
		for value in chain.values.clone() {
			self.visit(w, value, cx, effects);
		}
		if compound {
			self.read(w, target, &chain, cx, effects);
		}
		self.write(w, n, &chain, effects);
	}

	/// A write of the place of `chain` by node `n`.
	fn write(&mut self, w: &mut Walker<'_>, n: NodeId, chain: &Chain, effects: &mut Effects) {
		if w.path_is_reactive(&chain.path) {
			effects.writes_reactive = true;
		}
		effects.write(chain.path.clone(), n);
		let mut places: Vec<ReactivePlace> = chain.outer.clone();
		places.extend(chain.place.clone());
		if !places.is_empty() {
			let entry = w.a.reactive_writes.entry(n).or_default();
			for place in places {
				if !entry.contains(&place) {
					entry.push(place);
				}
			}
		}
	}

	/// A call: its callee's effects, with the arguments' places substituted.
	fn call(
		&mut self,
		w: &mut Walker<'_>,
		n: NodeId,
		callee: NodeId,
		cx: VisitCx,
		effects: &mut Effects,
	) {
		let Some(info) = w.a.call(n).cloned() else {
			for child in syntax::children(&Arc::clone(&w.tree), n) {
				self.visit(w, child, cx, effects);
			}
			return;
		};
		match info.callee {
			Callee::Builtin(builtin) => {
				let path = builtins::get(builtin).path;
				if let Some(what) = builtins::std_library::impurity(path) {
					effects.impure(n, format!("it uses {what}"));
				}
				if path == "Subscription#stop" {
					effects.impure(n, "it stops a `watch` block");
				}
				// a method: the receiver is read, or written by its effects
				let tree = Arc::clone(&w.tree);
				if path.contains('#')
					&& let View::Access { target, members } = syntax::view(&tree, callee)
					&& !members.is_empty()
				{
					let receiver = self.chain_members(w, callee, target, members.len() - 1, cx);
					match receiver {
						Some(chain) => {
							for value in chain.values.clone() {
								self.visit(w, value, cx, effects);
							}
							let operation = builtins::get(builtin).name();
							let receiver_ty = match members.len() {
								1 => w.type_of(w.strip_groups(target)),
								count => members
									.get(count - 2)
									.map_or(TypeId::UNKNOWN, |member| w.type_of(member)),
							};
							let written = self.builtin_writes(
								w,
								&chain,
								receiver_ty,
								operation,
								&info.arguments,
							);
							// a read of the receiver as a whole: tracked on the callee
							if written.is_empty() {
								self.read(w, callee, &chain, cx, effects);
							}
							for written in written {
								self.write(w, n, &written, effects);
							}
						}
						None => self.visit(w, target, cx, effects),
					}
				} else if !matches!(w.view(callee), View::Name(_)) {
					self.visit(w, callee, cx, effects);
				}
				self.visit_arguments(w, &info.arguments, cx, effects);
			}
			Callee::Function(declaration) => {
				let summary = self.summary(w, declaration);
				merge_outside(effects, &summary.effects);
				let requires = match w.a.function(declaration).and_then(|function| function.sig) {
					Some(sig) => {
						let list = w.s.types.sig(sig).requires;
						!w.s.types.requirements(list).is_empty()
					}
					None => false,
				};
				if requires && effects.requires.is_none() {
					effects.requires = Some(n);
				}
				// a method's callee: its receiver is an argument
				if !matches!(w.view(callee), View::Name(_) | View::Access { .. }) {
					self.visit(w, callee, cx, effects);
				}
				let mut places: Vec<Option<ReactivePlace>> = Vec::new();
				for (index, argument) in info.arguments.iter().enumerate() {
					let param = summary.params.get(index).copied();
					let chain = match argument {
						Argument::Given(value) => self.argument_chain(w, *value, param, cx),
						Argument::Receiver(access) => match w.view(*access) {
							View::Access { target, members } if !members.is_empty() => self
								.chain_members(w, *access, target, members.len() - 1, cx)
								.filter(|_| param.is_some_and(|param| !is_flat(w, param))),
							_ => None,
						},
						_ => None,
					};
					let Some(chain) = chain else {
						places.push(None);
						match argument {
							Argument::Given(value) => self.visit(w, *value, cx, effects),
							Argument::Variadic(values) => {
								for &value in values {
									self.visit(w, value, cx, effects);
								}
							}
							Argument::Receiver(access) => {
								if let View::Access { target, .. } = w.view(*access) {
									self.visit(w, target, cx, effects);
								}
							}
							Argument::Default | Argument::Missing => {}
						}
						continue;
					};
					for value in chain.values.clone() {
						self.visit(w, value, cx, effects);
					}
					// the callee's reads and writes of the parameter, on the argument
					if let Some(param) = param {
						for read in &summary.effects.reads {
							if read.root == SRoot::Param(param) {
								let path = append(&chain.path, &read.steps);
								if w.path_is_reactive(&path) || matches!(path.root, SRoot::Param(_))
								{
									effects.read(path);
								}
							}
						}
						for (written, _) in &summary.effects.writes {
							if written.root == SRoot::Param(param) {
								let path = append(&chain.path, &written.steps);
								let mut extended = chain.clone();
								extended.path = path;
								extended.place = chain.place.clone().map(|mut place| {
									place.steps.extend(written.steps.iter().filter_map(|step| {
										match step {
											SStep::Length => Some(ReactiveStep::Length),
											SStep::Size => Some(ReactiveStep::Size),
											_ => None,
										}
									}));
									place
								});
								// the callee notifies through the source it receives
								if w.path_is_reactive(&extended.path) {
									effects.writes_reactive = true;
								}
								effects.write(extended.path.clone(), n);
							}
						}
					}
					places.push(chain.place.clone());
				}
				if places.iter().any(Option::is_some) {
					w.a.reactive_args.insert(n, places);
				}
			}
			Callee::Value | Callee::Conversion(_) | Callee::Member(_) => {
				self.visit(w, callee, cx, effects);
				self.visit_arguments(w, &info.arguments, cx, effects);
			}
		}
	}

	/// The arguments of a call, read as values.
	fn visit_arguments(
		&mut self,
		w: &mut Walker<'_>,
		arguments: &[Argument],
		cx: VisitCx,
		effects: &mut Effects,
	) {
		for argument in arguments {
			match argument {
				Argument::Given(value) => self.visit(w, *value, cx, effects),
				Argument::Variadic(values) => {
					for &value in values {
						self.visit(w, value, cx, effects);
					}
				}
				Argument::Receiver(_) | Argument::Default | Argument::Missing => {}
			}
		}
	}

	/// The chain of an argument given to a parameter that receives its place
	/// (a non-flat parameter): `None` for a value that is copied.
	fn argument_chain(
		&mut self,
		w: &mut Walker<'_>,
		value: NodeId,
		param: Option<SymbolId>,
		cx: VisitCx,
	) -> Option<Chain> {
		let param = param?;
		if is_flat(w, param) {
			return None;
		}
		let chain = self.chain(w, value, cx)?;
		// a derived value is passed as its value
		match chain.path.root {
			SRoot::Symbol(symbol) if w.a.symbols.symbol(symbol).kind == SymbolKind::Derived => None,
			_ => Some(chain),
		}
	}

	/// The writes of a built-in method of a collection on its receiver.
	fn builtin_writes(
		&mut self,
		w: &mut Walker<'_>,
		receiver: &Chain,
		ty: TypeId,
		operation: &str,
		arguments: &[Argument],
	) -> Vec<Chain> {
		let TypeKind::Collection(kind, _, _) = w.s.types.kind(w.s.types.unfold(ty)) else {
			return Vec::new();
		};
		// `reserve` changes the room of the collection, not its value
		if operation == "reserve" {
			return Vec::new();
		}
		let effects = builtins::array::effects(kind, operation);
		let first = arguments.iter().find_map(|argument| match argument {
			Argument::Given(value) => Some(*value),
			_ => None,
		});
		let mut chains = Vec::new();
		for effect in effects {
			let step = match effect.step {
				// adding at the end changes the length (and the whole value)
				EffectStep::Whole if matches!(operation, "push" | "add") => {
					if matches!(
						kind,
						crate::types::CollKind::Array | crate::types::CollKind::GrowingArray
					) {
						Some((SStep::Length, ReactiveStep::Length))
					} else {
						Some((SStep::Size, ReactiveStep::Size))
					}
				}
				EffectStep::Whole
				| EffectStep::ElementsFrom
				| EffectStep::Last
				| EffectStep::Argument => None,
				EffectStep::Element => first.map(|index| {
					(
						SStep::Index(literal_index(w, index)),
						ReactiveStep::Index(index),
					)
				}),
				EffectStep::Size => Some((SStep::Size, ReactiveStep::Size)),
				EffectStep::Length => Some((SStep::Length, ReactiveStep::Length)),
				EffectStep::Capacity => continue,
			};
			let mut chain = receiver.clone();
			if let Some((step, reactive)) = step {
				chain.path.steps.push(step);
				if let Some(place) = &mut chain.place {
					place.steps.push(reactive);
				}
			}
			chains.push(chain);
		}
		chains
	}

	/// The chain of a place expression; `None` for a value that is not a
	/// place rooted at a value (a call result whose field is read is one: a
	/// temporary root).
	fn chain(&mut self, w: &mut Walker<'_>, n: NodeId, cx: VisitCx) -> Option<Chain> {
		let n = w.strip_groups(n);
		match w.view(n) {
			View::Name(_) => {
				let Resolution::Local(symbol) = w.a.resolution(n) else {
					return None;
				};
				let data = w.a.symbols.symbol(symbol);
				let place = match data.kind {
					SymbolKind::Reactive => Some(ReactivePlace {
						root: ReactiveRoot::Value(symbol),
						steps: Vec::new(),
					}),
					SymbolKind::Parameter { .. } | SymbolKind::SelfParameter
						if is_param_of(w, cx.function, symbol) && !is_flat(w, symbol) =>
					{
						Some(ReactivePlace {
							root: ReactiveRoot::Param(symbol),
							steps: Vec::new(),
						})
					}
					SymbolKind::Function
					| SymbolKind::Type
					| SymbolKind::TypeParameter
					| SymbolKind::Namespace
					| SymbolKind::Test
					| SymbolKind::NameAlias => return None,
					_ => None,
				};
				// a `use` alias or a `for` variable: the place it designates
				if let Some(&alias) = w.aliases.get(&symbol)
					&& let Some(path) = w.flow_path(alias)
				{
					return Some(Chain {
						path,
						place: None,
						outer: Vec::new(),
						values: Vec::new(),
					});
				}
				let root = match data.kind {
					SymbolKind::Parameter { .. } | SymbolKind::SelfParameter
						if is_param_of(w, cx.function, symbol) =>
					{
						SRoot::Param(symbol)
					}
					_ => SRoot::Symbol(symbol),
				};
				Some(Chain {
					path: SPath {
						root,
						steps: SmallVec::new(),
					},
					place,
					outer: Vec::new(),
					values: Vec::new(),
				})
			}
			View::Access { target, members } => self.chain_members(w, n, target, members.len(), cx),
			View::Index { target, index } => {
				let mut chain = self
					.chain(w, target, cx)
					.unwrap_or_else(|| temporary(target));
				chain.values.push(index);
				if w.type_of(w.strip_groups(index)) == TypeId::RANGE {
					// a slice reads the whole value
					return Some(chain);
				}
				chain.path.steps.push(SStep::Index(literal_index(w, index)));
				if let Some(place) = &mut chain.place {
					place.steps.push(ReactiveStep::Index(index));
				}
				Some(chain)
			}
			_ => None,
		}
	}

	/// The chain of `target` followed by the first `count` members of the
	/// `Access` node `access`.
	fn chain_members(
		&mut self,
		w: &mut Walker<'_>,
		access: NodeId,
		target: NodeId,
		count: usize,
		cx: VisitCx,
	) -> Option<Chain> {
		let tree = Arc::clone(&w.tree);
		let View::Access { members, .. } = syntax::view(&tree, access) else {
			return None;
		};
		let mut chain = match self.chain(w, target, cx) {
			Some(chain) => chain,
			// a value with fields: a temporary root
			None => {
				if count == 0 || !matches!(w.a.resolution(members.first()?), Resolution::Field(..))
				{
					return None;
				}
				temporary(target)
			}
		};
		for (index, member) in members.iter().take(count).enumerate() {
			match w.a.resolution(member) {
				Resolution::Field(shape, field) => {
					let entry = w.s.types.shape_fields(shape).get(field as usize).copied()?;
					let reactive = entry.flags.contains(FieldFlags::REACTIVE);
					chain.path.steps.push(SStep::Field(entry.name, reactive));
					if reactive {
						if let Some(outer) = chain.place.take() {
							let mut outer = outer;
							outer.steps.push(ReactiveStep::Field(entry.name));
							chain.outer.push(outer);
						}
						chain.place = Some(ReactivePlace {
							root: ReactiveRoot::Field {
								access,
								member: index as u16,
							},
							steps: Vec::new(),
						});
					} else if let Some(place) = &mut chain.place {
						place.steps.push(ReactiveStep::Field(entry.name));
					}
					for outer in &mut chain.outer {
						if !reactive {
							outer.steps.push(ReactiveStep::Field(entry.name));
						}
					}
				}
				Resolution::Builtin(builtin) => {
					let path = builtins::get(builtin).path;
					let (step, reactive) = if path.ends_with("#length") {
						(SStep::Length, ReactiveStep::Length)
					} else if path.ends_with("#size") {
						(SStep::Size, ReactiveStep::Size)
					} else if matches!(
						builtins::get(builtin).kind,
						builtins::BuiltinKind::Property(_)
					) {
						let name = w.s.types.syms.intern(builtins::get(builtin).name());
						(SStep::Field(name, false), ReactiveStep::Field(name))
					} else {
						return None;
					};
					chain.path.steps.push(step);
					if let Some(place) = &mut chain.place {
						place.steps.push(reactive);
					}
					for outer in &mut chain.outer {
						outer.steps.push(reactive);
					}
				}
				_ => return None,
			}
		}
		Some(chain)
	}
}

/// A chain rooted at a temporary value (evaluated once).
fn temporary(target: NodeId) -> Chain {
	Chain {
		path: SPath {
			root: SRoot::Temp,
			steps: SmallVec::new(),
		},
		place: None,
		outer: Vec::new(),
		values: vec![target],
	}
}

/// `path` followed by `steps`.
fn append(path: &SPath, steps: &[SStep]) -> SPath {
	let mut path = path.clone();
	path.steps.extend(steps.iter().copied());
	path
}

/// Adds what a callee does outside of its parameters to `effects`.
fn merge_outside(effects: &mut Effects, callee: &Effects) {
	for read in &callee.reads {
		if !matches!(read.root, SRoot::Param(_)) {
			effects.read(read.clone());
		}
	}
	for (written, node) in &callee.writes {
		if !matches!(written.root, SRoot::Param(_)) {
			effects.write(written.clone(), *node);
		}
	}
	if effects.impure.is_none() {
		effects.impure.clone_from(&callee.impure);
	}
	if effects.requires.is_none() {
		effects.requires = callee.requires;
	}
	effects.writes_reactive |= callee.writes_reactive;
	effects.creates_watch |= callee.creates_watch;
}

/// Whether a parameter's type is flat: it receives a copy.
fn is_flat(w: &Walker<'_>, symbol: SymbolId) -> bool {
	let ty = w.a.symbols.symbol(symbol).ty;
	ty.is_unset() || w.s.types.flags(ty).contains(TypeFlags::FLAT)
}

/// Whether `symbol` is a parameter of `function`.
fn is_param_of(w: &Walker<'_>, function: Option<NodeId>, symbol: SymbolId) -> bool {
	function
		.and_then(|function| w.a.function(function))
		.is_some_and(|info| info.params.iter().any(|param| param.symbol == symbol))
}

/// The value of an index node when it is an integer literal.
fn literal_index(w: &Walker<'_>, index: NodeId) -> Option<i128> {
	let ty = w.type_of(w.strip_groups(index));
	let int = w.s.types.as_int(ty)?;
	(int.min == int.max)
		.then(|| i128::try_from(int.min).ok())
		.flatten()
}

impl Walker<'_> {
	/// The static path of a flow place (a `use` alias, a `for` variable).
	fn flow_path(&self, place: crate::ids::PlaceId) -> Option<SPath> {
		let mut steps: SmallVec<[SStep; 4]> = SmallVec::new();
		let mut current = place;
		loop {
			let entry = self.places.get(current);
			let Some(parent) = entry.parent else {
				let root = match entry.root {
					PlaceRoot::Symbol(symbol) => SRoot::Symbol(symbol),
					PlaceRoot::ModuleValue(key) => SRoot::Symbol(*self.module_values.get(&key)?),
					PlaceRoot::Temporary(_) => SRoot::Temp,
				};
				steps.reverse();
				return Some(SPath { root, steps });
			};
			let step = match entry.step? {
				FlowStep::Field(name) => SStep::Field(name, false),
				FlowStep::Index(_) | FlowStep::All | FlowStep::Range(..) => SStep::Index(None),
				FlowStep::Length => SStep::Length,
				FlowStep::Size => SStep::Size,
				// the room of a collection is not a reactive source
				FlowStep::Capacity => return None,
				_ => SStep::Index(None),
			};
			steps.push(step);
			current = parent;
		}
	}
}
