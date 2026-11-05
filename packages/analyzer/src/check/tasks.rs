//! Async: `await`, `start`, `start do`, tasks and the async summaries
//! (`design/async.md` §1–§2, `documentation/advanced/concurrency.md`).
//!
//! - **Async contexts**: the body of an `async function`, an async lambda,
//!   a `start do` block, a test, and the top level of a standalone file.
//!   `await` elsewhere (a function or lambda that isn't `async`, a parallel
//!   body, `watch`, `derived`) is `CONC014`.
//! - **`await`** takes a call of an `async` function (or of a primitive
//!   that waits: `lock`, `send`, `receive`, `Task.yield`, `sleep`, a worker
//!   method), or a `Task`, which it consumes (`MEM002` afterwards). A call
//!   of a function that isn't `async` is `CONC015` ("call it directly"),
//!   unless it returns a `Task` (`await Task.all(…)`). Awaiting a task adds
//!   its errors, and `Cancelled` when it may have been cancelled (A11).
//! - **`start`** takes any call, or a `do` block (an async context, walked
//!   like a closure: [`Walker::check_start_do`]). It gives a
//!   `Task(T) throws E`: the errors of the call go to the task, not to the
//!   code around it. A `start` statement is **detached**: it can't throw
//!   (`CONC016`), and its `do` block copies what it captures.
//! - **Summaries** (`AsyncSummary`, §2.1): every call is recorded in the
//!   function being walked ([`Acc`]) with its mode (blocking, awaited,
//!   started, a lambda created); the calls of primitives and of other
//!   files' functions are merged at once, the calls of this file's
//!   functions at the end of the file ([`Walker::finish_tasks`]), by a
//!   fixpoint that settles recursive cycles. A blocking call whose callee
//!   can't be called blocking on the target is `CONC010`, with the call
//!   chain down to the primitive.

use rustc_hash::FxHashMap;

use crate::analysis::{
	Argument, AsyncSummary, AwaitInfo, AwaitKind, CallInfo, Callee, StartInfo, TargetSet,
};
use crate::builtins;
use crate::check::{BodyCx, FnCx, Walker};
use crate::diagnostics::codes::advanced::{
	CONC010, CONC014, CONC015, CONC016, CONC017, CONC018, CONC019, REF013,
};
use crate::diagnostics::codes::types::{FN063, FN084};
use crate::diagnostics::codes::values::E_TYPE_1;
use crate::ids::{NodeId, PlaceId, SymbolId, TypeId};
use crate::modules::FileRole;
use crate::symbols::{Resolution, ScopeKind, SymbolKind};
use crate::syntax::{NodeList, View};
use crate::types::{CollKind, SyncKind, TypeFlags, TypeKind};

/// The key of the top level of a file in `Interface::async_summaries`.
pub const TOP_LEVEL: NodeId = NodeId(u32::MAX);

/// How a call is made.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Mode {
	/// Called blocking: its synchronous instance runs.
	Blocking,
	/// `await f()`.
	Awaited,
	/// `start f()`.
	Started,
	/// A lambda created here (it runs during the call it is given to).
	Created,
}

/// What a function being walked contributes to its summary.
#[derive(Clone, Debug, Default)]
pub struct Acc {
	/// Its own part: primitives, other files' functions, `start`s. Its
	/// `suspends` means "awaits something that may suspend".
	pub own: AsyncSummary,
	/// The calls of this file's functions (by lambda node), with their
	/// mode and the callee's name.
	pub deps: Vec<(NodeId, Mode, String)>,
}

impl Acc {
	/// A new accumulator: no effect, blocking everywhere.
	pub fn new() -> Self {
		Self::default()
	}
}

/// A function of the file whose body was walked.
#[derive(Clone, Debug)]
struct Walked {
	/// Declared `async` (or an async context: test, `start do`, top level).
	is_async: bool,
	/// What its body contributes.
	acc: Acc,
}

/// The async state of the walk of a file.
#[derive(Default)]
pub(crate) struct TaskState {
	/// The calls awaited or started (the others are blocking).
	modes: FxHashMap<NodeId, Mode>,
	/// The errors of each started call: they belong to the task.
	start_errors: FxHashMap<NodeId, TypeId>,
	/// What the top level contributes.
	top: Acc,
	/// The functions walked, by lambda node (also `start do`s and tests).
	walked: FxHashMap<NodeId, Walked>,
	/// The blocking calls of this file's functions: `(call, callee key,
	/// callee name)`, checked once the summaries are settled.
	blocking_calls: Vec<(NodeId, NodeId, String)>,
	/// The awaited calls of this file's functions: `(await, callee key)`.
	awaited_calls: Vec<(NodeId, NodeId)>,
	/// What each kept task uses, by `start` node (A2).
	borrows: FxHashMap<NodeId, Borrow>,
	/// The places the effects of the call being started change.
	collected: Option<Vec<PlaceId>>,
	/// The tasks moved into each combinator call (`Task.all(…)`).
	groups: FxHashMap<NodeId, Vec<NodeId>>,
	/// The name each `start` node is declared with (`let task = start …`),
	/// recorded when it is checked, for messages.
	labels: FxHashMap<NodeId, crate::syntax::Name>,
	/// The calls checked so far that get a requirement from a `provide`
	/// (`requirements.rs`): a `start do` compares it before and after its body.
	pub(super) provided_calls: u32,
	/// The `start` awaited right away being checked (`await start f(…)`):
	/// it ends within its statement (A15).
	awaited_start: Option<NodeId>,
	/// The names used in the bodies of the file's functions, lambdas,
	/// `start do` blocks and tests (computed at the first back-edge of a
	/// loop of the top level or of a test): a module-level value whose name
	/// is not among them only changes by the code of the top level.
	other_code_names: Option<std::collections::HashSet<String>>,
}

/// What a call calls, for the summaries.
enum Target {
	/// A primitive that waits: its name and where it can block.
	Primitive(&'static str, TargetSet),
	/// A method of a worker (it waits for the worker's thread).
	Worker,
	/// A function of this file: its lambda node and name.
	Local(NodeId, String),
	/// A function of another file: its summary and name.
	External(Option<AsyncSummary>, String),
	/// A function value of an `async` type.
	AsyncValue,
	/// Anything else: no wait.
	Plain,
}

/// The primitive a built-in path is, with the targets where it can block.
fn primitive(path: &str) -> Option<(&'static str, TargetSet)> {
	Some(match path {
		"Mutex#lock" => ("Mutex.lock", TargetSet::NATIVE),
		"Channel#send" => ("Channel.send", TargetSet::NATIVE),
		"Channel#receive" => ("Channel.receive", TargetSet::NATIVE),
		"Task.yield" => ("Task.yield", TargetSet::all()),
		"std.time.sleep" => (
			"sleep",
			TargetSet::NATIVE | TargetSet::BUN | TargetSet::NODE,
		),
		// the file system: blocking wherever it exists (not on `web`)
		"File.read" => ("File.read", TargetSet::all() - TargetSet::WEB),
		"File.readBytes" => ("File.readBytes", TargetSet::all() - TargetSet::WEB),
		"File.write" => ("File.write", TargetSet::all() - TargetSet::WEB),
		// sockets: blocking natively only (async on `bun` and `node`)
		"TcpListener#accept" => ("TcpListener.accept", TargetSet::NATIVE),
		"TcpStream.connect" => ("TcpStream.connect", TargetSet::NATIVE),
		"TcpStream#read" => ("TcpStream.read", TargetSet::NATIVE),
		"TcpStream#write" => ("TcpStream.write", TargetSet::NATIVE),
		// it waits while the pool's queue is full (`maxQueueSize`): blocking
		// natively (the pools exist only there)
		"WorkerPool#submit" => ("WorkerPool.submit", TargetSet::NATIVE),
		_ => return None,
	})
}

/// Merges the summary `dep` of a callee called with `mode` into `into`.
fn merge(into: &mut AsyncSummary, dep: &AsyncSummary, mode: Mode, name: &str, is_async: bool) {
	if mode == Mode::Awaited && dep.suspends && is_async && !into.suspends {
		into.suspends = true;
		if into.reason.is_none() {
			into.reason = Some(match &dep.reason {
				Some(reason) => format!("awaits `{name}`, which {reason}"),
				None => format!("awaits `{name}`"),
			});
		}
	}
	if dep.starts && !into.starts {
		into.starts = true;
		if into.reason.is_none() {
			into.reason = Some(match &dep.reason {
				Some(reason) => format!("calls `{name}`, which {reason}"),
				None => format!("calls `{name}`, which starts a task"),
			});
		}
	}
	let removed = into.blocking - dep.blocking;
	if !removed.is_empty() {
		into.blocking -= removed;
		for (targets, chain) in &dep.blocked_by {
			let lost = *targets & removed;
			if !lost.is_empty() {
				into.blocked_by.push((lost, format!("{name} → {chain}")));
			}
		}
		let explained = dep
			.blocked_by
			.iter()
			.fold(TargetSet::empty(), |all, (targets, _)| all | *targets);
		let unexplained = removed - explained;
		if !unexplained.is_empty() {
			into.blocked_by.push((unexplained, name.to_string()));
		}
	}
}

/// Removes targets from a summary's `blocking`, with the reason.
fn block(into: &mut AsyncSummary, blocking: TargetSet, chain: impl FnOnce() -> String) {
	let removed = into.blocking - blocking;
	if !removed.is_empty() {
		into.blocking -= removed;
		into.blocked_by.push((removed, chain()));
	}
}

impl Walker<'_> {
	// ── contexts ──────────────────────────────────────────────────────────

	/// Whether the code being walked is an async context: an `async`
	/// function or lambda, a `start do`, a test, the top level of a
	/// standalone file. Never a parallel body, `watch` or `derived`.
	pub(crate) fn in_async_context(&self) -> bool {
		if self.in_parallel_body() {
			return false;
		}
		match self.body.functions.last() {
			Some(cx) => cx.reactive.is_none() && (cx.is_async || cx.test),
			None => self.s.role(self.file) == FileRole::Program,
		}
	}

