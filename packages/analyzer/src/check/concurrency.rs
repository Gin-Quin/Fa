//! Concurrency without async (advanced brief §14, D23, D31).
//!
//! - **`std.concurrency`**: its names come with `use { Mutex } =
//!   std.concurrency` (`std_library.rs`). `Mutex(T)` and `Channel(T)` in types,
//!   `AtomicInteger { value = 0 }`, `Mutex(T) { value = … }` and
//!   `Channel(T) { capacity = 16 }` build thread-safe handles (an
//!   [`ObjectInfo`] whose fields are the literal's). `T` holds no reference
//!   nor relation (`CONC006`); a handle is created `mutable` (`CONC007`,
//!   D23). `lock`, `send` and `receive` wait: called blocking, they need
//!   `native` (`CONC010`, reported with the async summaries: `tasks.rs`).
//! - **`for parallel`**: a [`ParallelFrame`] per loop being walked. Every
//!   effect recorded in its body ([`Walker::check_parallel_write`]) must be
//!   on the iteration's own places: below the loop variable of a `mutable`
//!   collection, `T[i]` (`i` the index binding, or the variable of a range)
//!   or below, or a local of the body; else `CONC002` (`CONC012` for a
//!   module-level value or a requirement changed by a call). Thread-safe
//!   values have no effects. At the end of the body, the reads of the
//!   collections written by index are checked (`CONC013`): only `T[i]` and
//!   `T.length`. `break` and `return` are `CONC003`; the index binding needs
//!   positions (`CONC005`); other targets than `native` are `CONC001`.
//! - `async`, `await`, `start` and tasks: `tasks.rs`.

use crate::analysis::{FieldValue, ObjectInfo};
use crate::builtins;
use crate::check::Walker;
use crate::diagnostics::codes::advanced::{
	CONC001, CONC002, CONC003, CONC005, CONC006, CONC007, CONC008, CONC010, CONC012, CONC013,
};
use crate::diagnostics::codes::values::{E_OBJ_1, E_OBJ_2, E_TYPE_1};
use crate::flow::{Identity, Step};
use crate::ids::{NodeId, PlaceId, ScopeId, SymbolId, TypeId};
use crate::symbols::{Resolution, ScopeKind, SymbolKind};
use crate::syntax::{self, NodeList, View};
use crate::types::{CollKind, FieldFlags, SyncKind, TypeKind};

/// A `for parallel` loop whose body is being walked.
#[derive(Clone, Debug)]
pub(crate) struct ParallelFrame {
	/// The `for` node.
	pub node: NodeId,
	/// The scope of the loop: what is declared in it belongs to the iteration.
	pub scope: ScopeId,
	/// The loop's pattern (its variable designates the iteration's element).
	pub pattern: Option<NodeId>,
	/// Whether the variable of a range loop is the iteration's position.
	pub range: bool,
	/// The function depth of the loop: the body's own code.
	pub function_depth: usize,
	/// The collections written by index (`results[index] = …`).
	pub indexed: Vec<SymbolId>,
	/// Whether the elements are written through the loop variable.
	pub through_variable: bool,
}

