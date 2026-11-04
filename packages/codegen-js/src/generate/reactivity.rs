//! Reactivity (advanced brief §11.6, `advanced/reactivity.md`): the
//! lowering of `reactive`, `derived` and `watch` over the runtime of
//! `runtime/reactive.js`.
//!
//! Values stay plain JavaScript values; a reactive value has a side source
//! (`count$s`, `faSource()`), a `reactive` field a source kept by its object
//! (`faFieldSource(object, "count")`). Only the places the analyzer lists are
//! instrumented (`Analysis::reactive_reads`, `reactive_writes`,
//! `reactive_args`, `flush_after`, `watches`): code that never touches a
//! reactive place is generated as if reactivity did not exist.
//!
//! - A read is `(faTrack(source), value)`; a `derived` value is a node read
//!   with `faRead(node)`.
//! - A write is followed by `faNotify(source)`; a flat value equal to the
//!   previous one notifies nothing. A call that writes a reactive place is
//!   `faChanged(call, sources…)`.
//! - A statement of the outermost running code that may write a reactive
//!   place (or create a `watch` block) is followed by `faFlush()`.
//! - `watch { … }` is `faWatch(() => { … })` (its last lambda returned: the
//!   cleanup); outside of the top level of the module, the rest of its
//!   block is in a `try` whose `finally` disposes it (`faDispose`), and
//!   `subscription.stop()` is `faDispose(subscription)`.
//! - A function that receives a reactive place (D24) gets a second
//!   instance, `name$r`, with a hidden source parameter per parameter after
//!   the others (`null` for a plain argument): one instance for every
//!   combination, the sources being optional (`p$s?.index(i)`).

use std::collections::HashSet;

use fa_analyzer::analysis::{Argument, Callee, ReactivePlace, ReactiveRoot, ReactiveStep};
use fa_analyzer::ids::{NodeId, SymbolId};
use fa_analyzer::symbols::{Resolution, SymbolKind};
use fa_analyzer::syntax::{self, View};
use fa_analyzer::types::{CollKind, TypeKind};

use crate::generate::{Dest, Generator, Repr};
use crate::js::{self, Expr, Prec};