	/// The accumulator of the code being walked.
	fn acc(&mut self) -> &mut Acc {
		match self.body.functions.last_mut() {
			Some(cx) => &mut cx.async_acc,
			None => &mut self.tasks.top,
		}
	}

	/// The analysis target as a set.
	fn current_target(&self) -> TargetSet {
		TargetSet::named(self.s.target())
	}

	// ── await ─────────────────────────────────────────────────────────────

	/// `await value`.
	pub(super) fn check_await(
		&mut self,
		n: NodeId,
		value: NodeId,
		expected: Option<TypeId>,
	) -> TypeId {
		let in_context = self.in_async_context();
		if !in_context && self.reported_once.insert((n, CONC014.id)) {
			let message = self.await_context_message();
			let diagnostic = self.diagnostic(&CONC014, n, message);
			self.report(diagnostic);
		}
		let operand = self.strip_groups(value);
		let ty = match self.view(operand) {
			View::Call { callee, .. } => {
				self.tasks.modes.insert(operand, Mode::Awaited);
				let ty = self.check(value, expected);
				self.await_call(n, operand, callee, ty)
			}
			View::Start(_) => {
				let outer = self.tasks.awaited_start.replace(operand);
				let ty = self.check(value, None);
				self.tasks.awaited_start = outer;
				self.await_task(n, operand, ty, true)
			}
			_ => {
				let ty = self.check(value, None);
				self.await_task(n, operand, ty, false)
			}
		};
		// the awaited tasks end, and their borrows with them
		self.end_tasks_of(operand);
		// other tasks may run here: what is known of module-level values goes (A2)
		if in_context {
			self.forget_module_facts(n);
		}
		ty
	}

	/// The message of `CONC014`, for the context of the walk.
	fn await_context_message(&self) -> String {
		if self.in_parallel_body() {
			return "`await` can't be used in the body of a parallel loop: it is not an async context".to_string();
		}
		match self.body.functions.last() {
			Some(cx) if cx.reactive.is_some() => {
				"`await` can't be used in a `watch` block or a `derived` value: they run synchronously".to_string()
			}
			Some(cx) => match cx.symbol {
				Some(symbol) => {
					let name = self
						.s
						.types
						.syms
						.resolve(self.a.symbols.symbol(symbol).name)
						.to_string();
					format!(
						"`await` is only allowed in an async context: declare `async function {name} = …`, or call the function blocking, without `await`"
					)
				}
				None => "`await` is only allowed in an async context: this function isn't `async` (its expected function type isn't), so call it blocking, without `await`".to_string(),
			},
			None => "`await` is only allowed in an async context: a module-level value is initialized blocking; call it without `await`".to_string(),
		}
	}

	/// `await f()`: the call's result, `CONC015` when `f` isn't `async`.
	fn await_call(&mut self, n: NodeId, call: NodeId, callee: NodeId, ty: TypeId) -> TypeId {
		let target = self.call_target(call, callee);
		let callee_ty = self.type_of(self.strip_groups(callee));
		let is_async = self.signature_of(callee_ty).is_some_and(|sig| sig.is_async);
		let info = match target {
			Target::Primitive(name, _) => Some(AwaitInfo {
				kind: AwaitKind::Primitive(name),
				suspends: true,
				cancelled: false,
				immediate_task: false,
			}),
			Target::Worker => Some(AwaitInfo {
				kind: AwaitKind::Primitive("worker"),
				suspends: true,
				cancelled: false,
				immediate_task: false,
			}),
			Target::Local(key, _) if is_async => {
				self.tasks.awaited_calls.push((n, key));
				Some(AwaitInfo {
					kind: AwaitKind::Call,
					suspends: false,
					cancelled: false,
					immediate_task: false,
				})
			}
			Target::External(summary, _) if is_async => Some(AwaitInfo {
				kind: AwaitKind::Call,
				suspends: summary.is_some_and(|summary| summary.suspends),
				cancelled: false,
				immediate_task: false,
			}),
			Target::AsyncValue => Some(AwaitInfo {
				kind: AwaitKind::FunctionValue,
				suspends: true,
				cancelled: false,
				immediate_task: false,
			}),
			_ => None,
		};
		if let Some(info) = info {
			self.a.awaits.insert(n, info);
			return self.memo(n, |_| ty);
		}
		// a function that isn't `async` giving a task: `await Task.all(…)`
		if matches!(self.s.types.kind(ty), TypeKind::Task(..)) {
			return self.await_task(n, call, ty, true);
		}
		self.memo(n, |w| {
			if !w.s.types.is_recovering(ty) && !w.s.types.is_recovering(callee_ty) {
				let name = w.tree.text(callee.index()).to_string();
				let message = format!(
					"`{name}` isn't `async`: it never waits, so call it directly, without `await`"
				);
				let diagnostic = w.diagnostic(&CONC015, n, message);
				w.report(diagnostic);
			}
			ty
		})
	}

	/// `await task`: its result; its errors (and `Cancelled`, A11); the
	/// task is consumed. `immediate`: the operand gives a new task (`start
	/// f()`, `Task.all(…)`).
	fn await_task(&mut self, n: NodeId, operand: NodeId, ty: TypeId, immediate: bool) -> TypeId {
		let TypeKind::Task(result, errors) = self.s.types.kind(ty) else {
			return self.memo(n, |w| {
				if !w.s.types.is_recovering(ty) {
					let message = format!(
						"`await` takes a call of an `async` function or a `Task`, not a `{}`",
						w.display(ty)
					);
					let diagnostic = w.diagnostic(&CONC015, n, message);
					w.report(diagnostic);
				}
				TypeId::UNKNOWN
			});
		};
		let local = self.read_local(operand).filter(|&symbol| {
			matches!(
				self.a.symbols.symbol(symbol).kind,
				SymbolKind::Let | SymbolKind::Mutable
			)
		});
		// A11: a local task when `cancel()` may have been called on it; a
		// task received (a parameter, a field, an element): always
		let cancelled = match local {
			Some(symbol) => self.body.flow.cancelled.contains(&symbol),
			None => !immediate,
		};
		let errors = if cancelled {
			let cancelled = self.builtin_error("Cancelled");
			if errors == TypeId::NEVER {
				cancelled
			} else {
				self.s.types.union(&[errors, cancelled])
			}
		} else {
			errors
		};
		self.can_fail(n, errors);
		self.a.awaits.insert(
			n,
			AwaitInfo {
				kind: AwaitKind::Task,
				suspends: true,
				cancelled,
				immediate_task: immediate,
			},
		);
		// awaiting a field moves it out of its local (foreign.rs)
		if local.is_none() {
			self.move_handles(&[operand]);
		}
		// awaiting consumes the task
		if let Some(symbol) = local
			&& self.body.flow.reachable
			&& !self
				.body
				.flow
				.consumed
				.iter()
				.any(|&(other, _)| other == symbol)
		{
			self.body.flow.consumed.push((symbol, n));
		}
		let acc = &mut self.acc().own;
		acc.suspends = true;
		if acc.reason.is_none() {
			acc.reason = Some("awaits a task".to_string());
		}
		// A13: a task received from outside can't be awaited by a blocking call
		if local.is_none() && !immediate {
			block(&mut self.acc().own, TargetSet::empty(), || {
				"awaits a task it receives".to_string()
			});
		}
		self.memo(n, |_| result)
	}

	// ── start ─────────────────────────────────────────────────────────────

	/// `start call` or `start do { … }`.
	pub(super) fn check_start(
		&mut self,
		n: NodeId,
		value: NodeId,
		expected: Option<TypeId>,
	) -> TypeId {
		let detached = self.is_statement(n);
		if let Some(statement) = self.statement
			&& let View::Declaration {
				name,
				value: Some(declared),
				..
			} = self.view(statement)
			&& self.strip_groups(declared) == n
		{
			self.tasks.labels.insert(n, name);
		}
		let operand = self.strip_groups(value);
		let acc = &mut self.acc().own;
		acc.starts = true;
		if acc.reason.is_none() {
			acc.reason = Some("starts a task".to_string());
		}
		match self.view(operand) {
			View::Do(body) => self.check_start_do(n, operand, body, detached, expected),
			View::Call { .. } => {
				self.tasks.modes.insert(operand, Mode::Started);
				self.tasks.start_errors.insert(operand, TypeId::NEVER);
				let expected_result =
					expected.and_then(|expected| match self.s.types.kind(expected) {
						TypeKind::Task(result, _) => Some(result),
						_ => None,
					});
				// the places its effects change: the task changes them (A2)
				let outer = self.tasks.collected.replace(Vec::new());
				let result = self.check(value, expected_result);
				let collected = std::mem::replace(&mut self.tasks.collected, outer);
				if let (Some(outer), Some(inner)) = (&mut self.tasks.collected, &collected) {
					outer.extend(inner.iter().copied());
				}
				self.check_worker_start(n, operand);
				let errors = self
					.tasks
					.start_errors
					.get(&operand)
					.copied()
					.unwrap_or(TypeId::NEVER);
				if detached && errors != TypeId::NEVER {
					self.report_detached_throws(n, errors);
				}
				let info = StartInfo {
					detached,
					block: None,
					captures: Vec::new(),
					copies: Vec::new(),
					scope: (!detached).then_some(self.body.scope),
				};
				self.a.starts.insert(n, info);
				self.start_task_borrows(n, operand, detached, collected.unwrap_or_default());
				self.memo(n, |w| {
					if w.s.types.is_recovering(result) {
						return TypeId::UNKNOWN;
					}
					w.task_type(result, errors)
				})
			}
			_ => {
				let ty = self.check(value, None);
				self.memo(n, |w| {
					if !w.s.types.is_recovering(ty) {
						let diagnostic = w.diagnostic(
							&E_TYPE_1,
							n,
							"`start` takes a call, ex: `start load(url)`, or a block: `start do { … }`",
						);
						w.report(diagnostic);
					}
					TypeId::UNKNOWN
				})
			}
		}
	}

