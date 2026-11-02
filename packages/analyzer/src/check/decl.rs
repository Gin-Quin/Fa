//! Blocks, scopes, hoisting and declarations: `let`, `mutable`, assignment;
//! `static`, `type`, `namespace`, `use`, exports (stubs). On-demand analysis
//! of hoisted declarations (design §3.2).

use std::sync::Arc;

use crate::analysis::NodeFlags;
use crate::builtins;
use crate::check::{BodyCx, PlaceInfo, Walker};
use crate::diagnostics::codes::advanced::{EFF006, EFF020, MOD004, MOD007};
use crate::diagnostics::codes::analyzer::AN001;
use crate::diagnostics::codes::types::FN015;
use crate::diagnostics::codes::values::{
	E_DECL_1, E_DECL_2, E_DECL_3, E_DECL_6, E_DECL_9, E_DECL_10, E_DECL_11, W_DECL_1, W_OBJ_1,
};
use crate::ids::{NodeId, SymbolId, TypeId};
use crate::symbols::{DeclState, ScopeKind, Symbol, SymbolKind};
use crate::syntax::{self, BindingKind, HoistKind, Name, NodeList, View};
use crate::types::TypeKind;

impl Walker<'_> {
	/// The file: its scope (created by `analyze`), hoisted declarations,
	/// statements. A hoisted declaration (`function`, `type`, `namespace`) is
	/// analyzed at its first use, or after the other statements: its body
	/// sees every module-level value, wherever it is declared.
	pub(super) fn check_module(&mut self, n: NodeId, statements: NodeList<'_>) -> TypeId {
		let scope = self.a.symbols.scope_for(n, None, ScopeKind::Module);
		let saved = std::mem::replace(&mut self.body.scope, scope);
		self.declare_hoisted(n);
		// the ones not hoisted in order, then the hoisted ones not analyzed yet (modules.rs)
		self.walk_declarations(statements);
		self.body.scope = saved;
		TypeId::VOID
	}

	/// A block in statement position (the body of a loop, a test…): a scope,
	/// its hoisted declarations, its statements in order. `Void`. Block
	/// values use [`Walker::check_block_value`].
	pub(super) fn check_block(
		&mut self,
		n: NodeId,
		_statements: NodeList<'_>,
		_expected: Option<TypeId>,
	) -> TypeId {
		self.walk_block(n, None);
		TypeId::VOID
	}

	/// A block whose value is used (a function body, a branch of an `if`
	/// value, a `do`): its last statement is checked as a value, with the
	/// expected type, when it can give one (see [`Walker::is_value_statement`]).
	/// Returns the value's type and node, or `(VOID, None)` when the block ends
	/// with a statement or is empty (the caller decides whether the end is
	/// reachable: `self.body.flow.reachable`). A node that is not a block is
	/// checked as a value.
	pub fn check_block_value(
		&mut self,
		n: NodeId,
		expected: Option<TypeId>,
	) -> (TypeId, Option<NodeId>) {
		if !matches!(self.view(n), View::Block { .. }) {
			return (self.check(n, expected), Some(n));
		}
		self.walk_block(n, Some(expected))
	}

	/// Walks a block in its scope; with `value`, its last statement is a value
	/// checked with that expected type.
	pub(super) fn walk_block(
		&mut self,
		n: NodeId,
		value: Option<Option<TypeId>>,
	) -> (TypeId, Option<NodeId>) {
		let tree = Arc::clone(&self.tree);
		let View::Block { statements } = syntax::view(&tree, n) else {
			return (TypeId::VOID, None);
		};
		let parent = self.body.scope;
		let scope = self.a.symbols.scope_for(n, Some(parent), ScopeKind::Block);
		self.body.scope = scope;
		self.declare_hoisted(n);
		let last = value
			.and(statements.last())
			.filter(|&last| self.is_value_statement(last));
		let mut result = (TypeId::VOID, None);
		let mut reported = !self.body.flow.reachable;
		for statement in statements.iter() {
			// `UNREACHABLE`, `W-DO-2` (control.rs)
			self.note_reachability(statement, &mut reported);
			if Some(statement) == last {
				let ty = self.check_value_stmt(statement, value.flatten());
				result = (ty, Some(statement));
			} else {
				self.check_stmt(statement);
			}
		}
		self.body.scope = parent;
		result
	}

	/// Whether a statement gives its block a value when it is last: an
	/// expression, an `if` with an `else`, a `when`, a `loop`, a `do`,
	/// `return`, `throw`…, but not a declaration, an assignment, a `while` or
	/// `for` loop, or an `if` without `else`.
	pub fn is_value_statement(&self, n: NodeId) -> bool {
		!matches!(
			self.view(n),
			View::Declaration { .. }
				| View::Use { .. }
				| View::Reuse { .. }
				| View::Destructuring { .. }
				| View::Assignment { .. }
				| View::CompoundAssignment { .. }
				| View::UpdateFields { .. }
				| View::TypeDecl { .. }
				| View::Namespace { .. }
				| View::Function { .. }
				| View::ExportValue { .. }
				| View::ExportFunction { .. }
				| View::ExportType { .. }
				| View::ExportNamespace(_)
				| View::While { .. }
				| View::WhileLet { .. }
				| View::For { .. }
				| View::Test { .. }
				| View::Assert(_)
				| View::Provide { .. }
				| View::Error
				| View::If {
					else_body: None,
					..
				}
		)
	}

	/// Declares the hoisted declarations of a block `Pending` (once per block).
	pub(super) fn declare_hoisted(&mut self, block: NodeId) {
		if !self.hoisted_blocks.insert(block) {
			return;
		}
		let tree = Arc::clone(&self.tree);
		// module-level values of a package file and of a namespace are hoisted too (modules.rs)
		let values = self.hoists_values(block);
		for declaration in syntax::block_declarations(&tree, block, values) {
			if self.a.symbols.symbol_of_decl(declaration.node).is_some() {
				continue;
			}
			let kind = match declaration.kind {
				HoistKind::Function => SymbolKind::Function,
				HoistKind::Type => SymbolKind::Type,
				HoistKind::Namespace => SymbolKind::Namespace,
				HoistKind::Value | HoistKind::Derived => {
					self.modules.hoisted_values.insert(declaration.node);
					match self.view(declaration.node) {
						View::Declaration {
							kind: BindingKind::Mutable,
							..
						} => SymbolKind::Mutable,
						View::Declaration {
							kind: BindingKind::Reactive,
							..
						} => SymbolKind::Reactive,
						View::Declaration {
							kind: BindingKind::Derived,
							..
						} => SymbolKind::Derived,
						_ => SymbolKind::Let,
					}
				}
			};
			let symbol = self.declare(declaration.node, declaration.name, kind);
			self.a.symbols.symbol_mut(symbol).state = DeclState::Pending;
		}
	}

	/// Declares a name in the current scope: `E-DECL-2` when the scope already
	/// has it, `W-DECL-1` when it shadows the prelude. Call it from the type
	/// part (inside `memo`).
	pub(super) fn declare(&mut self, node: NodeId, name: Name, kind: SymbolKind) -> SymbolId {
		let tree = Arc::clone(&self.tree);
		let text = tree.name(name);
		let sym = self.s.types.syms.intern(text);
		let symbol = Symbol::new(sym, kind, node, syntax::name_span(name));
		let (id, previous) = self.a.symbols.declare(self.body.scope, symbol);
		if let Some(previous) = previous
			&& self
				.modules
				.export
				.is_some_and(|(_, export)| export == previous)
		{
			// the file's own name designates its export (modules.rs)
			let diagnostic = self.diagnostic_at(
				&MOD004,
				syntax::name_span(name),
				format!("`{text}` is the name of this file, which designates its export: give this declaration another name"),
			);
			self.report(diagnostic);
		} else if let Some(previous) = previous {
			let previous = self.a.symbols.symbol(previous).name_span;
			let diagnostic = self
				.diagnostic_at(
					&E_DECL_2,
					syntax::name_span(name),
					format!("`{text}` is already declared in this scope: give it another name"),
				)
				.with_related(
					crate::diagnostics::Location {
						file: self.file,
						span: previous,
					},
					format!("`{text}` is declared here"),
				);
			self.report(diagnostic);
		} else if let Some(binding) = self.catch_binding_named(sym) {
			// the handler of a `catch` can't hide its error (E-ERR-9)
			let previous = self.a.symbols.symbol(binding).name_span;
			let diagnostic = self
				.diagnostic_at(
					&crate::diagnostics::codes::values::E_ERR_9,
					syntax::name_span(name),
					format!("`{text}` is the error of this `catch`: give this value another name"),
				)
				.with_related(
					crate::diagnostics::Location {
						file: self.file,
						span: previous,
					},
					format!("`{text}` is declared here"),
				);
			self.report(diagnostic);
		} else if builtins::lookup(text).is_some()
			// `arguments` is only implicit at the top of a program (D4)
			&& (text != "arguments" || self.s.role(self.file) == crate::modules::FileRole::Program)
		{
			let diagnostic = self.diagnostic_at(
				&W_DECL_1,
				syntax::name_span(name),
				format!("`{text}` hides the built-in `{text}`: give it another name"),
			);
			self.report(diagnostic);
		}
		id
	}

	/// The `catch` binding named `sym` when the current scope is the top
	/// scope of that `catch`'s handler.
	fn catch_binding_named(&self, sym: crate::ids::Sym) -> Option<SymbolId> {
		let parent = self.a.symbols.scope(self.body.scope).parent?;
		if !matches!(
			self.view(self.a.symbols.scope(parent).owner),
			View::Catch { .. }
		) {
			return None;
		}
		let binding = self.a.symbols.lookup_in(parent, sym)?;
		(self.a.symbols.symbol(binding).kind == SymbolKind::CatchBinding).then_some(binding)
	}

	/// The type of a symbol at a use (`use_node`; call it from the type part):
	/// analyzes a pending declaration on demand, with a fresh body context
	/// (decision A11). A cycle gives `UNKNOWN`: through a value initializer,
	/// `MOD007`; through a function without return type, `EFF020` (design §3.2).
	pub(super) fn symbol_type(&mut self, symbol: SymbolId, use_node: NodeId) -> TypeId {
		// another file's declaration, or a `use` alias of one (modules.rs)
		if let Some(ty) = self.proxy_type(symbol, use_node) {
			return ty;
		}
		let current = self.a.symbols.symbol(symbol);
		match current.state {
			DeclState::Pending => {
				let (node, scope) = (current.decl_node, current.scope);
				self.check_declared_before(symbol, use_node);
				let saved_body = std::mem::replace(&mut self.body, BodyCx::new(scope));
				let saved_statement = self.statement.take();
				let saved_quiet = std::mem::take(&mut self.quiet);
				let saved_implicit = std::mem::take(&mut self.pending_implicit);
				let saved_comptime = std::mem::take(&mut self.comptime_values);
				self.check_stmt(node);
				self.comptime_values = saved_comptime;
				self.pending_implicit = saved_implicit;
				self.quiet = saved_quiet;
				self.statement = saved_statement;
				self.body = saved_body;
			}
			// a type naming itself while being built: a recursive type (type_decls.rs)
			DeclState::Signature { .. } if current.kind == SymbolKind::Type => {
				return self.recursive_reference(symbol);
			}
			DeclState::Signature { .. } => {
				self.report_cycle(symbol, use_node);
				return TypeId::UNKNOWN;
			}
			DeclState::Body { .. } if current.ty.is_unset() => {
				let name = self.s.types.syms.resolve(current.name).to_string();
				let diagnostic = self.diagnostic(
					&EFF020,
					use_node,
					format!(
						"`{name}` is recursive: declare its return type, ex: `function {name} = (…): Integer => …`"
					),
				);
				self.report(diagnostic);
				return TypeId::UNKNOWN;
			}
			DeclState::Sequential | DeclState::Body { .. } | DeclState::Done => {}
		}
		let ty = self.a.symbols.symbol(symbol).ty;
		if ty.is_unset() { TypeId::UNKNOWN } else { ty }
	}

	/// Inside a block, a function is visible after its declaration (unlike at
	/// the top level or in a type): a use before it is `E-DECL-1`, and two
	/// functions of a block calling each other are `FN015`.
	fn check_declared_before(&mut self, symbol: SymbolId, use_node: NodeId) {
		let target = self.a.symbols.symbol(symbol);
		let scope = target.scope;
		if matches!(
			self.a.symbols.scope(scope).kind,
			ScopeKind::Module | ScopeKind::Namespace
		) {
			return;
		}
		let name = self.s.types.syms.resolve(target.name).to_string();
		let mutual = self
			.decl_stack
			.iter()
			.any(|&frame| self.a.symbols.symbol(frame).scope == scope);
		let diagnostic = if mutual {
			self.diagnostic(
				&FN015,
				use_node,
				format!(
					"`{name}` is declared later in this block: two functions of a block can't call each other, declare them at the top level"
				),
			)
		} else {
			self.diagnostic(
				&E_DECL_1,
				use_node,
				format!(
					"`{name}` is declared later in this block: move its declaration before this use"
				),
			)
		};
		self.report(diagnostic);
	}

	/// A use of a declaration whose signature or type is being resolved.
	fn report_cycle(&mut self, symbol: SymbolId, use_node: NodeId) {
		let symbol = self.a.symbols.symbol(symbol);
		let name = self.s.types.syms.resolve(symbol.name).to_string();
		let (code, message) = match symbol.kind {
			SymbolKind::Let | SymbolKind::Mutable | SymbolKind::Static => (
				&MOD007,
				format!("the initializer of `{name}` depends on itself"),
			),
			SymbolKind::Type => (
				&AN001,
				"recursive types are not supported yet by the analyzer".to_string(),
			),
			// a function's signature using the function (a default value): its type is not known yet
			_ => return,
		};
		let diagnostic = self.diagnostic(code, use_node, message);
		self.report(diagnostic);
	}

	/// `let` and `mutable`, with an optional annotation (values brief §5).
	pub(super) fn check_declaration(
		&mut self,
		n: NodeId,
		kind: BindingKind,
		name: Name,
		annotation: Option<NodeId>,
		value: Option<NodeId>,
	) -> TypeId {
		// a module-level value of a package file: analyzed once, maybe on demand (modules.rs)
		if let Some(ty) =
			self.hoisted_value(n, |w| w.check_declaration(n, kind, name, annotation, value))
		{
			return ty;
		}
		// a compile-time value (comptime.rs)
		if kind == BindingKind::Static {
			return self.check_static(n, name, annotation, value);
		}
		let symbol_kind = match kind {
			BindingKind::Let => SymbolKind::Let,
			BindingKind::Mutable => SymbolKind::Mutable,
			BindingKind::Static => SymbolKind::Static,
			BindingKind::Reactive => SymbolKind::Reactive,
			BindingKind::Derived => SymbolKind::Derived,
			BindingKind::Use => SymbolKind::Use,
			BindingKind::MutableUse => SymbolKind::MutableUse,
		};
		if !matches!(
			kind,
			BindingKind::Let | BindingKind::Mutable | BindingKind::Reactive | BindingKind::Derived
		) {
			let what = format!("`{}` declarations", kind.keyword());
			return self.unsupported_declaration(n, &[name], symbol_kind, &what);
		}
		let declared = annotation.map(|annotation| self.check_type_expr(annotation));
		// `let f = (x) => …` (E-DECL-6): the lambda is walked quietly
		let lambda = value.filter(|&value| matches!(self.view(value), View::Lambda { .. }));
		self.quiet += u32::from(lambda.is_some());
		let actual = value.map(|value| {
			let ty = if kind == BindingKind::Derived {
				// computed lazily, in a context of its own (reactivity.rs)
				self.check_derived_value(n, value, declared)
			} else {
				self.check(value, declared)
			};
			(value, ty)
		});
		self.quiet -= u32::from(lambda.is_some());
		match (actual, declared) {
			(Some((value, actual)), Some(declared)) => self.sink(value, actual, declared),
			(Some((value, actual)), None) => self.sink_unannotated(value, actual),
			_ => {}
		}
		// `let other = worker`: a worker's handle moves (concurrency.rs)
		if let Some(value) = value {
			self.move_worker_handle(value, n);
		}
		let ty = self.memo(n, |w| {
			let text = w.decl_text(n, name);
			// written twice: the same type, spelled the same (not `Round` for `Shape.Circle`)
			if let (Some(declared), Some(value), Some(annotation)) = (declared, value, annotation)
				&& let View::TypedObject { target, .. } = w.view(value)
				&& w.type_of(value) == declared
				&& w.tree.text(target.index()) == w.tree.text(annotation.index())
			{
				let diagnostic = w.diagnostic(
					&W_OBJ_1,
					annotation,
					"the type is written twice: drop the annotation, or the type before the braces",
				);
				w.report(diagnostic);
			}
			if let Some(lambda) = lambda {
				let diagnostic = w.diagnostic(
					&E_DECL_6,
					lambda,
					format!("a function is declared with `function`: write `function {text} = …`"),
				);
				w.report(diagnostic);
			}
			// `let copy = pool.add(…)`: a copy (pools.rs)
			if let Some(value) = value {
				w.warn_copied_add(value);
			}
			// `let counter = AtomicInteger { … }` (CONC007, concurrency.rs)
			if kind == BindingKind::Let
				&& let Some(value) = value
			{
				w.check_thread_safe_let(n, value);
			}
			// only relation fields store references (REF010, relations.rs)
			if let Some(annotation) = annotation {
				w.check_reference_annotation(annotation);
			}
			if value.is_none() {
				let diagnostic = w.diagnostic(
					&E_DECL_10,
					n,
					format!(
						"`{text}` has no value: give it one, ex: `{} {text} = …`",
						kind.keyword()
					),
				);
				w.report(diagnostic);
			}
			let ty = match (declared, actual) {
				(Some(TypeId::VOID), _) => {
					let diagnostic = w.diagnostic(
						&E_DECL_9,
						annotation.unwrap_or(n),
						"`Void` is not a value type: a declaration can't hold it",
					);
					w.report(diagnostic);
					TypeId::UNKNOWN
				}
				(Some(declared), _) => declared,
				(None, Some((value, actual))) => w.binding_type(value, actual, &text),
				(None, None) => TypeId::UNKNOWN,
			};
			if !name.is_empty() || w.a.symbols.symbol_of_decl(n).is_some() {
				// a module-level value of a package file is hoisted: already declared
				let (symbol, state) = match w.a.symbols.symbol_of_decl(n) {
					Some(symbol) => (symbol, DeclState::Done),
					None => (w.declare(n, name, symbol_kind), DeclState::Sequential),
				};
				// its methods are those of the type written here (structural.rs)
				w.record_written_type(symbol, annotation.or(value));
				let symbol = w.a.symbols.symbol_mut(symbol);
				symbol.kind = symbol_kind;
				symbol.ty = ty;
				symbol.state = state;
			}
			TypeId::VOID
		});
		// flow: the value's facts become the symbol's (numbers.rs)
		if let (Some(symbol), Some(value)) = (self.a.symbols.symbol_of_decl(n), value) {
			self.reset_consumed(symbol);
			self.bind_value(symbol, value);

			// a copy of its value, maybe a move (moves.rs)
			self.note_copy_site(value);
			self.note_declaration(symbol);
			// an iterator: a reference into its collection (iterators.rs)
			self.declare_iterator_local(symbol, value);
			// the literal keys of a literal map are present (collections.rs, D32)
			self.literal_map_keys(symbol, value);
		}
		ty
	}

	/// The type of an unannotated binding: the widened type of its value
	/// (values brief §2.1). `none` alone and `Void` are errors.
	fn binding_type(&mut self, value: NodeId, actual: TypeId, name: &str) -> TypeId {
		match actual {
			TypeId::NONE => {
				let diagnostic = self.diagnostic(
					&E_DECL_11,
					value,
					format!(
						"`none` alone doesn't give `{name}` a type: annotate it, ex: `{name}: Integer? = none`"
					),
				);
				self.report(diagnostic);
				TypeId::UNKNOWN
			}
			TypeId::VOID => {
				let diagnostic = self.diagnostic(
					&E_DECL_9,
					value,
					format!("this expression has no value (`Void`): `{name}` can't hold it"),
				);
				self.report(diagnostic);
				TypeId::UNKNOWN
			}
			_ if matches!(
				self.s.types.kind(actual),
				TypeKind::Collection(_, TypeId::NEVER, _)
			) =>
			{
				let diagnostic = self.diagnostic(
					&E_DECL_11,
					value,
					format!(
						"an empty list doesn't give `{name}` a type: annotate it, ex: `{name}: Array(Integer) = []`"
					),
				);
				self.report(diagnostic);
				TypeId::UNKNOWN
			}
			// a generic function as a value needs its parameters (GN015)
			_ if matches!(self.s.types.kind(actual), TypeKind::Function(_))
				&& !self.free_params(actual).is_empty() =>
			{
				let diagnostic = self.diagnostic(
					&crate::diagnostics::codes::types::GN015,
					value,
					format!("`{name}` would hold a generic function: annotate it with a function type, ex: `{name}: (value: Integer) => Integer`"),
				);
				self.report(diagnostic);
				TypeId::UNKNOWN
			}
			// a member of a named union or an enumeration: the whole type (D53)
			_ => self.join_member(actual),
		}
	}

	/// `target = value`: the target must be `mutable` (`E-DECL-3`).
	pub(super) fn check_assignment(
		&mut self,
		n: NodeId,
		target: NodeId,
		annotation: Option<NodeId>,
		value: Option<NodeId>,
	) -> TypeId {
		let (None, Some(value)) = (annotation, value) else {
			return self.unsupported(n, "a field declaration outside an object");
		};
		let place = self.check_place(target);
		// a name assigned as a whole is written, not read: it has its declared
		// type, and no narrowing to re-validate in the next rounds of a loop
		let name = self.strip_groups(target);
		if place.symbol.is_some() && matches!(self.view(name), View::Name(_)) {
			self.clear_flag(name, NodeFlags::NARROWED);
			self.set_type(name, place.ty);
		}
		let expected = (!self.s.types.is_recovering(place.ty)).then_some(place.ty);
		let actual = self.check(value, expected);
		self.sink(value, actual, place.ty);
		let ty = self.memo(n, |w| {
			let parameter = !place.below
				&& place.symbol.is_some_and(|symbol| {
					matches!(
						w.a.symbols.symbol(symbol).kind,
						SymbolKind::Parameter { .. } | SymbolKind::SelfParameter
					)
				});
			if parameter {
				// E-DECL-13: a parameter is never replaced, even with `mutates`
				let text = w.tree.text(target.index()).to_string();
				let diagnostic = w.diagnostic(
					&EFF006,
					target,
					format!(
						"`{text}` is a parameter: it can't be replaced, return the new value instead"
					),
				);
				w.report(diagnostic);
			} else if !w.is_relation_copy_write(target) {
				w.check_assignable(target, &place);
			}
			TypeId::VOID
		});
		// the write: an effect on the place (effects.rs)
		if place.assignable {
			self.write_effect(target, n);
		}
		// a relation side: written through its direct side (relations.rs)
		self.check_relation_write(n, target, value);
		// a closure stored into a place escapes; a copy, maybe a move (moves.rs)
		self.escape(value, n);
		self.note_copy_site(value);
		// flow: the value's facts replace the place's (numbers.rs)
		if let Some(symbol) = place.symbol.filter(|_| place.assignable && !place.below) {
			self.assign_value(symbol, value);
		}
		ty
	}

	/// `E-DECL-3` when a place can't be assigned as a whole. Type part.
	pub(super) fn check_assignable(&mut self, target: NodeId, place: &PlaceInfo) {
		if place.assignable {
			return;
		}
		// a set element or a map key (`CL031`), even below; an alias of an
		// immutable place (references.rs)
		if let Some(symbol) = place.symbol
			&& (self.rejected_effect_roots.contains(&symbol)
				|| self.report_readonly_element(target, symbol)
				|| self.report_read_only_alias(target, symbol, place.below))
		{
			return;
		}
		// a `derived` value is computed, never written (reactivity.rs)
		if let Some(symbol) = place.symbol
			&& self.report_derived_write(target, symbol)
		{
			return;
		}
		if place.below {
			self.report_immutable_root(target, place);
			return;
		}
		let tree = Arc::clone(&self.tree);
		let text = tree.text(target.index());
		let message = match place
			.symbol
			.map(|symbol| self.a.symbols.symbol(symbol).kind)
		{
			Some(SymbolKind::Let) => {
				format!("`{text}` is declared with `let`: declare it with `mutable` to assign it")
			}
			_ => format!("`{text}` can't be assigned"),
		};
		let diagnostic = self.diagnostic(&E_DECL_3, target, message);
		self.report(diagnostic);
	}

	/// A declaration the analyzer doesn't handle yet: `AN001`, and its names
	/// are declared with the type `UNKNOWN`, so that their uses don't cascade
	/// into `E-DECL-1`.
	pub(super) fn unsupported_declaration(
		&mut self,
		n: NodeId,
		names: &[Name],
		kind: SymbolKind,
		what: &str,
	) -> TypeId {
		if self.type_of(n).is_unset() {
			// quietly: the rules of redeclaration differ (an invalidated `use`)
			self.quiet += 1;
			for &name in names.iter().filter(|name| !name.is_empty()) {
				let symbol = self.declare(n, name, kind);
				self.a.symbols.symbol_mut(symbol).ty = TypeId::UNKNOWN;
			}
			self.quiet -= 1;
		}
		self.unsupported(n, what)
	}

	/// `type Name = …` (hoisted).
	pub(super) fn check_type_decl(
		&mut self,
		n: NodeId,
		name: Name,
		parameters: NodeList<'_>,
		value: NodeId,
	) -> TypeId {
		if parameters.is_empty() {
			return self.declare_type(n, name, value);
		}
		// `type Point(NumberType) = …`: the parameters are rigid in the declaration
		let text = self.decl_text(n, name);
		let saved = self.body.scope;
		let params = self.declare_generics(n, &text, parameters);
		let rigid = self.rigid_params.len();
		self.rigid_params.extend(params.iter().copied());
		let ty = self.declare_type(n, name, value);
		self.rigid_params.truncate(rigid);
		self.body.scope = saved;
		ty
	}

	/// `namespace Name = …` (hoisted).
	pub(super) fn check_namespace(&mut self, n: NodeId, _name: Name, value: NodeId) -> TypeId {
		self.check_namespace_members(n, value)
	}

	/// `export = value`, `export mutable: Type = value`: the file's export,
	/// declared like `let`/`mutable` (modules.rs).
	pub(super) fn check_export_value(
		&mut self,
		n: NodeId,
		mutable: bool,
		annotation: Option<NodeId>,
		value: NodeId,
	) -> TypeId {
		if self.is_extra_export(n) {
			return TypeId::VOID;
		}
		let kind = if mutable {
			BindingKind::Mutable
		} else {
			BindingKind::Let
		};
		self.hoisted(n, |w| {
			w.check_declaration(n, kind, Name::default(), annotation, Some(value))
		})
	}

	/// `export function = …`, `export function(Item) = …`.
	pub(super) fn check_export_function(
		&mut self,
		n: NodeId,
		is_async: bool,
		generics: NodeList<'_>,
		annotation: Option<NodeId>,
		value: NodeId,
	) -> TypeId {
		if self.is_extra_export(n) {
			return TypeId::VOID;
		}
		self.hoisted(n, |w| {
			w.check_function_decl(n, Name::default(), is_async, generics, annotation, value)
		})
	}

	/// `export type = …`, `export type(First, Second) = …`.
	pub(super) fn check_export_type(
		&mut self,
		n: NodeId,
		parameters: NodeList<'_>,
		value: NodeId,
	) -> TypeId {
		if self.is_extra_export(n) {
			return TypeId::VOID;
		}
		self.hoisted(n, |w| {
			w.check_type_decl(n, Name::default(), parameters, value)
		})
	}

	/// `export namespace = { … }`.
	pub(super) fn check_export_namespace(&mut self, n: NodeId, value: NodeId) -> TypeId {
		if self.is_extra_export(n) {
			return TypeId::VOID;
		}
		self.hoisted(n, |w| w.check_namespace_members(n, value))
	}
}
