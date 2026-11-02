//! Structural typing (advanced brief §10): the diagnostics of a value that
//! doesn't satisfy an object type (`STR001`, `STR003`, `STR009`), and the
//! narrowing copies the backends must make (§10.4): `let human: Human =
//! superHuman` copies `Human`'s fields only, while a parameter is a view.
//! Two declarations with the same fields are one type; the methods of a
//! value are those of the type **written** for it (§10.1): its annotation,
//! or the type of the literal it was built with.

use std::sync::Arc;

use crate::analysis::NodeFlags;
use crate::check::Walker;
use crate::diagnostics::codes::advanced::{STR001, STR003};
use crate::diagnostics::codes::types::TF020;
use crate::diagnostics::codes::values::E_TYPE_1;
use crate::ids::{NodeId, ParamId, Sym, SymbolId, TypeId};
use crate::symbols::{Resolution, SymbolKind};
use crate::syntax::{self, NodeList, View};
use crate::types::{FieldEntry, NominalKind, ParamInfo, TypeKind};

impl Walker<'_> {
	/// Records the `type` declaration written for a binding or a parameter:
	/// the annotation `Human`, the literal `Animal { … }`, or the written
	/// type of the binding it copies. Type part.
	pub(super) fn record_written_type(&mut self, symbol: SymbolId, written: Option<NodeId>) {
		if let Some(decl) = written.and_then(|node| self.written_decl(node)) {
			self.written_decls.insert(symbol, decl);
		}
	}

	// ── `Type(value)` (types brief §3.11.7, advanced brief §10.4) ─────────

	/// The parameters of a function whose type is named by `Type(parameter)`
	/// in its signature: each becomes a hidden generic parameter, bound to
	/// the type of the argument at each call (`renamed(hero, …)` returns a
	/// `SuperHuman`), with the written type as constraint. Called before the
	/// signature is checked; the parameters are rigid in the body.
	pub(super) fn implicit_generics(&mut self, function: NodeId, value: NodeId) -> Vec<ParamId> {
		let tree = Arc::clone(&self.tree);
		let View::Lambda { signature, .. } = syntax::view(&tree, value) else {
			return Vec::new();
		};
		let mut names: Vec<(Sym, NodeId)> = Vec::new();
		for parameter in signature.parameters.iter() {
			if let View::Assignment { target, .. } = syntax::view(&tree, parameter)
				&& let View::Name(name) = syntax::view(&tree, target)
			{
				names.push((self.s.types.syms.intern(tree.name(name)), parameter));
			}
		}
		let mut used: Vec<Sym> = Vec::new();
		let mut roots: Vec<NodeId> = signature.returns.into_iter().collect();
		for parameter in signature.parameters.iter() {
			if let View::Assignment {
				annotation: Some(annotation),
				..
			} = syntax::view(&tree, parameter)
			{
				roots.push(annotation);
			}
		}
		while let Some(node) = roots.pop() {
			if let View::Call { callee, arguments } = syntax::view(&tree, node)
				&& matches!(syntax::view(&tree, callee), View::Name(name) if tree.name(name) == "Type")
				&& let Some(argument) = arguments.first()
				&& let View::Name(name) = syntax::view(&tree, argument)
			{
				let sym = self.s.types.syms.intern(tree.name(name));
				if names.iter().any(|&(known, _)| known == sym) && !used.contains(&sym) {
					used.push(sym);
				}
				continue;
			}
			roots.extend(syntax::children(&tree, node));
		}
		if used.is_empty() {
			return Vec::new();
		}
		let owner_name = format!("{}#implicit", function.index());
		let key = self.nominal_key(&owner_name);
		let mut params = Vec::with_capacity(used.len());
		for (index, sym) in used.into_iter().enumerate() {
			let text = format!("Type({})", self.s.types.syms.resolve(sym));
			let name = self.s.types.syms.intern(&text);
			let param = self.s.types.add_param(ParamInfo {
				owner: key,
				index: index as u16,
				name,
				constraint: TypeId::UNKNOWN,
				default: TypeId::UNSET,
				value: false,
			});
			self.pending_implicit.push((sym, param));
			params.push(param);
		}
		params
	}

	/// The declared type of a parameter named by `Type(parameter)`: its
	/// hidden generic parameter, constrained by the written type.
	pub(super) fn implicit_parameter_type(&mut self, name: Sym, declared: TypeId) -> TypeId {
		let Some(&(_, param)) = self
			.pending_implicit
			.iter()
			.find(|(known, _)| *known == name)
		else {
			return declared;
		};
		self.s
			.types
			.set_param_bounds(param, declared, TypeId::UNSET);
		self.s.types.intern(TypeKind::Param(param))
	}

	/// `Type(value)` in a type: the type of a value without its facts; for
	/// a parameter, the type of its argument (an implicit generic, see
	/// [`Walker::implicit_generics`]). `TF020` on a type. Type part.
	pub(super) fn type_of_value(
		&mut self,
		n: NodeId,
		callee: NodeId,
		arguments: NodeList<'_>,
	) -> TypeId {
		self.set_flag(callee, NodeFlags::TYPE_EXPR);
		let tree = Arc::clone(&self.tree);
		let argument = arguments.first().filter(|_| arguments.len() == 1);
		let ty = match argument.map(|argument| (argument, syntax::view(&tree, argument))) {
			Some((argument, View::Name(name))) => {
				let sym = self.s.types.syms.intern(tree.name(name));
				let local = self.a.symbols.lookup(self.body.scope, sym);
				let builtin_type = local.is_none()
					&& (crate::builtins::lookup(tree.name(name)).is_some_and(|builtin| {
						crate::builtins::denoted_type(&mut self.s.types, builtin).is_some()
					}) || super::type_expr::collection_kind(tree.name(name)).is_some());
				match local {
					_ if builtin_type => self.memo(n, |w| {
						let message = format!(
							"`{}` is already a type: write it without `Type(…)`",
							tree.name(name)
						);
						let diagnostic = w.diagnostic(&TF020, argument, message);
						w.report(diagnostic);
						TypeId::UNKNOWN
					}),
					Some(symbol) if self.a.symbols.symbol(symbol).kind == SymbolKind::Type => self
						.memo(n, |w| {
							let message = format!(
								"`{}` is already a type: write it without `Type(…)`",
								tree.name(name)
							);
							let diagnostic = w.diagnostic(&TF020, argument, message);
							w.report(diagnostic);
							TypeId::UNKNOWN
						}),
					_ => {
						let ty = self.check(argument, None);
						self.s.types.widen(ty)
					}
				}
			}
			Some((argument, _)) => {
				let ty = self.check(argument, None);
				self.s.types.widen(ty)
			}
			None => TypeId::UNKNOWN,
		};
		let ty = self.memo(n, |w| {
			if argument.is_none() {
				let diagnostic = w.diagnostic(
					&crate::diagnostics::codes::types::TY003,
					n,
					"`Type` takes one value, ex: `Type(user)`",
				);
				w.report(diagnostic);
			}
			ty
		});
		self.memo(callee, |_| ty)
	}

	/// The `type` declaration an expression or annotation names.
	pub(super) fn written_decl(&self, n: NodeId) -> Option<NodeId> {
		match self.view(n) {
			View::Name(_) => match self.a.resolution(n) {
				Resolution::Local(symbol) => {
					let data = self.a.symbols.symbol(symbol);
					match data.kind {
						SymbolKind::Type => Some(data.decl_node),
						_ => self.written_decls.get(&symbol).copied(),
					}
				}
				_ => None,
			},
			View::TypedObject { target, .. } => self.written_decl(target),
			View::Group(inner) => self.written_decl(inner),
			_ => None,
		}
	}

	/// The object type of a value whose fields can be read: an object, or a
	/// member of a named union with object data.
	fn object_of(&self, ty: TypeId) -> Option<TypeId> {
		let ty = self.s.types.unfold(ty);
		match self.s.types.kind(ty) {
			TypeKind::Object(_) => Some(ty),
			TypeKind::Member(nominal, index) => {
				let payload = self.s.types.payload(nominal, index);
				matches!(self.s.types.kind(payload), TypeKind::Object(_)).then_some(payload)
			}
			_ => None,
		}
	}

	/// `STR001`: an existing value lacks a field of the expected type;
	/// `STR003` when that field is optional (only a literal may leave an
	/// optional field out). Flow part of a sink.
	pub(super) fn report_missing_field(&mut self, n: NodeId, actual: TypeId, target: TypeId) {
		let actual_fields: Vec<FieldEntry> = self
			.object_of(actual)
			.map(|object| self.s.types.fields_of(object).to_vec())
			.unwrap_or_default();
		let missing = self
			.s
			.types
			.fields_of(target)
			.iter()
			.copied()
			.find(|field| !actual_fields.iter().any(|other| other.name == field.name));
		let Some(missing) = missing else {
			return;
		};
		let name = self.s.types.syms.resolve(missing.name).to_string();
		let optional = self.s.types.satisfies(TypeId::NONE, missing.ty);
		let (code, message) = if optional {
			(
				&STR003,
				format!(
					"a `{}` has no `{name}` field: a `{}` is expected (only a literal may leave an optional field out)",
					self.display(actual),
					self.display(target)
				),
			)
		} else {
			(
				&STR001,
				format!(
					"a `{}` has no field `{name}: {}`: a `{}` is expected",
					self.display(actual),
					self.display(missing.ty),
					self.display(target)
				),
			)
		};
		let diagnostic = self.diagnostic(code, n, message);
		self.report_flow(diagnostic);
	}

	/// `E-TYPE-1` (alias `STR009`): a value of a nominal type (a named union, `Errors`, an
	/// enumeration, a type with private fields) where another nominal type is
	/// expected. Flow part of a sink that failed; whether it reported.
	pub(super) fn nominal_mismatch(&mut self, n: NodeId, actual: TypeId, target: TypeId) -> bool {
		let (Some(source), Some(expected)) = (self.nominal_of(actual), self.nominal_of(target))
		else {
			return false;
		};
		if source == expected {
			return false;
		}
		let message = format!(
			"a `{}` is never a `{}`: each declaration of a named union, an enumeration or a type with private fields is its own type",
			self.display(actual),
			self.display(target)
		);
		let diagnostic = self.diagnostic(&E_TYPE_1, n, message);
		self.report_flow(diagnostic);
		true
	}

	/// The declaration of a nominal type.
	fn nominal_of(&self, ty: TypeId) -> Option<crate::ids::DeclKey> {
		let nominal = match self.s.types.kind(self.s.types.unfold(ty)) {
			TypeKind::NamedUnion(nominal, _)
			| TypeKind::Member(nominal, _)
			| TypeKind::Enum(nominal, _)
			| TypeKind::EnumMember(nominal, _) => nominal,
			TypeKind::Object(shape) => self.s.types.shape(shape).nominal?,
			_ => return None,
		};
		let info = self.s.types.nominal(nominal);
		(info.kind != NominalKind::CStruct).then_some(info.key)
	}

	/// A value copied into a place of another object type (a binding, a
	/// field, a collection element, a return value): the copy keeps the
	/// target's fields only, element by element for a collection
	/// (`coercions[n]` = the target type). A parameter is a view and copies
	/// nothing ([`Walker::view_sink`]). Flow part of a sink that succeeded.
	pub(super) fn record_narrowing_copy(&mut self, n: NodeId, actual: TypeId, target: TypeId) {
		if self.view_sink || !self.narrows(actual, target, 0) {
			return;
		}
		self.a.coercions.insert(n, target);
	}

	/// Whether a copy from `actual` to `target` must drop or convert fields.
	fn narrows(&self, actual: TypeId, target: TypeId, depth: u32) -> bool {
		let (actual, target) = (self.s.types.unfold(actual), self.s.types.unfold(target));
		if actual == target || depth > 8 {
			return false;
		}
		match (self.s.types.kind(actual), self.s.types.kind(target)) {
			(TypeKind::Object(_) | TypeKind::Member(..), TypeKind::Object(_)) => self
				.object_of(actual)
				.is_some_and(|object| object != target),
			(
				TypeKind::Collection(kind, item, key),
				TypeKind::Collection(target_kind, target_item, target_key),
			) if kind == target_kind => {
				self.narrows(item, target_item, depth + 1)
					|| self.narrows(key, target_key, depth + 1)
			}
			(TypeKind::Tuple(items), TypeKind::Tuple(target_items)) => self
				.s
				.types
				.list_items(items)
				.iter()
				.zip(self.s.types.list_items(target_items))
				.any(|(&item, &target_item)| self.narrows(item, target_item, depth + 1)),
			_ => false,
		}
	}
}