	/// `Task(result) throws errors`.
	pub(crate) fn task_type(&mut self, result: TypeId, errors: TypeId) -> TypeId {
		self.s.types.intern(TypeKind::Task(result, errors))
	}

	/// `CONC016`: a detached task that may throw.
	fn report_detached_throws(&mut self, n: NodeId, errors: TypeId) {
		if self.s.types.is_recovering(errors) || !self.reported_once.insert((n, CONC016.id)) {
			return;
		}
		let message = format!(
			"this task is detached (it isn't kept), so nobody awaits it, and it can fail with `{}`: handle its errors with `catch` inside, or keep the task and await it",
			self.display(errors)
		);
		let diagnostic = self.diagnostic(&CONC016, n, message);
		self.report(diagnostic);
	}

	/// The errors of a call that is started belong to its task: whether
	/// `n` is one (they are kept for the task's type).
	pub(crate) fn task_absorbs(&mut self, n: NodeId, errors: TypeId) -> bool {
		let Some(current) = self.tasks.start_errors.get(&n).copied() else {
			return false;
		};
		let joined = if current == TypeId::NEVER {
			errors
		} else {
			self.s.types.union(&[current, errors])
		};
		self.tasks.start_errors.insert(n, joined);
		true
	}

	/// `start do { … }`: an async context walked like a closure (it runs
	/// later): it sees the facts of its creation; detached, it copies what it
	/// captures (`REF021` on a write).
	fn check_start_do(
		&mut self,
		n: NodeId,
		node: NodeId,
		body: NodeId,
		detached: bool,
		expected: Option<TypeId>,
	) -> TypeId {
		let parent = self.body.scope;
		let scope = self
			.a
			.symbols
			.scope_for(body, Some(parent), ScopeKind::Function);
		let inner = BodyCx {
			scope: parent,
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
			infer_returns: true,
			is_async: true,
			unsupported_at_start: self.unsupported_seen,
			..FnCx::new(node, None, TypeId::UNSET, scope)
		});
		let expected_result = expected.and_then(|expected| match self.s.types.kind(expected) {
			TypeKind::Task(result, _) => Some(result),
			_ => None,
		});
		let provided_before = self.tasks.provided_calls;
		let value = if detached {
			self.check_stmt(body);
			TypeId::VOID
		} else {
			self.check_block_value(body, expected_result).0
		};
		let provided = self.tasks.provided_calls != provided_before;
		let cx = self.body.functions.pop();
		let inner = std::mem::replace(&mut self.body, saved);
		self.body.flow_buffers = inner.flow_buffers;
		self.body.requirements = inner.requirements;
		let Some(cx) = cx else {
			return TypeId::UNKNOWN;
		};
		self.finish_captures(&cx, node, None);
		// its effects on what it captures happen at the start (effects.rs)
		let outer = self.tasks.collected.replace(Vec::new());
		self.finish_effects(&cx, None);
		let writes = std::mem::replace(&mut self.tasks.collected, outer).unwrap_or_default();
		if let Some(outer) = &mut self.tasks.collected {
			outer.extend(writes.iter().copied());
		}
		let throws = cx.throws;
		let returned = cx.returned;
		let captures = cx.captures.clone();
		self.finish_function_async(&cx, node, None, Mode::Started);
		if detached {
			// it outlives the statement: it copies what it captures (A12)
			self.escape_closure(node, n);
			if throws != TypeId::NEVER {
				self.report_detached_throws(n, throws);
			}
		}
		self.a.starts.insert(
			n,
			StartInfo {
				detached,
				block: Some(body),
				captures,
				copies: Vec::new(),
				scope: (!detached).then_some(parent),
			},
		);
		self.start_do_borrows(n, &cx, writes, detached, provided);
		self.memo(n, |w| {
			let result = if returned == TypeId::NEVER {
				value
			} else if value == TypeId::VOID || value == TypeId::NEVER {
				returned
			} else {
				w.s.types.join(value, returned)
			};
			let result = if result == TypeId::NEVER && value == TypeId::VOID {
				TypeId::VOID
			} else {
				w.s.types.widen(result)
			};
			w.task_type(result, throws)
		})
	}

	// ── calls ─────────────────────────────────────────────────────────────

	/// After a call is checked: its contribution to the summary of the code
	/// around it, and `CONC010` for a blocking call of a primitive or of
	/// another file's function that can't block on the target.
	pub(super) fn after_call(&mut self, n: NodeId, callee: NodeId) {
		let mode = self.tasks.modes.get(&n).copied().unwrap_or(Mode::Blocking);
		let target = self.call_target(n, callee);
		// `task.cancel()` (A11)
		if let View::Access {
			target: receiver, ..
		} = self.view(self.strip_groups(callee))
			&& self.builtin_path(n, callee) == Some("Task#cancel")
			&& let Some(symbol) = self.read_local(receiver)
		{
			if !self.body.flow.cancelled.contains(&symbol) {
				self.body.flow.cancelled.push(symbol);
			}
			self.end_task_borrow(symbol);
		}
		// a task or a worker's handle given to a function moves into it
		// (concurrency.md: they can't be copied; the callee owns them)
		if let Some(CallInfo {
			callee: crate::analysis::Callee::Function(_) | crate::analysis::Callee::Value,
			arguments,
		}) = self.a.calls.get(&n).cloned()
		{
			// (the receiver of a method, `worker.squares(…)`, is not given)
			let receiver = match self.view(self.strip_groups(callee)) {
				View::Access { target, .. } => Some(self.strip_groups(target)),
				_ => None,
			};
			let given: Vec<NodeId> = arguments
				.iter()
				.filter_map(|argument| match argument {
					Argument::Given(value) if Some(self.strip_groups(*value)) != receiver => {
						Some(*value)
					}
					_ => None,
				})
				.collect();
			self.move_handles(&given);
		}
		// `pool.submit(task)`: the task is stored until a worker is free, so it
		// escapes, and its captures are copies (concurrency.md)
		if self.builtin_path(n, callee) == Some("WorkerPool#submit")
			&& let Some(CallInfo { arguments, .. }) = self.a.calls.get(&n).cloned()
		{
			for argument in arguments {
				if let Argument::Given(value) = argument {
					self.escape(value, n);
					// it runs on a worker's thread: the rules of worker code
					self.check_submitted_work(value);
				}
			}
			// it starts a task (A6)
			let acc = &mut self.acc().own;
			acc.starts = true;
			if acc.reason.is_none() {
				acc.reason = Some("submits a task to a worker pool".to_string());
			}
		}
		// `worker.stop()` while a started call is pending (CONC019)
		if self.builtin_path(n, callee) == Some("Worker#stop") {
			self.check_worker_stop(n, callee);
		}
		let current = self.current_target();
		match target {
			Target::Primitive(name, blocking) => {
				// a blocking call: whether it may block depends on the target
				self.a.reads_target |= mode == Mode::Blocking && !blocking.is_all();
				let acc = &mut self.acc().own;
				if mode == Mode::Awaited {
					acc.suspends = true;
					if acc.reason.is_none() {
						acc.reason = Some(format!("awaits `{name}`"));
					}
				}
				block(acc, blocking, || primitive_chain(name, blocking));
				if mode == Mode::Blocking && !blocking.contains(current) {
					self.report_blocking_primitive(n, callee, name);
				}
			}
			Target::Worker => {
				let acc = &mut self.acc().own;
				if mode == Mode::Awaited {
					acc.suspends = true;
					if acc.reason.is_none() {
						acc.reason = Some("awaits a worker".to_string());
					}
				}
				block(acc, TargetSet::NATIVE, || {
					primitive_chain("a worker call", TargetSet::NATIVE)
				});
				// workers of JavaScript are awaited (J2): a blocking call
				// needs a thread of `native`
				self.a.reads_target |= mode == Mode::Blocking;
				if mode == Mode::Blocking
					&& !TargetSet::NATIVE.contains(current)
					&& builtins::concurrency::has_workers(self.s.target())
				{
					self.report_blocking_primitive(n, callee, "worker");
				}
			}
			Target::Local(key, name) => {
				self.acc().deps.push((key, mode, name.clone()));
				if mode == Mode::Blocking {
					self.tasks.blocking_calls.push((n, key, name));
				}
			}
			Target::External(Some(summary), name) => {
				self.a.reads_target |= mode == Mode::Blocking && !summary.blocking.is_all();
				merge(&mut self.acc().own, &summary, mode, &name, true);
				if mode == Mode::Blocking && !summary.blocking.contains(current) {
					self.report_blocking_call(n, &name, &summary);
				}
			}
			Target::AsyncValue if mode == Mode::Awaited => {
				let acc = &mut self.acc().own;
				acc.suspends = true;
				if acc.reason.is_none() {
					acc.reason = Some("awaits an async function value".to_string());
				}
			}
			_ => {}
		}
	}

	/// The path of the built-in a call calls, if any.
	pub(super) fn builtin_path(&self, n: NodeId, callee: NodeId) -> Option<&'static str> {
		if let Some(CallInfo {
			callee: Callee::Builtin(id),
			..
		}) = self.a.calls.get(&n)
		{
			return Some(builtins::get(*id).path);
		}
		let callee = self.strip_groups(callee);
		let node = match self.view(callee) {
			View::Access { members, .. } => members.last()?,
			_ => callee,
		};
		match self.a.resolution(node) {
			Resolution::Builtin(id) => Some(builtins::get(id).path),
			_ => None,
		}
	}

	/// What a call calls, for the summaries.
	fn call_target(&mut self, n: NodeId, callee: NodeId) -> Target {
		if let Some(path) = self.builtin_path(n, callee) {
			return match primitive(path) {
				Some((name, blocking)) => Target::Primitive(name, blocking),
				None => Target::Plain,
			};
		}
		let callee = self.strip_groups(callee);
		// a method of a worker: it waits for the worker's thread
		if let View::Access { target, .. } = self.view(callee)
			&& matches!(
				self.s.types.kind(self.type_of(self.strip_groups(target))),
				TypeKind::Sync(SyncKind::Worker, _)
			) {
			return Target::Worker;
		}
		let name = self.tree.text(callee.index()).to_string();
		if let Some(&CallInfo {
			callee: Callee::Function(declaration),
			..
		}) = self.a.calls.get(&n)
			&& let Some(info) = self.a.functions.get(&declaration)
		{
			return Target::Local(info.lambda, self.function_label_of(info.symbol, &name));
		}
		let node = match self.view(callee) {
			View::Access { members, .. } => members.last().unwrap_or(callee),
			_ => callee,
		};
		let resolution = match self.a.resolution(node) {
			Resolution::Local(symbol) => {
				if let Some(declaration) = self.direct_function(symbol)
					&& let Some(info) = self.a.functions.get(&declaration)
				{
					return Target::Local(info.lambda, self.function_label_of(info.symbol, &name));
				}
				self.external_resolution(symbol)
					.unwrap_or(Resolution::Local(symbol))
			}
			other => other,
		};
		if let Resolution::External(_) = resolution {
			let summary = self.s.external_node(resolution).and_then(|(module, decl)| {
				self.s
					.interface(module)
					.and_then(|interface| interface.async_summaries.get(&decl).cloned())
			});
			return Target::External(summary, name);
		}
		let callee_ty = self.type_of(callee);
		match self.signature_of(callee_ty) {
			Some(sig) if sig.is_async => Target::AsyncValue,
			_ => Target::Plain,
		}
	}

	/// A function's name for chains and messages.
	fn function_label_of(&self, symbol: Option<SymbolId>, fallback: &str) -> String {
		match symbol {
			Some(symbol) => self
				.s
				.types
				.syms
				.resolve(self.a.symbols.symbol(symbol).name)
				.to_string(),
			None => fallback.to_string(),
		}
	}

	/// `CONC010` for a blocking call of a primitive that can't block here.
	fn report_blocking_primitive(&mut self, n: NodeId, callee: NodeId, name: &'static str) {
		if !self.body.flow.reachable || !self.reported_once.insert((n, CONC010.id)) {
			return;
		}
		let target = self.s.target().to_string();
		// a primitive missing on the target is `AN004` already (std_library.rs)
		if builtins::std_library::missing_on(self.builtin_path(n, callee).unwrap_or(""))
			.contains(&target.as_str())
		{
			return;
		}
		let short = name.rsplit('.').next().unwrap_or(name);
		let instead = match short {
			"lock" => "`await`, `tryLock()`",
			"send" => "`await`, `trySend(value)`",
			"receive" => "`await`, `tryReceive()`",
			_ => "`await`",
		};
		let message = if name == "worker" {
			format!(
				"a worker's method can't be called blocking on target `{target}`: its call waits for the worker's thread, which only threads of `native` can do; use `await` or `start`"
			)
		} else if name == "WorkerPool.submit" {
			format!(
				"`submit` can't be called blocking on target `{target}`: it waits while the pool's queue is full (`maxQueueSize`); await it, `await pool.submit(…)`, which gives the task of the work"
			)
		} else if name.starts_with("Tcp") {
			format!(
				"`{name}` can't be called blocking on target `{target}`: sockets can only be awaited there; use {instead}, or branch with `@if target`"
			)
		} else if name.starts_with("File.") || name == "sleep" {
			format!(
				"`{name}` can't be called blocking on target `{target}`: it can only be awaited there; use {instead}, or branch with `@if target`"
			)
		} else {
			format!(
				"`{short}` can't be called blocking on target `{target}`: it waits, which only threads of `native` can do; use {instead}, or branch with `@if target`"
			)
		};
		let at = match self.view(self.strip_groups(callee)) {
			View::Access { members, .. } => members.last().unwrap_or(n),
			_ => n,
		};
		let diagnostic = self.diagnostic(&CONC010, at, message);
		self.report(diagnostic);
	}

	/// `CONC010` for a blocking call of a function that can't block on the
	/// target, with the call chain.
	fn report_blocking_call(&mut self, n: NodeId, name: &str, summary: &AsyncSummary) {
		if !self.reported_once.insert((n, CONC010.id)) {
			return;
		}
		let target = self.s.target().to_string();
		let current = TargetSet::named(&target);
		let chain = match summary.chain(current) {
			Some(chain) => format!("{name} → {chain}"),
			None => name.to_string(),
		};
		let message = format!(
			"`{name}` can't be called blocking on target `{target}`: {chain}; use `await {name}(…)` or `start {name}(…)`"
		);
		let diagnostic = self.diagnostic(&CONC010, n, message);
		self.report(diagnostic);
	}

	// ── functions ─────────────────────────────────────────────────────────

	/// Whether a function or lambda being declared is `async`: written, or
	/// (a lambda) its expected function type is. `FN063` for `async` written
	/// on a lambda whose expected type isn't.
	pub(super) fn lambda_is_async(
		&mut self,
		lambda: NodeId,
		written: bool,
		declared: bool,
		expected: Option<crate::types::FnSig>,
	) -> bool {
		if declared {
			return true;
		}
		let expected_async = expected.is_some_and(|sig| sig.is_async);
		if written && expected.is_some() && !expected_async {
			if self.reported_once.insert((lambda, FN063.id)) {
				let diagnostic = self.diagnostic(
					&FN063,
					lambda,
					"this function is `async`, but the function type expected here isn't: a call through it would have to block; remove `async`, or make the type `async`",
				);
				self.report(diagnostic);
			}
			// reported once: the lambda is typed as expected
			return false;
		}
		written || expected_async
	}

	/// A function or lambda whose body was walked: its contribution is kept
	/// for [`Walker::finish_tasks`]; a lambda's is part of the code that
	/// creates it (it runs during the call it is given to).
	pub(super) fn finish_function_async(
		&mut self,
		cx: &FnCx,
		lambda: NodeId,
		declaration: Option<NodeId>,
		mode: Mode,
	) {
		self.tasks.walked.insert(
			lambda,
			Walked {
				is_async: cx.is_async,
				acc: cx.async_acc.clone(),
			},
		);
		if declaration.is_none() && (mode == Mode::Started || !cx.is_async) {
			let name = if mode == Mode::Started {
				"start do".to_string()
			} else {
				"a function".to_string()
			};
			self.acc().deps.push((lambda, mode, name));
		}
	}

	/// A test body: an async context, part of the file's top level for the
	/// detection (A6).
	pub(super) fn finish_test_async(&mut self, cx: &FnCx, test: NodeId) {
		self.tasks.walked.insert(
			test,
			Walked {
				is_async: true,
				acc: cx.async_acc.clone(),
			},
		);
		self.tasks
			.top
			.deps
			.push((test, Mode::Awaited, "a test".to_string()));
	}

	/// `FN084`: an `async` function given where a function type without
	/// `async` is expected (a call through it would have to block).
	pub(crate) fn async_mismatch(&mut self, n: NodeId, actual: TypeId, target: TypeId) -> bool {
		let (Some(actual_sig), Some(target_sig)) =
			(self.signature_of(actual), self.signature_of(target))
		else {
			return false;
		};
		if !actual_sig.is_async || target_sig.is_async {
			return false;
		}
		let message = format!(
			"an `async` function doesn't fit `{}`, which isn't `async`: a call through it would have to block, which the caller didn't ask for; make the type `async`",
			self.display(target)
		);
		let diagnostic = self.diagnostic(&FN084, n, message);
		self.report_flow(diagnostic);
		true
	}

	// ── Task ──────────────────────────────────────────────────────────────

	/// `Task(T)` in a type (`Task(T) throws E` with `errors`).
	pub(super) fn task_type_expr(
		&mut self,
		n: NodeId,
		callee: NodeId,
		arguments: NodeList<'_>,
		errors: TypeId,
	) -> TypeId {
		self.set_flag(callee, crate::analysis::NodeFlags::TYPE_EXPR);
		let items: Vec<NodeId> = arguments.iter().collect();
		let result = match items.as_slice() {
			[item] => Some(self.check_type_expr(*item)),
			_ => None,
		};
		self.memo(n, |w| {
			let Some(result) = result else {
				let diagnostic = w.diagnostic(
					&crate::diagnostics::codes::types::TY003,
					n,
					"`Task` takes the type of its result, ex: `Task<Integer>`, `Task<Response> throws HttpError`",
				);
				w.report(diagnostic);
				return TypeId::UNKNOWN;
			};
			if w.s.types.is_recovering(result) || w.s.types.is_recovering(errors) {
				return TypeId::UNKNOWN;
			}
			w.task_type(result, errors)
		})
	}

	/// `Task.all(tasks)`, `Task.any(tasks)`, `Task.settled(tasks)`,
	/// `Task.yield()`: `None` when the callee isn't one of them.
	pub(super) fn check_task_call(
		&mut self,
		n: NodeId,
		callee: NodeId,
		arguments: NodeList<'_>,
	) -> Option<TypeId> {
		let callee = self.strip_groups(callee);
		let tree = std::sync::Arc::clone(&self.tree);
		let View::Access { target, members } = crate::syntax::view(&tree, callee) else {
			return None;
		};
		let target = self.strip_groups(target);
		let View::Name(name) = crate::syntax::view(&tree, target) else {
			return None;
		};
		if tree.name(name) != "Task" || members.len() != 1 || self.is_declared(name) {
			return None;
		}
		let member = members.first()?;
		let View::Name(member_name) = self.view(member) else {
			return None;
		};
		let text = self.tree.name(member_name).to_string();
		let path = format!("Task.{text}");
		let builtin = builtins::by_path(&path)?;
		if let Some(task) = builtins::lookup("Task") {
			self.set_resolution(target, Resolution::Builtin(task));
		}
		self.set_type(target, TypeId::VOID);
		self.set_resolution(member, Resolution::Builtin(builtin));
		let values: Vec<NodeId> = arguments.iter().collect();
		if text == "yield" {
			for &value in &values {
				self.check(value, None);
			}
			let ty = self.memo(n, |w| {
				if !values.is_empty() {
					let diagnostic = w.diagnostic(&E_TYPE_1, n, "`Task.yield()` takes no argument");
					w.report(diagnostic);
				}
				w.a.calls.insert(
					n,
					CallInfo {
						callee: Callee::Builtin(builtin),
						arguments: Vec::new(),
					},
				);
				TypeId::VOID
			});
			self.set_type(callee, TypeId::UNKNOWN);
			return Some(ty);
		}
		let [value] = values.as_slice() else {
			for &value in &values {
				self.check(value, None);
			}
			return Some(self.memo(n, |w| {
				let message =
					format!("`Task.{text}` takes one array of tasks, ex: `Task.{text}(tasks)`");
				let diagnostic = w.diagnostic(&E_TYPE_1, n, message);
				w.report(diagnostic);
				TypeId::UNKNOWN
			}));
		};
		let value = *value;
		let ty = self.check(value, None);
		// the tasks are moved into the combinator: so are their borrows
		self.move_tasks(value);
		let starts = self.task_starts(value);
		self.tasks.groups.insert(n, starts);
		self.set_type(callee, TypeId::UNKNOWN);
		Some(self.memo(n, |w| {
			w.a.calls.insert(
				n,
				CallInfo {
					callee: Callee::Builtin(builtin),
					arguments: vec![Argument::Given(value)],
				},
			);
			if w.s.types.is_recovering(ty) {
				return TypeId::UNKNOWN;
			}
			let item = match w.s.types.kind(ty) {
				TypeKind::Collection(CollKind::Array | CollKind::GrowingArray, item, _) => item,
				_ => TypeId::UNKNOWN,
			};
			let (result, errors) = match w.s.types.kind(item) {
				TypeKind::Task(result, errors) => (result, errors),
				TypeKind::Never => (TypeId::NEVER, TypeId::NEVER),
				_ => {
					if !w.s.types.is_recovering(item) {
						let message = format!(
							"`Task.{text}` takes an array of tasks, not a `{}`",
							w.display(ty)
						);
						let diagnostic = w.diagnostic(&E_TYPE_1, value, message);
						w.report(diagnostic);
					}
					return TypeId::UNKNOWN;
				}
			};
			match text.as_str() {
				"all" => {
					let array = w.s.types.array(result);
					w.task_type(array, errors)
				}
				"any" => w.task_type(result, errors),
				_ => {
					let outcome = if errors == TypeId::NEVER {
						result
					} else {
						w.s.types.union(&[result, errors])
					};
					let array = w.s.types.array(outcome);
					w.task_type(array, TypeId::NEVER)
				}
			}
		}))
	}

	/// The tasks given to a combinator are moved: a local array, or the
	/// local tasks of an array literal.
	fn move_tasks(&mut self, value: NodeId) {
		let value = self.strip_groups(value);
		let names: Vec<NodeId> = match self.view(value) {
			View::Array { items } => items.iter().collect(),
			_ => vec![value],
		};
		for name in names {
			if let Some(symbol) = self.read_local(name)
				&& matches!(
					self.a.symbols.symbol(symbol).kind,
					SymbolKind::Let | SymbolKind::Mutable
				) && self.body.flow.reachable
				&& !self
					.body
					.flow
					.consumed
					.iter()
					.any(|&(other, _)| other == symbol)
			{
				self.body.flow.consumed.push((symbol, name));
			}
		}
	}

	// ── the end of the file ───────────────────────────────────────────────

	/// Settles the summaries of the file's functions (a fixpoint over the
	/// calls between them, for recursive cycles), records them, reports
	/// the blocking calls they make impossible (`CONC010`) and publishes
	/// them in the interface.
	pub(super) fn finish_tasks(&mut self, root: NodeId) {
		let top_async = matches!(self.s.role(self.file), FileRole::Program | FileRole::Test);
		let mut walked = std::mem::take(&mut self.tasks.walked);
		walked.insert(
			root,
			Walked {
				is_async: top_async,
				acc: std::mem::take(&mut self.tasks.top),
			},
		);
		let mut keys: Vec<NodeId> = walked.keys().copied().collect();
		keys.sort_unstable();
		let mut settled: FxHashMap<NodeId, AsyncSummary> = keys
			.iter()
			.map(|&key| {
				let entry = &walked[&key];
				let mut own = entry.acc.own.clone();
				own.suspends &= entry.is_async;
				(key, own)
			})
			.collect();
		loop {
			let mut changed = false;
			for &key in &keys {
				let entry = &walked[&key];
				let mut summary = entry.acc.own.clone();
				summary.suspends &= entry.is_async;
				for (dep, mode, name) in &entry.acc.deps {
					if let Some(dep) = settled.get(dep) {
						merge(&mut summary, dep, *mode, name, entry.is_async);
					}
				}
				let before = &settled[&key];
				if (before.suspends, before.starts, before.blocking)
					!= (summary.suspends, summary.starts, summary.blocking)
				{
					changed = true;
				}
				settled.insert(key, summary);
			}
			if !changed {
				break;
			}
		}
		// the tables: by lambda and by declaration
		let mut by_declaration: Vec<(NodeId, NodeId)> = self
			.a
			.functions
			.iter()
			.filter_map(|(&key, info)| (key != info.lambda).then_some((key, info.lambda)))
			.collect();
		by_declaration.sort_unstable();
		for (&key, summary) in &settled {
			self.a.async_summaries.insert(key, summary.clone());
		}
		for &(declaration, lambda) in &by_declaration {
			if let Some(summary) = settled.get(&lambda) {
				self.a.async_summaries.insert(declaration, summary.clone());
			}
		}
		// the copies of the detached tasks: moved at their last use (A12)
		let flags: Vec<(NodeId, Vec<(NodeId, bool)>)> = self
			.a
			.starts
			.iter()
			.map(|(&start, info)| {
				let copies = info
					.copies
					.iter()
					.map(|&(node, _)| {
						(
							node,
							self.a.has_flags(node, crate::analysis::NodeFlags::MOVE),
						)
					})
					.collect();
				(start, copies)
			})
			.collect();
		for (start, copies) in flags {
			if let Some(info) = self.a.starts.get_mut(&start) {
				info.copies = copies;
			}
		}
		// the awaits of this file's functions: whether they suspend
		for (node, key) in std::mem::take(&mut self.tasks.awaited_calls) {
			let suspends = settled.get(&key).is_some_and(|summary| summary.suspends);
			if let Some(info) = self.a.awaits.get_mut(&node) {
				info.suspends = suspends;
			}
		}
		// the blocking calls they make impossible on the target
		let current = self.current_target();
		for (call, key, name) in std::mem::take(&mut self.tasks.blocking_calls) {
			// a call that can't block everywhere: its check depends on the target
			if settled
				.get(&key)
				.is_some_and(|summary| !summary.blocking.is_all())
			{
				self.a.reads_target = true;
			}
			if let Some(summary) = settled.get(&key)
				&& !summary.blocking.contains(current)
			{
				let summary = summary.clone();
				self.report_blocking_call(call, &name, &summary);
			}
		}
		// the interface: the declarations, and the top level
		let mut published: FxHashMap<NodeId, AsyncSummary> = by_declaration
			.iter()
			.filter_map(|&(declaration, lambda)| Some((declaration, settled.get(&lambda)?.clone())))
			.collect();
		if let Some(top) = settled.get(&root) {
			published.insert(TOP_LEVEL, top.clone());
		}
		self.s.source_file_mut(self.file).interface.async_summaries = published;
		self.tasks.walked = walked;
	}
}

