//! Blocks, scopes, hoisting and declarations: `let`, `mutable`, assignment;
//! `static`, `type`, `namespace`, `use`, exports (stubs). On-demand analysis
//! of hoisted declarations (design §3.2).

use std::sync::Arc;

use crate::builtins;
use crate::check::{BodyCx, PlaceInfo, Walker};
use crate::diagnostics::codes::values::{
	E_DECL_2, E_DECL_3, E_DECL_9, E_DECL_10, E_DECL_11, W_DECL_1,
};
use crate::ids::{NodeId, SymbolId, TypeId};
use crate::symbols::{DeclState, ScopeKind, Symbol, SymbolKind};
use crate::syntax::{self, BindingKind, HoistKind, Name, NodeList};

impl Walker<'_> {
	/// The file: its scope (created by `analyze`), hoisted declarations, statements.
	pub(super) fn check_module(&mut self, n: NodeId, statements: NodeList<'_>) -> TypeId {
		let scope = self.a.symbols.scope_for(n, None, ScopeKind::Module);
		let saved = std::mem::replace(&mut self.body.scope, scope);
		self.declare_hoisted(n);
		for statement in statements.iter() {
			self.check_stmt(statement);
		}
		self.body.scope = saved;
		TypeId::VOID
	}

	/// A block: a scope, its hoisted declarations, its statements in order.
	/// Block values (`do`, branches of an `if` value) are computed by their
	/// parent from the last statement.
	pub(super) fn check_block(
		&mut self,
		n: NodeId,
		statements: NodeList<'_>,
		_expected: Option<TypeId>,
	) -> TypeId {
		let parent = self.body.scope;
		let scope = self.a.symbols.scope_for(n, Some(parent), ScopeKind::Block);
		self.body.scope = scope;
		self.declare_hoisted(n);
		for statement in statements.iter() {
			self.check_stmt(statement);
		}
		self.body.scope = parent;
		TypeId::VOID
	}

	/// Declares the hoisted declarations of a block `Pending` (once per block).
	fn declare_hoisted(&mut self, block: NodeId) {
		let tree = Arc::clone(&self.tree);
		for declaration in syntax::block_declarations(&tree, block, false) {
			if self.a.symbols.symbol_of_decl(declaration.node).is_some() {
				continue;
			}
			let kind = match declaration.kind {
				HoistKind::Function => SymbolKind::Function,
				HoistKind::Type => SymbolKind::Type,
				HoistKind::Namespace => SymbolKind::Namespace,
				HoistKind::Value => SymbolKind::Let,
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
		if let Some(previous) = previous {
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
		} else if builtins::lookup(text).is_some() {
			let diagnostic = self.diagnostic_at(
				&W_DECL_1,
				syntax::name_span(name),
				format!("`{text}` hides the built-in `{text}`: give it another name"),
			);
			self.report(diagnostic);
		}
		id
	}

	/// The type of a symbol at a use: analyzes a pending declaration on demand,
	/// with a fresh body context (decision A11); `UNKNOWN` on a cycle.
	pub(super) fn symbol_type(&mut self, symbol: SymbolId) -> TypeId {
		let current = self.a.symbols.symbol(symbol);
		match current.state {
			DeclState::Pending => {
				let (node, scope) = (current.decl_node, current.scope);
				let saved_body = std::mem::replace(&mut self.body, BodyCx::new(scope));
				let saved_statement = self.statement.take();
				self.check_stmt(node);
				self.statement = saved_statement;
				self.body = saved_body;
			}
			// a cycle through a signature or a type: recursive types and `MOD007` come later
			DeclState::Signature { .. } => return TypeId::UNKNOWN,
			DeclState::Sequential | DeclState::Body { .. } | DeclState::Done => {}
		}
		let ty = self.a.symbols.symbol(symbol).ty;
		if ty.is_unset() { TypeId::UNKNOWN } else { ty }
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
		let symbol_kind = match kind {
			BindingKind::Let => SymbolKind::Let,
			BindingKind::Mutable => SymbolKind::Mutable,
			_ => return self.unsupported(n, &format!("`{}` declarations", kind.keyword())),
		};
		let declared = annotation.map(|annotation| self.check_type_expr(annotation));
		let actual = value.map(|value| (value, self.check(value, declared)));
		if let (Some((value, actual)), Some(declared)) = (actual, declared) {
			self.sink(value, actual, declared);
		}
		self.memo(n, |w| {
			let text = w.name_text(name).to_string();
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
			if !name.is_empty() {
				let symbol = w.declare(n, name, symbol_kind);
				w.a.symbols.symbol_mut(symbol).ty = ty;
			}
			TypeId::VOID
		})
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
			_ => self.s.types.widen(actual),
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
		let expected = (!self.s.types.is_recovering(place.ty)).then_some(place.ty);
		let actual = self.check(value, expected);
		self.sink(value, actual, place.ty);
		self.memo(n, |w| {
			w.check_assignable(target, &place);
			TypeId::VOID
		})
	}

	/// `E-DECL-3` when a place can't be assigned as a whole. Type part.
	pub(super) fn check_assignable(&mut self, target: NodeId, place: &PlaceInfo) {
		if place.assignable {
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

	/// `use` and `mutable use` (M2, references).
	pub(super) fn check_use(
		&mut self,
		n: NodeId,
		_name: Name,
		_annotation: Option<NodeId>,
		_value: NodeId,
		_mutable: bool,
	) -> TypeId {
		self.unsupported(n, "`use` declarations")
	}

	/// `reuse` (M2, references).
	pub(super) fn check_reuse(&mut self, n: NodeId, _name: Name, _value: NodeId) -> TypeId {
		self.unsupported(n, "`reuse`")
	}

	/// `type Name = …` (hoisted).
	pub(super) fn check_type_decl(
		&mut self,
		n: NodeId,
		_name: Name,
		_parameters: NodeList<'_>,
		_value: NodeId,
	) -> TypeId {
		self.unsupported(n, "type declarations")
	}

	/// `namespace Name = …` (hoisted).
	pub(super) fn check_namespace(&mut self, n: NodeId, _name: Name, _value: NodeId) -> TypeId {
		self.unsupported(n, "namespaces")
	}

	/// `export = value` (M2 modules).
	pub(super) fn check_export_value(
		&mut self,
		n: NodeId,
		_mutable: bool,
		_annotation: Option<NodeId>,
		_value: NodeId,
	) -> TypeId {
		self.unsupported(n, "exports")
	}

	/// `export function = …` (M2 modules).
	pub(super) fn check_export_function(
		&mut self,
		n: NodeId,
		_is_async: bool,
		_generics: NodeList<'_>,
		_annotation: Option<NodeId>,
		_value: NodeId,
	) -> TypeId {
		self.unsupported(n, "exports")
	}

	/// `export type = …` (M2 modules).
	pub(super) fn check_export_type(
		&mut self,
		n: NodeId,
		_parameters: NodeList<'_>,
		_value: NodeId,
	) -> TypeId {
		self.unsupported(n, "exports")
	}

	/// `export namespace …` (M2 modules).
	pub(super) fn check_export_namespace(&mut self, n: NodeId, _value: NodeId) -> TypeId {
		self.unsupported(n, "exports")
	}
}
