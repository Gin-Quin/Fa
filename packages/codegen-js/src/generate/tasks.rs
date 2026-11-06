//! Async on JavaScript (design/async.md §4, step J1; runtime `tasks.js`).
//!
//! - **Instances.** In an asynchronous program (`Program::asynchronous`), a
//!   function whose summary `suspends` gets an async instance (a JavaScript
//!   `async function` taking its task, `$task`, first) and/or a synchronous
//!   one, each emitted only when a call needs it ([`Generator::plan_instances`]).
//!   With both, they are `name___async` and `name___sync`; with one, it keeps
//!   the plain name. An exported one always has both (when the target allows
//!   the synchronous one), and `name` is a Promise API for JavaScript callers.
//! - **Modes** ([`Mode`]): the code of an async instance (and of the top
//!   level of an asynchronous program, of a test, of a `start do` there)
//!   awaits for real; a blocking one (a synchronous instance of an `async`
//!   function, an async lambda, everything in synchronous mode) runs `start`
//!   at once (`faFinished`) and reads tasks (`faResult`); the code of a
//!   function that isn't `async` decides at run time (`faStartDynamic`, the
//!   blocking depth).
//! - **Interleaving points**: the awaits of the runtime (`faJoin`, `faSleep`,
//!   `faLock`…) check the cancellation when they resume; the loops of async
//!   instances spend the coop budget (`faCoop`, amendment A2).

use std::collections::{HashMap, HashSet};

use fa_analyzer::analysis::{Argument, AsyncSummary, AwaitKind, Callee, TargetSet};
use fa_analyzer::ids::{NodeId, TypeId};
use fa_analyzer::symbols::Resolution;
use fa_analyzer::syntax::View;
use fa_analyzer::types::{TypeFlags, TypeKind};

use crate::generate::{Dest, Generator};
use crate::js::{self, Expr, Prec, Writer};

/// How the code being generated runs.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub(crate) enum Mode {
	/// A function that isn't `async`: `start` checks the blocking depth.
	#[default]
	Dynamic,
	/// A blocking call: `start` runs the task at once, `await` blocks.
	Blocking,
	/// An async instance: `await` awaits; `coop`: its loops spend the coop
	/// budget (everywhere since the top level and tests yield too:
	/// `design/decisions.md`).
	Async { coop: bool },
}

/// The instances of a function that may suspend.
#[derive(Clone, Debug, Default)]
pub(crate) struct Instances {
	/// The async instance's name, when emitted.
	pub(crate) async_name: Option<String>,
	/// The synchronous instance's name, when emitted.
	pub(crate) sync_name: Option<String>,
	/// Whether it is exported (the Promise API name).
	pub(crate) exported: bool,
	/// The name of its Promise API, once emitted.
	pub(crate) promise_name: Option<String>,
}

/// The async state of a generator.
#[derive(Default)]
pub(crate) struct TasksCx {
	/// The program runs in asynchronous mode.
	pub(crate) asynchronous: bool,
	/// The instances of each function declaration that may suspend.
	pub(crate) instances: HashMap<NodeId, Instances>,
	/// The function declarations that have an async instance.
	pub(crate) async_functions: HashSet<NodeId>,
	/// The calls awaited or started (the operand of an `await` or a `start`).
	pub(crate) awaited_calls: HashSet<NodeId>,
	/// The call being generated as the operand of a `start`.
	pub(crate) pending_start: Option<NodeId>,
	/// The mode of the next function generated (`function_parts`).
	pub(crate) next_mode: Option<Mode>,
	/// The arguments of the detached `start` being generated that its task
	/// copies (`StartInfo::copies`, not moved).
	pub(crate) detached_copies: HashSet<NodeId>,
}