/// The chain text of a primitive that can't block everywhere.
fn primitive_chain(name: &str, blocking: TargetSet) -> String {
	let targets = blocking
		.names()
		.iter()
		.map(|name| format!("`{name}`"))
		.collect::<Vec<_>>()
		.join(", ");
	if blocking.is_empty() {
		format!("{name} (async only)")
	} else {
		format!("{name} (blocking only on {targets})")
	}
}

// ── the program's mode (A5–A7) ───────────────────────────────────────────

impl crate::Session {
	/// The transitive summary of a program's entry (A6): the top level of a
	/// standalone file or a script and its exported function, the exported
	/// function of the application's entry file, and with `tests` the test
	/// bodies of the test files. With the name of what it summarizes.
	fn entry_summary(
		&self,
		program: &crate::modules::Program,
		tests: bool,
	) -> (AsyncSummary, String) {
		let mut summary = AsyncSummary::default();
		let mut name = "the program".to_string();
		let mut parts: Vec<(crate::ids::FileId, NodeId)> = Vec::new();
		if let Some(entry) = program.entry {
			let interface = &self.source_file(entry).interface;
			if self.role(entry) == FileRole::Program {
				parts.push((entry, TOP_LEVEL));
				name = self.display_path(entry);
			}
			if let Some(export) = &interface.export {
				parts.push((entry, export.decl));
				name = "main".to_string();
			}
		}
		if tests {
			parts.extend(program.tests.iter().map(|&file| (file, TOP_LEVEL)));
			name = "a test".to_string();
		}
		for (file, key) in parts {
			if let Some(part) = self.source_file(file).interface.async_summaries.get(&key) {
				if summary.reason.is_none() && (part.starts || part.suspends) {
					summary.reason.clone_from(&part.reason);
				}
				merge(&mut summary, part, Mode::Awaited, &name, true);
			}
		}
		(summary, name)
	}

