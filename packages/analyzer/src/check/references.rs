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
			if !name.is_empty() {
				let ty = declared.unwrap_or_else(|| w.s.types.widen(actual));
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
			}
		}
		TypeId::VOID
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
		let member = members.last()?;
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
			"add" if path.contains("Pool") || path.contains("Columnar") => {
				self.body.flow.time += 1;
				crate::flow::Identity::Fresh(self.body.flow.time)
			}
			_ => return None,
		};
		let collection = self.place_of(target)?;
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
		}
		let kind = self.a.symbols.symbol(symbol).kind;
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
		if kind == SymbolKind::MutableUse
			&& self.is_captured(symbol)
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
		if !self.body.flow.reachable {
			return;
		}
		let Some(state) = self.body.flow.ref_state(symbol) else {
			return;
		};
		let Some((by, maybe)) = self.facts.refs.get(state).invalidated else {
			return;
		};
		let entry = self.a.symbols.symbol(symbol);
		let name = self.s.types.syms.resolve(entry.name).to_string();
		let operation = self.operation_text(by);
		let line = self.line_of(by);
		let fix = match kind {
			SymbolKind::MutableUse => format!("move it with `reuse {name} = …`"),
			SymbolKind::Let | SymbolKind::Mutable => {
				"an iterator reads its collection: ask for a new one with `values()`".to_string()
			}
			_ => format!(
				"declare it again with `use {name} = {}`",
				self.fix_text(entry.decl_node)
			),
		};
		let (code, message) = if maybe {
			(
				&REF002,
				format!("`{name}` may have been invalidated by `{operation}` (line {line}): {fix}"),
			)
		} else {
			(
				&REF001,
				format!("`{name}` was invalidated by `{operation}` (line {line}): {fix}"),
			)
		};
		let diagnostic = self.diagnostic(code, n, message);
		self.report_flow(diagnostic);
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
	fn line_of(&self, n: NodeId) -> usize {
		let start = self.span(n).start as usize;
		self.tree.source[..start.min(self.tree.source.len())]
			.matches('\n')
			.count() + 1
	}

	/// The value of a `use` declaration with its index replaced by `...`
	/// when not a literal: `persons[...]`.
	fn fix_text(&self, declaration: NodeId) -> String {
		let View::Use { value, .. } = self.view(declaration) else {
			return "…".to_string();
		};
		let value = self.strip_groups(value);
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
				// the facts read through the alias may be stale
				let own = self.places.symbol(symbol);
				self.body.flow.drop_below(&self.places, own);
			}
			if !reached || current.invalidated.is_some_and(|(_, maybe)| !maybe) {
				continue;
			}
			let next = self.facts.refs.state(RefState {
				invalidated: Some((n, false)),
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
				Step::Length | Step::Size => return false,
				Step::Payload(member) => ty = *member,
				_ => return !self.is_flat(ty),
			}
		}
		false
	}

	/// The declared type of a place: its root's type, then the fields and
	/// elements along its steps.
	fn place_type(&self, place: PlaceId) -> Option<TypeId> {
		let symbol = self.place_symbol(place)?;
		let mut ty = self.a.symbols.symbol(symbol).ty;
		for step in self.place_steps(place) {
			ty = match step {
				Step::Field(name) => self
					.s
					.types
					.fields_of(ty)
					.iter()
					.find(|field| field.name == name)
					.map(|field| field.ty)?,
				Step::Length | Step::Size => TypeId::INTEGER,
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