impl Walker<'_> {
	/// `Mutex(T)`, `Channel(T)` in a type.
	pub(super) fn sync_type_application(
		&mut self,
		n: NodeId,
		callee: NodeId,
		kind: SyncKind,
		arguments: NodeList<'_>,
	) -> TypeId {
		self.set_flag(callee, crate::analysis::NodeFlags::TYPE_EXPR);
		if let Some(builtin) = builtins::by_path(kind.name()) {
			self.set_resolution(callee, Resolution::Builtin(builtin));
		}
		let items: Vec<NodeId> = arguments.iter().collect();
		let item = match items.as_slice() {
			[item] => Some((*item, self.check_type_expr(*item))),
			_ => None,
		};
		self.memo(n, |w| {
			let Some((node, item)) = item else {
				let message = format!(
					"`{0}` takes the type of its value, ex: `{0}(Integer)`",
					kind.name()
				);
				let diagnostic = w.diagnostic(&crate::diagnostics::codes::types::TY003, n, message);
				w.report(diagnostic);
				return TypeId::UNKNOWN;
			};
			// `WorkerPool(T)`: `T` is the type of a worker
			if kind == SyncKind::WorkerPool
				&& !w.s.types.is_recovering(item)
				&& !w.is_worker_type_name(node)
			{
				let message = format!(
					"a `WorkerPool` runs workers: `{}` is not the type exported by a `*.worker.fa` file",
					w.tree.text(node.index())
				);
				let diagnostic = w.diagnostic(&E_TYPE_1, node, message);
				w.report(diagnostic);
			}
			if w.contains_reference_type(node) || w.type_has_references(item, 0) {
				let message = format!(
					"a `{}` holds owned data, shared between threads: `{}` contains a reference or a relation; store the values themselves",
					kind.name(),
					w.tree.text(node.index())
				);
				let diagnostic = w.diagnostic(&CONC006, node, message);
				w.report(diagnostic);
			}
			builtins::concurrency::sync(&mut w.s.types, kind, item)
		})
	}

	/// Whether a type contains a reference (`T in c`) or a relation field.
	fn type_has_references(&self, ty: TypeId, depth: u32) -> bool {
		if depth > 8 {
			return false;
		}
		let types = &self.s.types;
		match types.kind(ty) {
			TypeKind::Ref(..) => true,
			TypeKind::Collection(CollKind::RelationPool(_), ..) => true,
			TypeKind::Object(_) => types.fields_of(ty).iter().any(|field| {
				field.flags.contains(FieldFlags::RELATION)
					|| self.type_has_references(field.ty, depth + 1)
			}),
			TypeKind::Collection(_, item, key) => {
				self.type_has_references(item, depth + 1)
					|| (key != TypeId::NONE && self.type_has_references(key, depth + 1))
			}
			TypeKind::Tuple(list) | TypeKind::Union(list) => types
				.list_items(list)
				.iter()
				.any(|&item| self.type_has_references(item, depth + 1)),
			_ => false,
		}
	}

	/// `AtomicInteger { value = 0 }`, `Mutex(Stats) { value = … }`,
	/// `Channel(String) { capacity = 16 }`: a new thread-safe handle.
	pub(super) fn check_sync_literal(
		&mut self,
		n: NodeId,
		ty: TypeId,
		kind: SyncKind,
		item: TypeId,
		members: NodeList<'_>,
	) -> TypeId {
		let mut given: Vec<(String, NodeId, NodeId)> = Vec::new();
		for member in members.iter() {
			match self.view(member) {
				View::Assignment {
					target,
					value: Some(value),
					..
				} => {
					if let View::Name(name) = self.view(target) {
						given.push((self.tree.name(name).to_string(), target, value));
					}
				}
				View::Name(name) => given.push((self.tree.name(name).to_string(), member, member)),
				_ => {}
			}
		}
		let expected_fields = builtins::concurrency::literal_fields(kind, item);
		for (name, _, value) in &given {
			let expected = match (kind, name.as_str()) {
				(SyncKind::AtomicInteger, "value") => Some(TypeId::INTEGER),
				(SyncKind::AtomicBoolean, "value") => Some(TypeId::BOOLEAN),
				(SyncKind::Mutex, "value") => Some(item),
				(SyncKind::Channel, "capacity") => Some(TypeId::INTEGER),
				(SyncKind::TcpListener, "host") => Some(TypeId::STRING),
				(SyncKind::TcpListener, "port") => Some(TypeId::INTEGER),
				(SyncKind::WorkerPool, "count" | "maxQueueSize" | "timeout") => {
					Some(TypeId::INTEGER)
				}
				// the initial state of each worker (`design/decisions.md`)
				(SyncKind::WorkerPool, "state") => Some(item),
				_ => None,
			};
			// a pool's state is the worker type's object, not a worker
			let saved = self.pool_state;
			if (kind, name.as_str()) == (SyncKind::WorkerPool, "state") {
				self.pool_state = Some(self.strip_groups(*value));
			}
			let actual = self.check(*value, expected);
			self.pool_state = saved;
			if let Some(expected) = expected {
				self.sink(*value, actual, expected);
			}
		}
		self.memo(n, |w| {
			// a pool's workers run on threads, like workers (`worker_created`)
			if kind == SyncKind::WorkerPool {
				w.a.reads_target = true;
				let target_name = w.s.target().to_string();
				if !builtins::concurrency::has_workers(&target_name) && w.reported_once.insert((n, CONC010.id)) {
					let message = format!(
						"a worker pool can't be used on target `{target_name}` yet: its workers are threads of `native`, or workers of Bun, Node and the web"
					);
					let diagnostic = w.diagnostic(&CONC010, n, message);
					w.report(diagnostic);
				}
			}
			let mut fields = Vec::new();
			for (name, target, _) in &given {
				if !expected_fields.iter().any(|(field, _)| field == name) {
					let message = format!(
						"`{}` has no field `{name}`: it is built with {}",
						w.display(ty),
						expected_fields
							.iter()
							.map(|(field, _)| format!("`{field}`"))
							.collect::<Vec<_>>()
							.join(", ")
					);
					let diagnostic = w.diagnostic(&E_OBJ_1, *target, message);
					w.report(diagnostic);
				}
			}
			for (field, optional) in expected_fields {
				let value = given
					.iter()
					.find(|(name, _, _)| name == field)
					.map(|(_, _, value)| *value);
				// a pool's `state` may be left out when `T {}` is a value
				let optional = *optional
					&& !(kind == SyncKind::WorkerPool
						&& *field == "state"
						&& !w.builds_without_fields(item));
				if value.is_none() && !optional {
					let message = format!(
						"`{}` needs its `{field}`, ex: `{} {{ {field} = … }}`",
						w.display(ty),
						w.display(ty)
					);
					let diagnostic = w.diagnostic(&E_OBJ_2, n, message);
					w.report(diagnostic);
				}
				let sym = w.s.types.syms.intern(field);
				fields.push((sym, value.map_or(FieldValue::None, FieldValue::Given)));
			}
			w.a.objects.insert(
				n,
				ObjectInfo {
					ty,
					decl: None,
					member: None,
					fields,
				},
			);
			ty
		})
	}

	/// Whether `T {}` is a value of the object type `ty`: every field has a
	/// default or accepts `none`.
	fn builds_without_fields(&mut self, ty: TypeId) -> bool {
		if self.s.types.is_recovering(ty) {
			return true;
		}
		let (_, fields) = self.object_declaration(ty);
		fields
			.iter()
			.all(|field| field.default.is_some() || self.s.types.satisfies(TypeId::NONE, field.ty))
	}

	/// A member of a worker pool (`concurrency.md`, "Worker pools"):
	/// `submit(task)`, where `task` is an `async` function receiving a free
	/// worker, gives `Task(R) throws WorkerTimeout`; `shutdown()`.
	pub(super) fn worker_pool_member(
		&mut self,
		member: NodeId,
		receiver: TypeId,
		state: TypeId,
		text: &str,
	) -> Option<(TypeId, super::objects::MemberKind)> {
		let Some(builtin) = builtins::by_path(&format!("WorkerPool#{text}")) else {
			self.no_member(member, receiver, text);
			return None;
		};
		self.set_resolution(member, Resolution::Builtin(builtin));
		let ty = match text {
			"submit" => {
				let result = self.builtin_param("WorkerPool.submit", "Result", TypeId::UNKNOWN);
				let worker =
					builtins::concurrency::sync(&mut self.s.types, SyncKind::Worker, state);
				let callback = self.callback_with(&[worker], result, 0);
				// the task is `async`: it awaits the worker's methods
				let callback = match self.s.types.kind(callback) {
					TypeKind::Function(sig) => {
						let sig = self.s.types.sig(sig);
						let throws = TypeId::NEVER;
						self.s.types.function(crate::types::FnSig {
							is_async: true,
							throws,
							..sig
						})
					}
					_ => callback,
				};
				let timeout = builtins::errors::error_type(&mut self.s.types, "WorkerTimeout")
					.unwrap_or(TypeId::UNKNOWN);
				let task = self.task_type(result, timeout);
				self.method(&[callback], task)
			}
			_ => self.method(&[], TypeId::VOID),
		};
		Some((ty, super::objects::MemberKind::Builtin))
	}

	/// `WorkerPool(T) { … }` without `state` (allowed when `T {}` is a
	/// value, `check_sync_literal`): the object `T {}` the backends build is
	/// recorded on the literal's type node (`target`).
	pub(super) fn check_pool_literal(
		&mut self,
		n: NodeId,
		target: NodeId,
		ty: TypeId,
		item: TypeId,
		members: NodeList<'_>,
	) -> TypeId {
		let built = self.check_sync_literal(n, ty, SyncKind::WorkerPool, item, members);
		let given_state = members.iter().any(|member| {
			let name = match self.view(member) {
				View::Assignment { target, .. } => match self.view(target) {
					View::Name(name) => Some(name),
					_ => None,
				},
				View::Name(name) => Some(name),
				_ => None,
			};
			name.is_some_and(|name| self.tree.name(name) == "state")
		});
		if given_state || self.s.types.is_recovering(item) {
			return built;
		}
		let (decl, declared) = self.object_declaration(item);
		let mut fields = Vec::new();
		for field in &declared {
			let value = match field.default {
				Some(default) => FieldValue::Default(default),
				None if self.s.types.satisfies(TypeId::NONE, field.ty) => FieldValue::None,
				// reported by `check_sync_literal`
				None => return built,
			};
			fields.push((field.name, value));
		}
		self.a.objects.insert(
			target,
			ObjectInfo {
				ty: item,
				decl,
				member: None,
				fields,
			},
		);
		built
	}

	/// The thread-safe handle a value creates (`AtomicInteger { … }`), if any.
	fn creates_handle(&self, value: NodeId) -> Option<TypeId> {
		let value = self.strip_groups(value);
		let View::TypedObject { .. } = self.view(value) else {
			return None;
		};
		let ty = self.type_of(value);
		matches!(self.s.types.kind(ty), TypeKind::Sync(kind, _) if kind.is_handle()).then_some(ty)
	}

	/// `CONC007` (D23): `let counter = AtomicInteger { value = 0 }`.
	pub(super) fn check_thread_safe_let(&mut self, n: NodeId, value: NodeId) {
		let Some(ty) = self.creates_handle(value) else {
			return;
		};
		let shown = self.display(ty);
		let message = format!(
			"a `{shown}` changes: declare it with `mutable`, ex: `mutable … = {}`",
			self.tree.text(value.index())
		);
		let diagnostic = self.diagnostic(&CONC007, n, message);
		self.report(diagnostic);
	}

	// ── for parallel ─────────────────────────────────────────────────────

	/// Starts a `for parallel` loop: its target, its options, its frame.
	pub(super) fn enter_parallel(
		&mut self,
		n: NodeId,
		pattern: Option<NodeId>,
		iterable: NodeId,
		options: NodeList<'_>,
	) {
		self.a.reads_target = true;
		for option in options.iter() {
			match self.view(option) {
				View::Assignment {
					target,
					value: Some(value),
					..
				} if matches!(self.view(target), View::Name(name) if self.tree.name(name) == "threads") => {
					let actual = self.check(value, Some(TypeId::INTEGER));
					self.sink(value, actual, TypeId::INTEGER);
				}
				_ => {
					self.memo(option, |w| {
						let diagnostic = w.diagnostic(
							&E_OBJ_1,
							option,
							"`parallel` takes one option: `parallel(threads = 4)`",
						);
						w.report(diagnostic);
						TypeId::VOID
					});
				}
			}
		}
		let target = self.s.target().to_string();
		if target != "native" && self.reported_once.insert((n, CONC001.id)) {
			let message = format!(
				"`for parallel` is only available on `native`, not on `{target}`: branch with `@if target is \"native\"`, or write a plain `for`"
			);
			let diagnostic = self.diagnostic(&CONC001, n, message);
			self.report(diagnostic);
		}
		let parent = self.body.scope;
		let scope = self.a.symbols.scope_for(n, Some(parent), ScopeKind::Block);
		let range = matches!(self.view(self.strip_groups(iterable)), View::Range { .. });
		self.parallel.push(ParallelFrame {
			node: n,
			scope,
			pattern,
			range,
			function_depth: self.body.functions.len(),
			indexed: Vec::new(),
			through_variable: false,
		});
	}

	/// Ends a `for parallel` loop: the reads of its body (`CONC013`).
	pub(super) fn leave_parallel(&mut self, body: NodeId) {
		let Some(frame) = self.parallel.pop() else {
			return;
		};
		let mut shared: Vec<SymbolId> = frame.indexed.clone();
		// the collection whose elements the loop variable writes
		if frame.through_variable
			&& let View::For { iterable, .. } = self.view(frame.node)
			&& let Some(symbol) = self.read_local(iterable)
		{
			shared.push(symbol);
		}
		if shared.is_empty() {
			return;
		}
		let index = self.parallel_index(&frame);
		// with each node, the innermost index expression around it that
		// starts where it starts (`values[0]` for `values`), for messages:
		// one walk of the body, no scan of the tree per diagnostic
		let mut stack: Vec<(NodeId, Option<NodeId>)> = vec![(body, None)];
		while let Some((node, around)) = stack.pop() {
			match self.view(node) {
				// a write (reported as `CONC002` unless it is the iteration's own)
				View::Assignment {
					target,
					value: Some(value),
					..
				} if matches!(self.view(self.strip_groups(target)), View::Index { target: collection, .. }
						if self.read_local(collection).is_some_and(|s| shared.contains(&s))) =>
				{
					if let View::Index { index: at, .. } = self.view(self.strip_groups(target)) {
						stack.push((at, None));
					}
					stack.push((value, None));
					continue;
				}
				// `T[i]`: the iteration's own element
				View::Index { target, index: at }
					if self.read_local(target).is_some_and(|s| shared.contains(&s))
						&& index.is_some() && self.read_local(at) == index =>
				{
					stack.push((at, None));
					continue;
				}
				// `T.length`
				View::Access { target, members }
					if self.read_local(target).is_some_and(|s| shared.contains(&s))
						&& members.first().is_some_and(
							|member| matches!(self.view(member), View::Name(name) if self.tree.name(name) == "length"),
						) =>
				{
					continue;
				}
				View::Name(_) => {
					if let Some(symbol) = self.read_local(node)
						&& shared.contains(&symbol)
						&& self.reported_once.insert((node, CONC013.id))
					{
						let name = self.tree.text(node.index()).to_string();
						let shown = self.tree.text(around.unwrap_or(node).index()).to_string();
						let message = format!(
							"`{shown}` reads `{name}`, whose elements other iterations write at the same time: read only `{name}[index]`, or read a copy made before the loop"
						);
						let diagnostic = self.diagnostic(&CONC013, node, message);
						self.report(diagnostic);
					}
				}
				_ => {}
			}
			let start = self.tree.spans[node.index()].start;
			let is_index = matches!(self.view(node), View::Index { .. });
			for child in syntax::children(&self.tree, node) {
				let around = match self.tree.spans[child.index()].start == start {
					true if is_index => Some(node),
					true => around,
					false => None,
				};
				stack.push((child, around));
			}
		}
	}

	/// The symbol holding the iteration's position: the index binding, or
	/// the variable of a range.
	fn parallel_index(&self, frame: &ParallelFrame) -> Option<SymbolId> {
		if frame.range {
			return frame
				.pattern
				.and_then(|pattern| self.a.symbols.symbol_of_decl(pattern));
		}
		self.a.symbols.symbol_of_decl(frame.node)
	}

	/// Whether a scope is (inside) another.
	fn scope_within(&self, scope: ScopeId, outer: ScopeId) -> bool {
		let mut current = Some(scope);
		while let Some(scope) = current {
			if scope == outer {
				return true;
			}
			current = self.a.symbols.scope(scope).parent;
		}
		false
	}

	/// An effect on `place` in the body of the `for parallel` loops being
	/// walked: only the iteration's own places (advanced brief §14.2).
	pub(crate) fn check_parallel_write(&mut self, place: PlaceId, n: NodeId, call: bool) {
		if self.parallel.is_empty() {
			return;
		}
		let depth = self.body.functions.len();
		let frames = self.parallel.clone();
		for (position, frame) in frames.iter().enumerate() {
			if frame.function_depth != depth {
				continue;
			}
			match self.parallel_write_allowed(frame, place) {
				Some(Allowed::Indexed(symbol)) => {
					if !self.parallel[position].indexed.contains(&symbol) {
						self.parallel[position].indexed.push(symbol);
					}
				}
				Some(Allowed::Variable) => self.parallel[position].through_variable = true,
				Some(Allowed::Own) => {}
				None => {
					self.report_parallel_write(place, n, call);
					return;
				}
			}
		}
	}

	fn report_parallel_write(&mut self, place: PlaceId, n: NodeId, call: bool) {
		if !self.reported_once.insert((n, CONC002.id)) {
			return;
		}
		let shown = self.place_text(place);
		let root = self.place_symbol(place);
		let module_level = root.is_some_and(|symbol| {
			let entry = self.a.symbols.symbol(symbol);
			matches!(entry.kind, SymbolKind::Requirement { .. })
				|| matches!(
					self.a.symbols.scope(entry.scope).kind,
					ScopeKind::Module | ScopeKind::Namespace
				)
		});
		let indexed = root.is_some_and(|symbol| {
			matches!(
				self.s.types.kind(self.a.symbols.symbol(symbol).ty),
				TypeKind::Collection(CollKind::Array | CollKind::Columnar, ..)
			)
		});
		let root_name = root.map_or_else(
			|| shown.clone(),
			|symbol| {
				let name = self.a.symbols.symbol(symbol).name;
				self.s.types.syms.resolve(name).to_string()
			},
		);
		let what = if call {
			let callee = match self.view(n) {
				View::Call { callee, .. } => self.tree.text(callee.index()).to_string(),
				_ => self.tree.text(n.index()).to_string(),
			};
			format!("`{callee}` changes `{shown}`")
		} else {
			format!("this writes `{shown}`")
		};
		// a value the call reaches by itself, not one it is given
		let given = root.is_some_and(|symbol| {
			let mut stack = vec![n];
			while let Some(node) = stack.pop() {
				if self.read_local(node) == Some(symbol) {
					return true;
				}
				stack.extend(syntax::children(&self.tree, node));
			}
			false
		});
		let (code, message) = if call && module_level && !given {
			(
				&CONC012,
				format!(
					"{what}, which every iteration shares: a parallel loop can only change a module-level value or a requirement that is thread-safe (a `Mutex`, an atomic, a `Channel`)"
				),
			)
		} else {
			(
				&CONC002,
				format!(
					"{what}, shared by every iteration of the parallel loop: {}",
					if indexed {
						format!("write to `{root_name}[index]` instead")
					} else {
						"use a thread-safe value instead (an atomic, a `Mutex`)".to_string()
					}
				),
			)
		};
		let diagnostic = self.diagnostic(code, n, message);
		self.report(diagnostic);
	}

	/// Why a write is allowed in a parallel body, `None` when it isn't.
	fn parallel_write_allowed(&self, frame: &ParallelFrame, place: PlaceId) -> Option<Allowed> {
		let symbol = self.place_symbol(place)?;
		let entry = self.a.symbols.symbol(symbol);
		// a local of the iteration
		if self.scope_within(entry.scope, frame.scope) && !self.aliases.contains_key(&symbol) {
			return Some(Allowed::Own);
		}
		// through the loop variable (an element of a `mutable` collection)
		if let Some(pattern) = frame.pattern
			&& let Some(variable) = self.a.symbols.symbol_of_decl(pattern)
			&& let Some(&item) = self.aliases.get(&variable)
			&& self.places.is_at_or_below(place, item)
		{
			return Some(Allowed::Variable);
		}
		// `T[i]` or below
		let index = self.parallel_index(frame)?;
		let steps = self.place_steps(place);
		let first_index = steps
			.iter()
			.position(|step| !matches!(step, Step::Field(_)))?;
		let own = matches!(steps[first_index], Step::Index(Identity::Symbol(s, 0)) if s == index);
		let ty = self.a.symbols.symbol(symbol).ty;
		let positional = first_index == 0
			&& matches!(
				self.s.types.kind(ty),
				TypeKind::Collection(CollKind::Array | CollKind::Columnar, ..) | TypeKind::Unknown
			);
		(own && (positional || first_index > 0)).then_some(Allowed::Indexed(symbol))
	}

	/// `break` or `return` in a parallel body (`CONC003`): `Some` when reported.
	pub(super) fn check_parallel_jump(&mut self, n: NodeId, keyword: &str) -> bool {
		let depth = self.body.functions.len();
		let Some(frame) = self
			.parallel
			.iter()
			.rev()
			.find(|frame| frame.function_depth == depth)
		else {
			return false;
		};
		// a `break` of a loop inside the body is fine
		if keyword == "break"
			&& self
				.body
				.loops
				.last()
				.is_some_and(|cx| cx.node != frame.node && cx.function_depth == depth)
		{
			let inner = self
				.body
				.loops
				.iter()
				.rposition(|cx| cx.node == frame.node)
				.is_some_and(|position| position + 1 < self.body.loops.len());
			if inner {
				return false;
			}
		}
		if self.reported_once.insert((n, CONC003.id)) {
			let message = format!(
				"`{keyword}` can't leave a parallel loop: its iterations run at the same time, in any order; `continue` ends the current iteration"
			);
			let diagnostic = self.diagnostic(&CONC003, n, message);
			self.report(diagnostic);
		}
		true
	}

	/// `CONC005`: the index binding of a parallel loop needs positions
	/// (`Array`, `Columnar`, `GrowingArray`).
	pub(super) fn check_parallel_index(&mut self, iterable: NodeId, ty: TypeId) {
		let positional = matches!(
			self.s.types.kind(ty),
			TypeKind::Collection(
				CollKind::Array | CollKind::Columnar | CollKind::GrowingArray,
				..
			) | TypeKind::Unknown
		);
		if !positional && self.reported_once.insert((iterable, CONC005.id)) {
			let message = format!(
				"a parallel loop over a `{}` has no index: only an `Array`, a `Columnar` and a `GrowingArray` have positions",
				self.display(ty)
			);
			let diagnostic = self.diagnostic(&CONC005, iterable, message);
			self.report(diagnostic);
		}
	}

	/// Whether the walk is in a parallel body (of this function).
	pub(super) fn in_parallel_body(&self) -> bool {
		let depth = self.body.functions.len();
		self.parallel
			.iter()
			.any(|frame| frame.function_depth == depth)
	}
}