	/// Resolves whether a program runs in asynchronous mode (A5–A7):
	/// `configured` (`--asynchronous`, then `build.asynchronous`), or the
	/// detection of A6: the entry's transitive summary starts a task or may
	/// suspend. With `tests`, the test bodies count (`fa test`). Sets
	/// `Program::asynchronous` and its reason; forced off, a program that
	/// starts tasks gets `WCONC001`, and one whose entry can't block on the
	/// target `CONC010`, on its entry file.
	pub fn configure_asynchronous(
		&mut self,
		program: &mut crate::modules::Program,
		configured: Option<bool>,
		tests: bool,
	) {
		let (summary, name) = self.entry_summary(program, tests);
		let detected = summary.starts || summary.suspends;
		let why = summary
			.reason
			.clone()
			.unwrap_or_else(|| "starts a task".to_string());
		program.asynchronous = configured.unwrap_or(detected);
		program.asynchronous_reason = match configured {
			Some(true) => "on: `build.asynchronous` or `--asynchronous` is `true`".to_string(),
			Some(false) => "off: `build.asynchronous` or `--asynchronous` is `false`".to_string(),
			None if detected => format!("on: {name} {why}"),
			None => format!("off: {name} starts no task and never waits"),
		};
		if configured != Some(false) {
			return;
		}
		let Some(entry) = program.entry.or_else(|| program.tests.first().copied()) else {
			return;
		};
		let location = crate::diagnostics::Location {
			file: entry,
			span: crate::diagnostics::SourceSpan::new(0, 0),
		};
		if summary.starts {
			let diagnostic = crate::diagnostics::Diagnostic::new(
				&crate::diagnostics::codes::advanced::WCONC001,
				location,
				format!(
					"asynchronous mode is off, and {name} starts tasks: tasks run to their end when they are started, so the output order may change"
				),
			);
			self.push_diagnostic(entry, diagnostic);
		}
		let target = TargetSet::named(self.target());
		if !target.is_empty() && !summary.blocking.contains(target) {
			let chain = summary
				.chain(target)
				.unwrap_or("a primitive that only waits");
			let diagnostic = crate::diagnostics::Diagnostic::new(
				&CONC010,
				location,
				format!(
					"asynchronous mode is off, so {name} runs as one blocking call, which can't wait on target `{}`: {chain}; build with `--asynchronous=true`",
					self.target()
				),
			);
			self.push_diagnostic(entry, diagnostic);
		}
	}
}

