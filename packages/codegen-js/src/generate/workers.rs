//! Workers on JavaScript (design/async.md §3.4, step J2; runtime
//! `workers.js`): a worker is a thread (`worker_threads` on Bun and Node, a
//! Web Worker on the web) and its calls are messages.
//!
//! - `MathWorker { … }` is `new FaWorker(state)`: the thread receives a copy
//!   of the state.
//! - A call names its method as `"<URL of the worker's module>#<export>"`
//!   (`new URL("./MathWorker.worker.js", import.meta.url)`): the thread
//!   imports the module on its first call. Awaited, it is `await
//!   faWorkerCall($task, worker, method, [arguments], [error classes])`;
//!   started, `faWorkerStart(…)` (the message leaves at once). The error
//!   classes are the ones the method may throw, rebuilt on this thread.
//! - `WorkerPool(T) { … }` is `new FaWorkerPool(count, state, maxQueueSize,
//!   timeout)`; `await pool.submit(work)` is `await faPoolSubmit($task,
//!   pool, work)`.
//! - A blocking call of a worker, or of `submit`, is `CONC010` there (the
//!   analyzer reports it): only threads of `native` can wait.

use fa_analyzer::analysis::{Argument, CallInfo, Callee, FieldValue, ObjectInfo};
use fa_analyzer::ids::{NodeId, TypeId};
use fa_analyzer::syntax::View;
use fa_analyzer::types::{SyncKind, TypeKind};

use crate::generate::Generator;
use crate::generate::tasks::Mode;
use crate::js::{self, Expr, Prec};