impl Generator<'_> {
	// ── sources ──────────────────────────────────────────────────────────

	/// The functions of this file that get a reactive instance (D24): the
	/// callees of the calls that give them a reactive place.
	pub(crate) fn find_reactive_instances(&mut self) {
		let mut instances = HashSet::new();
		for call in self.analysis.reactive_args.keys() {
			let Some(info) = self.analysis.call(*call) else {
				continue;
			};
			let Callee::Function(declaration) = info.callee else {
				continue;
			};
			if info
				.arguments
				.iter()
				.any(|argument| matches!(argument, Argument::Receiver(_)))
			{
				continue;
			}
			let plain = matches!(
				self.view(declaration),
				View::Function { .. } | View::ExportFunction { .. }
			);
			let variadic = self
				.analysis
				.function(declaration)
				.is_some_and(|function| function.params.iter().any(|param| param.variadic));
			if plain && !variadic {
				instances.insert(declaration);
			}
		}
		self.reactive_instances = instances;
	}

	/// The source of a `reactive` value: a module-level constant for a
	/// value of the module, a `const` declared here otherwise.
	pub(crate) fn value_source(&mut self, symbol: SymbolId) -> String {
		if let Some(name) = self.sources.get(&symbol) {
			return name.clone();
		}
		let info = self.analysis.symbols.symbol(symbol);
		let wanted = format!("{}$s", self.types.syms.resolve(info.name));
		let helper = self.helper("faSource");
		let name = if Some(info.scope) == self.module_scope {
			let name = self.scopes.declare_global(&wanted);
			self.module_constant(name.clone(), format!("{helper}()"));
			name
		} else {
			let name = self.scopes.declare(&wanted);
			self.out.line(format!("const {name} = {helper}();"));
			name
		};
		self.sources.insert(symbol, name.clone());
		name
	}

	/// At the declaration of a `reactive` value: its source (declared again
	/// in each instance of a function generated twice).
	pub(crate) fn declare_value_source(&mut self, symbol: SymbolId) {
		if Some(self.analysis.symbols.symbol(symbol).scope) != self.module_scope {
			self.sources.remove(&symbol);
		}
		self.value_source(symbol);
	}

	/// The JavaScript expression of the source of a reactive place, `None`
	/// when it isn't reactive here (a parameter of the plain instance) or
	/// can't be designated again without evaluating code twice (a reactive
	/// field of a value with effects). An index with effects is not
	/// followed: its container's source is used (coarser, still correct).
	pub(crate) fn place_source(&mut self, place: &ReactivePlace) -> Option<String> {
		let (mut text, optional) = match place.root {
			ReactiveRoot::Value(symbol) => (self.value_source(symbol), false),
			ReactiveRoot::Param(symbol) => (self.param_sources.get(&symbol)?.clone(), true),
			ReactiveRoot::Field { access, member } => {
				let (View::Access { target, members } | View::OptionalAccess { target, members }) =
					self.view(access)
				else {
					return None;
				};
				let members: Vec<NodeId> = members.iter().collect();
				let field = *members.get(member as usize)?;
				let object =
					self.untracked(|g| g.access_chain(access, target, &members[..member as usize]));
				if !object.pure {
					return None;
				}
				let name = match self.analysis.resolution(field) {
					Resolution::Field(shape, index) => {
						let field = self.types.shape_fields(shape)[index as usize];
						self.types.syms.resolve(field.name).to_string()
					}
					_ => self.text(field).to_string(),
				};
				let helper = self.helper("faFieldSource");
				(
					format!("{helper}({}, {})", object.text, js::string_literal(&name)),
					false,
				)
			}
		};
		let dot = if optional { "?." } else { "." };
		for step in &place.steps {
			match *step {
				ReactiveStep::Field(name) => {
					let name = js::string_literal(self.types.syms.resolve(name));
					text = format!("{text}{dot}field({name})");
				}
				ReactiveStep::Index(index) => {
					let key = self.untracked(|g| g.expr(index));
					if !key.pure {
						break;
					}
					text = format!("{text}{dot}index({})", key.text);
				}
				ReactiveStep::Length | ReactiveStep::Size => {
					text = format!("{text}{dot}length()");
				}
			}
		}
		Some(text)
	}

	/// Runs `f` without instrumenting the reads it generates (the second
	/// evaluation of a place, to designate its source).
	pub(crate) fn untracked<T>(&mut self, f: impl FnOnce(&mut Self) -> T) -> T {
		self.untracked += 1;
		let result = f(self);
		self.untracked -= 1;
		result
	}

	// ── reads and writes ─────────────────────────────────────────────────

	/// The instrumentation of the value of node `n`: a read of a reactive
	/// place is tracked, a call that writes reactive places notifies them.
	pub(crate) fn reactive_expr(&mut self, n: NodeId, value: Expr) -> Expr {
		if self.untracked > 0 || !self.has_reactivity {
			return value;
		}
		let mut value = value;
		if let View::Call { callee, .. } = self.view(n) {
			// a built-in method reading its receiver as a whole: tracked on the callee
			value = self.track(callee, value);
			if let Some(places) = self.analysis.reactive_writes.get(&n).cloned() {
				let sources: Vec<String> = places
					.iter()
					.filter_map(|place| self.place_source(place))
					.collect();
				if !sources.is_empty() {
					let helper = self.helper("faChanged");
					let mut arguments = vec![value.text];
					arguments.extend(sources);
					value = js::call(helper, &arguments);
				}
			}
			return value;
		}
		self.track(n, value)
	}

	/// `(faTrack(source), value)` when node `n` reads a reactive place.
	fn track(&mut self, n: NodeId, value: Expr) -> Expr {
		let Some(place) = self.analysis.reactive_reads.get(&n).cloned() else {
			return value;
		};
		let Some(source) = self.place_source(&place) else {
			return value;
		};
		let helper = self.helper("faTrack");
		Expr::primary(format!("({helper}({source}), {})", value.at(Prec::Assign)))
			.with_purity(false, false)
	}

	/// A read of a `derived` value: `faRead(node)`.
	pub(crate) fn derived_read(&mut self, symbol: SymbolId, name: String) -> Expr {
		if self.analysis.symbols.symbol(symbol).kind != SymbolKind::Derived {
			return Expr::name(name, false);
		}
		let helper = self.helper("faRead");
		js::call(helper, &[name])
	}

	/// An assignment (`n`, to `target`) that writes reactive places: the
	/// previous value of a flat place, kept to notify only on a change,
	/// then the assignment (`write`), then the notifications.
	pub(crate) fn reactive_assignment(
		&mut self,
		n: NodeId,
		target: NodeId,
		write: impl FnOnce(&mut Self),
	) {
		let places = match self.analysis.reactive_writes.get(&n) {
			Some(places) if self.untracked == 0 => places.clone(),
			_ => return write(self),
		};
		// a map entry written: a new key changes the size
		let mut size = None;
		if let View::Index {
			target: container, ..
		} = self.view(self.strip_groups(target))
			&& matches!(
				self.types.kind(self.type_of(container)),
				TypeKind::Collection(CollKind::Map | CollKind::GrowingMap, ..)
			) && let Some(place) = places
			.iter()
			.find(|place| matches!(place.steps.last(), Some(ReactiveStep::Index(_))))
		{
			let mut place = place.clone();
			place.steps.pop();
			place.steps.push(ReactiveStep::Size);
			let map = self.untracked(|g| g.expr_raw(container));
			if let Some(source) = self.place_source(&place)
				&& map.pure
			{
				let old = self.temp();
				let map = map.primary_text();
				self.out.line(format!("const {old} = {map}.size;"));
				size = Some((old, map, source));
			}
		}
		let sources: Vec<String> = places
			.iter()
			.filter_map(|place| self.place_source(place))
			.collect();
		if sources.is_empty() {
			return write(self);
		}
		let helper = self.helper("faNotify");
		let write = |g: &mut Self| {
			write(g);
			if let Some((old, map, source)) = size {
				g.out
					.line(format!("if ({old} !== {map}.size) {helper}({source});"));
			}
		};
		// a flat value: only a different one notifies
		let flat = !matches!(self.repr(self.type_of(target)), Repr::Other);
		let previous = if flat {
			let current = self.untracked(|g| g.expr_raw(target));
			current.pure.then_some(current)
		} else {
			None
		};
		let Some(previous) = previous else {
			write(self);
			for source in sources {
				self.out.line(format!("{helper}({source});"));
			}
			return;
		};
		let old = self.temp();
		self.out
			.line(format!("const {old} = {};", previous.at(Prec::Assign)));
		write(self);
		let current = self.untracked(|g| g.expr_raw(target));
		self.out
			.open(format!("if ({old} !== {})", current.at(Prec::Relational)));
		for source in sources {
			self.out.line(format!("{helper}({source});"));
		}
		self.out.close("");
	}

	/// `faFlush()` after a statement of the outermost running code that may
	/// write a reactive place.
	pub(crate) fn flushes_after(&self, statement: NodeId) -> bool {
		self.analysis.flush_after.contains(&statement)
	}

	/// Writes `faFlush();`.
	pub(crate) fn flush(&mut self) {
		let helper = self.helper("faFlush");
		self.out.line(format!("{helper}();"));
	}

	// ── reactive instances (D24) ─────────────────────────────────────────

	/// The hidden source arguments of a call of a function's reactive
	/// instance (one per parameter, `null` for a plain argument), or `None`
	/// for a call of its plain instance.
	pub(crate) fn reactive_call_sources(&mut self, call: NodeId) -> Option<Vec<String>> {
		let places = self.analysis.reactive_args.get(&call)?.clone();
		let declaration = match self.analysis.call(call)?.callee {
			Callee::Function(declaration) => declaration,
			_ => return None,
		};
		if !self.reactive_instances.contains(&declaration) {
			return None;
		}
		let count = self.analysis.function(declaration)?.params.len();
		let mut sources = Vec::with_capacity(count);
		for index in 0..count {
			let source = match places.get(index).cloned().flatten() {
				Some(place) => self.place_source(&place),
				None => None,
			};
			sources.push(source);
		}
		if sources.iter().all(Option::is_none) {
			return None;
		}
		Some(
			sources
				.into_iter()
				.map(|source| source.unwrap_or_else(|| "null".to_string()))
				.collect(),
		)
	}

	/// Whether a call may call a reactive instance: its arguments fill
	/// every parameter, the sources following them.
	pub(crate) fn calls_reactive_instance(&self, call: NodeId) -> bool {
		self.analysis.reactive_args.contains_key(&call)
			&& self
				.analysis
				.call(call)
				.is_some_and(|info| match info.callee {
					Callee::Function(declaration) => self.reactive_instances.contains(&declaration),
					_ => false,
				})
	}

	/// The name of a function's reactive instance.
	pub(crate) fn reactive_instance_name(&mut self, symbol: SymbolId) -> String {
		format!("{}$r", self.name_of(symbol))
	}

	/// The hidden source parameters of the reactive instance being
	/// generated: declared after the other parameters.
	pub(crate) fn declare_param_sources(&mut self, params: &[SymbolId]) -> Vec<String> {
		let mut names = Vec::new();
		for &param in params {
			let wanted = format!("{}$s", self.name_of(param));
			let name = self.scopes.declare(&wanted);
			self.param_sources.insert(param, name.clone());
			names.push(name);
		}
		names
	}

	// ── derived values and watch blocks ──────────────────────────────────

	/// `derived name = value`: `const name = faDerived(() => value)`.
	pub(crate) fn derived_declaration(&mut self, n: NodeId, value: NodeId) {
		let Some(symbol) = self.analysis.symbols.symbol_of_decl(n) else {
			return;
		};
		let module_value = self.is_module_value(symbol);
		let name = self.declare(symbol);
		let compute = self.reactive_closure(n, value, false, None);
		let helper = self.helper("faDerived");
		if module_value {
			self.out
				.line(format!("{name} = {helper}({});", compute.text));
		} else {
			self.out
				.line(format!("const {name} = {helper}({});", compute.text));
		}
	}

	/// `watch { … }` as a value: `faWatch(() => { … })`.
	pub(crate) fn watch_expr(&mut self, n: NodeId) -> Expr {
		let Some(info) = self.analysis.watches.get(&n).copied() else {
			return self.unsupported(n, "this `watch` block");
		};
		let compute = self.reactive_closure(n, info.body, true, info.cleanup);
		let helper = self.helper("faWatch");
		js::call(helper, &[compute.text])
	}

	/// Whether statement `n` creates a `watch` block that must be disposed
	/// at the end of its block: a `watch` statement or a subscription,
	/// outside of the top level of the module.
	pub(crate) fn scoped_watch(&self, n: NodeId) -> bool {
		// a resource of foreign code: destroyed at the end of its block,
		// at the top level too (the end of the program)
		if self.owned_resource(n).is_some() {
			return true;
		}
		if self.top_statements.contains(&n) {
			return false;
		}
		match self.view(n) {
			View::Watch(_) => true,
			View::Declaration {
				value: Some(value), ..
			} => {
				matches!(self.view(self.strip_groups(value)), View::Watch(_))
					|| self.holds_guard(n)
					|| self.holds_task(n)
			}
			_ => false,
		}
	}

	/// Whether a declaration holds the guard of a mutex (`mutable guard =
	/// stats.tryLock()`): it is released at the end of its block.
	pub(crate) fn holds_guard(&self, n: NodeId) -> bool {
		let Some(symbol) = self.analysis.symbols.symbol_of_decl(n) else {
			return false;
		};
		let ty = self.analysis.symbols.symbol(symbol).ty;
		let guard = |ty| {
			matches!(
				self.types.kind(ty),
				fa_analyzer::types::TypeKind::Sync(fa_analyzer::types::SyncKind::Guard, _)
			)
		};
		guard(ty)
			|| self
				.types
				.union_members(&ty)
				.iter()
				.any(|&member| guard(member))
	}

	/// `mutex.tryLock()` or `await mutex.lock()` (the JavaScript `call`) whose guard is a
	/// temporary of its statement (`m.tryLock()?.value`, `m.tryLock() ==
	/// none`): kept in a temporary, released after the statement
	/// ([`Generator::guarded`]). A guard held by a value is released at the
	/// end of its block instead (`scoped_watch_statements`).
	pub(crate) fn guard_temporary(&mut self, n: NodeId, call: Expr) -> Expr {
		if self.guard_frames.is_empty() || !self.is_guard_temporary(n) {
			return call;
		}
		let temp = self.inline_temp();
		if let Some(frame) = self.guard_frames.last_mut() {
			frame.push(temp.clone());
		}
		Expr::primary(format!("({temp} = {})", call.text))
	}

	/// Whether the guard of a call is only read by its statement: the
	/// target of a member access, or compared.
	fn is_guard_temporary(&mut self, n: NodeId) -> bool {
		let parents = self.parents();
		let mut child = n;
		while let Some(parent) = parents[child.index()] {
			match self.view(parent) {
				// `(await mutex.lock()).value`: the awaited guard
				View::Group(_) | View::Await(_) => child = parent,
				View::Access { target, .. } | View::OptionalAccess { target, .. } => {
					return target == child;
				}
				View::Equality { .. } | View::Is { .. } => return true,
				_ => return false,
			}
		}
		false
	}

	/// Generates statement `n` (`generate`), then releases the guards its
	/// temporaries hold: after it, or in a `finally` when it leaves the
	/// block (`return`, `throw`, `break`, `continue`, a value returned).
	pub(crate) fn guarded(&mut self, n: NodeId, dest: &Dest, generate: impl FnOnce(&mut Self)) {
		let has_try_lock = match self.has_try_lock {
			Some(known) => known,
			None => {
				let known = (0..self.tree.nodes.len()).any(|index| {
					matches!(
						self.analysis.call(NodeId::new(index)).map(|call| call.callee),
						Some(fa_analyzer::analysis::Callee::Builtin(builtin))
							if matches!(
								fa_analyzer::builtins::get(builtin).path,
								"Mutex#tryLock" | "Mutex#lock"
							)
					)
				});
				self.has_try_lock = Some(known);
				known
			}
		};
		if !has_try_lock {
			generate(self);
			return;
		}
		self.guard_frames.push(Vec::new());
		let (lines, ()) = self.capture(generate);
		let temps = self.guard_frames.pop().unwrap_or_default();
		if temps.is_empty() {
			self.out.append(lines);
			return;
		}
		let helper = self.helper("faUnlock");
		let release: Vec<String> = temps
			.iter()
			.map(|temp| format!("{helper}({temp});"))
			.collect();
		let leaves = *dest == Dest::Return
			|| matches!(
				self.view(n),
				View::Return(_) | View::Throw(_) | View::Break(_) | View::Continue
			);
		if leaves {
			self.out.line("try {");
			self.out.append(lines);
			self.out
				.line(format!("}} finally {{ {} }}", release.join(" ")));
		} else {
			self.out.append(lines);
			self.out.line(release.join(" "));
		}
	}

	/// A `watch` statement or subscription disposed at the end of its block:
	/// the rest of the block runs in a `try` whose `finally` disposes it.
	pub(crate) fn scoped_watch_statements(&mut self, n: NodeId, rest: &[NodeId], dest: Dest) {
		let node = match self.view(n) {
			View::Watch(_) => {
				let value = self.watch_expr(n);
				let name = self.scopes.declare("$watch");
				self.out.line(format!("const {name} = {};", value.text));
				name
			}
			_ => {
				self.statement(n);
				let symbol = self.analysis.symbols.symbol_of_decl(n);
				match symbol {
					Some(symbol) => self.name_of(symbol),
					None => return self.statements(rest, dest),
				}
			}
		};
		if self.flushes_after(n) {
			self.flush();
		}
		// a value holding a resource: dropped unless it moved out (resources.rs)
		if let Some(symbol) = self.owned_resource(n) {
			let Some(drop) = self.own_resource(symbol) else {
				return self.statements(rest, dest);
			};
			if rest.is_empty() {
				self.out.line(drop);
				return;
			}
			self.out.open("try");
			self.statements(rest, dest);
			self.out.close(&format!("finally {{ {drop} }}"));
			return;
		}
		let helper = if self.holds_guard(n) {
			self.helper("faUnlock").to_string()
		} else if self.holds_task(n) {
			self.helper("faDrop").to_string()
		} else {
			self.helper("faDispose").to_string()
		};
		if rest.is_empty() {
			if dest == Dest::Discard {
				self.out.line(format!("{helper}({node});"));
			}
			return;
		}
		self.out.open("try");
		self.statements(rest, dest);
		self.out.close(&format!("finally {{ {helper}({node}); }}"));
	}

	/// The function computing a derived value or running a watch block: an
	/// arrow function of no parameter, after the copies of the local values
	/// it captures that may change later.
	fn reactive_closure(
		&mut self,
		n: NodeId,
		body: NodeId,
		block: bool,
		cleanup: Option<NodeId>,
	) -> Expr {
		let captures = self.closure_captures(n);
		let saved = self.copy_captured(&captures);
		let indent = self.out.indentation();
		let (lines, temps) = self.in_function(n, |g| {
			let (lines, ()) = g.capture_at(indent + 1, |g| {
				// the cleanup copies the reactive values it reads: it sees
				// the values of this run, not the next one's
				let reactive = cleanup
					.map(|cleanup| g.reactive_reads_of(cleanup))
					.unwrap_or_default();
				let copied = g.copy_captured(&reactive);
				let dest = if cleanup.is_some() || !block {
					Dest::Return
				} else {
					Dest::Discard
				};
				match g.view(body) {
					View::Block { statements } if block => {
						let statements: Vec<NodeId> = statements.iter().collect();
						g.statements(&statements, dest);
					}
					_ => g.into(body, dest),
				}
				g.restore_captures(copied);
			});
			lines
		});
		self.restore_captures(saved);
		let mut body_lines = js::Writer::at(indent + 1);
		if !temps.is_empty() {
			body_lines.line(format!("let {};", temps.join(", ")));
		}
		body_lines.append(lines);
		let closing = "\t".repeat(indent);
		Expr::new(
			format!("() => {{\n{}{closing}}}", body_lines.finish()),
			Prec::Assign,
		)
	}

	/// The local at the root of a place expression (`a` of `a.b[0].c`).
	fn base_symbol(&self, n: NodeId) -> Option<SymbolId> {
		let mut current = n;
		loop {
			match self.view(current) {
				View::Access { target, .. }
				| View::OptionalAccess { target, .. }
				| View::Index { target, .. } => current = target,
				View::Group(inner) => current = inner,
				View::Name(_) => {
					return match self.analysis.resolution(current) {
						Resolution::Local(symbol) => Some(symbol),
						_ => None,
					};
				}
				_ => return None,
			}
		}
	}

	/// The `reactive` values read inside node `n`.
	fn reactive_reads_of(&self, n: NodeId) -> Vec<SymbolId> {
		let mut found = Vec::new();
		let mut pending = vec![n];
		while let Some(node) = pending.pop() {
			if let Resolution::Local(symbol) = self.analysis.resolution(node)
				&& self.analysis.symbols.symbol(symbol).kind == SymbolKind::Reactive
				&& !found.contains(&symbol)
			{
				found.push(symbol);
			}
			pending.extend(syntax::children(self.tree, node));
		}
		found
	}

	/// The local values a derived value or a watch block (node `n`) reads
	/// that are declared outside of it.
	fn closure_captures(&self, n: NodeId) -> Vec<SymbolId> {
		let span = self.tree.spans[n.index()];
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
			if let Some(symbol) = self.base_symbol(access) {
				referred.push(symbol);
			}
		}
		let mut found = Vec::new();
		let mut pending = vec![n];
		while let Some(node) = pending.pop() {
			if let Resolution::Local(symbol) = self.analysis.resolution(node) {
				let info = self.analysis.symbols.symbol(symbol);
				let declared = self.tree.spans[info.decl_node.index()];
				let outside = declared.end <= span.start || declared.start >= span.end;
				let value = matches!(
					info.kind,
					SymbolKind::Let
						| SymbolKind::Mutable
						| SymbolKind::Use | SymbolKind::MutableUse
						| SymbolKind::Parameter { .. }
				);
				if outside && value && !found.contains(&symbol) && !referred.contains(&symbol) {
					found.push(symbol);
				}
			}
			pending.extend(syntax::children(self.tree, node));
		}
		found
	}
}