impl Walker<'_> {
	// ── interleaving points (A1, A2) ──────────────────────────────────────

	/// At an `await` or a loop back-edge of async code, other tasks may run
	/// and change the module-level `mutable` values: what is known about
	/// them is forgotten, and the references into them are invalidated
	/// (they must be declared again).
	pub(crate) fn forget_module_facts(&mut self, n: NodeId) {
		self.forget_facts(n, false);
	}

	/// [`Walker::forget_module_facts`], at the back-edge of a loop of the top
	/// level or of a test when `top_level`: the module-level values that
	/// only the top level's own code changes keep their facts (no other
	/// task can change them while this one yields).
	fn forget_facts(&mut self, n: NodeId, top_level: bool) {
		if !self.body.flow.reachable {
			return;
		}
		if top_level && self.tasks.other_code_names.is_none() {
			self.tasks.other_code_names = Some(self.other_code_names());
		}
		let places: Vec<PlaceId> = self
			.body
			.flow
			.facts
			.iter()
			.map(|(place, _)| place)
			.filter(|&place| {
				self.is_shared_place(place) && !(top_level && self.top_level_only(place))
			})
			.collect();
		for place in places {
			self.drop_place_facts(place);
		}
		let refs = self.body.flow.refs.clone();
		for (symbol, state) in refs {
			let current = self.facts.refs.get(state).clone();
			if current.invalidated.is_some()
				|| !current.places.iter().any(|&place| {
					self.is_shared_place(place) && !(top_level && self.top_level_only(place))
				}) {
				continue;
			}
			let next = self.facts.refs.state(crate::flow::refs::RefState {
				invalidated: Some((n, false)),
				reported: false,
				..current
			});
			self.body.flow.set_ref_state(symbol, next);
		}
	}

	/// Whether a place is (below) a module-level `mutable` value: other
	/// tasks may change it while this one waits.
	fn is_shared_place(&self, place: PlaceId) -> bool {
		let Some(symbol) = self.place_symbol(place) else {
			return false;
		};
		let data = self.a.symbols.symbol(symbol);
		matches!(data.kind, SymbolKind::Mutable | SymbolKind::Reactive)
			&& matches!(
				self.a.symbols.scope(data.scope).kind,
				ScopeKind::Module | ScopeKind::Namespace
			)
	}

	/// Whether a place is below a module-level value of this file that only
	/// the code of the top level changes: declared in the module's scope,
	/// not exported, and named in no function, lambda, `start do` block or
	/// test of the file (`other_code_names`), so no other task can change it.
	fn top_level_only(&self, place: PlaceId) -> bool {
		let Some(symbol) = self.place_symbol(place) else {
			return false;
		};
		let data = self.a.symbols.symbol(symbol);
		if self.a.symbols.scope(data.scope).kind != ScopeKind::Module
			|| matches!(self.view(data.decl_node), View::ExportValue { .. })
		{
			return false;
		}
		let name = self.s.types.syms.resolve(data.name);
		self.tasks
			.other_code_names
			.as_ref()
			.is_some_and(|names| !names.contains(name))
	}

	/// The names used in the bodies of the functions, lambdas, `start do`
	/// blocks and tests of the file (by text: a local of the same name
	/// counts too, which only makes the analysis forget more). Empty when
	/// the file starts no task (no `start`, no `submit`): then no other
	/// task runs its code (the other modules can't see its top level).
	fn other_code_names(&self) -> std::collections::HashSet<String> {
		let mut names = std::collections::HashSet::new();
		let mut starts = false;
		let mut stack = vec![(NodeId::new(self.tree.root), false)];
		while let Some((node, inside)) = stack.pop() {
			let view = self.view(node);
			let inside = inside
				|| match view {
					View::Lambda { .. } | View::Test { .. } => true,
					View::Start(operand) => matches!(self.view(operand), View::Do(_)),
					_ => false,
				};
			match view {
				View::Start(_) => starts = true,
				View::Access { members, .. }
					if members
						.last()
						.is_some_and(|member| self.tree.text(member.index()) == "submit") =>
				{
					starts = true;
				}
				View::Name(_) if inside => {
					names.insert(self.tree.text(node.index()).to_string());
				}
				_ => {}
			}
			for child in crate::syntax::children(&self.tree, node) {
				stack.push((child, inside));
			}
		}
		if !starts {
			names.clear();
		}
		names
	}

	/// The back-edge of a loop (a new round of its body): in async code, an
	/// interleaving point (the lead's amendment of A2: the backend yields
	/// there when the turn has lasted too long). In the body of an async
	/// function, an async lambda or a `start do`, the module-level values
	/// are forgotten; at the top level of a program and in a test (async
	/// contexts too, `design/decisions.md`), only those that the code of
	/// other tasks may change: the top level's own variables keep their
	/// narrowing.
	pub(crate) fn loop_back_edge(&mut self) {
		if self.in_parallel_body() {
			return;
		}
		let (in_async_function, top_level) = match self.body.functions.last() {
			Some(cx) if cx.reactive.is_none() => (cx.is_async && !cx.test, cx.test),
			Some(_) => (false, false),
			None => (false, self.s.role(self.file) == FileRole::Program),
		};
		if !in_async_function && !top_level {
			return;
		}
		let Some(node) = self.body.loops.last().map(|cx| cx.node) else {
			return;
		};
		self.forget_facts(node, top_level);
	}

	// ── tasks and references (A2) ─────────────────────────────────────────

	/// A started call: a kept task borrows the places of its arguments
	/// (read), those its effects change (written), its requirements and a
	/// worker's handle; a detached one copies its arguments (moved at their
	/// last use, A12) and can't use requirements (`CONC017`).
	fn start_task_borrows(
		&mut self,
		n: NodeId,
		call: NodeId,
		detached: bool,
		writes: Vec<PlaceId>,
	) {
		let arguments: Vec<NodeId> = match self.a.calls.get(&call) {
			Some(info) => info
				.arguments
				.iter()
				.flat_map(|argument| match argument {
					Argument::Given(value) => vec![*value],
					Argument::Variadic(values) => values.clone(),
					Argument::Receiver(callee) => match self.view(self.strip_groups(*callee)) {
						View::Access { target, .. } => vec![target],
						_ => Vec::new(),
					},
					_ => Vec::new(),
				})
				.collect(),
			None => Vec::new(),
		};
		let provided = self.a.requirement_args.get(&call).is_some_and(|args| {
			args.iter()
				.any(|arg| matches!(arg, crate::analysis::RequirementArg::Provided(_)))
		});
		let required = self
			.a
			.requirement_args
			.get(&call)
			.is_some_and(|args| !args.is_empty());
		if detached {
			if required {
				self.report_detached_use(n, "a requirement".to_string());
			}
			let mut copies = Vec::new();
			for value in arguments {
				let value = self.strip_groups(value);
				if self.read_local(value).is_some() {
					// copied, moved at its last use (moves.rs, foreign.rs)
					self.note_copy_site(value);
					copies.push((value, false));
				}
			}
			if let Some(info) = self.a.starts.get_mut(&n) {
				info.copies = copies;
			}
			return;
		}
		let mut reads = Vec::new();
		// a worker's call copies its arguments to the worker's thread
		let worker = match self.view(call) {
			View::Call { callee, .. } => match self.view(self.strip_groups(callee)) {
				View::Access { target, .. } => matches!(
					self.s.types.kind(self.type_of(self.strip_groups(target))),
					TypeKind::Sync(SyncKind::Worker, _)
				),
				_ => false,
			},
			_ => false,
		};
		let awaited = self.tasks.awaited_start == Some(n) || worker;
		for value in arguments {
			let value = self.strip_groups(value);
			if !awaited {
				self.check_temporary_argument(n, value);
			}
			if matches!(
				self.view(value),
				View::Name(_) | View::Access { .. } | View::Index { .. }
			) && let Some(place) = self.place_of(value)
			{
				reads.push(place);
			}
		}
		self.add_task(
			n,
			Borrow {
				reads,
				writes,
				provided,
			},
		);
	}

	/// An argument of a kept task that is a temporary owning memory
	/// (`start f(Channel(String) { capacity = 1 })`): it is destroyed at the
	/// end of the statement, while the task borrows it (`CONC018`).
	fn check_temporary_argument(&mut self, start: NodeId, value: NodeId) {
		if matches!(
			self.view(value),
			View::Name(_)
				| View::Access { .. }
				| View::Index { .. }
				| View::String(_)
				| View::Integer(_)
				| View::Float(_)
				| View::Boolean(_)
				| View::None
		) {
			return;
		}
		let ty = self.type_of(value);
		if self.s.types.is_recovering(ty)
			|| self
				.s
				.types
				.flags(ty)
				.contains(TypeFlags::TRIVIALLY_COPYABLE)
			|| !self.reported_once.insert((start, CONC018.id))
		{
			return;
		}
		let message = format!(
			"this task borrows `{}`, a temporary value destroyed at the end of the statement while the task still runs: declare it first (`let value = …`) and give the task its name",
			self.tree.text(value.index())
		);
		let diagnostic = self.diagnostic(&CONC018, value, message);
		self.report(diagnostic);
	}

	/// A `start do`: a kept task borrows the locals it uses and the places
	/// its effects change; a detached one copies the locals it uses
	/// (`CONC017` for a reference or a move-only value, which can't be
	/// copied, and for a requirement).
	///
	/// `provided`: a call of its body gets a requirement from a `provide`
	/// (counted while the body was walked, not by scanning it again).
	fn start_do_borrows(
		&mut self,
		n: NodeId,
		cx: &FnCx,
		writes: Vec<PlaceId>,
		detached: bool,
		provided: bool,
	) {
		if self.a.starts.get(&n).and_then(|info| info.block).is_none() {
			return;
		}
		if detached {
			for &symbol in &cx.captures {
				let data = self.a.symbols.symbol(symbol);
				let name = self.s.types.syms.resolve(data.name).to_string();
				let what = match data.kind {
					SymbolKind::Use | SymbolKind::MutableUse => {
						Some(format!("`{name}`, a reference (it can't be copied)"))
					}
					SymbolKind::Requirement { .. } => Some(format!("the requirement `{name}`")),
					_ if self.s.types.flags(data.ty).contains(TypeFlags::MOVE_ONLY) => Some(
						format!("`{name}`, which can't be copied (it can only be moved)"),
					),
					_ => None,
				};
				if let Some(what) = what {
					self.report_detached_use(n, what);
				}
			}
			if provided {
				self.report_detached_use(n, "a requirement".to_string());
			}
			// the module-level values (the top level's included) are read,
			// not copied: they are not captured (`concurrency.md`, tasks)
			let copies = cx
				.captures
				.iter()
				.filter(|&&symbol| {
					let scope = self.a.symbols.symbol(symbol).scope;
					!matches!(self.a.symbols.scope(scope).kind, ScopeKind::Module)
				})
				.map(|&symbol| (self.a.symbols.symbol(symbol).decl_node, false))
				.collect();
			if let Some(info) = self.a.starts.get_mut(&n) {
				info.copies = copies;
			}
			return;
		}
		let reads = cx
			.captures
			.iter()
			.filter_map(|&symbol| self.symbol_target(symbol))
			.collect();
		self.add_task(
			n,
			Borrow {
				reads,
				writes,
				provided,
			},
		);
	}

	/// `CONC017`: a detached task uses what it can't copy.
	fn report_detached_use(&mut self, n: NodeId, what: String) {
		if !self.reported_once.insert((n, CONC017.id)) {
			return;
		}
		let message = format!(
			"this task is detached, so it copies what it uses, and it uses {what}: keep the task and await it, or give it values it can own"
		);
		let diagnostic = self.diagnostic(&CONC017, n, message);
		self.report(diagnostic);
	}

	/// A kept task starts: it is pending until awaited, cancelled or dropped.
	fn add_task(&mut self, n: NodeId, borrow: Borrow) {
		self.tasks.borrows.insert(n, borrow);
		if self.body.flow.reachable && !self.body.flow.tasks.contains(&n) {
			self.body.flow.tasks.push(n);
		}
	}

	/// The `start` nodes of the tasks an expression designates: `start`s in
	/// it, the tasks of the locals it names, of the combinators it calls.
	fn task_starts(&self, n: NodeId) -> Vec<NodeId> {
		let mut found = Vec::new();
		let mut stack = vec![n];
		while let Some(node) = stack.pop() {
			match self.view(node) {
				View::Start(_) => {
					found.push(node);
					continue;
				}
				View::Name(_) => {
					if let Resolution::Local(symbol) = self.a.resolution(node) {
						found.extend(self.starts_of_symbol(symbol));
					}
					continue;
				}
				_ => {}
			}
			if let Some(group) = self.tasks.groups.get(&node) {
				found.extend(group.iter().copied());
			}
			stack.extend(crate::syntax::children(&self.tree, node));
		}
		found
	}

	/// The `start` nodes of the task a local holds (its declaration's value).
	fn starts_of_symbol(&self, symbol: SymbolId) -> Vec<NodeId> {
		let data = self.a.symbols.symbol(symbol);
		if !matches!(data.kind, SymbolKind::Let | SymbolKind::Mutable)
			|| !matches!(self.s.types.kind(data.ty), TypeKind::Task(..))
		{
			return Vec::new();
		}
		let View::Declaration {
			value: Some(value), ..
		} = self.view(data.decl_node)
		else {
			return Vec::new();
		};
		let value = self.strip_groups(value);
		match self.view(value) {
			View::Start(_) => vec![value],
			_ => self.tasks.groups.get(&value).cloned().unwrap_or_default(),
		}
	}

	/// The tasks an awaited expression designates end: their borrows too.
	fn end_tasks_of(&mut self, operand: NodeId) {
		let starts = self.task_starts(operand);
		self.body.flow.tasks.retain(|task| !starts.contains(task));
	}

	/// The end of a block (decision of the second analyzer review): the
	/// tasks bound in it are cancelled and waited for, so their borrows end
	/// (structured tasks). A task that is the block's value leaves it.
	pub(super) fn end_block_tasks(&mut self, scope: crate::ids::ScopeId, value: Option<NodeId>) {
		if self.body.flow.tasks.is_empty() {
			return;
		}
		let kept = value
			.map(|value| self.strip_groups(value))
			.and_then(|value| match self.a.resolution(value) {
				Resolution::Local(symbol) => Some(symbol),
				_ => None,
			});
		let symbols = self.a.symbols.scope(scope).symbols.clone();
		for symbol in symbols {
			if Some(symbol) != kept {
				self.end_task_borrow(symbol);
			}
		}
	}

	/// A task that ends (awaited, cancelled): its borrows end.
	fn end_task_borrow(&mut self, task: SymbolId) {
		let starts = self.starts_of_symbol(task);
		self.body.flow.tasks.retain(|start| !starts.contains(start));
	}

	/// The name of the task started at `start`, for messages.
	fn task_label(&self, start: NodeId) -> String {
		match self.tasks.labels.get(&start) {
			Some(&name) => format!("`{}`", self.tree.name(name)),
			None => format!("the task started on line {}", self.line_of(start)),
		}
	}

	/// A write of `place` (an assignment, a call's effect): the started
	/// call being checked collects it; a pending task that uses the place
	/// forbids it (`REF013`).
	pub(crate) fn task_write(&mut self, place: PlaceId, n: NodeId) {
		if let Some(collected) = &mut self.tasks.collected {
			collected.push(place);
		}
		if self.body.flow.tasks.is_empty() {
			return;
		}
		let pending = self.body.flow.tasks.clone();
		for start in pending {
			let Some(borrow) = self.tasks.borrows.get(&start) else {
				continue;
			};
			let read = borrow
				.reads
				.iter()
				.chain(&borrow.writes)
				.any(|&used| self.may_overlap(used, place));
			if !read {
				continue;
			}
			let modified = borrow
				.writes
				.iter()
				.any(|&used| self.may_overlap(used, place));
			let text = self.place_text(place);
			let task = self.task_label(start);
			let verb = if modified { "modified" } else { "read" };
			let message = format!(
				"`{text}` is {verb} by {task}, which may still run: await it (or cancel it) first"
			);
			let diagnostic = self.diagnostic(&REF013, n, message);
			self.report_flow(diagnostic);
			return;
		}
	}

	/// The read of a local: a pending task that modifies it forbids it
	/// (`REF013`).
	pub(crate) fn task_read(&mut self, n: NodeId, symbol: SymbolId) {
		if self.body.flow.tasks.is_empty() {
			return;
		}
		let pending = self.body.flow.tasks.clone();
		for start in pending {
			let Some(borrow) = self.tasks.borrows.get(&start) else {
				continue;
			};
			let modified = borrow
				.writes
				.iter()
				.any(|&place| self.place_symbol(place) == Some(symbol));
			if !modified {
				continue;
			}
			let name = self.tree.text(n.index()).to_string();
			let task = self.task_label(start);
			let message = format!(
				"`{name}` is modified by {task}, which may still run: await it (or cancel it) first"
			);
			let diagnostic = self.diagnostic(&REF013, n, message);
			self.report_flow(diagnostic);
			return;
		}
	}

	/// The value of a function: a task it returns can't use the function's
	/// locals, which are destroyed when it returns (`CONC018`).
	pub(super) fn task_returned(&mut self, value: NodeId) {
		if !matches!(self.s.types.kind(self.type_of(value)), TypeKind::Task(..)) {
			return;
		}
		let Some(scope) = self.body.functions.last().map(|cx| cx.scope) else {
			return;
		};
		for start in self.task_starts(value) {
			let Some(borrow) = self.tasks.borrows.get(&start) else {
				continue;
			};
			let local = borrow
				.reads
				.iter()
				.chain(&borrow.writes)
				.filter_map(|&place| self.place_symbol(place))
				.find(|&symbol| self.is_local_of(symbol, scope));
			let what = match local {
				Some(symbol) => {
					let name = self
						.s
						.types
						.syms
						.resolve(self.a.symbols.symbol(symbol).name)
						.to_string();
					format!("`{name}`, which is destroyed when the function returns")
				}
				None if borrow.provided => {
					"a provided requirement, which ends with its block".to_string()
				}
				None => continue,
			};
			if !self.reported_once.insert((start, CONC018.id)) {
				continue;
			}
			let message = format!(
				"this task outlives a value it uses: {what}; give the task values it owns (`start do {{ … }}` declaring them), or copies"
			);
			let diagnostic = self.diagnostic(&CONC018, start, message);
			self.report(diagnostic);
		}
	}

	/// A task stored into a collection that outlives the function (a
	/// parameter's, a module-level one): like a returned task, it can't use
	/// the function's locals (`CONC018`).
	pub(crate) fn task_stored(&mut self, value: NodeId) {
		self.task_returned(value);
	}

	/// Whether a symbol is a local of the function being walked (everything
	/// is, at the top level).
	pub(crate) fn is_function_local(&self, symbol: SymbolId) -> bool {
		match self.body.functions.last() {
			Some(cx) => self.is_local_of(symbol, cx.scope),
			None => true,
		}
	}

	/// Whether a symbol is a local of the function whose scope is `scope`
	/// (not a parameter, nor a module-level value).
	fn is_local_of(&self, symbol: SymbolId, scope: crate::ids::ScopeId) -> bool {
		let data = self.a.symbols.symbol(symbol);
		if matches!(
			data.kind,
			SymbolKind::Parameter { .. }
				| SymbolKind::SelfParameter
				| SymbolKind::Requirement { .. }
				| SymbolKind::Function
				| SymbolKind::Type
				| SymbolKind::Namespace
		) {
			return false;
		}
		let mut current = Some(data.scope);
		while let Some(id) = current {
			if id == scope {
				return true;
			}
			let scope_data = self.a.symbols.scope(id);
			if matches!(scope_data.kind, ScopeKind::Module | ScopeKind::Namespace) {
				return false;
			}
			current = scope_data.parent;
		}
		false
	}

	/// `CONC019`: a worker's handle stopped while a started call is pending.
	fn check_worker_stop(&mut self, n: NodeId, callee: NodeId) {
		let View::Access { target, .. } = self.view(self.strip_groups(callee)) else {
			return;
		};
		let Some(handle) = self.read_local(target) else {
			return;
		};
		let pending = self.body.flow.tasks.clone();
		for start in pending {
			let uses = self.tasks.borrows.get(&start).is_some_and(|borrow| {
				borrow
					.reads
					.iter()
					.any(|&place| self.place_symbol(place) == Some(handle))
			});
			if uses && self.reported_once.insert((n, CONC019.id)) {
				let task = self.task_label(start);
				let message = format!(
					"the worker is used by {task}, a call that may still be pending: await it (or cancel it) before stopping the worker"
				);
				let diagnostic = self.diagnostic(&CONC019, n, message);
				self.report_flow(diagnostic);
				return;
			}
		}
	}

	/// `CONC019`: `start worker.method()` of a method that `mutates self` on
	/// a handle that isn't `mutable`.
	fn check_worker_start(&mut self, n: NodeId, call: NodeId) {
		let View::Call { callee, .. } = self.view(call) else {
			return;
		};
		let callee = self.strip_groups(callee);
		let View::Access { target, .. } = self.view(callee) else {
			return;
		};
		if !matches!(
			self.s.types.kind(self.type_of(self.strip_groups(target))),
			TypeKind::Sync(SyncKind::Worker, _)
		) {
			return;
		}
		let Some(handle) = self.read_local(target) else {
			return;
		};
		if self.a.symbols.symbol(handle).kind.is_assignable() {
			return;
		}
		let mutates = self
			.signature_of(self.type_of(callee))
			.is_some_and(|sig| sig.effects != crate::ids::EffectSetId::EMPTY);
		if mutates && self.reported_once.insert((n, CONC019.id)) {
			let diagnostic = self.diagnostic(
				&CONC019,
				n,
				"this worker method `mutates self`, so the worker's handle must be `mutable`: declare it with `mutable`",
			);
			self.report(diagnostic);
		}
	}
}

