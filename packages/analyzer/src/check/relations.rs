//! Relations (advanced brief §8, `advanced/relations.md`): the fields that
//! link the elements of stable-address collections, kept consistent by the
//! compiler.
//!
//! - **Classification** (§8.2), when a type declaration's fields are
//!   resolved ([`Walker::relation_field`]): `(T in c)?` and `Pool(T in c)`
//!   are direct sides, `Pool(H->f)`, `(H->f)?` and `Pool(in H->f)` reverse
//!   sides, `(T in c)?` on the type that owns `c` an owner field. The
//!   collection is a field of the type or of an enclosing type, or a
//!   module-level value (`REL005`), with stable addresses (`REL003`), of the
//!   written element type (`REL004`), `mutable` when module-level
//!   (`REL019`); a direct side is optional or a `Pool` (`REL002`), an owner
//!   field optional (`REL015`), no side has an initializer (`REL013`).
//! - **Pairing** (§8.2.5), at the end of the file ([`Walker::finish_relations`]):
//!   each reverse side names a direct side (`REL008`) whose targets are the
//!   reverse side's type (`REL009`), with the matching shape (`REL007`),
//!   once (`REL006`); a direct side without reverse side is `REL001`. The
//!   result is `Analysis::relations` and `Analysis::relation_fields`.
//! - **Writes** (§8.4): only direct sides are written (`REL010`), through a
//!   reference into the collection (`REL011` for a copy), to a target of
//!   the same owner (`REL012`); a whole-value assignment keeps the links and
//!   can't set them (`REL021`, D20). `Analysis::relation_writes` and
//!   `Analysis::relation_unlinks` tell the backends where to generate the
//!   two-sided updates and the cleanups.
//! - A collection that owner fields point into is modified only through its
//!   owner (`REL016`, D48), and a local can't have a reference type
//!   (`REF010`, D17).
//! - **Across files** ([`Walker::import_relations_for`]): a relation may
//!   name a module-level collection of another file, by key; both sides are
//!   declared in one file. Each file publishes its relations in its
//!   interface; the files whose values may be of their types import them,
//!   so their writes and removals get the checks and the two-sided updates.
//!   The collection's file checks it ([`Walker::check_external_collections`]).

use std::sync::Arc;

use rustc_hash::{FxHashMap, FxHashSet};

use crate::analysis::{CollectionRef, Relation, RelationKind, RelationSide};
use crate::check::Walker;
use crate::check::objects::MemberKind;
use crate::check::pools::has_stable_addresses;
use crate::diagnostics::codes::advanced::{
	REF010, REL001, REL002, REL003, REL004, REL005, REL006, REL007, REL008, REL009, REL010, REL011,
	REL012, REL013, REL015, REL016, REL018, REL019, REL021,
};
use crate::ids::{DeclKey, ModuleId, NodeId, Sym, SymbolId, TypeId};
use crate::symbols::{Resolution, ScopeKind, SymbolKind};
use crate::syntax::{self, BindingKind, NodeList, View};
use crate::types::{CollKind, TypeKind};

/// The shape of a relation field.
#[derive(Copy, Clone, Debug, PartialEq)]
enum Shape {
	/// `(T in c)?` (`many`: `Pool(T in c)`).
	Direct { many: bool },
	/// `(H->f)?`, `Pool(H->f)` (`many`), `Pool(in H->f)` (`many`, `inside`).
	Reverse {
		field: Sym,
		many: bool,
		inside: bool,
	},
	/// `(T in c)?` on the type that owns `c`.
	Owner,
}

/// A collection named by a relation field, as declared.
#[derive(Copy, Clone, Debug)]
pub(crate) struct CollectionInfo {
	reference: CollectionRef,
	/// Its kind, when its type is written.
	kind: Option<CollKind>,
	/// The type declaration of its elements, when written as a name.
	element_decl: Option<NodeId>,
	/// The node of its element type (`Person` in `Pool(Person)`).
	element_node: Option<NodeId>,
	/// A module-level value declared `mutable` (always true for a field).
	mutable: bool,
}

/// A relation field found in a type declaration.
#[derive(Clone, Debug)]
pub(crate) struct SideDecl {
	/// The field node.
	node: NodeId,
	/// The type declaration that declares it.
	type_decl: NodeId,
	/// Its name.
	field: Sym,
	shape: Shape,
	/// The collection it names: the targets of a direct side or owner field,
	/// the holders of a reverse side.
	collection: Option<CollectionInfo>,
	/// A side of a relation declared by another file: the index of the
	/// relation in [`RelationState::imported`] (its `node` and `type_decl`
	/// mean nothing here).
	imported: Option<u32>,
}

/// A relation as paired in this file: the relation, and the type
/// declarations of its sides (`None` for a relation of another file, whose
/// types are known).
#[derive(Clone, Debug)]
struct Paired {
	relation: Relation,
	holder_decl: Option<NodeId>,
	target_decl: Option<NodeId>,
}

/// The relation state of a file's walk.
#[derive(Default)]
pub(crate) struct RelationState {
	/// The relation fields, in the order they were classified.
	sides: Vec<SideDecl>,
	/// The type declarations whose fields are being resolved.
	pub(crate) declaring: Vec<NodeId>,
	/// The relation field of an object type, by field name.
	by_type: FxHashMap<(TypeId, Sym), usize>,
	/// The writes of direct sides: the node and the side.
	writes: Vec<(NodeId, usize)>,
	/// The removals from collections: the call and the collection.
	unlinks: Vec<(NodeId, CollectionRef)>,
	/// The type expressions `T in c` (reference types).
	pub(crate) reference_types: rustc_hash::FxHashSet<NodeId>,
	/// The relations declared by other files whose types this file may
	/// see (relations across files), their collections as this file names
	/// them.
	imported: Vec<Relation>,
	/// The modules whose relations are imported.
	imported_modules: FxHashSet<ModuleId>,
	/// The object types whose declaring files were looked at.
	imported_types: FxHashSet<TypeId>,
	/// The proxy symbol of each module-level collection of another file
	/// named here (the place of its implied effects).
	external_collections: FxHashMap<DeclKey, SymbolId>,
}