impl Walker<'_> {
	// ── workers ──────────────────────────────────────────────────────────

	/// Whether a function with these parameters is a method (`self` first)
	/// of a `*.worker.fa` file.
	pub(super) fn is_worker_method(&self, parameters: NodeList<'_>) -> bool {
		self.s.module(self.file).ends_with(".worker")
			&& parameters.first().is_some_and(
				|first| matches!(self.view(first), View::Name(name) if self.tree.name(name) == "self"),
			)
	}

	/// Whether a type name (`MathWorker`) is the export of a `*.worker.fa` file.
	pub(super) fn is_worker_type_name(&self, target: NodeId) -> bool {
		let target = self.strip_groups(target);
		let Resolution::Local(symbol) = self.a.resolution(target) else {
			return false;
		};
		let symbol = self.modules.aliases.get(&symbol).copied().unwrap_or(symbol);
		match self.modules.proxies.get(&symbol) {
			Some(super::modules::Proxy::Export { module }) => {
				self.s.types.module_path(*module).ends_with(".worker")
			}
			_ => false,
		}
	}

	/// `MathWorker { numbers = … }`: a new worker, whose state is the object
	/// built (D31: natively, its calls run one at a time, blocking).
	pub(super) fn worker_created(&mut self, n: NodeId, target: NodeId, state: TypeId) -> TypeId {
		if self.s.types.is_recovering(state) {
			return state;
		}
		self.a.reads_target = true;
		let target_name = self.s.target().to_string();
		if !builtins::concurrency::has_workers(&target_name)
			&& self.reported_once.insert((n, CONC010.id))
		{
			let message = format!(
				"a worker can't be used on target `{target_name}` yet: its thread is a thread of `native`, or a worker of Bun, Node and the web"
			);
			let diagnostic = self.diagnostic(&CONC010, n, message);
			self.report(diagnostic);
		}
		if self.type_function(state, "stop").is_some()
			&& self.reported_once.insert((target, CONC008.id))
		{
			let diagnostic = self.diagnostic(
				&CONC008,
				target,
				"a worker type can't declare a method named `stop`: `stop()` stops the worker's thread; rename it",
			);
			self.report(diagnostic);
		}
		let worker = builtins::concurrency::sync(&mut self.s.types, SyncKind::Worker, state);
		self.set_type(n, worker);
		worker
	}

	/// `let other = worker`: the handle moves, `worker` can't be used
	/// afterwards (`MEM002`); a socket of `std.net` too.
	pub(super) fn move_worker_handle(&mut self, value: NodeId, at: NodeId) {
		if !self.body.flow.reachable {
			return;
		}
		let Some(symbol) = self.read_local(value) else {
			return;
		};
		let ty = self.a.symbols.symbol(symbol).ty;
		if matches!(
			self.s.types.kind(ty),
			TypeKind::Sync(
				SyncKind::Worker
					| SyncKind::TcpListener
					| SyncKind::TcpStream
					| SyncKind::WorkerPool,
				_
			)
		) && !self
			.body
			.flow
			.consumed
			.iter()
			.any(|&(other, _)| other == symbol)
		{
			self.body.flow.consumed.push((symbol, at));
		}
	}

	/// The work given to `pool.submit(work)` runs on a worker's thread (its
	/// calls of the worker do, natively the whole function): the rules of
	/// worker code apply to it (`CONC008`, concurrency.md "Workers"): it
	/// can't use a module-level `mutable` value that isn't thread-safe, nor
	/// a requirement. Checked on its body (a lambda, or a function of this
	/// file given by name); the functions it calls are not followed. A
	/// requirement a call of the work needs from outside it is `REQ007`
	/// already (the work is stored).
	pub(super) fn check_submitted_work(&mut self, work: NodeId) {
		let work = self.strip_groups(work);
		let body = match self.view(work) {
			View::Lambda { .. } => work,
			View::Name(_) => {
				let Resolution::Local(symbol) = self.a.resolution(work) else {
					return;
				};
				let decl = self.a.symbols.symbol(symbol).decl_node;
				match self.view(decl) {
					View::Function { value, .. } | View::ExportFunction { value, .. } => value,
					_ => return,
				}
			}
			_ => return,
		};
		let mut nodes = Vec::new();
		let mut stack = vec![body];
		while let Some(node) = stack.pop() {
			nodes.push(node);
			stack.extend(syntax::children(&self.tree, node));
		}
		nodes.sort();
		for node in nodes {
			if !matches!(self.view(node), View::Name(_)) {
				continue;
			}
			let Resolution::Local(symbol) = self.a.resolution(node) else {
				continue;
			};
			let entry = self.a.symbols.symbol(symbol);
			let module_level = matches!(
				self.a.symbols.scope(entry.scope).kind,
				ScopeKind::Module | ScopeKind::Namespace
			);
			let thread_safe = matches!(
				self.s.types.kind(entry.ty),
				TypeKind::Sync(kind, _) if kind.is_handle()
			);
			let message = if matches!(entry.kind, SymbolKind::Requirement { .. }) {
				"a requirement can't be used in the work given to `submit`: it runs on a worker's thread, and requirements don't cross threads; pass the value as an argument"
			} else if entry.kind == SymbolKind::Mutable && module_level && !thread_safe {
				"the work given to `submit` runs on a worker's thread: it can't use a module-level `mutable` value unless it is thread-safe (a `Mutex`, an atomic, a `Channel`); pass a copy as an argument"
			} else {
				continue;
			};
			if self.reported_once.insert((node, CONC008.id)) {
				let diagnostic = self.diagnostic(&CONC008, node, message);
				self.report(diagnostic);
			}
		}
	}

	/// A member of a worker: one of its methods, or `stop()`; its fields
	/// live on its thread (`CONC008`).
	pub(super) fn worker_member(
		&mut self,
		member: NodeId,
		state: TypeId,
		text: &str,
	) -> Option<(TypeId, super::objects::MemberKind)> {
		if text == "stop" {
			let builtin = builtins::by_path("Worker#stop")?;
			self.set_resolution(member, Resolution::Builtin(builtin));
			let ty = builtins::member_type(&mut self.s.types, builtin, state)?;
			return Some((ty, super::objects::MemberKind::Builtin));
		}
		if let Some(function) = self.type_function(state, text) {
			self.set_resolution(member, Resolution::Local(function));
			let function_ty = self.symbol_type(function, member);
			let bound = self.bound_method_type(function, function_ty);
			return Some((bound, super::objects::MemberKind::Function));
		}
		if self.reported_once.insert((member, CONC008.id)) {
			let message = format!(
				"`{text}` is not a method of this worker: a worker's fields live on its thread, only its methods can be called"
			);
			let diagnostic = self.diagnostic(&CONC008, member, message);
			self.report(diagnostic);
		}
		None
	}
}

/// Why a write is allowed in a parallel body.
enum Allowed {
	/// A local of the iteration.
	Own,
	/// An element written through the loop variable.
	Variable,
	/// `T[i]` of this collection.
	Indexed(SymbolId),
}
