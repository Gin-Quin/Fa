//! The reference checker (advanced brief §4–§5): `use`, `mutable use`,
//! `reuse`, and the validity of local references along the flow.
//!
//! - `use x = place` declares an alias of a place (`aliases`): reads and
//!   writes of `x` are reads and writes of the place, and its state in the
//!   flow (`FlowState::refs`) is valid.
//! - An effect (`effects.rs`) invalidates the references it may reach:
//!   `invalidates P` those that may designate `P` or below, `mutates P`
//!   those strictly below `P` through an element of a collection or a
//!   string (an owning edge, §5.5).
//! - A use of an invalidated reference is `REF001`, of one invalidated on
//!   some paths only (after a branch, a loop) `REF002`. `use x` again in
//!   the same scope is allowed only then (`REF003` otherwise).
//! - `reuse x = place` moves a `mutable use` (`REF005` for anything else)
//!   and makes it valid again.

use std::sync::Arc;

use crate::analysis::NodeFlags;
use crate::check::Walker;
use crate::diagnostics::codes::advanced::{
	REF001, REF002, REF003, REF004, REF005, REF006, REF007, REF011,
};
use crate::flow::Step;
use crate::flow::refs::RefState;
use crate::ids::{NodeId, PlaceId, SymbolId, TypeId};
use crate::symbols::{DeclState, Resolution, Symbol, SymbolKind};
use crate::syntax::{self, Name, View};
use crate::types::{CollKind, TypeKind};