impl Walker<'_> {
	// ── classification ───────────────────────────────────────────────────

	/// The type of a field whose annotation is a relation side, after
	/// classifying it; `None` for any other field. Type part of the field.
	pub(super) fn relation_field(
		&mut self,
		field: NodeId,
		name: Sym,
		annotation: NodeId,
		default: Option<NodeId>,
		items: NodeList<'_>,
	) -> Option<TypeId> {
		let type_decl = *self.relations.declaring.last()?;
		let tree = Arc::clone(&self.tree);
		let (optional, inner) = match syntax::view(&tree, annotation) {
			View::Optional(inner) => (true, self.strip_groups(inner)),
			_ => (false, self.strip_groups(annotation)),
		};
		// what the annotation is: a reference or a reverse side
		enum Found {
			Reference {
				item: NodeId,
				owner: NodeId,
				many: bool,
			},
			Reverse {
				collection: NodeId,
				field: NodeId,
				many: bool,
				inside: bool,
			},
		}
		let pool_argument = |inner: NodeId| match syntax::view(&tree, inner) {
			View::Call { callee, arguments }
				if matches!(syntax::view(&tree, callee), View::Name(name) if tree.name(name) == "Pool")
					&& arguments.len() == 1 =>
			{
				arguments.first()
			}
			_ => None,
		};
		let found = if let View::RefType { item, owner } = syntax::view(&tree, inner) {
			Found::Reference {
				item,
				owner,
				many: false,
			}
		} else if let Some((collection, field)) = syntax::relation_parts(&tree, inner) {
			Found::Reverse {
				collection,
				field,
				many: false,
				inside: false,
			}
		} else {
			let argument = pool_argument(inner).filter(|_| !optional)?;
			let argument = self.strip_groups(argument);
			if let View::RefType { item, owner } = syntax::view(&tree, argument) {
				Found::Reference {
					item,
					owner,
					many: true,
				}
			} else if let Some((collection, field)) = syntax::relation_parts(&tree, argument) {
				Found::Reverse {
					collection,
					field,
					many: true,
					inside: false,
				}
			} else if let Some(relation) = syntax::in_relation_inner(&tree, argument)
				&& let Some((collection, field)) = syntax::relation_parts(&tree, relation)
			{
				Found::Reverse {
					collection,
					field,
					many: true,
					inside: true,
				}
			} else {
				return None;
			}
		};
		// `Pool(…)` is the application of a type (read by `migrate.rs`)
		if matches!(
			found,
			Found::Reference { many: true, .. } | Found::Reverse { many: true, .. }
		) {
			self.set_flag(inner, crate::analysis::NodeFlags::TYPE_EXPR);
		}
		// neither side is initialized (REL013)
		if let Some(default) = default {
			let text = self.s.types.syms.resolve(name).to_string();
			let message = format!(
				"`{text}` is a side of a relation: it has no initial value (an optional side starts as `none`, a pool side empty); remove `= …`"
			);
			let diagnostic = self.diagnostic(&REL013, default, message);
			self.report(diagnostic);
		}
		let (shape, collection_node, ty) = match found {
			Found::Reference { item, owner, many } => {
				let item_ty = self.check_type_expr(item);
				let collection = self.find_collection(owner, items);
				let own = matches!(collection, Some(CollectionInfo { reference: CollectionRef::Field { owner: decl, .. }, .. }) if decl == type_decl);
				let shape = if own && !many {
					Shape::Owner
				} else {
					Shape::Direct { many }
				};
				if !optional && !many {
					let (code, message) = if own {
						(
							&REL015,
							"an owner field is optional: it becomes `none` when its target is deleted, ex: `(Node in #nodes)?`".to_string(),
						)
					} else {
						(
							&REL002,
							"a direct side is optional or a `Pool`: it becomes `none` (or loses its target) when the target is deleted, ex: `(Person in people)?`".to_string(),
						)
					};
					let diagnostic = self.diagnostic(code, field, message);
					self.report(diagnostic);
				}
				if let Some(info) = collection {
					let item_decl = self.type_decl_of(item);
					if let (Some(expected), Some(actual)) = (info.element_decl, item_decl)
						&& expected != actual
					{
						let collection_text = self.tree.text(owner.index()).to_string();
						let code = if own { &REL018 } else { &REL004 };
						let message = format!(
							"`{collection_text}` holds `{}` elements: write `{} in {collection_text}`",
							self.tree.text(info.element_node.unwrap_or(item).index()),
							self.tree.text(info.element_node.unwrap_or(item).index())
						);
						let diagnostic = self.diagnostic(code, item, message);
						self.report(diagnostic);
					}
				}
				let ty = if many {
					self.s
						.types
						.collection(CollKind::RelationPool(0), item_ty, TypeId::NONE)
				} else {
					self.s.types.optional(item_ty)
				};
				(shape, (owner, collection), ty)
			}
			Found::Reverse {
				collection,
				field: field_node,
				many,
				inside,
			} => {
				let mut info = self.find_collection(collection, items);
				// a collection of another file: its elements are the type of
				// this file declaring the direct side (both sides of a
				// relation are declared in one file)
				if let Some(found) = &mut info
					&& matches!(found.reference, CollectionRef::External { .. })
				{
					let text = self.tree.text(field_node.index()).to_string();
					// a symmetric side (`spouse: (people->spouse)?`) is its own
					found.element_decl = self.direct_side_declarer(&text).or_else(|| {
						(self.s.types.syms.get(&text) == Some(name)).then_some(type_decl)
					});
				}
				let element_ty = match info {
					Some(CollectionInfo {
						element_node: Some(node),
						..
					}) => self.check_type_expr(node),
					Some(CollectionInfo {
						element_decl: Some(decl),
						..
					}) => self.declared_type_at(decl, field),
					_ => TypeId::UNKNOWN,
				};
				let direct = self.s.types.syms.intern(self.tree.text(field_node.index()));
				if !optional && !many {
					let diagnostic = self.diagnostic(
						&REL007,
						field,
						"a one-to-one reverse side is optional: `(people->mentor)?`",
					);
					self.report(diagnostic);
				}
				let ty = if many {
					self.s
						.types
						.collection(CollKind::RelationPool(1), element_ty, TypeId::NONE)
				} else {
					self.s.types.optional(element_ty)
				};
				let shape = Shape::Reverse {
					field: direct,
					many,
					inside,
				};
				(shape, (collection, info), ty)
			}
		};
		let (collection_node, collection) = collection_node;
		match collection {
			None => {
				let text = self.tree.text(collection_node.index()).to_string();
				let message = format!(
					"`{text}` is not a collection visible from this type: declare it as a field of the enclosing type, or as a module-level value"
				);
				let diagnostic = self.diagnostic(&REL005, collection_node, message);
				self.report(diagnostic);
			}
			Some(info) => {
				let text = self.tree.text(collection_node.index()).to_string();
				// reported on the direct side (the reverse one names the same collection)
				let direct = !matches!(shape, Shape::Reverse { .. });
				if let Some(kind) = info.kind
					&& !has_stable_addresses(kind)
					&& direct
				{
					let message = format!(
						"`{text}` is a `{}`, whose elements move: a relation needs a `Pool`, a `Columnar`, a `GrowingPool` or a `GrowingColumnar`",
						kind.name()
					);
					let diagnostic = self.diagnostic(&REL003, field, message);
					self.report(diagnostic);
				}
				if !info.mutable && direct {
					let message = format!(
						"`{text}` must be declared `mutable`: deleting an element writes the relation fields that point at it"
					);
					let diagnostic = self.diagnostic(&REL019, collection_node, message);
					self.report(diagnostic);
				}
				// the holders of a reverse side are written when a target is
				// deleted: reported on the collection's declaration, unless a
				// direct side naming it was reported
				let named_by_direct = self.relations.sides.iter().any(|side| {
					!matches!(side.shape, Shape::Reverse { .. })
						&& side
							.collection
							.is_some_and(|other| other.reference == info.reference)
				});
				if !info.mutable
					&& !direct && !named_by_direct
					&& let CollectionRef::Module { declaration } = info.reference
					&& self.reported_once.insert((declaration, REL019.id))
				{
					let message = format!(
						"`{text}` must be declared `mutable`: deleting a target of its relation writes the fields of its elements"
					);
					let diagnostic = self.diagnostic(&REL019, declaration, message);
					self.report(diagnostic);
				}
			}
		}
		self.relations.sides.push(SideDecl {
			node: field,
			type_decl,
			field: name,
			shape,
			collection,
			imported: None,
		});
		self.memo(annotation, |_| ty);
		Some(ty)
	}

	/// Registers the relation fields of a declared object type, once its
	/// type is known (writes and reads find them by type).
	pub(super) fn register_relation_fields(&mut self, type_decl: NodeId, ty: TypeId) {
		for (index, side) in self.relations.sides.iter().enumerate() {
			if side.type_decl == type_decl {
				self.relations.by_type.insert((ty, side.field), index);
			}
		}
	}

	/// The type declaration a type name designates (`Person`).
	fn type_decl_of(&mut self, node: NodeId) -> Option<NodeId> {
		let View::Name(name) = self.view(self.strip_groups(node)) else {
			return None;
		};
		let sym = self.s.types.syms.intern(self.tree.name(name));
		let symbol = self.a.symbols.lookup(self.body.scope, sym)?;
		let symbol = self.a.symbols.symbol(symbol);
		(symbol.kind == SymbolKind::Type).then_some(symbol.decl_node)
	}

	/// The collection a relation names: a field of the type being declared
	/// (an owner field's collection) or of an enclosing type, or a
	/// module-level value.
	fn find_collection(
		&mut self,
		name_node: NodeId,
		own_items: NodeList<'_>,
	) -> Option<CollectionInfo> {
		let View::Name(name) = self.view(self.strip_groups(name_node)) else {
			return None;
		};
		let text = self.tree.name(name).to_string();
		let tree = Arc::clone(&self.tree);
		let own = *self.relations.declaring.last()?;
		if let Some(info) = self.collection_field(own_items, &text, own) {
			return Some(info);
		}
		// the enclosing types, from the scope of the type's members
		let mut current = self.a.symbols.scope(self.body.scope).parent;
		while let Some(scope) = current {
			let data = self.a.symbols.scope(scope);
			let (owner, kind, parent) = (data.owner, data.kind, data.parent);
			if kind == ScopeKind::Namespace
				&& let View::Object { items } = syntax::view(&tree, owner)
				&& let Some(decl) =
					parent.and_then(|parent| self.type_decl_with_body(parent, owner))
				&& let Some(info) = self.collection_field(items, &text, decl)
			{
				return Some(info);
			}
			current = parent;
		}
		// a module-level value
		let sym = self.s.types.syms.intern(&text);
		let found = self.a.symbols.lookup(self.body.scope, sym);
		// of another file: named by its key (relations across files)
		if let Some(symbol) = found
			&& self.is_proxy(symbol)
		{
			return self.external_collection(symbol);
		}
		let declaration = match found {
			Some(symbol)
				if matches!(
					self.a.symbols.symbol(symbol).kind,
					SymbolKind::Let | SymbolKind::Mutable | SymbolKind::Reactive
				) =>
			{
				Some(self.a.symbols.symbol(symbol).decl_node)
			}
			_ => {
				let root = NodeId::new(tree.root);
				let statements: Vec<NodeId> = match syntax::view(&tree, root) {
					View::Block { statements } => statements.iter().collect(),
					_ => syntax::children(&tree, root).collect(),
				};
				statements.into_iter().find(|&statement| {
					matches!(syntax::view(&tree, statement), View::Declaration { name, .. } if tree.name(name) == text)
				})
			}
		}?;
		let View::Declaration {
			kind, annotation, ..
		} = syntax::view(&tree, declaration)
		else {
			return None;
		};
		let mut info = self.collection_info(annotation, CollectionRef::Module { declaration });
		info.mutable = matches!(kind, BindingKind::Mutable | BindingKind::Reactive);
		Some(info)
	}

	/// The field named `text` of a type body, as a collection.
	fn collection_field(
		&mut self,
		items: NodeList<'_>,
		text: &str,
		owner: NodeId,
	) -> Option<CollectionInfo> {
		let tree = Arc::clone(&self.tree);
		for item in items.iter() {
			let item = match syntax::view(&tree, item) {
				View::Readonly(inner) => inner,
				_ => item,
			};
			let View::Assignment {
				target, annotation, ..
			} = syntax::view(&tree, item)
			else {
				continue;
			};
			if !matches!(syntax::view(&tree, target), View::Name(name) if tree.name(name) == text) {
				continue;
			}
			let name = self.s.types.syms.intern(text);
			return Some(self.collection_info(annotation, CollectionRef::Field { owner, name }));
		}
		None
	}

	/// Whether a symbol stands for a declaration of another file (a proxy,
	/// or an alias of one).
	fn is_proxy(&self, symbol: SymbolId) -> bool {
		self.modules.proxies.contains_key(&symbol) || self.modules.aliases.contains_key(&symbol)
	}

	/// A module-level value of another file named by a relation: its key,
	/// without analyzing that file (which reads this one, for the type of
	/// its elements). Its kind and its element type are checked in its own
	/// file ([`Walker::check_external_collections`]).
	fn external_collection(&mut self, symbol: SymbolId) -> Option<CollectionInfo> {
		let kind = self.a.symbols.symbol(symbol).kind;
		if !matches!(
			kind,
			SymbolKind::Let | SymbolKind::Mutable | SymbolKind::Reactive
		) {
			return None;
		}
		let key = self.external_value_key(symbol)?;
		self.relations.external_collections.insert(key, symbol);
		Some(CollectionInfo {
			reference: CollectionRef::External { key },
			kind: None,
			element_decl: None,
			element_node: None,
			mutable: kind != SymbolKind::Let,
		})
	}

	/// The type declaration of this file that declares a direct side named
	/// `field` (a field typed `(T in c)?` or `Pool<T in c>`).
	fn direct_side_declarer(&self, field: &str) -> Option<NodeId> {
		let tree = Arc::clone(&self.tree);
		for index in 0..tree.nodes.len() {
			let node = NodeId::new(index);
			let value = match syntax::view(&tree, node) {
				View::TypeDecl { value, .. } | View::ExportType { value, .. } => value,
				_ => continue,
			};
			let View::Object { items } = syntax::view(&tree, value) else {
				continue;
			};
			for item in items.iter() {
				let item = match syntax::view(&tree, item) {
					View::Readonly(inner) => inner,
					_ => item,
				};
				let View::Assignment {
					target,
					annotation: Some(annotation),
					..
				} = syntax::view(&tree, item)
				else {
					continue;
				};
				if matches!(syntax::view(&tree, target), View::Name(name) if tree.name(name) == field)
					&& self.is_reference_annotation(annotation)
				{
					return Some(node);
				}
			}
		}
		None
	}

	/// Whether a field's annotation is a direct side: `(T in c)?`,
	/// `Pool<T in c>`.
	fn is_reference_annotation(&self, annotation: NodeId) -> bool {
		let inner = match self.view(annotation) {
			View::Optional(inner) => self.strip_groups(inner),
			_ => self.strip_groups(annotation),
		};
		match self.view(inner) {
			View::RefType { .. } => true,
			View::Call { callee, arguments } => {
				matches!(self.view(callee), View::Name(name) if self.tree.name(name) == "Pool")
					&& arguments.first().is_some_and(|argument| {
						matches!(self.view(self.strip_groups(argument)), View::RefType { .. })
					})
			}
			_ => false,
		}
	}

	/// The type a type declaration of this file declares, read at `at`
	/// (the declaration in progress: a recursive reference).
	fn declared_type_at(&mut self, decl: NodeId, at: NodeId) -> TypeId {
		match self.a.symbols.symbol_of_decl(decl) {
			Some(symbol) => self.symbol_type(symbol, at),
			None => TypeId::UNKNOWN,
		}
	}

	/// What a collection's written type says: its kind and element type.
	fn collection_info(
		&mut self,
		annotation: Option<NodeId>,
		reference: CollectionRef,
	) -> CollectionInfo {
		let mut info = CollectionInfo {
			reference,
			kind: None,
			element_decl: None,
			element_node: None,
			mutable: true,
		};
		let Some(annotation) = annotation else {
			return info;
		};
		let tree = Arc::clone(&self.tree);
		if let View::Call { callee, arguments } = syntax::view(&tree, annotation)
			&& let View::Name(name) = syntax::view(&tree, callee)
		{
			info.kind = super::type_expr::collection_kind(tree.name(name));
			if let Some(element) = arguments.first().filter(|_| arguments.len() == 1) {
				info.element_node = Some(element);
				info.element_decl = self.type_decl_of(element);
			}
		}
		info
	}

	/// The type declaration in `scope` whose body is `body`.
	/// (or an enclosing scope: the parameters of a generic type sit between).
	fn type_decl_with_body(&self, scope: crate::ids::ScopeId, body: NodeId) -> Option<NodeId> {
		let mut current = Some(scope);
		while let Some(scope) = current {
			let found = self
				.a
				.symbols
				.scope(scope)
				.symbols
				.iter()
				.find_map(|&symbol| {
					let symbol = self.a.symbols.symbol(symbol);
					if symbol.kind != SymbolKind::Type {
						return None;
					}
					match self.view(symbol.decl_node) {
						View::TypeDecl { value, .. } | View::ExportType { value, .. }
							if value == body =>
						{
							Some(symbol.decl_node)
						}
						_ => None,
					}
				});
			if found.is_some() {
				return found;
			}
			current = self.a.symbols.scope(scope).parent;
		}
		None
	}

	/// Whether an assignment target is a relation field of a value that is
	/// not an element of a collection (a copy): `REL011` replaces the
	/// mutability error there.
	pub(super) fn is_relation_copy_write(&mut self, target: NodeId) -> bool {
		let target = self.strip_groups(target);
		let View::Access {
			target: holder,
			members,
		} = self.view(target)
		else {
			return false;
		};
		let Some(member) = members.first().filter(|_| members.len() == 1) else {
			return false;
		};
		let field = self.tree.text(member.index()).to_string();
		if self.side_of(self.type_of(holder), &field).is_none() {
			return false;
		}
		match self.a.resolution(self.strip_groups(holder)) {
			Resolution::Local(symbol) => !self.is_reference_symbol(symbol),
			_ => false,
		}
	}

	// ── pairing ──────────────────────────────────────────────────────────

	/// Pairs the direct and reverse sides of the file into relations
	/// (advanced brief §8.2), imports the relations of the files it reads,
	/// resolves the recorded writes and removals, and publishes the file's
	/// relations in its interface (relations across files).
	pub(super) fn finish_relations(&mut self) {
		// the relations of the files this one reads: its values may be of
		// their types (literals, copies and printing skip their sides)
		self.import_closure_relations();
		self.check_external_collections();
		let sides = std::mem::take(&mut self.relations.sides);
		if sides.is_empty() {
			let interface = &mut self.s.source_file_mut(self.file).interface;
			interface.relations.clear();
			return;
		}
		let mut relation_of: Vec<Option<u32>> = vec![None; sides.len()];
		// a direct side whose reverse side was reported: no `REL001`
		let mut explained: Vec<bool> = vec![false; sides.len()];
		let mut relations: Vec<Paired> = Vec::new();
		let push = |relations: &mut Vec<Paired>, relation: Paired| -> u32 {
			relations.push(relation);
			(relations.len() - 1) as u32
		};
		let local =
			|kind, holder_decl: NodeId, direct, target_decl: NodeId, reverse, holder, target| {
				Paired {
					relation: Relation {
						kind,
						holder_type: TypeId::UNKNOWN,
						direct,
						target_type: TypeId::UNKNOWN,
						reverse,
						holder,
						target,
					},
					holder_decl: Some(holder_decl),
					target_decl: Some(target_decl),
				}
			};
		for (index, side) in sides.iter().enumerate() {
			let Shape::Reverse {
				field,
				many,
				inside,
			} = side.shape
			else {
				continue;
			};
			if side.imported.is_some() {
				continue;
			}
			let Some(holder) = side.collection else {
				continue;
			};
			let field_text = self.s.types.syms.resolve(field).to_string();
			let holder_text = self.collection_text(holder.reference);
			// symmetric: `spouse: (people->spouse)?`
			if field == side.field && !many && holder.element_decl == Some(side.type_decl) {
				let id = push(
					&mut relations,
					local(
						RelationKind::Symmetric,
						side.type_decl,
						field,
						side.type_decl,
						Some(field),
						holder.reference,
						holder.reference,
					),
				);
				relation_of[index] = Some(id);
				continue;
			}
			let direct = sides.iter().position(|other| {
				other.imported.is_none()
					&& Some(other.type_decl) == holder.element_decl
					&& other.field == field
					&& matches!(other.shape, Shape::Direct { .. })
			});
			let Some(direct) = direct else {
				if holder.element_decl.is_some() {
					let message = format!(
						"the elements of `{holder_text}` have no direct side `{field_text}`: a reverse side names a field `(T in c)?` or `Pool<T in c>` of the elements of its collection"
					);
					let diagnostic = self.diagnostic(&REL008, side.node, message);
					self.report(diagnostic);
				} else if matches!(holder.reference, CollectionRef::External { .. }) {
					let message = format!(
						"no type of this file declares the direct side `{field_text}`: both sides of a relation are declared in the same file, ex: `{field_text}: (T in {holder_text})?`"
					);
					let diagnostic = self.diagnostic(&REL008, side.node, message);
					self.report(diagnostic);
				}
				continue;
			};
			let direct_side = &sides[direct];
			let Shape::Direct { many: direct_many } = direct_side.shape else {
				continue;
			};
			if let Some(target) = direct_side.collection
				&& target.element_decl.is_some()
				&& target.element_decl != Some(side.type_decl)
			{
				let target_text = self.collection_text(target.reference);
				let message = format!(
					"`{field_text}` points into `{target_text}`: its reverse side is declared on the type of the elements of `{target_text}`"
				);
				let diagnostic = self.diagnostic(&REL009, side.node, message);
				self.report(diagnostic);
				explained[direct] = true;
				continue;
			}
			let kind = match (direct_many, many, inside) {
				(false, true, false) => Some(RelationKind::OneToMany),
				(false, false, _) => Some(RelationKind::OneToOne),
				(true, true, true) => Some(RelationKind::ManyToMany),
				_ => None,
			};
			let Some(kind) = kind else {
				let fix = if direct_many {
					format!("`Pool<in {holder_text}->{field_text}>`")
				} else {
					format!(
						"`Pool<{holder_text}->{field_text}>` or `({holder_text}->{field_text})?`"
					)
				};
				let message = format!(
					"this reverse side doesn't match the direct side `{field_text}`: use {fix}"
				);
				let diagnostic = self.diagnostic(&REL007, side.node, message);
				self.report(diagnostic);
				explained[direct] = true;
				continue;
			};
			if relation_of[direct].is_some() {
				let message = format!(
					"`{field_text}` already has a reverse side: a direct side has exactly one"
				);
				let diagnostic = self.diagnostic(&REL006, side.node, message);
				self.report(diagnostic);
				continue;
			}
			let target = direct_side
				.collection
				.map_or(holder.reference, |info| info.reference);
			let id = push(
				&mut relations,
				local(
					kind,
					direct_side.type_decl,
					field,
					side.type_decl,
					Some(side.field),
					holder.reference,
					target,
				),
			);
			relation_of[direct] = Some(id);
			relation_of[index] = Some(id);
		}
		for (index, side) in sides.iter().enumerate() {
			if side.imported.is_some() {
				continue;
			}
			match side.shape {
				Shape::Direct { many } if relation_of[index].is_none() && !explained[index] => {
					let Some(target) = side.collection else {
						continue;
					};
					let field = self.s.types.syms.resolve(side.field).to_string();
					let collection = self.collection_text(target.reference);
					let example = if many {
						format!("`{field}Of: Pool<in {collection}->{field}>`")
					} else {
						format!("`{field}Of: Pool<{collection}->{field}>`")
					};
					let message = format!(
						"`{field}` has no reverse side: declare one on the elements of `{collection}`, for example {example}"
					);
					let diagnostic = self.diagnostic(&REL001, side.node, message);
					self.report(diagnostic);
				}
				Shape::Owner => {
					let Some(target) = side.collection else {
						continue;
					};
					let id = push(
						&mut relations,
						local(
							RelationKind::OwnerField,
							side.type_decl,
							side.field,
							target.element_decl.unwrap_or(side.type_decl),
							None,
							target.reference,
							target.reference,
						),
					);
					relation_of[index] = Some(id);
				}
				_ => {}
			}
		}
		// the types of the sides, now that the declarations are done
		for paired in &mut relations {
			paired.relation.holder_type = self.decl_object_type(paired.holder_decl);
			paired.relation.target_type = self.decl_object_type(paired.target_decl);
		}
		// the sides of the relations of other files, after this file's own
		let count = relations.len() as u32;
		for (index, side) in sides.iter().enumerate() {
			if let Some(imported) = side.imported {
				relation_of[index] = Some(count + imported);
			}
		}
		// the tables of the backends
		for (index, side) in sides.iter().enumerate() {
			let Some(id) = relation_of[index] else {
				continue;
			};
			if side.imported.is_some() {
				continue;
			}
			let role = match side.shape {
				Shape::Owner => RelationSide::Owner,
				Shape::Reverse { field, .. }
					if field == side.field
						&& relations[id as usize].relation.kind == RelationKind::Symmetric =>
				{
					RelationSide::Direct
				}
				Shape::Reverse { .. } => RelationSide::Reverse,
				Shape::Direct { .. } => RelationSide::Direct,
			};
			self.a
				.relation_fields
				.insert((side.type_decl, side.field), (id, role));
		}
		let mut all: Vec<Relation> = relations
			.iter()
			.map(|paired| paired.relation.clone())
			.collect();
		all.extend(self.relations.imported.iter().cloned());
		for (id, relation) in all.iter().enumerate() {
			let id = id as u32;
			let direct_role = match relation.kind {
				RelationKind::OwnerField => RelationSide::Owner,
				_ => RelationSide::Direct,
			};
			self.a
				.relation_sides
				.entry((relation.holder_type, relation.direct))
				.or_insert((id, direct_role));
			if let Some(reverse) = relation.reverse
				&& relation.kind != RelationKind::Symmetric
			{
				self.a
					.relation_sides
					.entry((relation.target_type, reverse))
					.or_insert((id, RelationSide::Reverse));
			}
		}
		let writes = std::mem::take(&mut self.relations.writes);
		for (node, side) in writes {
			if let Some(Some(id)) = relation_of.get(side) {
				let entry = self.a.relation_writes.entry(node).or_default();
				if !entry.contains(id) {
					entry.push(*id);
				}
			}
		}
		let unlinks = std::mem::take(&mut self.relations.unlinks);
		for (node, collection) in unlinks {
			let ids: Vec<u32> = all
				.iter()
				.enumerate()
				.filter(|(_, relation)| {
					relation.holder == collection || relation.target == collection
				})
				.map(|(id, _)| id as u32)
				.collect();
			if !ids.is_empty() {
				self.a.relation_unlinks.insert(node, ids);
			}
		}
		// what the other files see of the file's own relations
		let published: Vec<Relation> = relations
			.iter()
			.map(|paired| {
				let mut relation = paired.relation.clone();
				relation.holder = self.published_collection(relation.holder);
				relation.target = self.published_collection(relation.target);
				relation
			})
			.collect();
		self.s.source_file_mut(self.file).interface.relations = published;
		self.a.relations = all;
		self.relations.sides = sides;
	}

	/// The object type a type declaration of this file declares.
	fn decl_object_type(&self, decl: Option<NodeId>) -> TypeId {
		let Some(ty) = decl
			.and_then(|decl| self.a.type_decls.get(&decl))
			.map(|decl| decl.ty)
		else {
			return TypeId::UNKNOWN;
		};
		match self.s.types.kind(ty) {
			TypeKind::Rec(_) => self.s.types.unfold(ty),
			_ => ty,
		}
	}

	/// A collection as other files name it: by key, or by owner type.
	fn published_collection(&mut self, reference: CollectionRef) -> CollectionRef {
		match reference {
			CollectionRef::Field { owner, name } => CollectionRef::ExternalField {
				owner: self.decl_object_type(Some(owner)),
				name,
			},
			CollectionRef::Module { declaration } => match self.node_key(declaration) {
				Some(key) => CollectionRef::External { key },
				None => reference,
			},
			other => other,
		}
	}

	/// The name of a collection for messages.
	fn collection_text(&self, reference: CollectionRef) -> String {
		match reference {
			CollectionRef::Field { name, .. } | CollectionRef::ExternalField { name, .. } => {
				self.s.types.syms.resolve(name).to_string()
			}
			CollectionRef::Module { declaration } => match self.view(declaration) {
				View::Declaration { name, .. } => self.tree.name(name).to_string(),
				_ => "the collection".to_string(),
			},
			CollectionRef::External { key } => {
				let names = self.s.types.decl_key_data(key).names;
				self.s.types.sym_list(names).last().map_or_else(
					|| "the collection".to_string(),
					|&name| self.s.types.syms.resolve(name).to_string(),
				)
			}
		}
	}

	// ── relations across files ───────────────────────────────────────────

	/// Imports the relations of the files this one may read that declare
	/// some (their scan says so), in the order of their paths.
	fn import_closure_relations(&mut self) {
		if self.s.source_file(self.file).scan.dependencies.is_empty() {
			return;
		}
		if self.modules.closure.is_none() {
			self.modules.closure = Some(self.s.dependency_closure(self.file));
		}
		let own = self.s.module_id(self.file);
		let mut modules: Vec<(String, ModuleId)> = self
			.modules
			.closure
			.iter()
			.flatten()
			.copied()
			.filter(|&module| module != own)
			.map(|module| (self.s.types.module_path(module).to_string(), module))
			.filter(|(path, _)| self.s.scan_of(path).is_some_and(|scan| scan.has_relations))
			.collect();
		modules.sort();
		for (_, module) in modules {
			self.import_module_relations(module);
		}
	}

	/// Imports the relations of the files that declared the object type
	/// `ty`, once per type.
	fn import_relations_for(&mut self, ty: TypeId) {
		if !matches!(self.s.types.kind(ty), TypeKind::Object(_))
			|| !self.relations.imported_types.insert(ty)
		{
			return;
		}
		let own = self.s.module_id(self.file);
		let modules: Vec<ModuleId> = self
			.s
			.type_origins(ty, |module| module != own)
			.map(|key| self.s.types.decl_key_data(key).module)
			.collect();
		for module in modules {
			self.import_module_relations(module);
		}
	}

	/// Imports the relations another file published. A file of a cycle
	/// whose walk is in progress hasn't published them yet: this file is
	/// then walked again once it is done.
	fn import_module_relations(&mut self, module: ModuleId) {
		if self.relations.imported_modules.contains(&module) {
			return;
		}
		let Some(interface) = self.s.interface(module) else {
			return;
		};
		let complete = interface.complete;
		let relations = interface.relations.clone();
		if !complete {
			let path = self.s.types.module_path(module).to_string();
			if self.s.scan_of(&path).is_some_and(|scan| scan.has_relations) {
				self.modules.deferred = true;
			}
			return;
		}
		self.relations.imported_modules.insert(module);
		for relation in relations {
			self.add_imported_relation(relation);
		}
	}

	/// Registers a relation of another file: its collections as this file
	/// names them, and its sides by type.
	fn add_imported_relation(&mut self, relation: Relation) {
		let mut relation = relation;
		relation.holder = self.localized_collection(relation.holder);
		relation.target = self.localized_collection(relation.target);
		let id = self.relations.imported.len() as u32;
		let direct = relation.direct;
		let (direct_shape, reverse_shape) = match relation.kind {
			RelationKind::OneToMany => (
				Shape::Direct { many: false },
				Some(Shape::Reverse {
					field: direct,
					many: true,
					inside: false,
				}),
			),
			RelationKind::OneToOne => (
				Shape::Direct { many: false },
				Some(Shape::Reverse {
					field: direct,
					many: false,
					inside: false,
				}),
			),
			RelationKind::ManyToMany => (
				Shape::Direct { many: true },
				Some(Shape::Reverse {
					field: direct,
					many: true,
					inside: true,
				}),
			),
			RelationKind::Symmetric => (
				Shape::Reverse {
					field: direct,
					many: false,
					inside: false,
				},
				None,
			),
			RelationKind::OwnerField => (Shape::Owner, None),
		};
		self.push_imported_side(relation.holder_type, direct, direct_shape, id);
		if let (Some(reverse), Some(shape)) = (relation.reverse, reverse_shape) {
			self.push_imported_side(relation.target_type, reverse, shape, id);
		}
		self.relations.imported.push(relation);
	}

	/// A side of a relation of another file, found by type.
	fn push_imported_side(&mut self, ty: TypeId, field: Sym, shape: Shape, id: u32) {
		if self.relations.by_type.contains_key(&(ty, field)) {
			return;
		}
		let index = self.relations.sides.len();
		self.relations.sides.push(SideDecl {
			node: NodeId(u32::MAX),
			type_decl: NodeId(u32::MAX),
			field,
			shape,
			collection: None,
			imported: Some(id),
		});
		self.relations.by_type.insert((ty, field), index);
	}

	/// A collection of a published relation as this file names it: its own
	/// module-level value by its declaration.
	fn localized_collection(&mut self, reference: CollectionRef) -> CollectionRef {
		let CollectionRef::External { key } = reference else {
			return reference;
		};
		if self.s.types.decl_key_data(key).module != self.s.module_id(self.file) {
			return reference;
		}
		if let Some(&symbol) = self.modules.keyed.get(&key) {
			return CollectionRef::Module {
				declaration: self.a.symbols.symbol(symbol).decl_node,
			};
		}
		if let Some((node, export)) = self.modules.export
			&& self.symbol_key(export) == key
		{
			return CollectionRef::Module { declaration: node };
		}
		reference
	}

	/// The module-level collections of this file that relations of the
	/// files it reads name (they read this file for the type of their
	/// elements): a `Pool`, a `Columnar` or their growing variants
	/// (`REL003`), of the type that declares the sides (`REL004`).
	fn check_external_collections(&mut self) {
		let own: Vec<Relation> = self
			.relations
			.imported
			.iter()
			.filter(|relation| {
				matches!(relation.holder, CollectionRef::Module { .. })
					|| matches!(relation.target, CollectionRef::Module { .. })
			})
			.cloned()
			.collect();
		for relation in own {
			for (reference, elements) in [
				(relation.holder, relation.holder_type),
				(relation.target, relation.target_type),
			] {
				let CollectionRef::Module { declaration } = reference else {
					continue;
				};
				let Some(symbol) = self.a.symbols.symbol_of_decl(declaration) else {
					continue;
				};
				let ty = self.a.symbols.symbol(symbol).ty;
				let TypeKind::Collection(kind, item, _) = self.s.types.kind(ty) else {
					continue;
				};
				let name = self
					.s
					.types
					.syms
					.resolve(self.a.symbols.symbol(symbol).name)
					.to_string();
				if !has_stable_addresses(kind) {
					if self.reported_once.insert((declaration, REL003.id)) {
						let message = format!(
							"`{name}` is a `{}`, whose elements move: a relation of another file links them, which needs a `Pool`, a `Columnar`, a `GrowingPool` or a `GrowingColumnar`",
							kind.name()
						);
						let diagnostic = self.diagnostic(&REL003, declaration, message);
						self.report(diagnostic);
					}
					continue;
				}
				let item = match self.s.types.kind(item) {
					TypeKind::Rec(_) => self.s.types.unfold(item),
					_ => item,
				};
				if item != elements
					&& item != TypeId::UNKNOWN
					&& elements != TypeId::UNKNOWN
					&& self.reported_once.insert((declaration, REL004.id))
				{
					let message = format!(
						"a relation of another file links the elements of `{name}` as `{}`, but they are `{}`",
						self.s.types.display(elements),
						self.s.types.display(item)
					);
					let diagnostic = self.diagnostic(&REL004, declaration, message);
					self.report(diagnostic);
				}
			}
		}
	}

	// ── reads and writes ─────────────────────────────────────────────────

	/// Whether `field` of a value of type `ty` is a reverse relation side.
	pub(crate) fn is_reverse_side(&mut self, ty: TypeId, field: &str) -> bool {
		self.side_of(ty, field)
			.is_some_and(|side| matches!(self.relations.sides[side].shape, Shape::Reverse { .. }))
	}

	/// Whether `field` of a value of type `ty` is a relation side (written
	/// by the writes of its other side too: never narrowed).
	pub(crate) fn is_relation_side(&mut self, ty: TypeId, field: &str) -> bool {
		self.side_of(ty, field).is_some()
	}

	/// The relation field `field` of a value of type `ty`, if any: declared
	/// in this file, or by the file that declared the type.
	fn side_of(&mut self, ty: TypeId, field: &str) -> Option<usize> {
		let ty = match self.s.types.kind(ty) {
			TypeKind::Rec(_) => self.s.types.unfold(ty),
			_ => ty,
		};
		let sym = self.s.types.syms.get(field)?;
		if let Some(&side) = self.relations.by_type.get(&(ty, sym)) {
			return Some(side);
		}
		self.import_relations_for(ty);
		self.relations.by_type.get(&(ty, sym)).copied()
	}

	/// Whether a symbol designates an element of a collection: a `use`, a
	/// loop variable over a collection, a parameter typed `T in c`.
	pub(crate) fn is_reference_symbol(&self, symbol: SymbolId) -> bool {
		let entry = self.a.symbols.symbol(symbol);
		match entry.kind {
			SymbolKind::Use | SymbolKind::MutableUse | SymbolKind::LoopVariable { copy: false } => {
				true
			}
			SymbolKind::Parameter { .. } => match self.view(entry.decl_node) {
				View::Assignment {
					annotation: Some(annotation),
					..
				} => self.contains_reference_type(annotation),
				_ => false,
			},
			_ => false,
		}
	}

	/// Whether a type expression contains `T in c`.
	pub(crate) fn contains_reference_type(&self, n: NodeId) -> bool {
		let mut stack = vec![n];
		while let Some(node) = stack.pop() {
			if matches!(self.view(node), View::RefType { .. }) {
				return true;
			}
			stack.extend(syntax::children(&self.tree, node));
		}
		false
	}

	/// `x.f = t` (and `x = literal`): the checks of a relation write
	/// (advanced brief §8.4). Called after the assignment is checked.
	pub(super) fn check_relation_write(&mut self, n: NodeId, target: NodeId, value: NodeId) {
		let target = self.strip_groups(target);
		match self.view(target) {
			View::Access {
				target: holder,
				members,
			} if members.len() == 1 => {
				let Some(member) = members.first() else {
					return;
				};
				let field = self.tree.text(member.index()).to_string();
				let Some(side) = self.side_of(self.type_of(holder), &field) else {
					return;
				};
				self.check_side_write(n, holder, side, Some(value));
			}
			// a whole-value assignment keeps the links (D20)
			View::Name(_) => {
				let Resolution::Local(symbol) = self.a.resolution(target) else {
					return;
				};
				if !self.is_reference_symbol(symbol) {
					return;
				}
				let value = self.strip_groups(value);
				let items = match self.view(value) {
					View::Object { items } => items.iter().collect::<Vec<NodeId>>(),
					View::TypedObject { members, .. } => members.iter().collect(),
					_ => return,
				};
				let ty = self.type_of(value);
				for item in items {
					let View::Assignment { target: name, .. } = self.view(item) else {
						continue;
					};
					let text = self.tree.text(name.index()).to_string();
					if self.side_of(ty, &text).is_some()
						&& self.reported_once.insert((item, REL021.id))
					{
						let message = format!(
							"a whole-value assignment keeps the links: write `{}.{text} = …` separately",
							self.tree.text(target.index())
						);
						let diagnostic = self.diagnostic(&REL021, item, message);
						self.report(diagnostic);
					}
				}
			}
			_ => {}
		}
	}

	/// A write of the relation field `side` of `holder` (an assignment, or a
	/// call of `add`/`delete`/`clear` on a many-to-many side): only a direct
	/// side, of an element of the collection, to a target of the same owner.
	fn check_side_write(&mut self, n: NodeId, holder: NodeId, side: usize, value: Option<NodeId>) {
		let decl = self.relations.sides[side].clone();
		let field = self.s.types.syms.resolve(decl.field).to_string();
		match decl.shape {
			Shape::Reverse { field: direct, .. } if direct != decl.field => {
				if self.reported_once.insert((n, REL010.id)) {
					let direct = self.s.types.syms.resolve(direct).to_string();
					let message = format!(
						"`{field}` is the reverse side of `{direct}` and is read-only: write the `{direct}` of the other element"
					);
					let diagnostic = self.diagnostic(&REL010, n, message);
					self.report(diagnostic);
				}
				return;
			}
			Shape::Owner => {
				self.relations.writes.push((n, side));
				return;
			}
			_ => {}
		}
		// the holder: an element of the collection (REL011)
		let holder_node = self.strip_groups(holder);
		let active = match self.view(holder_node) {
			View::Name(_) => match self.a.resolution(holder_node) {
				Resolution::Local(symbol) => {
					self.is_reference_symbol(symbol)
						|| self.a.symbols.symbol(symbol).kind == SymbolKind::SelfParameter
				}
				_ => true,
			},
			_ => true,
		};
		if !active {
			if self.reported_once.insert((n, REL011.id)) {
				let holder_text = self.tree.text(holder_node.index()).to_string();
				let message = format!(
					"`{holder_text}` is a copy: its relation fields are `none` and read-only. Write `{field}` through a reference into the collection (`use {holder_text} = …`)"
				);
				let diagnostic = self.diagnostic(&REL011, n, message);
				self.report(diagnostic);
			}
			return;
		}
		// the target: an element of the same owner (REL012)
		if let Some(value) = value
			&& let Some(from) = self.place_of(holder_node)
			&& let Some(to) = self.place_of(value)
			&& let (Some(from), Some(to)) = (self.collection_owner(from), self.collection_owner(to))
			&& from != to
			&& self.reported_once.insert((n, REL012.id))
		{
			let message = format!(
				"`{}` is an element of `{}`, `{}` of `{}`: a relation links elements of the same owner",
				self.tree.text(value.index()),
				self.place_text(to),
				self.tree.text(holder_node.index()),
				self.place_text(from)
			);
			let diagnostic = self.diagnostic(&REL012, n, message);
			self.report(diagnostic);
		}
		self.relations.writes.push((n, side));
		// the other sides it writes (§8.6)
		if let Some(place) = self.place_of(holder_node) {
			let cause = format!("writing `{}.{field}`", self.tree.text(holder_node.index()));
			for implied in self.implied_write(place, decl.field) {
				self.record_implied_effect(implied, n, &cause);
			}
		}
	}

	/// The owner of the collection an element place belongs to (`town1` for
	/// an element of `town1.people`); `None` for a module-level collection.
	fn collection_owner(&self, place: crate::ids::PlaceId) -> Option<crate::ids::PlaceId> {
		let mut current = place;
		loop {
			let entry = self.places.get(current);
			let parent = entry.parent?;
			if matches!(
				entry.step,
				Some(crate::flow::Step::Index(_) | crate::flow::Step::All)
			) {
				// a target reached through a relation pool (`ana.friends`):
				// an element of the collection of the same owner
				if self.relation_pool_places.contains(&parent)
					&& let Some(holder) = self.places.parent(parent)
				{
					return self.collection_owner(holder);
				}
				return self.places.get(parent).parent;
			}
			current = parent;
		}
	}

	/// The calls of collection methods that touch relations: a removal from
	/// a related collection (`relation_unlinks`), `add` of a literal setting
	/// sides, `add`/`delete`/`clear` of a many-to-many side. Flow part.
	pub(super) fn check_relation_call(
		&mut self,
		n: NodeId,
		path: &str,
		callee: NodeId,
		arguments: &[crate::analysis::Argument],
	) {
		let (owner, method) = path.split_once('#').unwrap_or((path, ""));
		// the collection: the callee without its method (`town.people`)
		let base = match self.view(self.strip_groups(callee)) {
			View::Access { target, members } => {
				let members: Vec<NodeId> = members.iter().collect();
				self.access_place(target, &members[..members.len().saturating_sub(1)])
			}
			_ => None,
		};
		let Some(base) = base else {
			return;
		};
		match (owner, method) {
			(
				"Pool" | "Columnar" | "OrderedColumnar",
				"delete" | "take" | "deleteWhere" | "clear",
			) => {
				if let Some(collection) = self.callee_collection_ref(callee) {
					self.relations.unlinks.push((n, collection));
				}
				// the sides that point at the removed elements (§8.6)
				let cause = match arguments.first() {
					Some(crate::analysis::Argument::Given(element))
						if matches!(method, "delete" | "take") =>
					{
						format!("deleting `{}`", self.tree.text(element.index()))
					}
					_ => format!("`{method}`"),
				};
				for implied in self.implied_delete(base) {
					self.record_implied_effect(implied, n, &cause);
				}
			}
			("Pool" | "Columnar" | "OrderedColumnar", "add") => {
				let Some(crate::analysis::Argument::Given(value)) = arguments.first() else {
					return;
				};
				let value = self.strip_groups(*value);
				let items = match self.view(value) {
					View::Object { items } => items.iter().collect::<Vec<NodeId>>(),
					View::TypedObject { members, .. } => members.iter().collect(),
					_ => return,
				};
				let ty = self.type_of(value);
				for item in items {
					let View::Assignment { target: name, .. } = self.view(item) else {
						continue;
					};
					let text = self.tree.text(name.index()).to_string();
					let Some(side) = self.side_of(ty, &text) else {
						continue;
					};
					match self.relations.sides[side].shape {
						Shape::Reverse { field, .. }
							if field != self.relations.sides[side].field =>
						{
							if self.reported_once.insert((item, REL010.id)) {
								let direct = self.s.types.syms.resolve(field).to_string();
								let message = format!(
									"`{text}` is the reverse side of `{direct}`: it can't be set; set `{direct}` of the other element"
								);
								let diagnostic = self.diagnostic(&REL010, item, message);
								self.report(diagnostic);
							}
						}
						_ => {
							self.relations.writes.push((n, side));
							// the sides the new element links (§8.6)
							let field = self.relations.sides[side].field;
							let element = self.places.child(
								base,
								crate::flow::Step::Index(crate::flow::Identity::Unknown),
							);
							let cause = format!("setting `{text}`");
							for implied in self.implied_write(element, field) {
								self.record_implied_effect(implied, n, &cause);
							}
						}
					}
				}
			}
			("RelationPool", "add" | "delete" | "clear") => {
				let callee = self.strip_groups(callee);
				if let View::Access {
					target: holder,
					members,
				} = self.view(callee)
					&& members.len() == 2
					&& let Some(member) = members.first()
				{
					let field = self.tree.text(member.index()).to_string();
					if let Some(side) = self.side_of(self.type_of(holder), &field) {
						let value = match arguments.first() {
							Some(crate::analysis::Argument::Given(value)) => Some(*value),
							_ => None,
						};
						self.check_side_write(n, holder, side, value);
					}
				}
			}
			_ => {}
		}
	}

	/// The collection the receiver of a method call names (`town.people` in
	/// `town.people.delete(…)`).
	fn callee_collection_ref(&mut self, callee: NodeId) -> Option<CollectionRef> {
		let callee = self.strip_groups(callee);
		let View::Access { target, members } = self.view(callee) else {
			return None;
		};
		let members: Vec<NodeId> = members.iter().collect();
		match members.len() {
			0 => None,
			1 => self.collection_ref(target),
			count => self.member_collection_ref(target, &members[..count - 1]),
		}
	}

	/// The collection a receiver names: a field of an owner type, or a
	/// module-level value (of this file or of another one).
	fn collection_ref(&mut self, receiver: NodeId) -> Option<CollectionRef> {
		let receiver = self.strip_groups(receiver);
		match self.view(receiver) {
			View::Access { target, members } => {
				let members: Vec<NodeId> = members.iter().collect();
				self.member_collection_ref(target, &members)
			}
			View::Name(_) => {
				let Resolution::Local(symbol) = self.a.resolution(receiver) else {
					return None;
				};
				if self.is_proxy(symbol) {
					return self.external_collection_ref(symbol);
				}
				let entry = self.a.symbols.symbol(symbol);
				matches!(
					entry.kind,
					SymbolKind::Let | SymbolKind::Mutable | SymbolKind::Reactive
				)
				.then_some(CollectionRef::Module {
					declaration: entry.decl_node,
				})
			}
			_ => None,
		}
	}

	/// The collection `target.members…` names (its last member): a
	/// module-level value of another file reached through a namespace
	/// (`library.people`), or a field of an owner type.
	fn member_collection_ref(
		&mut self,
		target: NodeId,
		members: &[NodeId],
	) -> Option<CollectionRef> {
		let (&member, before) = members.split_last()?;
		if let Resolution::Local(symbol) = self.a.resolution(member)
			&& self.is_proxy(symbol)
		{
			return self.external_collection_ref(symbol);
		}
		let owner_ty = match before.last() {
			Some(&previous) => self.type_of(previous),
			None => self.type_of(target),
		};
		let name = self.s.types.syms.intern(self.tree.text(member.index()));
		Some(self.field_collection_ref(owner_ty, name))
	}

	/// The field `name` of a value of type `owner` as a collection: of a
	/// type of this file by its declaration, else by type.
	fn field_collection_ref(&mut self, owner: TypeId, name: Sym) -> CollectionRef {
		if let Some(&decl) = self
			.decls_by_type
			.get(&owner)
			.and_then(|decls| decls.first())
		{
			return CollectionRef::Field { owner: decl, name };
		}
		let owner = match self.s.types.kind(owner) {
			TypeKind::Rec(_) => self.s.types.unfold(owner),
			_ => owner,
		};
		CollectionRef::ExternalField { owner, name }
	}

	/// A module-level value of another file a proxy stands for, as a
	/// collection.
	fn external_collection_ref(&mut self, symbol: SymbolId) -> Option<CollectionRef> {
		if !matches!(
			self.a.symbols.symbol(symbol).kind,
			SymbolKind::Let | SymbolKind::Mutable | SymbolKind::Reactive
		) {
			return None;
		}
		let key = self.external_value_key(symbol)?;
		self.relations
			.external_collections
			.entry(key)
			.or_insert(symbol);
		Some(CollectionRef::External { key })
	}

	/// `REL016` (D48): a collection that owner fields point into can't be
	/// aliased with `use`, nor given to a parameter a user function mutates:
	/// its owner must be known where it changes.
	pub(super) fn check_owned_collection_alias(&mut self, value: NodeId) {
		let Some(collection) = self.collection_ref(value) else {
			return;
		};
		let owned = self.relations.sides.iter().any(|side| {
			side.shape == Shape::Owner
				&& side
					.collection
					.is_some_and(|info| info.reference == collection)
		});
		if owned && self.reported_once.insert((value, REL016.id)) {
			let text = self.tree.text(value.index()).to_string();
			let message = format!(
				"owner fields point into `{text}`: change it only through its full path (`{text}.delete(…)`), so that deleting clears them"
			);
			let diagnostic = self.diagnostic(&REL016, value, message);
			self.report(diagnostic);
		}
	}

	/// `REF010` (D17): a local or module-level value can't have a reference
	/// type: only relation fields store references.
	pub(super) fn check_reference_annotation(&mut self, annotation: NodeId) {
		if self.contains_reference_type(annotation)
			&& self.reported_once.insert((annotation, REF010.id))
		{
			let diagnostic = self.diagnostic(
				&REF010,
				annotation,
				"only the fields of a relation store references: declare it with `use`, or keep an index",
			);
			self.report(diagnostic);
		}
	}

	// ── relation pools ────────────────────────────────────────────────────

	/// The members of the pool of a relation side (`ana.friends`,
	/// `bao.bestFriendOf`): links to targets. `add(target)` and
	/// `delete(target)` return whether something changed; the reverse side
	/// (`RelationPool(1)`) is read-only (`REL010`).
	pub(super) fn relation_pool_member(
		&mut self,
		member: NodeId,
		receiver: TypeId,
		reverse: bool,
		item: TypeId,
		text: &str,
	) -> Option<(TypeId, MemberKind)> {
		if reverse && matches!(text, "add" | "delete" | "clear") {
			let message = format!(
				"this pool is the reverse side of a relation and is read-only: `{text}` the targets through the direct side of the other element"
			);
			let diagnostic = self.diagnostic(&REL010, member, message);
			self.report(diagnostic);
			return None;
		}
		let natural = {
			let storage = crate::types::IntStorage::Default;
			self.s
				.types
				.int_range(ethnum::I256::ZERO, storage.range().hi, storage)
		};
		let predicate = self.callback_with(&[item], TypeId::BOOLEAN, 0);
		let optional_item = self.s.types.optional(item);
		let ty = match text {
			"size" => natural,
			"isEmpty" => self.method(&[], TypeId::BOOLEAN),
			"add" | "delete" | "includes" => self.method(&[item], TypeId::BOOLEAN),
			"clear" => self.method(&[], TypeId::VOID),
			"find" => self.method(&[predicate], optional_item),
			"every" | "some" => self.method(&[predicate], TypeId::BOOLEAN),
			"map" => {
				let result = self.builtin_param("Pool.map", "Result", TypeId::UNKNOWN);
				let transform = self.callback_with(&[item], result, 0);
				let mapped = self.s.types.array(result);
				self.method(&[transform], mapped)
			}
			"reduce" => {
				let accumulator = self.builtin_param("Pool.reduce", "Accumulator", TypeId::UNKNOWN);
				let combine = self.callback_with(&[accumulator, item], accumulator, 0);
				self.method(&[combine, accumulator], accumulator)
			}
			"values" => self.values_method(item),
			_ => {
				self.no_member(member, receiver, text);
				return None;
			}
		};
		let builtin = crate::builtins::by_path(&format!("RelationPool#{text}"))?;
		self.set_resolution(member, Resolution::Builtin(builtin));
		let ty = if text == "size" {
			ty
		} else {
			self.with_effects(ty, CollKind::RelationPool(u16::from(reverse)), text)
		};
		Some((ty, MemberKind::Builtin))
	}

	// ── implied effects (§8.6, D47) ──────────────────────────────────────

	/// The relations as paired so far, without diagnostics
	/// ([`Walker::finish_relations`] pairs them for good at the end), then
	/// the relations of other files.
	fn paired_relations(&self) -> Vec<Paired> {
		let sides = &self.relations.sides;
		let mut relations = Vec::new();
		let local =
			|kind, holder_decl: NodeId, direct, target_decl: NodeId, reverse, holder, target| {
				Paired {
					relation: Relation {
						kind,
						holder_type: TypeId::UNKNOWN,
						direct,
						target_type: TypeId::UNKNOWN,
						reverse,
						holder,
						target,
					},
					holder_decl: Some(holder_decl),
					target_decl: Some(target_decl),
				}
			};
		for side in sides {
			if side.imported.is_some() {
				continue;
			}
			match side.shape {
				Shape::Reverse {
					field,
					many,
					inside,
				} => {
					let Some(holder) = side.collection else {
						continue;
					};
					if field == side.field && !many && holder.element_decl == Some(side.type_decl) {
						relations.push(local(
							RelationKind::Symmetric,
							side.type_decl,
							field,
							side.type_decl,
							Some(field),
							holder.reference,
							holder.reference,
						));
						continue;
					}
					let Some(direct) = sides.iter().find(|other| {
						other.imported.is_none()
							&& Some(other.type_decl) == holder.element_decl
							&& other.field == field
							&& matches!(other.shape, Shape::Direct { .. })
					}) else {
						continue;
					};
					let Shape::Direct { many: direct_many } = direct.shape else {
						continue;
					};
					let kind = match (direct_many, many, inside) {
						(false, true, false) => RelationKind::OneToMany,
						(false, false, _) => RelationKind::OneToOne,
						(true, true, true) => RelationKind::ManyToMany,
						_ => continue,
					};
					relations.push(local(
						kind,
						direct.type_decl,
						field,
						side.type_decl,
						Some(side.field),
						holder.reference,
						direct
							.collection
							.map_or(holder.reference, |info| info.reference),
					));
				}
				Shape::Owner => {
					let Some(target) = side.collection else {
						continue;
					};
					relations.push(local(
						RelationKind::OwnerField,
						side.type_decl,
						side.field,
						target.element_decl.unwrap_or(side.type_decl),
						None,
						target.reference,
						target.reference,
					));
				}
				Shape::Direct { .. } => {}
			}
		}
		relations.extend(self.relations.imported.iter().map(|relation| Paired {
			relation: relation.clone(),
			holder_decl: None,
			target_decl: None,
		}));
		relations
	}

	/// The relation whose direct side is the side `side`.
	fn relation_of_direct_side(&self, side: usize) -> Option<Relation> {
		let decl = &self.relations.sides[side];
		if let Some(imported) = decl.imported {
			return self.relations.imported.get(imported as usize).cloned();
		}
		self.paired_relations()
			.into_iter()
			.find(|paired| {
				paired.holder_decl == Some(decl.type_decl) && paired.relation.direct == decl.field
			})
			.map(|paired| paired.relation)
	}

	/// The place of a relation's collection for the owner `owner` (`None`:
	/// not known, or module-level).
	fn relation_collection_place(
		&mut self,
		collection: CollectionRef,
		owner: Option<crate::ids::PlaceId>,
	) -> Option<crate::ids::PlaceId> {
		match collection {
			CollectionRef::Module { declaration } => {
				let symbol = self.a.symbols.symbol_of_decl(declaration)?;
				Some(self.places.symbol(symbol))
			}
			CollectionRef::Field { name, .. } | CollectionRef::ExternalField { name, .. } => {
				Some(self.places.child(owner?, crate::flow::Step::Field(name)))
			}
			// a module-level value of another file: its proxy here
			CollectionRef::External { key } => {
				let symbol = *self.relations.external_collections.get(&key)?;
				Some(self.places.symbol(symbol))
			}
		}
	}

	/// `C[..].f`: the field `f` of every element of `collection`.
	fn every_element_field(
		&mut self,
		collection: Option<crate::ids::PlaceId>,
		field: Option<Sym>,
		out: &mut Vec<crate::ids::PlaceId>,
	) {
		let (Some(collection), Some(field)) = (collection, field) else {
			return;
		};
		let all = self.places.child(collection, crate::flow::Step::All);
		let place = self.places.child(all, crate::flow::Step::Field(field));
		if !out.contains(&place) {
			out.push(place);
		}
	}

	/// The implied effects of writing the relation side `field` of the
	/// element at `holder` (§8.6): the reverse sides of the targets, and for
	/// a one-to-one relation the direct sides of the previous holder.
	pub(crate) fn implied_write(
		&mut self,
		holder: crate::ids::PlaceId,
		field: Sym,
	) -> Vec<crate::ids::PlaceId> {
		let mut out = Vec::new();
		let Some(ty) = self.place_type(holder) else {
			return out;
		};
		let ty = match self.s.types.kind(ty) {
			TypeKind::Rec(_) => self.s.types.unfold(ty),
			_ => ty,
		};
		let field_text = self.s.types.syms.resolve(field).to_string();
		let Some(side) = self.side_of(ty, &field_text) else {
			return out;
		};
		let Some(relation) = self.relation_of_direct_side(side) else {
			return out;
		};
		let owner = self.collection_owner(holder);
		let targets = self.relation_collection_place(relation.target, owner);
		match relation.kind {
			RelationKind::OneToMany | RelationKind::ManyToMany => {
				self.every_element_field(targets, relation.reverse, &mut out);
			}
			RelationKind::OneToOne => {
				self.every_element_field(targets, relation.reverse, &mut out);
				let holders = self.relation_collection_place(relation.holder, owner);
				self.every_element_field(holders, Some(relation.direct), &mut out);
			}
			RelationKind::Symmetric => {
				self.every_element_field(targets, Some(relation.direct), &mut out);
			}
			RelationKind::OwnerField => {}
		}
		out
	}

	/// The implied effects of removing elements from the collection at
	/// `collection` (`delete`, `take`, `clear`, any `mutates` of it, §8.6):
	/// the direct sides pointing into it, the reverse sides its elements are
	/// in, the owner fields into it. On the owner of the collection (D47).
	pub(crate) fn implied_delete(
		&mut self,
		collection: crate::ids::PlaceId,
	) -> Vec<crate::ids::PlaceId> {
		let mut out = Vec::new();
		if self.relations.sides.is_empty() {
			return out;
		}
		let reference = self.collection_ref_of_place(collection);
		let Some(reference) = reference else {
			return out;
		};
		let owner = self.places.parent(collection);
		for paired in self.paired_relations() {
			let relation = paired.relation;
			if relation.kind == RelationKind::OwnerField {
				if relation.holder == reference
					&& let Some(owner) = owner
				{
					let place = self
						.places
						.child(owner, crate::flow::Step::Field(relation.direct));
					if !out.contains(&place) {
						out.push(place);
					}
				}
				continue;
			}
			if relation.target == reference {
				let holders = self.relation_collection_place(relation.holder, owner);
				self.every_element_field(holders, Some(relation.direct), &mut out);
			}
			if relation.holder == reference {
				let targets = self.relation_collection_place(relation.target, owner);
				self.every_element_field(targets, relation.reverse, &mut out);
			}
		}
		out
	}

	/// The relation collection a place is: a field of an owner type, or a
	/// module-level value.
	fn collection_ref_of_place(&mut self, place: crate::ids::PlaceId) -> Option<CollectionRef> {
		let entry = self.places.get(place);
		match (entry.parent, entry.step) {
			(Some(parent), Some(crate::flow::Step::Field(name))) => {
				let ty = self.place_type(parent)?;
				Some(self.field_collection_ref(ty, name))
			}
			(None, _) => {
				let symbol = self.place_symbol(place)?;
				if self.is_proxy(symbol) {
					return self.external_collection_ref(symbol);
				}
				let entry = self.a.symbols.symbol(symbol);
				matches!(
					entry.kind,
					SymbolKind::Let | SymbolKind::Mutable | SymbolKind::Reactive
				)
				.then_some(CollectionRef::Module {
					declaration: entry.decl_node,
				})
			}
			_ => None,
		}
	}

	/// The implied effects of the places a call changes (its declared
	/// effects, substituted): writes of relation sides, removals from
	/// related collections (D47: `mutates users` of a collection received
	/// alone is a change of `owner(users)`).
	pub(crate) fn implied_by_changes(
		&mut self,
		changed: &[(crate::ids::PlaceId, bool, Option<NodeId>)],
	) -> Vec<crate::ids::PlaceId> {
		let mut out = Vec::new();
		if self.relations.sides.is_empty() {
			return out;
		}
		for &(place, invalidates, _) in changed {
			let entry = self.places.get(place);
			let found = match (entry.parent, entry.step) {
				(Some(holder), Some(crate::flow::Step::Field(field))) => {
					let mut found = self.implied_write(holder, field);
					found.extend(self.implied_delete(place));
					found
				}
				(Some(collection), Some(crate::flow::Step::Index(_) | crate::flow::Step::All))
					if invalidates =>
				{
					self.implied_delete(collection)
				}
				_ => self.implied_delete(place),
			};
			for place in found {
				if !out.contains(&place) {
					out.push(place);
				}
			}
		}
		out
	}

	/// The alias rule for implied effects (§5.9): an implied `C[..].f` may
	/// overlap an argument that is an element of `C` (`befriend(ana, bao)`),
	/// not one that reads the sides it writes.
	pub(crate) fn check_implied_aliasing(
		&mut self,
		n: NodeId,
		callee: &str,
		implied: &[crate::ids::PlaceId],
		used: &[NodeId],
	) {
		for &place in implied {
			let collection = self
				.places
				.parent(place)
				.and_then(|all| self.places.parent(all));
			for &argument in used {
				let Some(other) = self.place_of(argument) else {
					continue;
				};
				let element = self.places.parent(other) == collection
					&& matches!(
						self.places.get(other).step,
						Some(crate::flow::Step::Index(_) | crate::flow::Step::All)
					);
				if element
					|| matches!(
						self.places.get(other).step,
						Some(crate::flow::Step::Length | crate::flow::Step::Size)
					) || !self.may_overlap(place, other)
				{
					continue;
				}
				let message = format!(
					"`{callee}` changes `{}` (the other side of a relation), which its argument `{}` reads: copy it first with `let`",
					self.place_text(place),
					self.tree.text(argument.index())
				);
				let diagnostic =
					self.diagnostic(&crate::diagnostics::codes::advanced::REF013, n, message);
				self.report_flow(diagnostic);
				return;
			}
		}
	}
}