impl Generator<'_> {
	/// The mode of the code being generated.
	pub(crate) fn mode(&self) -> Mode {
		// synchronous mode: the whole program is one blocking call
		if !self.tasks.asynchronous {
			return Mode::Blocking;
		}
		self.fcx.mode
	}

	/// The summary of a function declaration (of this file).
	pub(crate) fn summary(&self, declaration: NodeId) -> Option<&AsyncSummary> {
		self.analysis.async_summaries.get(&declaration)
	}

	/// Whether a function declaration has an async instance: it may
	/// suspend, or it is `async` and loops (its loops spend the coop budget,
	/// amendment A2), in an asynchronous program.
	pub(crate) fn suspends(&self, declaration: NodeId) -> bool {
		self.tasks.async_functions.contains(&declaration)
	}

	/// Whether a body has a loop of its own (not in a nested function).
	fn has_loop(&self, body: NodeId) -> bool {
		let mut stack = vec![body];
		while let Some(node) = stack.pop() {
			match self.view(node) {
				View::Function { .. } | View::ExportFunction { .. } | View::Lambda { .. } => {
					continue;
				}
				View::Loop(_) | View::While { .. } | View::WhileLet { .. } | View::For { .. } => {
					return true;
				}
				_ => {}
			}
			stack.extend(fa_analyzer::syntax::children(self.tree, node));
		}
		false
	}

	/// The target as a set of the analysis.
	fn target_set(&self) -> TargetSet {
		TargetSet::named(self.options.target.name())
	}

	/// Whether the synchronous instance of a function can exist on the target.
	fn blocking_possible(&self, declaration: NodeId) -> bool {
		self.summary(declaration)
			.is_none_or(|summary| summary.blocking.intersects(self.target_set()))
	}

	// ── the plan ─────────────────────────────────────────────────────────

	/// Finds the calls awaited or started, and which instances of the
	/// functions that may suspend are needed (from the top level, the tests,
	/// the exports and the functions used as values), then names them.
	pub(crate) fn plan_instances(&mut self) {
		self.find_awaited_calls();
		if !self.tasks.asynchronous {
			return;
		}
		let declarations = self.find_async_functions();
		let mut wanted: HashMap<NodeId, (bool, bool)> = HashMap::new();
		let mut pending: Vec<(NodeId, Mode)> = Vec::new();
		let mut walks: Vec<(NodeId, Mode)> = Vec::new();
		// the top level and the tests are async
		let root = NodeId::new(self.tree.root);
		if let View::Module { statements } = self.view(root) {
			for statement in statements.iter() {
				match self.view(statement) {
					View::Function { .. }
					| View::ExportFunction { .. }
					| View::TypeDecl { .. }
					| View::ExportType { .. } => {}
					_ => walks.push((statement, Mode::Async { coop: true })),
				}
			}
		}
		for &declaration in &declarations {
			let exported = self.is_exported(declaration);
			if !self.suspends(declaration) {
				if let Some(body) = self.function_body(declaration) {
					walks.push((body, self.single_instance_mode(declaration)));
				}
				continue;
			}
			if exported {
				pending.push((declaration, Mode::Async { coop: true }));
				if self.blocking_possible(declaration) {
					pending.push((declaration, Mode::Blocking));
				}
			}
		}
		// the functions used as values: both instances (`faAsyncValue`)
		for index in 0..self.tree.nodes.len() {
			let node = NodeId::new(index);
			if !matches!(self.view(node), View::Name(_)) {
				continue;
			}
			let Resolution::Local(symbol) = self.analysis.resolution(node) else {
				continue;
			};
			let declaration = self.analysis.symbols.symbol(symbol).decl_node;
			if !self.suspends(declaration) || self.is_callee(node) {
				continue;
			}
			pending.push((declaration, Mode::Blocking));
			pending.push((declaration, Mode::Async { coop: true }));
		}
		// A method dictionary is a function value too, even though no name
		// node in the source denotes the function selected for its parameter.
		let mut methods: Vec<_> = self
			.analysis
			.instantiated_methods
			.values()
			.flatten()
			.collect();
		methods.extend(
			self.analysis
				.method_instantiations
				.values()
				.flat_map(|owner| owner.methods.iter()),
		);
		while let Some(method) = methods.pop() {
			methods.extend(method.methods.iter());
			if let Some((module, keyed)) = self.session.keyed(method.function)
				&& module == self.session.module_id(self.file)
				&& self.suspends(keyed.node)
			{
				pending.push((keyed.node, Mode::Async { coop: true }));
				if self.blocking_possible(keyed.node) {
					pending.push((keyed.node, Mode::Blocking));
				}
			}
		}
		loop {
			while let Some((node, mode)) = walks.pop() {
				self.plan_walk(node, mode, &mut pending);
			}
			let Some((declaration, mode)) = pending.pop() else {
				break;
			};
			let entry = wanted.entry(declaration).or_default();
			let seen = match mode {
				Mode::Async { .. } => std::mem::replace(&mut entry.0, true),
				_ => std::mem::replace(&mut entry.1, true),
			};
			if !seen && let Some(body) = self.function_body(declaration) {
				walks.push((body, mode));
			}
		}
		// the names
		let mut wanted: Vec<(NodeId, (bool, bool))> = wanted.into_iter().collect();
		wanted.sort_by_key(|(declaration, _)| *declaration);
		for (declaration, (asynchronous, synchronous)) in wanted {
			let Some(symbol) = self.analysis.symbols.symbol_of_decl(declaration) else {
				continue;
			};
			let exported = self.is_exported(declaration);
			let base = self.name_of(symbol);
			let mut instances = Instances {
				exported,
				..Instances::default()
			};
			if asynchronous && (synchronous || exported) {
				instances.async_name = Some(self.scopes.declare_global(&format!("{base}___async")));
				if synchronous {
					instances.sync_name = Some(self.declare_as(symbol, &format!("{base}___sync")));
				}
			} else if asynchronous {
				instances.async_name = Some(base);
			} else {
				instances.sync_name = Some(base);
			}
			self.tasks.instances.insert(declaration, instances);
		}
	}

	/// The calls awaited or started (`awaited_calls`).
	fn find_awaited_calls(&mut self) {
		for &node in self.analysis.awaits.keys() {
			let View::Await(operand) = self.view(node) else {
				continue;
			};
			let operand = self.strip_groups(operand);
			match self.view(operand) {
				View::Call { .. } => {
					self.tasks.awaited_calls.insert(operand);
				}
				View::Start(call) => {
					let call = self.strip_groups(call);
					if matches!(self.view(call), View::Call { .. }) {
						self.tasks.awaited_calls.insert(call);
					}
				}
				_ => {}
			}
		}
		for (&node, info) in self.analysis.starts.iter() {
			if info.block.is_some() {
				continue;
			}
			if let View::Start(call) = self.view(node) {
				let call = self.strip_groups(call);
				if matches!(self.view(call), View::Call { .. }) {
					self.tasks.awaited_calls.insert(call);
				}
			}
		}
	}

	/// The function declarations of the file; those with an async instance
	/// go to `async_functions`.
	fn find_async_functions(&mut self) -> Vec<NodeId> {
		// every function declaration of the file
		let mut declarations = Vec::new();
		for index in 0..self.tree.nodes.len() {
			let node = NodeId::new(index);
			if matches!(
				self.view(node),
				View::Function { .. } | View::ExportFunction { .. }
			) && self.analysis.symbols.symbol_of_decl(node).is_some()
			{
				declarations.push(node);
			}
		}
		for &declaration in &declarations {
			let Some(info) = self.function_info(declaration) else {
				continue;
			};
			let is_async = info.sig.is_some_and(|sig| self.types.sig(sig).is_async);
			let suspends = self
				.summary(declaration)
				.is_some_and(|summary| summary.suspends);
			// an `async` function that loops gets an async instance (its loops
			// spend the coop budget), unless it starts tasks: one that can't
			// suspend has one instance, as natively (`single_instance_mode`)
			let starts = self
				.summary(declaration)
				.is_some_and(|summary| summary.starts);
			if suspends || (is_async && !starts && self.has_loop(info.body)) {
				self.tasks.async_functions.insert(declaration);
			}
		}
		declarations
	}

	/// Whether a function declaration is exported (to the other modules of
	/// a program, or as the entry function of a file).
	fn is_exported(&self, declaration: NodeId) -> bool {
		matches!(self.view(declaration), View::ExportFunction { .. })
			|| self
				.program
				.is_some_and(|program| program.exports.contains_key(&declaration))
	}

	/// The exports of a function of this module that may suspend (`None`
	/// for another declaration): its name gives the Promise API (or the
	/// synchronous instance in synchronous mode), `name$sync` and
	/// `name$async` its instances, for the other modules.
	pub(crate) fn instance_exports(
		&self,
		declaration: NodeId,
		local: &str,
		export: &str,
	) -> Option<Vec<String>> {
		let summary = self.summary(declaration)?;
		if !summary.suspends {
			return None;
		}
		let Some(instances) = self.tasks.instances.get(&declaration) else {
			// synchronous mode, or a function never used: one instance
			return Some(vec![
				format!("{local} as {export}"),
				format!("{local} as {export}$sync"),
			]);
		};
		let mut exports = Vec::new();
		let main = instances
			.promise_name
			.as_ref()
			.or(instances.sync_name.as_ref())
			.or(instances.async_name.as_ref())?;
		exports.push(format!("{main} as {export}"));
		if let Some(sync) = &instances.sync_name {
			exports.push(format!("{sync} as {export}$sync"));
		}
		if let Some(name) = &instances.async_name {
			exports.push(format!("{name} as {export}$async"));
		}
		Some(exports)
	}

	/// A call of a function of another module that may suspend: the module
	/// path and its export name.
	pub(crate) fn external_suspending(&self, callee: NodeId) -> Option<(String, String)> {
		self.program?;
		let callee = self.strip_groups(callee);
		let node = match self.view(callee) {
			View::Access { members, .. } => members.last()?,
			View::Name(_) => callee,
			_ => return None,
		};
		let (module, declaration) = self.session.external_node(self.analysis.resolution(node))?;
		let interface = self.session.interface(module)?;
		if !interface
			.async_summaries
			.get(&declaration)
			.is_some_and(|summary| summary.suspends)
		{
			return None;
		}
		let export = interface.export_names.get(&declaration)?.clone();
		Some((
			crate::generate::modules::js_path(self.types.module_path(module)),
			export,
		))
	}

	/// The body of a function declaration.
	fn function_body(&self, declaration: NodeId) -> Option<NodeId> {
		self.function_info(declaration).map(|info| info.body)
	}

	/// The function of a declaration.
	fn function_info(&self, declaration: NodeId) -> Option<&fa_analyzer::analysis::FnInfo> {
		let value = match self.view(declaration) {
			View::Function { value, .. } | View::ExportFunction { value, .. } => value,
			_ => return None,
		};
		self.analysis
			.function(declaration)
			.or_else(|| self.analysis.function(value))
	}

	/// Whether a name is the callee of a call.
	fn is_callee(&mut self, node: NodeId) -> bool {
		let parents = self.parents();
		let mut current = node;
		while let Some(parent) = parents[current.index()] {
			match self.view(parent) {
				View::Group(_) => current = parent,
				View::Call { callee, .. } => return self.strip_groups(callee) == node,
				View::Access { target, .. } => return target != current,
				_ => return false,
			}
		}
		false
	}

	/// The instances a body in `mode` needs.
	fn plan_walk(&self, node: NodeId, mode: Mode, pending: &mut Vec<(NodeId, Mode)>) {
		let mut stack = vec![(node, mode)];
		while let Some((node, mode)) = stack.pop() {
			match self.view(node) {
				View::Function { .. } | View::ExportFunction { .. } => continue,
				View::Lambda { .. } => {
					// a lambda is called through its value: blocking, and
					// awaited too when it may suspend (`faAsyncValue`)
					let lambda_mode = match self.analysis.function(node) {
						Some(info) if info.sig.is_some_and(|sig| self.types.sig(sig).is_async) => {
							Mode::Blocking
						}
						_ => Mode::Dynamic,
					};
					let both = self.lambda_suspends(node);
					for child in fa_analyzer::syntax::children(self.tree, node) {
						stack.push((child, lambda_mode));
						if both {
							stack.push((child, Mode::Async { coop: true }));
						}
					}
					continue;
				}
				View::Start(_) => {
					if let Some(block) = self.analysis.starts.get(&node).and_then(|info| info.block)
					{
						match mode {
							Mode::Dynamic => {
								stack.push((block, Mode::Async { coop: true }));
								stack.push((block, Mode::Blocking));
							}
							Mode::Async { .. } => stack.push((block, Mode::Async { coop: true })),
							Mode::Blocking => stack.push((block, Mode::Blocking)),
						}
						continue;
					}
				}
				View::Call { .. } => {
					if let Some(info) = self.analysis.call(node)
						&& let Callee::Function(declaration) = info.callee
						&& self.suspends(declaration)
					{
						let awaited = self.tasks.awaited_calls.contains(&node);
						match mode {
							Mode::Async { .. } if awaited => {
								pending.push((declaration, Mode::Async { coop: true }));
							}
							Mode::Dynamic if awaited => {
								pending.push((declaration, Mode::Async { coop: true }));
								pending.push((declaration, Mode::Blocking));
							}
							_ => pending.push((declaration, Mode::Blocking)),
						}
					}
				}
				_ => {}
			}
			for child in fa_analyzer::syntax::children(self.tree, node) {
				stack.push((child, mode));
			}
		}
	}

	/// The mode of the one instance of a function: an `async` function that
	/// can't suspend is a blocking call (A4), any other is `Dynamic`. One
	/// that starts tasks is `Dynamic` too: awaited, it is not a blocking
	/// call, so its tasks start later (`concurrency.md`); called blocking,
	/// the caller raises the blocking depth ([`Generator::blocking_starter`]).
	fn single_instance_mode(&self, declaration: NodeId) -> Mode {
		let is_async = self
			.function_info(declaration)
			.and_then(|info| info.sig)
			.is_some_and(|sig| self.types.sig(sig).is_async);
		let starts = self
			.summary(declaration)
			.is_some_and(|summary| summary.starts);
		if is_async && !starts && self.tasks.asynchronous {
			Mode::Blocking
		} else {
			Mode::Dynamic
		}
	}

	/// Whether a call of `declaration` is a blocking call of an `async`
	/// function that starts tasks without suspending: not awaited nor
	/// started, outside a blocking call. The caller raises the blocking
	/// depth around it, so that its `start`s run at once.
	fn blocking_starter(&self, n: NodeId, declaration: NodeId) -> bool {
		let Some(summary) = self.summary(declaration) else {
			return false;
		};
		let is_async = self
			.function_info(declaration)
			.and_then(|info| info.sig)
			.is_some_and(|sig| self.types.sig(sig).is_async);
		self.tasks.asynchronous
			&& is_async
			&& summary.starts
			&& !self.suspends(declaration)
			&& !self.tasks.awaited_calls.contains(&n)
			&& self.mode() != Mode::Blocking
	}

	/// The mode of the instance of `declaration` generated next.
	pub(crate) fn instance_modes(&self, declaration: NodeId) -> Vec<(Mode, Option<String>)> {
		match self.tasks.instances.get(&declaration) {
			Some(instances) => {
				let mut modes = Vec::new();
				if let Some(name) = &instances.async_name {
					modes.push((Mode::Async { coop: true }, Some(name.clone())));
				}
				if let Some(name) = &instances.sync_name {
					modes.push((Mode::Blocking, Some(name.clone())));
				}
				modes
			}
			None => vec![(self.single_instance_mode(declaration), None)],
		}
	}

	/// The lines after the instances of an exported function: its Promise API.
	pub(crate) fn promise_api(&mut self, declaration: NodeId) -> Option<(String, String)> {
		let instances = self.tasks.instances.get(&declaration)?;
		if !instances.exported {
			return None;
		}
		let async_name = instances.async_name.clone()?;
		let symbol = self.analysis.symbols.symbol_of_decl(declaration)?;
		let base = self
			.types
			.syms
			.resolve(self.analysis.symbols.symbol(symbol).name)
			.to_string();
		let helper = self.helper("faPromise");
		if let Some(instances) = self.tasks.instances.get_mut(&declaration) {
			instances.promise_name = Some(base.clone());
		}
		Some((base, format!("(...args) => {helper}({async_name}, args)")))
	}

	// ── calls ────────────────────────────────────────────────────────────

	/// The end of a call: `f(values)`, or its awaited async instance, or the
	/// task of a `start`.
	pub(crate) fn finish_call(
		&mut self,
		n: NodeId,
		declaration: Option<NodeId>,
		function: String,
		values: Vec<String>,
	) -> Expr {
		self.finish_call_of(n, declaration, None, function, values)
	}

	/// [`Generator::finish_call`] of a function of another module that may
	/// suspend (`external`: its module path and export name): its instances
	/// are its exports `name$sync` and `name$async`.
	pub(crate) fn finish_call_of(
		&mut self,
		n: NodeId,
		declaration: Option<NodeId>,
		external: Option<(String, String)>,
		function: String,
		mut values: Vec<String>,
	) -> Expr {
		let mut suspends = declaration.is_some_and(|declaration| self.suspends(declaration));
		let mut instances = declaration
			.and_then(|declaration| self.tasks.instances.get(&declaration))
			.cloned()
			.unwrap_or_default();
		if let Some((path, export)) = &external {
			suspends = true;
			let asynchronous = self.tasks.asynchronous
				&& (self.tasks.pending_start == Some(n) || self.tasks.awaited_calls.contains(&n))
				&& self.mode() != Mode::Blocking;
			if asynchronous {
				instances.async_name = Some(self.import(path, &format!("{export}$async")));
			}
			if !asynchronous || self.mode() == Mode::Dynamic {
				instances.sync_name = Some(self.import(path, &format!("{export}$sync")));
			}
		}
		// a value of an `async` function type: its instances are found at
		// run time (`$async`, design/async.md §2.2)
		if declaration.is_none() && external.is_none() && self.calls_async_value(n) {
			let started = self.tasks.pending_start == Some(n);
			let awaited = self.tasks.awaited_calls.contains(&n);
			if started {
				self.tasks.pending_start = None;
			}
			values.insert(0, function.clone());
			return match self.mode() {
				Mode::Async { .. } if started => {
					let helper = self.helper("faStartValue");
					values.insert(0, "$task".to_string());
					js::call(helper, &values)
				}
				Mode::Dynamic if started => {
					let helper = self.helper("faStartValueDynamic");
					js::call(helper, &values)
				}
				Mode::Async { .. } if awaited => {
					let helper = self.helper("faCallValue");
					values.insert(0, "$task".to_string());
					let call = js::call(helper, &values);
					Expr::new(format!("await {}", call.text), Prec::Unary)
				}
				_ if started => {
					let helper = self.helper("faFinished");
					js::call(helper, &values)
				}
				_ => {
					values.remove(0);
					js::call(&function, &values)
				}
			};
		}
		let async_name = instances
			.async_name
			.clone()
			.unwrap_or_else(|| function.clone());
		let sync_name = instances
			.sync_name
			.clone()
			.unwrap_or_else(|| function.clone());
		if self.tasks.pending_start == Some(n) {
			self.tasks.pending_start = None;
			return match self.mode() {
				Mode::Async { .. } => {
					let (helper, name) = if suspends {
						(self.helper("faStart"), async_name)
					} else {
						(self.helper("faStartCall"), sync_name)
					};
					values.splice(0..0, ["$task".to_string(), name]);
					js::call(helper, &values)
				}
				Mode::Blocking => {
					let helper = self.helper("faFinished");
					values.insert(0, sync_name);
					js::call(helper, &values)
				}
				Mode::Dynamic => {
					let helper = self.helper("faStartDynamic");
					let first = if suspends && instances.async_name.is_some() {
						async_name
					} else {
						"undefined".to_string()
					};
					values.splice(0..0, [first, sync_name]);
					js::call(helper, &values)
				}
			};
		}
		if suspends
			&& matches!(self.mode(), Mode::Async { .. })
			&& self.tasks.awaited_calls.contains(&n)
		{
			values.insert(0, "$task".to_string());
			let call = js::call(&async_name, &values);
			return Expr::new(format!("await {}", call.text), Prec::Unary);
		}
		if suspends {
			return js::call(&sync_name, &values);
		}
		if declaration.is_some_and(|declaration| self.blocking_starter(n, declaration)) {
			let helper = self.helper("faBlocking");
			values.insert(0, function);
			return js::call(helper, &values);
		}
		js::call(&function, &values)
	}

	/// Whether a call goes through a value of an `async` function type, in
	/// an asynchronous program.
	fn calls_async_value(&self, n: NodeId) -> bool {
		if !self.tasks.asynchronous {
			return false;
		}
		let View::Call { callee, .. } = self.view(n) else {
			return false;
		};
		if !self
			.analysis
			.call(n)
			.is_some_and(|info| info.callee == Callee::Value)
		{
			return false;
		}
		match self.types.kind(self.type_of(callee)) {
			TypeKind::Function(sig) => self.types.sig(sig).is_async,
			_ => false,
		}
	}

	/// The value of a function that may suspend, named in value position
	/// (not called): its synchronous instance carrying its async one
	/// (`faAsyncValue`); `None` for another name.
	pub(crate) fn async_function_value(&mut self, n: NodeId, declaration: NodeId) -> Option<Expr> {
		let instances = self.tasks.instances.get(&declaration)?;
		let (Some(sync), Some(asynchronous)) =
			(instances.sync_name.clone(), instances.async_name.clone())
		else {
			return None;
		};
		if self.is_callee(n) {
			return None;
		}
		let helper = self.helper("faAsyncValue");
		Some(js::call(helper, &[sync, asynchronous]))
	}

	/// Whether a lambda has two instances (`faAsyncValue`): it is `async`
	/// and may suspend, in an asynchronous program.
	pub(crate) fn lambda_suspends(&self, lambda: NodeId) -> bool {
		self.tasks.asynchronous
			&& self
				.analysis
				.function(lambda)
				.is_some_and(|info| info.sig.is_some_and(|sig| self.types.sig(sig).is_async))
			&& self.summary(lambda).is_some_and(|summary| summary.suspends)
	}

	// ── await ────────────────────────────────────────────────────────────

	/// `await operand`.
	pub(crate) fn await_expr(&mut self, n: NodeId, operand: NodeId) -> Expr {
		let Some(info) = self.analysis.awaits.get(&n).cloned() else {
			return self.expr(operand);
		};
		let asynchronous = matches!(self.mode(), Mode::Async { .. });
		let stripped = self.strip_groups(operand);
		match info.kind {
			AwaitKind::Call | AwaitKind::FunctionValue => self.expr(operand),
			AwaitKind::Task => {
				// `await start f()`: a direct call of the async instance (A15)
				if let View::Start(call) = self.view(stripped) {
					let call = self.strip_groups(call);
					if asynchronous
						&& self
							.analysis
							.starts
							.get(&stripped)
							.is_some_and(|start| start.block.is_none())
						&& let Some(callee) = self.analysis.call(call)
						&& let Callee::Function(declaration) = callee.callee
						&& self.suspends(declaration)
					{
						return self.expr(call);
					}
				}
				let task = self.expr(operand);
				if asynchronous {
					let helper = self.helper("faJoin");
					let call = js::call(helper, &["$task".to_string(), task.text]);
					Expr::new(format!("await {}", call.text), Prec::Unary)
				} else {
					let helper = self.helper("faResult");
					js::call(helper, &[task.text])
				}
			}
			AwaitKind::Primitive(name) => self.await_primitive(n, name, stripped, asynchronous),
		}
	}

	/// `await` of a primitive that waits.
	fn await_primitive(&mut self, n: NodeId, name: &str, call: NodeId, asynchronous: bool) -> Expr {
		let View::Call { callee, .. } = self.view(call) else {
			return self.expr(call);
		};
		let arguments = self
			.analysis
			.call(call)
			.map(|info| info.arguments.clone())
			.unwrap_or_default();
		let awaited = |helper: &str, values: Vec<String>| {
			let call = js::call(helper, &values);
			Expr::new(format!("await {}", call.text), Prec::Unary)
		};
		match name {
			"Task.yield" => {
				if asynchronous {
					let helper = self.helper("faYield");
					awaited(helper, vec!["$task".to_string()])
				} else {
					Expr::constant("undefined", Prec::Primary)
				}
			}
			"sleep" | "std.time.sleep" => {
				let values = self.given_values(&arguments);
				if asynchronous {
					let helper = self.helper("faSleep");
					let mut all = vec!["$task".to_string()];
					all.extend(values);
					awaited(helper, all)
				} else {
					let helper = self.helper("faSleepSync");
					js::call(helper, &values)
				}
			}
			"Mutex.lock" | "Channel.send" | "Channel.receive" => {
				let receiver = self.receiver(callee);
				let (lines, values) = self.capture(|g| g.given_values_copied(&arguments));
				let receiver = self.keep_before(receiver, &lines);
				self.out.append(lines);
				let location = self.location(n);
				if !asynchronous && name == "Mutex.lock" {
					// on a worker's thread, a mutex shared with it is waited for
					let helper = self.helper("faLockSync");
					let result = js::call(helper, &[receiver.text, location]);
					return self.guard_temporary(call, result);
				}
				if !asynchronous {
					// on a worker's thread, a channel shared with it is waited for
					let helper = self.helper(if name == "Channel.send" {
						"faSendSync"
					} else {
						"faReceiveSync"
					});
					let mut all = vec![receiver.text];
					all.extend(values);
					all.push(location);
					return js::call(helper, &all);
				}
				// ready at once: no `await` (an `await` that doesn't wait is a
				// plain call, A1), the wait otherwise
				let receiver = self.keep_value(receiver);
				let values: Vec<String> = values
					.into_iter()
					.map(|value| self.keep_value(Expr::new(value, Prec::Assign)).text)
					.collect();
				let (now, helper) = match name {
					"Mutex.lock" => (self.helper("faLockNow"), self.helper("faLock")),
					"Channel.send" => (self.helper("faSendNow"), self.helper("faSend")),
					_ => (self.helper("faReceiveNow"), self.helper("faReceive")),
				};
				let mut quick = vec![receiver.text.clone()];
				quick.extend(values.iter().cloned());
				let quick = js::call(now, &quick);
				let mut all = vec!["$task".to_string(), receiver.text];
				all.extend(values);
				all.push(location);
				let slow = awaited(helper, all);
				let result = if name == "Channel.send" {
					// `faSendNow` gives whether it was sent
					Expr::new(format!("{} || {}", quick.text, slow.at(Prec::Or)), Prec::Or)
				} else {
					let waiting = self.helper("faWaiting");
					let temp = self.inline_temp();
					Expr::new(
						format!(
							"({temp} = {}) !== {waiting} ? {temp} : {}",
							quick.text,
							slow.at(Prec::Assign)
						),
						Prec::Conditional,
					)
				};
				if name == "Mutex.lock" {
					return self.guard_temporary(call, result);
				}
				result
			}
			// `std.fs` and `std.net` (system.rs, net.js): awaited in an async
			// instance, blocking otherwise (files only: a blocking socket
			// is `CONC010` here)
			_ if asynchronous && self.io_await_helper(name).is_some() => {
				let (helper, takes_receiver) = self.io_await_helper(name).expect("a primitive");
				let mut all = vec!["$task".to_string()];
				if takes_receiver {
					let receiver = self.receiver(callee);
					let (lines, values) = self.capture(|g| g.given_values(&arguments));
					let receiver = self.keep_before(receiver, &lines);
					self.out.append(lines);
					all.push(receiver.text);
					all.extend(values);
				} else {
					all.extend(self.given_values(&arguments));
				}
				awaited(helper, all)
			}
			// a worker's calls run on the caller's thread (D31)
			_ => self.expr(call),
		}
	}

	/// `start` of a primitive that waits, in an async instance: a task
	/// running its awaited form (`faStart($task, faReadFileAsync, path)`);
	/// `None` for another call.
	fn start_primitive(&mut self, call: NodeId) -> Option<Expr> {
		if !matches!(self.mode(), Mode::Async { .. }) {
			return None;
		}
		let View::Call { callee, .. } = self.view(call) else {
			return None;
		};
		let info = self.analysis.call(call)?.clone();
		let Callee::Builtin(builtin) = info.callee else {
			return None;
		};
		let path = fa_analyzer::builtins::get(builtin).path;
		let name = match path {
			"std.time.sleep" => "sleep",
			"TcpListener#accept" => "TcpListener.accept",
			"TcpStream#read" => "TcpStream.read",
			"TcpStream#write" => "TcpStream.write",
			_ => path,
		};
		let (helper, takes_receiver) = if name == "sleep" {
			(self.helper("faSleep"), false)
		} else {
			self.io_await_helper(name)?
		};
		let start = self.helper("faStart");
		let mut all = vec!["$task".to_string(), helper.to_string()];
		if takes_receiver {
			let receiver = self.receiver(callee);
			let (lines, values) = self.capture(|g| g.given_values(&info.arguments));
			let receiver = self.keep_before(receiver, &lines);
			self.out.append(lines);
			all.push(receiver.text);
			all.extend(values);
		} else {
			all.extend(self.given_values(&info.arguments));
		}
		Some(js::call(start, &all))
	}

	/// The values of the given arguments, in order.
	fn given_values(&mut self, arguments: &[Argument]) -> Vec<String> {
		let nodes: Vec<NodeId> = arguments
			.iter()
			.filter_map(|argument| match argument {
				Argument::Given(value) => Some(*value),
				_ => None,
			})
			.collect();
		self.in_order(&nodes, |g, node| g.expr(node))
			.into_iter()
			.map(|value| value.text)
			.collect()
	}

	/// The values of the given arguments, copied as stored values.
	fn given_values_copied(&mut self, arguments: &[Argument]) -> Vec<String> {
		let nodes: Vec<NodeId> = arguments
			.iter()
			.filter_map(|argument| match argument {
				Argument::Given(value) => Some(*value),
				_ => None,
			})
			.collect();
		self.in_order(&nodes, |g, node| {
			let value = g.expr(node);
			g.copy_stored(node, value)
		})
		.into_iter()
		.map(|value| value.text)
		.collect()
	}

	// ── start ────────────────────────────────────────────────────────────

	/// `start f(…)` or `start do { … }`: the task.
	pub(crate) fn start_expr(&mut self, n: NodeId, operand: NodeId) -> Expr {
		let info = self.analysis.starts.get(&n).cloned();
		let detached = info.as_ref().is_some_and(|info| info.detached);
		// a detached task copies the values it uses that the starter may
		// still change (A12): the argument values, the captured locals
		let copied: Vec<NodeId> = info
			.as_ref()
			.filter(|info| info.detached)
			.map(|info| {
				info.copies
					.iter()
					.filter(|(_, moved)| !moved)
					.map(|(node, _)| *node)
					.collect()
			})
			.unwrap_or_default();
		let task = match info.as_ref().and_then(|info| info.block) {
			Some(block) => {
				// the module-level values (the top level's too) are read, not
				// copied: they are not captured (`concurrency.md`, tasks)
				let symbols: Vec<fa_analyzer::ids::SymbolId> = copied
					.iter()
					.filter_map(|&node| self.analysis.symbols.symbol_of_decl(node))
					.collect();
				let saved = self.copy_values(&symbols, true);
				let task = self.start_block(n, block);
				self.restore_captures(saved);
				task
			}
			None if !copied.is_empty() => {
				let saved = std::mem::replace(
					&mut self.tasks.detached_copies,
					copied.iter().copied().collect(),
				);
				self.tasks.detached_copies.extend(saved.iter().copied());
				let task = self.start_call(operand);
				self.tasks.detached_copies = saved;
				task
			}
			None => self.start_call(operand),
		};
		if detached && matches!(self.mode(), Mode::Async { .. } | Mode::Dynamic) {
			let helper = self.helper("faDetach");
			return js::call(helper, &[task.text]);
		}
		task
	}

	/// `start f(…)`: the task of a call.
	fn start_call(&mut self, operand: NodeId) -> Expr {
		let call = self.strip_groups(operand);
		let direct = matches!(self.view(call), View::Call { .. })
			&& self.analysis.call(call).is_some_and(|info| {
				!matches!(
					info.callee,
					Callee::Builtin(_) | Callee::Conversion(_) | Callee::Member(_)
				)
			});
		if direct {
			self.tasks.pending_start = Some(call);
			let task = self.expr(call);
			self.tasks.pending_start = None;
			return task;
		}
		if let Some(task) = self.start_primitive(call) {
			return task;
		}
		// a built-in: it runs at once, and gives a finished task
		let (lines, value) = self.capture_at(self.out.indentation() + 1, |g| g.expr(operand));
		let helper = self.helper("faFinished");
		let body = if lines.is_empty() {
			format!("() => {}", value.at(Prec::Assign))
		} else {
			let mut body = lines;
			body.line(format!("return {};", value.text));
			let closing = "\t".repeat(self.out.indentation());
			format!("() => {{\n{}{closing}}}", body.finish())
		};
		js::call(helper, &[body])
	}

	/// `start do { … }`.
	fn start_block(&mut self, n: NodeId, block: NodeId) -> Expr {
		match self.mode() {
			Mode::Async { .. } => {
				let body = self.task_body(n, block, Mode::Async { coop: true });
				let helper = self.helper("faStart");
				js::call(helper, &["$task".to_string(), body])
			}
			Mode::Blocking => {
				let body = self.task_body(n, block, Mode::Blocking);
				let helper = self.helper("faFinished");
				js::call(helper, &[body])
			}
			Mode::Dynamic => {
				let asynchronous = self.task_body(n, block, Mode::Async { coop: true });
				let blocking = self.task_body(n, block, Mode::Blocking);
				let helper = self.helper("faStartDynamic");
				js::call(helper, &[asynchronous, blocking])
			}
		}
	}

	/// The function running a `start do` block in a mode.
	fn task_body(&mut self, n: NodeId, block: NodeId, mode: Mode) -> String {
		let indent = self.out.indentation();
		let block = match self.view(block) {
			View::Do(body) => body,
			_ => block,
		};
		let dest = if self.type_of(block) == fa_analyzer::ids::TypeId::VOID {
			Dest::Discard
		} else {
			Dest::Return
		};
		let saved = self.fcx.function;
		let (lines, temps) = self.in_function(saved.unwrap_or(n), |g| {
			g.fcx.mode = mode;
			let saved = std::mem::replace(&mut g.out, Writer::at(indent + 1));
			match g.view(block) {
				View::Block { statements } => {
					let statements: Vec<NodeId> = statements.iter().collect();
					g.statements(&statements, dest.clone());
				}
				_ => g.into(block, dest.clone()),
			}
			std::mem::replace(&mut g.out, saved)
		});
		let mut body = Writer::at(indent + 1);
		if !temps.is_empty() {
			body.line(format!("let {};", temps.join(", ")));
		}
		body.append(lines);
		let closing = "\t".repeat(indent);
		let header = match mode {
			Mode::Async { .. } => "async ($task) =>",
			_ => "() =>",
		};
		format!("{header} {{\n{}{closing}}}", body.finish())
	}

	// ── built-ins ────────────────────────────────────────────────────────

	/// A call of a built-in of tasks (`Task.all`, `task.cancel()`…), or of
	/// a wait called blocking; `None` for another built-in.
	pub(crate) fn task_builtin(
		&mut self,
		n: NodeId,
		callee: NodeId,
		path: &str,
		arguments: &[Argument],
	) -> Option<Expr> {
		Some(match path {
			"Task.all" | "Task.any" | "Task.settled" => {
				let values = self.given_values(arguments);
				let parent = if matches!(self.mode(), Mode::Async { .. }) {
					"$task"
				} else {
					"null"
				};
				let mode = match path {
					"Task.all" => "0",
					"Task.any" => "1",
					_ => "2",
				};
				let helper = self.helper("faCombine");
				let tasks = values
					.into_iter()
					.next()
					.unwrap_or_else(|| "[]".to_string());
				js::call(helper, &[parent.to_string(), tasks, mode.to_string()])
			}
			"Task#cancel" => {
				let receiver = self.receiver(callee);
				let helper = self.helper("faCancel");
				js::call(helper, &[receiver.text])
			}
			"Task.yield" => Expr::constant("undefined", Prec::Primary),
			"std.time.sleep" => {
				let values = self.given_values(arguments);
				let helper = self.helper("faSleepSync");
				js::call(helper, &values)
			}
			"Mutex#lock" | "Channel#send" | "Channel#receive" => {
				let what = js::string_literal(&path.replace('#', "."));
				let location = self.location(n);
				let helper = self.helper("faWaitSync");
				js::call(helper, &[what, location])
			}
			_ => return None,
		})
	}

	// ── loops and scopes ─────────────────────────────────────────────────

	/// The start of an iteration of a loop: in an async instance, the coop
	/// budget (A2).
	pub(crate) fn coop_point(&mut self) {
		if let Mode::Async { coop: true } = self.mode() {
			let sched = self.helper("faSched");
			let helper = self.helper("faCoop");
			self.out
				.line(format!("if (--{sched}.n < 0) await {helper}($task);"));
		}
	}

	/// Whether a value of a type holds tasks: a `Task`, or a collection,
	/// an object, a tuple or a union holding one (a move-only type, the
	/// types with `destroy` and subscriptions aside).
	pub(crate) fn contains_task(&self, ty: TypeId) -> bool {
		let mut seen = HashSet::new();
		let mut pending = vec![ty];
		while let Some(ty) = pending.pop() {
			if !self.types.flags(ty).contains(TypeFlags::MOVE_ONLY) || !seen.insert(ty) {
				continue;
			}
			match self.types.kind(ty) {
				TypeKind::Task(..) => return true,
				TypeKind::Collection(_, item, key) => pending.extend([item, key]),
				TypeKind::Tuple(list) | TypeKind::Union(list) => {
					pending.extend(self.types.list_items(list).iter().copied());
				}
				TypeKind::Object(_) => {
					pending.extend(self.types.fields_of(ty).iter().map(|field| field.ty));
				}
				TypeKind::NamedUnion(nominal, _) => {
					let count = self
						.types
						.sym_list(self.types.nominal(nominal).members)
						.len();
					pending
						.extend((0..count as u16).map(|index| self.types.payload(nominal, index)));
				}
				_ => {}
			}
		}
		false
	}

	/// The helper dropping the tasks a declaration holds at the end of its
	/// block: `faDrop` for a task, `faDropTasks` for a value holding tasks.
	pub(crate) fn task_drop(&mut self, n: NodeId) -> String {
		let ty = self
			.analysis
			.symbols
			.symbol_of_decl(n)
			.map_or(TypeId::UNKNOWN, |symbol| {
				self.analysis.symbols.symbol(symbol).ty
			});
		if matches!(self.types.kind(ty), TypeKind::Task(..)) {
			self.helper("faDrop").to_string()
		} else {
			self.helper("faDropTasks").to_string()
		}
	}

	/// Whether a declaration holds tasks that its block drops (cancels) at
	/// its end: a `Task`, or a value holding tasks (an array, an object, an
	/// optional…), never moved elsewhere.
	pub(crate) fn holds_task(&self, n: NodeId) -> bool {
		let Some(symbol) = self.analysis.symbols.symbol_of_decl(n) else {
			return false;
		};
		let ty = self.analysis.symbols.symbol(symbol).ty;
		if !self.contains_task(ty) {
			return false;
		}
		// a task given to a call is moved (`kept.push(task)`), even in one
		// branch only: a task is move-only, so the call keeps it
		let arguments: HashSet<NodeId> = self
			.analysis
			.calls
			.values()
			.flat_map(|info| info.arguments.iter())
			.filter_map(|argument| match argument {
				Argument::Given(value) => Some(self.strip_groups(*value)),
				_ => None,
			})
			.collect();
		!(0..self.tree.nodes.len()).any(|index| {
			let node = NodeId::new(index);
			matches!(self.view(node), View::Name(_))
				&& self.analysis.resolution(node) == Resolution::Local(symbol)
				&& (self.is_move(node) || arguments.contains(&node))
				&& !self.is_awaited(node)
		})
	}

	/// Whether a node is the operand of an `await`.
	fn is_awaited(&self, node: NodeId) -> bool {
		self.analysis.awaits.keys().any(|&awaited| {
			matches!(self.view(awaited), View::Await(operand) if self.strip_groups(operand) == node)
		})
	}
}