impl Walker<'_> {
	// ── declaring references ──────────────────────────────────────────────

	/// `use name = place` and `mutable use name = place`.
	pub(super) fn check_use(
		&mut self,
		n: NodeId,
		name: Name,
		annotation: Option<NodeId>,
		value: NodeId,
		mutable: bool,
	) -> TypeId {
		// `use c = std.c` (foreign.rs)
		if !mutable && let Some(ty) = self.check_std_c_use(n, name, value) {
			return ty;
		}
		// `use Component = react.Component`: a name alias (D14, modules.rs)
		if !mutable && self.is_module_path(value) {
			return self.check_use_alias(n, name, value);
		}
		let declared = annotation.map(|annotation| self.check_type_expr(annotation));
		let actual = self.check(value, declared);
		if let Some(declared) = declared {
			self.sink(value, actual, declared);
		}
		let place = self.reference_place(value);
		let kind = if mutable {
			SymbolKind::MutableUse
		} else {
			SymbolKind::Use
		};
		self.memo(n, |w| {
			if place.is_none() && !w.s.types.is_recovering(actual) {
				w.report_not_a_place(value, "use");
			}
			// a collection that owner fields point into (REL016, relations.rs)
			w.check_owned_collection_alias(value);
			if !name.is_empty() {
				// the place's declared type: a narrowing is a fact of the
				// alias, gone when the place changes (`box.value = none`)
				let ty = declared.unwrap_or_else(|| {
					place
						.and_then(|place| w.place_type(place))
						.filter(|&ty| w.s.types.satisfies(actual, ty))
						.unwrap_or_else(|| w.s.types.widen(actual))
				});
				let symbol = w.declare_reference(n, name, kind);
				let entry = w.a.symbols.symbol_mut(symbol);
				entry.ty = ty;
				entry.state = DeclState::Sequential;
			}
			TypeId::VOID
		});
		self.set_flag(value, NodeFlags::PLACE);
		let Some(symbol) = self.a.symbols.symbol_of_decl(n) else {
			return TypeId::VOID;
		};
		// declaring the name again: only over an invalidated `use`
		self.check_declared_again(n, name, symbol);
		if let Some(place) = place {
			self.aliases.insert(symbol, place);
			if self.body.flow.reachable {
				let state = self.facts.refs.state(RefState::valid(place));
				self.body.flow.set_ref_state(symbol, state);
				// the narrowing of the place, as a fact of the alias
				let declared_ty = self.a.symbols.symbol(symbol).ty;
				let narrowed = self.s.types.widen(actual);
				if annotation.is_none()
					&& narrowed != declared_ty
					&& self.is_union_like(declared_ty)
					&& !self.s.types.is_recovering(narrowed)
					&& self.s.types.satisfies(narrowed, declared_ty)
				{
					let own = self.places.symbol(symbol);
					let mut state = std::mem::take(&mut self.body.flow);
					self.narrow_place_in(&mut state, own, declared_ty, narrowed, n);
					self.body.flow = state;
				}
			}
		}
		TypeId::VOID
	}

	/// The owner of a reference return type `T in owner`: its path must
	/// start at a parameter or a module-level value (`REF008`).
	pub(super) fn reference_return_owner(&mut self, returns: NodeId) -> Option<PlaceId> {
		let View::RefType { owner, .. } = self.view(returns) else {
			return None;
		};
		// the root name of the path
		let mut root = self.strip_groups(owner);
		while let View::Access { target, .. } = self.view(root) {
			root = self.strip_groups(target);
		}
		let View::Name(name) = self.view(root) else {
			return None;
		};
		let sym = self.s.types.syms.intern(self.tree.name(name));
		let symbol = self.a.symbols.lookup(self.body.scope, sym);
		let allowed = symbol.is_some_and(|symbol| {
			let entry = self.a.symbols.symbol(symbol);
			matches!(
				entry.kind,
				SymbolKind::Parameter { .. } | SymbolKind::SelfParameter
			) || self.declared_outside_functions(symbol)
		});
		if !allowed {
			if self.reported_once.insert((owner, "REF008")) {
				let text = self.tree.text(owner.index()).to_string();
				let diagnostic = self.diagnostic(
					&crate::diagnostics::codes::advanced::REF008,
					owner,
					format!(
						"`{text}` is not a parameter nor a module-level value: a returned reference must point into something that outlives the call"
					),
				);
				self.report(diagnostic);
			}
			return None;
		}
		self.quiet += 1;
		self.check(owner, None);
		self.quiet -= 1;
		self.place_of(owner)
	}

	/// `REF009`: a returned reference must designate a place below the
	/// declared owner.
	pub(super) fn check_returned_reference(&mut self, value: NodeId, owner: Option<PlaceId>) {
		let Some(owner) = owner else {
			return;
		};
		if !self.body.flow.reachable {
			return;
		}
		let Some(place) = self.reference_place(value) else {
			return;
		};
		if self.may_be_at_or_below(place, owner) || self.may_be_at_or_below(owner, place) {
			return;
		}
		// a reference given by a parameter or a module-level value of the
		// same owner: unknown, accepted
		if let Some(symbol) = self.place_symbol(place)
			&& self.aliases.contains_key(&symbol)
		{
			return;
		}
		let text = self.tree.text(value.index()).to_string();
		let owner_text = self.place_text(owner);
		let diagnostic = self.diagnostic(
			&crate::diagnostics::codes::advanced::REF009,
			value,
			format!(
				"`{text}` is not inside `{owner_text}`, the owner the return type declares: return an element of `{owner_text}`, or change the return type"
			),
		);
		self.report_flow(diagnostic);
	}

	/// The place a `use` designates: a place expression (through `try`).
	fn reference_place(&mut self, value: NodeId) -> Option<PlaceId> {
		let value = self.strip_groups(value);
		let tree = Arc::clone(&self.tree);
		match syntax::view(&tree, value) {
			View::Try(inner) => self.reference_place(inner),
			View::Name(_) | View::Access { .. } | View::Index { .. } => self.place_of(value),
			View::Call { callee, arguments } => self.returned_reference(callee, arguments),
			_ => None,
		}
	}

	/// The place a call of a built-in returning a reference designates
	/// (advanced brief §5.3): `items.get(i)` and `map.get(key)` the element
	/// of that index or key, `find` an unknown element, `add` of a pool a
	/// fresh one.
	fn returned_reference(
		&mut self,
		callee: NodeId,
		arguments: crate::syntax::NodeList<'_>,
	) -> Option<PlaceId> {
		let callee = self.strip_groups(callee);
		let View::Access { target, members } = self.view(callee) else {
			return None;
		};
		let members: Vec<NodeId> = members.iter().collect();
		let member = *members.last()?;
		let Resolution::Builtin(builtin) = self.a.resolution(member) else {
			return None;
		};
		let path = crate::builtins::get(builtin).path;
		let method = path.rsplit('#').next().unwrap_or(path);
		let identity = match method {
			"get" | "getOrInsert" => match arguments.first() {
				Some(key) => self.identity_of(key),
				None => crate::flow::Identity::Unknown,
			},
			// an element that exists now: distinct from the ones added later
			"find" => crate::flow::Identity::Existing(self.body.flow.time),
			"add" if path == "OrderedColumnar#add" => crate::flow::Identity::Unknown,
			"add" if path.starts_with("Pool#") || path.starts_with("Columnar#") => {
				self.body.flow.time += 1;
				crate::flow::Identity::Fresh(self.body.flow.time)
			}
			_ => return None,
		};
		// the collection: the target and the members before the method
		let collection = self.access_place(target, &members[..members.len() - 1])?;
		Some(self.places.child(collection, Step::Index(identity)))
	}

	/// `REF004`: `use`/`reuse` of something that is not a place.
	fn report_not_a_place(&mut self, value: NodeId, keyword: &str) {
		let text = self.tree.text(value.index()).to_string();
		let message = format!(
			"`{keyword}` needs a place: `{text}` has no place to designate. Use `let` to keep a value, or `mutable use` and `reuse` to choose between places"
		);
		let diagnostic = self.diagnostic(&REF004, value, message);
		self.report(diagnostic);
	}

	/// Declares a reference: a name declared again in the same scope is a
	/// new symbol (`REF003`/`REF007` are flow checks, see
	/// [`Walker::check_declared_again`]).
	fn declare_reference(&mut self, node: NodeId, name: Name, kind: SymbolKind) -> SymbolId {
		let tree = Arc::clone(&self.tree);
		let text = tree.name(name);
		let sym = self.s.types.syms.intern(text);
		let previous = self.a.symbols.lookup_in(self.body.scope, sym);
		match previous {
			Some(previous)
				if matches!(
					self.a.symbols.symbol(previous).kind,
					SymbolKind::Use | SymbolKind::MutableUse
				) =>
			{
				let symbol = Symbol::new(sym, kind, node, syntax::name_span(name));
				let (id, _) = self.a.symbols.declare(self.body.scope, symbol);
				self.previous_reference.insert(node, previous);
				id
			}
			_ => self.declare(node, name, kind),
		}
	}

	/// `use x` again in the scope of a `use x`: allowed only when the
	/// existing one is invalidated here (`REF003`), never over a `mutable
	/// use` (`REF007`). Flow part.
	fn check_declared_again(&mut self, n: NodeId, name: Name, _symbol: SymbolId) {
		let Some(&previous) = self.previous_reference.get(&n) else {
			return;
		};
		let text = self.tree.name(name).to_string();
		if self.a.symbols.symbol(previous).kind == SymbolKind::MutableUse {
			let message = format!("`{text}` is a `mutable use`: move it with `reuse {text} = …`");
			let diagnostic = self.diagnostic(&REF007, n, message);
			self.report_flow(diagnostic);
			return;
		}
		let invalidated = self
			.body
			.flow
			.ref_state(previous)
			.and_then(|state| self.facts.refs.get(state).invalidated);
		if invalidated.is_none() && self.body.flow.reachable {
			let message = format!(
				"`{text}` is still valid: declaring it again is an error, choose another name"
			);
			let diagnostic = self.diagnostic(&REF003, n, message);
			self.report_flow(diagnostic);
		}
	}

	/// `reuse name = place`: moves a `mutable use` and makes it valid.
	pub(super) fn check_reuse(&mut self, n: NodeId, name: Name, value: NodeId) -> TypeId {
		let target = {
			let tree = Arc::clone(&self.tree);
			let sym = self.s.types.syms.intern(tree.name(name));
			self.a.symbols.lookup(self.body.scope, sym)
		};
		let expected = target
			.map(|symbol| self.a.symbols.symbol(symbol).ty)
			.filter(|ty| !ty.is_unset());
		let is_none = matches!(self.view(self.strip_groups(value)), View::None);
		let actual = self.check(value, expected);
		let place = if is_none {
			None
		} else {
			self.reference_place(value)
		};
		self.memo(n, |w| {
			let text = w.tree.name(name).to_string();
			let Some(symbol) = target else {
				let diagnostic = w.diagnostic(
					&crate::diagnostics::codes::values::E_DECL_1,
					n,
					format!(
						"`{text}` is not declared: declare it with `mutable use {text} = …` first"
					),
				);
				w.report(diagnostic);
				return TypeId::VOID;
			};
			w.set_resolution(n, Resolution::Local(symbol));
			if w.a.symbols.symbol(symbol).kind != SymbolKind::MutableUse {
				let message = format!(
					"only a `mutable use` can be moved with `reuse`: `{text}` is declared with `{}`",
					keyword_of(w.a.symbols.symbol(symbol).kind)
				);
				let diagnostic = w.diagnostic(&REF005, n, message);
				w.report(diagnostic);
				return TypeId::VOID;
			}
			if place.is_none() && !is_none && !w.s.types.is_recovering(actual) {
				w.report_not_a_place(value, "reuse");
				return TypeId::VOID;
			}
			if let Some(expected) = expected
				&& !w.s.types.is_recovering(actual)
				&& !w.s.types.is_recovering(expected)
				&& w.s
					.types
					.assignable(actual, expected, crate::types::Mode::Exact)
					.is_err() && !(is_none && w.s.types.union_members(&expected).contains(&TypeId::NONE))
			{
				let message = format!(
					"`{text}` is a `{}`: it can't designate a `{}`",
					w.display(expected),
					w.display(actual)
				);
				let diagnostic = w.diagnostic(&REF006, value, message);
				w.report(diagnostic);
			}
			TypeId::VOID
		});
		let Some(symbol) =
			target.filter(|&symbol| self.a.symbols.symbol(symbol).kind == SymbolKind::MutableUse)
		else {
			return TypeId::VOID;
		};
		if let Some(place) = place {
			// the owner must be the same collection (`REF006`)
			if let Some(&current) = self.aliases.get(&symbol)
				&& let (Some(old), Some(new)) = (self.owner_place(current), self.owner_place(place))
				&& self.places.get(old).root != self.places.get(new).root
			{
				let message = format!(
					"`{}` designates an element of `{}`: it can't designate an element of `{}`",
					self.tree.name(name),
					self.place_text(old),
					self.place_text(new)
				);
				let diagnostic = self.diagnostic(&REF006, value, message);
				self.report_flow(diagnostic);
			}
			self.aliases.insert(symbol, place);
		}
		if self.body.flow.reachable {
			let place = place.or_else(|| self.aliases.get(&symbol).copied());
			if let Some(place) = place {
				let state = self.facts.refs.state(RefState::valid(place));
				self.body.flow.set_ref_state(symbol, state);
				// the facts known of the old place are not the new one's
				let own = self.places.symbol(symbol);
				self.body.flow.drop_below(&self.places, own);
			}
		}
		TypeId::VOID
	}

	/// The collection or string a place is an element of: the place above
	/// its last element step (`None` for a plain alias).
	fn owner_place(&self, place: PlaceId) -> Option<PlaceId> {
		let mut current = place;
		loop {
			let entry = self.places.get(current);
			let parent = entry.parent?;
			if matches!(
				entry.step,
				Some(Step::Index(_) | Step::Range(..) | Step::All | Step::Characters)
			) {
				return Some(parent);
			}
			current = parent;
		}
	}

	// ── using references ──────────────────────────────────────────────────

	/// A read or a write of a name: when it is a local reference, it must be
	/// valid here (`REF001`, `REF002`); a closure can't capture a `mutable
	/// use` (`REF011`). Flow part of every name.
	pub(crate) fn use_reference(&mut self, n: NodeId) {
		let Resolution::Local(symbol) = self.a.resolution(n) else {
			return;
		};
		// the last use of a local may be a move (moves.rs)
		if self.body.flow.reachable {
			self.note_name_use(n, symbol);
			// a value a pending task modifies (tasks.rs)
			self.task_read(n, symbol);
		}
		let kind = self.a.symbols.symbol(symbol).kind;
		// reactivity is single-threaded (advanced brief §11)
		if matches!(kind, SymbolKind::Reactive | SymbolKind::Derived)
			&& self.in_parallel_body()
			&& self.reported_once.insert((n, "REA007"))
		{
			let name = self
				.s
				.types
				.syms
				.resolve(self.a.symbols.symbol(symbol).name);
			let what = if kind == SymbolKind::Reactive {
				"reactive"
			} else {
				"derived"
			};
			let message = format!(
				"`{name}` is a {what} value: a `for parallel` body can't use it. Copy it with `let` before the loop"
			);
			let diagnostic =
				self.diagnostic(&crate::diagnostics::codes::advanced::REA007, n, message);
			self.report(diagnostic);
		}
		// a local of an enclosing function: captured (`FnInfo::captures`)
		if !matches!(
			kind,
			SymbolKind::Function
				| SymbolKind::Type
				| SymbolKind::Namespace
				| SymbolKind::TypeParameter
		) && self.is_captured(symbol)
			&& let Some(cx) = self.body.functions.last_mut()
			&& !cx.captures.contains(&symbol)
		{
			cx.captures.push(symbol);
		}
		if !matches!(kind, SymbolKind::Use | SymbolKind::MutableUse)
			&& !self.iterator_locals.contains_key(&symbol)
		{
			return;
		}
		// a top-level `mutable use` of a file read by a lambda is captured too
		let top_level_in_lambda = || {
			self.body
				.functions
				.last()
				.is_some_and(|cx| cx.symbol.is_none() && !cx.test)
				&& self.declared_outside_functions(symbol)
		};
		if kind == SymbolKind::MutableUse
			&& (self.is_captured(symbol) || top_level_in_lambda())
			&& self.reported_once.insert((n, "REF011"))
		{
			let name = self
				.s
				.types
				.syms
				.resolve(self.a.symbols.symbol(symbol).name);
			let message = format!(
				"`{name}` is a `mutable use`: a closure can't capture it. Copy what you need with `let` first"
			);
			let diagnostic = self.diagnostic(&REF011, n, message);
			self.report(diagnostic);
			return;
		}
		self.check_reference_valid(n, symbol, None);
	}

	/// The captured references a local function reads are read at its calls
	/// (and where it is given to a callee): each must still be valid there.
	pub(crate) fn check_captured_references(&mut self, n: NodeId, function: NodeId) {
		let Some(captures) = self
			.a
			.functions
			.get(&function)
			.map(|info| info.captures.clone())
		else {
			return;
		};
		for symbol in captures {
			// a captured task (worker's handle, resource) moved away since
			if self.body.flow.reachable
				&& let Some(&(_, at)) = self
					.body
					.flow
					.consumed
					.iter()
					.find(|&&(other, _)| other == symbol)
				&& self.reported_once.insert((n, "MEM002"))
			{
				let name = self
					.s
					.types
					.syms
					.resolve(self.a.symbols.symbol(symbol).name)
					.to_string();
				let message = format!(
					"this call runs `{}`, which uses `{name}`, but `{name}` was consumed on line {}: a task, a worker's handle or a resource can't be used after it is moved or awaited",
					self.function_name_of(function),
					self.line_of(at)
				);
				let diagnostic =
					self.diagnostic(&crate::diagnostics::codes::advanced::MEM002, n, message);
				self.report_flow(diagnostic);
			}
			let kind = self.a.symbols.symbol(symbol).kind;
			if matches!(kind, SymbolKind::Use | SymbolKind::MutableUse)
				|| self.iterator_locals.contains_key(&symbol)
			{
				self.check_reference_valid(n, symbol, Some(function));
			}
		}
	}

	/// `REF001`/`REF002` when the reference `symbol`, read at `n` (or by the
	/// local function `through` called at `n`), is invalidated here.
	fn check_reference_valid(&mut self, n: NodeId, symbol: SymbolId, through: Option<NodeId>) {
		if !self.body.flow.reachable {
			return;
		}
		let Some(state) = self.body.flow.ref_state(symbol) else {
			return;
		};
		let Some((by, maybe)) = self.facts.refs.get(state).invalidated else {
			return;
		};
		if self.facts.refs.get(state).reported {
			return;
		}
		let kind = self.a.symbols.symbol(symbol).kind;
		let entry = self.a.symbols.symbol(symbol);
		let name = self.s.types.syms.resolve(entry.name).to_string();
		let operation = self.operation_text(by);
		let line = self.line_of(by);
		let fix = match kind {
			SymbolKind::MutableUse => format!("move it with `reuse {name} = …`"),
			SymbolKind::Let | SymbolKind::Mutable => {
				"an iterator reads its collection: ask for a new one with `values()`".to_string()
			}
			// a call initializer would run again (`people.add(…)` adds another)
			_ if self.initialized_by_call(entry.decl_node) => format!(
				"find the element again (`use {name} = ….find(…)`), or copy what you need with `let` before `{operation}`"
			),
			_ => format!(
				"declare it again with `use {name} = {}`",
				self.fix_text(entry.decl_node)
			),
		};
		let reads = match through {
			Some(function) => format!(
				"this call runs `{}`, which reads `{name}`: ",
				self.function_name_of(function)
			),
			None => String::new(),
		};
		let (code, message) = if maybe {
			(
				&REF002,
				format!(
					"{reads}`{name}` may have been invalidated by `{operation}` (line {line}): {fix}"
				),
			)
		} else {
			(
				&REF001,
				format!("{reads}`{name}` was invalidated by `{operation}` (line {line}): {fix}"),
			)
		};
		let diagnostic = self.diagnostic(code, n, message);
		self.report_flow(diagnostic);
		// reported once: its later uses on this path are not reported again
		let current = self.facts.refs.get(state).clone();
		let quiet = self.facts.refs.state(RefState {
			reported: true,
			..current
		});
		self.body.flow.set_ref_state(symbol, quiet);
	}

	/// The name of a local function (or `the function` for a lambda).
	fn function_name_of(&self, function: NodeId) -> String {
		self.a
			.functions
			.get(&function)
			.and_then(|info| info.symbol)
			.map_or_else(
				|| "the function".to_string(),
				|symbol| {
					self.s
						.types
						.syms
						.resolve(self.a.symbols.symbol(symbol).name)
						.to_string()
				},
			)
	}

	/// Whether a symbol is a local of an enclosing function (a capture).
	pub(crate) fn is_captured(&self, symbol: SymbolId) -> bool {
		let Some(top) = self.body.functions.last() else {
			return false;
		};
		let mut current = Some(self.a.symbols.symbol(symbol).scope);
		while let Some(scope) = current {
			if scope == top.scope {
				return false;
			}
			let data = self.a.symbols.scope(scope);
			if data.kind == crate::symbols::ScopeKind::Function
				|| self.body.functions.iter().any(|cx| cx.scope == scope)
			{
				return true;
			}
			current = data.parent;
		}
		false
	}

	/// Whether a symbol is declared at the top level of the file (or in a
	/// block of it), outside every function.
	fn declared_outside_functions(&self, symbol: SymbolId) -> bool {
		let mut current = Some(self.a.symbols.symbol(symbol).scope);
		while let Some(scope) = current {
			let data = self.a.symbols.scope(scope);
			if data.kind == crate::symbols::ScopeKind::Function
				|| self.body.functions.iter().any(|cx| cx.scope == scope)
			{
				return false;
			}
			current = data.parent;
		}
		true
	}

	/// The operation that invalidated a reference, for messages:
	/// `persons.push`, `people[0] = …`.
	fn operation_text(&self, by: NodeId) -> String {
		match self.view(by) {
			View::Call { callee, .. } => self.tree.text(callee.index()).to_string(),
			View::Assignment { target, .. } | View::CompoundAssignment { target, .. } => {
				format!("{} = …", self.tree.text(target.index()))
			}
			_ => self.tree.text(by.index()).to_string(),
		}
	}

	/// The line of a node.
	pub(crate) fn line_of(&self, n: NodeId) -> usize {
		let start = self.span(n).start as usize;
		self.tree.source[..start.min(self.tree.source.len())]
			.matches('\n')
			.count() + 1
	}

	/// Whether a `use` declaration's value is a call (through `try`).
	fn initialized_by_call(&self, declaration: NodeId) -> bool {
		let View::Use { value, .. } = self.view(declaration) else {
			return false;
		};
		let mut value = self.strip_groups(value);
		if let View::Try(inner) = self.view(value) {
			value = self.strip_groups(inner);
		}
		matches!(self.view(value), View::Call { .. })
	}

	/// The value of a `use` declaration with its index replaced by `...`
	/// when not a literal: `persons[...]`.
	fn fix_text(&self, declaration: NodeId) -> String {
		let View::Use { value, .. } = self.view(declaration) else {
			return "…".to_string();
		};
		let value = self.strip_groups(value);
		let value = match self.view(value) {
			View::Try(inner) => self.strip_groups(inner),
			_ => value,
		};
		match self.view(value) {
			View::Index { target, index }
				if !matches!(self.view(self.strip_groups(index)), View::Integer(_)) =>
			{
				format!("{}[...]", self.tree.text(target.index()))
			}
			_ => self.tree.text(value.index()).to_string(),
		}
	}

	// ── invalidation ──────────────────────────────────────────────────────

	/// Marks the references an effect on `place` invalidates (advanced
	/// brief §5.5), and drops the facts known through them.
	pub(crate) fn invalidate_references(&mut self, place: PlaceId, invalidates: bool, n: NodeId) {
		if self.body.flow.refs.is_empty() {
			return;
		}
		let refs = self.body.flow.refs.clone();
		for (symbol, state) in refs {
			let current = self.facts.refs.get(state).clone();
			if current.invalidated == Some((n, false)) {
				continue;
			}
			let reached = current.places.iter().any(|&designated| {
				if invalidates {
					self.may_be_at_or_below(designated, place)
				} else {
					self.mutation_invalidates(designated, place)
				}
			});
			let touched = reached
				|| current
					.places
					.iter()
					.any(|&designated| self.may_overlap(designated, place));
			if touched {
				// the facts read through the alias may be stale; its narrowing
				// stays when the effect is below what it designates, or changes
				// its payload only (`found.count = 1`, `found.add(1)`)
				let depth = self.place_steps(place).len();
				let payload_only = current.places.iter().all(|&designated| {
					self.place_steps(designated).len() < depth
						|| designated == place && self.kept_narrowings.contains(&designated)
				});
				let own = self.places.symbol(symbol);
				let narrowed = payload_only
					.then(|| self.body.flow.fact(own))
					.flatten()
					.map(|fact| self.facts.fact_value(fact))
					.filter(|fact| !fact.narrowed.is_unset());
				self.body.flow.drop_below(&self.places, own);
				if let Some(narrowed) = narrowed {
					let fact = self.facts.fact(crate::flow::Fact {
						narrowed: narrowed.narrowed,
						narrowed_at: narrowed.narrowed_at,
						..crate::flow::Fact::TOP
					});
					self.body.flow.set_fact(own, fact);
				}
			}
			if !reached || current.invalidated.is_some_and(|(_, maybe)| !maybe) {
				continue;
			}
			let next = self.facts.refs.state(RefState {
				invalidated: Some((n, false)),
				reported: false,
				..current
			});
			self.body.flow.set_ref_state(symbol, next);
		}
	}

	/// `mutates p` invalidates `r` when `r` is strictly below `p` and the
	/// route from `p` to `r` crosses an element of a collection or a string
	/// (advanced brief §5.5, `mutation_invalidates`).
	fn mutation_invalidates(&mut self, r: PlaceId, p: PlaceId) -> bool {
		if !self.may_be_at_or_below(r, p) {
			return false;
		}
		let (r_steps, p_steps) = (self.place_steps(r), self.place_steps(p));
		if r_steps.len() <= p_steps.len() {
			return false;
		}
		// the type the effect sees: the callee's declared one (`effects.rs`)
		let Some(mut ty) = self.effect_type.or_else(|| self.place_type(p)) else {
			return true;
		};
		for step in &r_steps[p_steps.len()..] {
			// below a union (or an optional) the change may replace the member
			// that held the payload (reference-checking.md, owning edges)
			if matches!(
				self.s.types.kind(ty),
				TypeKind::Union(_) | TypeKind::NamedUnion(..)
			) && !matches!(step, Step::Length | Step::Size | Step::Capacity)
				&& !self.kept_narrowings.contains(&p)
			{
				return true;
			}
			match step {
				Step::Field(name) => {
					let field = self
						.s
						.types
						.fields_of(ty)
						.iter()
						.find(|field| field.name == *name)
						.map(|field| field.ty);
					match field {
						Some(field) => ty = field,
						None => return false,
					}
				}
				Step::Length | Step::Size | Step::Capacity => return false,
				Step::Payload(member) => ty = *member,
				_ => return !self.is_flat(ty),
			}
		}
		false
	}

	/// The declared type of a place: its root's type, then the fields and
	/// elements along its steps.
	pub(crate) fn place_type(&self, place: PlaceId) -> Option<TypeId> {
		let symbol = self.place_symbol(place)?;
		let mut ty = self.a.symbols.symbol(symbol).ty;
		for step in self.place_steps(place) {
			ty = match step {
				Step::Field(name)
					if self.s.types.syms.resolve(name) == "value"
						&& matches!(self.s.types.kind(ty), TypeKind::Pointer(_)) =>
				{
					self.s.types.pointer_target(ty)?
				}
				Step::Field(name) => self
					.s
					.types
					.fields_of(ty)
					.iter()
					.find(|field| field.name == name)
					.map(|field| field.ty)?,
				Step::Length | Step::Size | Step::Capacity => TypeId::INTEGER,
				Step::Payload(member) => member,
				Step::Index(_) | Step::All | Step::Range(..) | Step::Child | Step::Characters => {
					match self.s.types.kind(ty) {
						TypeKind::Collection(CollKind::Map, value, _) => value,
						TypeKind::Collection(_, item, _) => item,
						TypeKind::String => TypeId::STRING,
						_ => return None,
					}
				}
			};
		}
		Some(ty)
	}

	/// A flat value owns no memory (advanced brief §3.2): a primitive, a
	/// union of literals, an enumeration, an object of flat fields.
	fn is_flat(&self, ty: TypeId) -> bool {
		match self.s.types.kind(ty) {
			TypeKind::Int(_)
			| TypeKind::Float(_)
			| TypeKind::FloatLit(..)
			| TypeKind::Boolean
			| TypeKind::BoolLit(_)
			| TypeKind::Character
			| TypeKind::Percentage
			| TypeKind::Enum(..)
			| TypeKind::EnumMember(..)
			| TypeKind::None => true,
			TypeKind::Union(list) => self.s.types.list_items(list).iter().all(|&member| {
				matches!(
					self.s.types.kind(member),
					TypeKind::BoolLit(_) | TypeKind::StringLit(_) | TypeKind::None
				) || self
					.s
					.types
					.as_int(member)
					.is_some_and(|int| int.min == int.max)
			}),
			TypeKind::Object(_) => self
				.s
				.types
				.fields_of(ty)
				.iter()
				.all(|field| self.is_flat(field.ty)),
			_ => false,
		}
	}
}

/// The keyword of a declaration kind, for messages.
fn keyword_of(kind: SymbolKind) -> &'static str {
	match kind {
		SymbolKind::Let => "let",
		SymbolKind::Mutable => "mutable",
		SymbolKind::Use => "use",
		SymbolKind::Parameter { .. } | SymbolKind::SelfParameter => "a parameter",
		_ => "another declaration",
	}
}