impl Generator<'_> {
	/// Whether a node's value is a worker (`Sync(Worker, T)`).
	pub(crate) fn is_worker(&self, node: NodeId) -> bool {
		matches!(
			self.types.kind(self.type_of(self.strip_groups(node))),
			TypeKind::Sync(SyncKind::Worker, _)
		)
	}

	/// `MathWorker { … }`: the worker made from the state object built.
	pub(crate) fn worker_literal(&mut self, state: Expr) -> Expr {
		let class = self.helper("FaWorker");
		Expr::primary(format!("new {class}({})", state.text))
	}

	/// `WorkerPool(T) { count, state, maxQueueSize, timeout }`.
	pub(crate) fn pool_literal(&mut self, n: NodeId, info: &ObjectInfo) -> Expr {
		let mut values = vec!["0".to_string(), "undefined".to_string()];
		values.extend(["undefined".to_string(), "undefined".to_string()]);
		for (name, value) in &info.fields {
			let slot = match self.types.syms.resolve(*name) {
				"count" => 0,
				"state" => 1,
				"maxQueueSize" => 2,
				_ => 3,
			};
			if let FieldValue::Given(node) = value {
				values[slot] = self.expr(*node).text;
			}
		}
		// a missing `state`: `T {}` (the object entry of the literal's type node)
		if values[1] == "undefined"
			&& let View::TypedObject { target, .. } = self.view(n)
			&& self.analysis.object(target).is_some()
		{
			values[1] = self.object_literal(target).text;
		}
		// its workers: `$worker.js` is written (`fa_codegen_js::worker_module`)
		self.helper("FaWorker");
		let class = self.helper("FaWorkerPool");
		Expr::primary(format!("new {class}({})", values.join(", ")))
	}

	/// A call of a method of a worker: awaited or started (a blocking call
	/// is `CONC010` here). `None` when the receiver isn't a worker.
	pub(crate) fn worker_call(
		&mut self,
		n: NodeId,
		callee: NodeId,
		info: &CallInfo,
		arguments: fa_analyzer::syntax::NodeList<'_>,
	) -> Option<Expr> {
		let View::Access { members, .. } = self.view(self.strip_groups(callee)) else {
			return None;
		};
		// `stop()` is a built-in (`worker_builtin`)
		if !matches!(
			self.types.kind(self.receiver_type(callee)),
			TypeKind::Sync(SyncKind::Worker, _)
		) || matches!(info.callee, Callee::Builtin(_))
		{
			return None;
		}
		let member = members.last()?;
		let Some(external) = self.external(self.analysis.resolution(member)) else {
			return Some(self.unsupported(n, "this method of a worker"));
		};
		let started = self.tasks.pending_start == Some(n);
		if started {
			self.tasks.pending_start = None;
		}
		let awaited = self.tasks.awaited_calls.contains(&n);
		let asynchronous = matches!(self.mode(), Mode::Async { .. });
		if !started && !(awaited && asynchronous) {
			return Some(self.cannot_block(n, "a worker's method"));
		}
		let receiver = self.receiver(callee);
		let rest: Vec<Argument> = info
			.arguments
			.iter()
			.filter(|argument| !matches!(argument, Argument::Receiver(_)))
			.cloned()
			.collect();
		let (receiver, values) = self.arguments(n, receiver, &rest, arguments, &[]);
		let path = match self.program {
			Some(program) => super::modules::relative(&program.path, &external.path),
			None => format!("./{}", external.path),
		};
		let method = format!(
			"`${{new URL({}, import.meta.url).href}}#{}`",
			js::string_literal(&path),
			external.export
		);
		let classes = self.thrown_classes(self.type_of(callee));
		let all = vec![
			receiver.text,
			method,
			format!("[{}]", values.join(", ")),
			format!("[{}]", classes.join(", ")),
		];
		Some(if started {
			let parent = if asynchronous { "$task" } else { "null" };
			let helper = self.helper("faWorkerStart");
			let mut call = vec![parent.to_string()];
			call.extend(all);
			js::call(helper, &call)
		} else {
			let helper = self.helper("faWorkerCall");
			let mut call = vec!["$task".to_string()];
			call.extend(all);
			let call = js::call(helper, &call);
			Expr::new(format!("await {}", call.text), Prec::Unary)
		})
	}

	/// A blocking call of a worker in a synchronous instance that the
	/// program never calls blocking (the analyzer reports the blocking calls
	/// it can see, `CONC010`): a stop, should it run.
	fn cannot_block(&mut self, n: NodeId, what: &str) -> Expr {
		let helper = self.helper("faStop");
		let reason = js::string_literal(&format!(
			"{what} can't be called blocking on JavaScript: await it"
		));
		let location = self.location(n);
		js::call(helper, &[reason, location])
	}

	/// The classes of the errors a function type may throw.
	fn thrown_classes(&mut self, ty: TypeId) -> Vec<String> {
		let TypeKind::Function(sig) = self.types.kind(ty) else {
			return Vec::new();
		};
		let throws = self.types.sig(sig).throws;
		let mut classes = Vec::new();
		let mut pending = vec![throws];
		while let Some(ty) = pending.pop() {
			match self.types.kind(ty) {
				TypeKind::Member(nominal, index) if self.is_errors(nominal) => {
					classes.push(self.error_class(nominal, index));
				}
				TypeKind::NamedUnion(nominal, mask) if self.is_errors(nominal) => {
					let count = self
						.types
						.sym_list(self.types.nominal(nominal).members)
						.len();
					for index in 0..count as u16 {
						if self.types.mask_has(mask, index) {
							classes.push(self.error_class(nominal, index));
						}
					}
				}
				TypeKind::Union(list) => pending.extend(self.types.list_items(list).iter()),
				_ => {}
			}
		}
		classes
	}

	/// `worker.stop()`, `pool.submit(work)` (awaited) and `pool.shutdown()`;
	/// `None` for another built-in.
	pub(crate) fn worker_builtin(
		&mut self,
		n: NodeId,
		callee: NodeId,
		path: &str,
		arguments: &[Argument],
	) -> Option<Expr> {
		Some(match path {
			"Worker#stop" => {
				let receiver = self.receiver(callee);
				let helper = self.helper("faWorkerStop");
				js::call(helper, &[receiver.text])
			}
			"WorkerPool#shutdown" => {
				let receiver = self.receiver(callee);
				let helper = self.helper("faPoolShutdown");
				js::call(helper, &[receiver.text])
			}
			"WorkerPool#submit" => {
				if !matches!(self.mode(), Mode::Async { .. })
					|| !self.tasks.awaited_calls.contains(&n)
				{
					return Some(self.cannot_block(n, "`submit`"));
				}
				let receiver = self.receiver(callee);
				let work = arguments.iter().find_map(|argument| match argument {
					Argument::Given(value) => Some(*value),
					_ => None,
				})?;
				let work = self.expr(work);
				let helper = self.helper("faPoolSubmit");
				let call = js::call(helper, &["$task".to_string(), receiver.text, work.text]);
				Expr::new(format!("await {}", call.text), Prec::Unary)
			}
			_ => return None,
		})
	}
}