/// What a kept task uses until it ends.
#[derive(Clone, Debug, Default)]
pub(crate) struct Borrow {
	/// The places it reads (its arguments, the locals a `start do` uses).
	reads: Vec<PlaceId>,
	/// The places its effects change.
	writes: Vec<PlaceId>,
	/// It gets a requirement from a `provide` of the code around it.
	provided: bool,
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn merging_keeps_the_chain_of_the_targets_removed() {
		let mut callee = AsyncSummary::default();
		block(&mut callee, TargetSet::NATIVE, || "lock".to_string());
		callee.suspends = true;
		let mut caller = AsyncSummary::default();
		merge(&mut caller, &callee, Mode::Awaited, "load", true);
		assert!(caller.suspends);
		assert_eq!(caller.blocking, TargetSet::NATIVE);
		assert_eq!(caller.chain(TargetSet::WEB), Some("load → lock"));
		let mut blocking = AsyncSummary::default();
		merge(&mut blocking, &callee, Mode::Blocking, "load", false);
		assert!(!blocking.suspends);
	}

	fn summary_of(session: &crate::Session, file: crate::ids::FileId, name: &str) -> AsyncSummary {
		let analysis = session.analysis(file);
		let declaration = analysis
			.functions
			.iter()
			.find_map(|(&key, info)| {
				let symbol = info.symbol?;
				let text = session
					.types()
					.syms
					.resolve(analysis.symbols.symbol(symbol).name);
				(text == name && Some(key) == info.declaration).then_some(key)
			})
			.expect("a function");
		analysis.async_summaries[&declaration].clone()
	}

	/// The mode of a standalone program, with its reason and diagnostic codes.
	fn mode(source: &str, configured: Option<bool>) -> (bool, String, Vec<&'static str>) {
		let mut session = crate::Session::new();
		session.set_source("main", std::sync::Arc::from(source));
		let mut program = session.analyze_program("main").expect("a program");
		session.configure_asynchronous(&mut program, configured, false);
		let entry = program.entry.expect("an entry");
		let codes = session
			.diagnostics(entry)
			.iter()
			.map(|diagnostic| diagnostic.code.id)
			.collect();
		(program.asynchronous, program.asynchronous_reason, codes)
	}

	#[test]
	fn the_mode_is_detected_from_the_entry() {
		// nothing async
		let (on, reason, codes) = mode("console.log(\"hello\")\n", None);
		assert!(!on, "{reason}");
		assert!(codes.is_empty(), "{codes:?}");
		// `start` alone: deferred start changes the output order
		let (on, reason, _) = mode(
			"function work = () => console.log(\"work\")\nstart work()\n",
			None,
		);
		assert!(on);
		assert!(reason.contains("starts a task"), "{reason}");
		// an awaited function that can't suspend is a plain call
		let (on, reason, codes) = mode(
			"async function work = (): Integer => 1\nlet value = await work()\n",
			None,
		);
		assert!(!on, "{reason}");
		assert!(codes.is_empty(), "{codes:?}");
		// an `async` entry function that never waits
		let (on, _, _) = mode(
			"export async function = () => console.log(\"main\")\n",
			None,
		);
		assert!(!on);
		// a suspending await, through a function
		let (on, reason, _) = mode(
			"use { sleep } = std.time\nasync function pause = () => await sleep(1)\nawait pause()\n",
			None,
		);
		assert!(on);
		assert!(
			reason.contains("awaits `pause`, which awaits `sleep`"),
			"{reason}"
		);
		// forced off while the program starts tasks: WCONC001
		let (on, reason, codes) = mode(
			"function work = () => console.log(\"work\")\nstart work()\n",
			Some(false),
		);
		assert!(!on);
		assert!(reason.starts_with("off"), "{reason}");
		assert_eq!(codes, ["WCONC001"]);
		// forced on
		let (on, _, codes) = mode("console.log(\"hello\")\n", Some(true));
		assert!(on);
		assert!(codes.is_empty());
	}

	#[test]
	fn the_side_tables_describe_awaits_and_starts() {
		let source = "async function work = (): Integer => 1\nuse { sleep } = std.time\nasync function main = () => {\n\tlet task = start work()\n\tlet value = await task\n\tlet direct = await work()\n\tawait sleep(1)\n\tstart do {\n\t\tconsole.log(\"{value} {direct}\")\n\t}\n}\nfunction plain = () => main()\n";
		let (session, file) = crate::session::analyze_source(source);
		assert!(
			session.diagnostics(file).is_empty(),
			"{:?}",
			session.diagnostics(file)
		);
		let analysis = session.analysis(file);
		let mut kinds: Vec<(AwaitKind, bool)> = analysis
			.awaits
			.values()
			.map(|info| (info.kind.clone(), info.suspends))
			.collect();
		kinds.sort_by_key(|(kind, _)| format!("{kind:?}"));
		assert_eq!(
			kinds,
			[
				(AwaitKind::Call, false),
				(AwaitKind::Primitive("sleep"), true),
				(AwaitKind::Task, true),
			]
		);
		let mut detached: Vec<(bool, bool)> = analysis
			.starts
			.values()
			.map(|info| (info.detached, info.block.is_some()))
			.collect();
		detached.sort();
		assert_eq!(detached, [(false, false), (true, true)]);
		let work = summary_of(&session, file, "work");
		assert!(!work.suspends && !work.starts);
		assert_eq!(work.blocking, TargetSet::all());
		let main = summary_of(&session, file, "main");
		assert!(main.suspends && main.starts);
		assert_eq!(
			main.blocking,
			TargetSet::NATIVE | TargetSet::BUN | TargetSet::NODE
		);
		// a function that isn't async never suspends, but starts what it calls
		let plain = summary_of(&session, file, "plain");
		assert!(!plain.suspends && plain.starts);
		// the interface publishes them, and its hash depends on them
		let interface = &session.source_file(file).interface;
		assert!(
			interface
				.async_summaries
				.values()
				.any(|summary| summary.suspends)
		);
		let mut changed = interface.clone();
		let hash = interface.hash(session.types());
		for summary in changed.async_summaries.values_mut() {
			summary.suspends = false;
		}
		assert_ne!(changed.hash(session.types()), hash);
	}
}
