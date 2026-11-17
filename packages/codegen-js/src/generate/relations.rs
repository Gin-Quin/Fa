//! Relations (advanced brief §8, `relations.md`; lowering §8.8): the sides
//! are plain properties of the elements (pools of objects are `Set`s, a
//! reference is the object).
//!
//! - A direct side `(T in c)?`, a reverse one-to-one side, a symmetric side
//!   and an owner field hold the object, or `undefined`; a many-to-many
//!   direct side and the `Pool` reverse sides hold a `Set` of objects
//!   (`new Set()` in every new element: an object literal leaves them out).
//! - A write of a direct side (`Analysis::relation_writes`) is the
//!   two-sided update of §8.5: `faRelateOne` (one-to-many, one-to-one,
//!   symmetric), `faRelateAdd`/`faRelateDelete`/`faRelateClear` (many to
//!   many); an owner field is a plain write. A literal given to `add` that
//!   sets direct sides is linked once the element is stored.
//! - A removal from a related collection (`Analysis::relation_unlinks`)
//!   first unlinks each removed element (`faUnlink`, with a table of the
//!   relations it leaves: kind, role, direct side, reverse side).
//! - A copy of an element has no links (its sides are reset); a copy of an
//!   owner of related collections copies the graph and relinks the copies
//!   (`faCopyGraph`). Printing and `==` skip relation sides (D21).

use fa_analyzer::analysis::{
	Argument, CollectionRef, FieldValue, Relation, RelationKind, RelationSide,
};
use fa_analyzer::ids::{NodeId, Sym, TypeId};
use fa_analyzer::symbols::Resolution;
use fa_analyzer::syntax::View;
use fa_analyzer::types::{CollKind, TypeKind};

use crate::generate::Generator;
use crate::js::{self, Expr};

impl Generator<'_> {
	/// The relation side a field of an object type is, if any (whichever
	/// file declared the relation: `Analysis::relation_sides`).
	pub(crate) fn relation_side(&mut self, ty: TypeId, name: Sym) -> Option<(u32, RelationSide)> {
		if self.analysis.relation_sides.is_empty() {
			return None;
		}
		let ty = match self.types.kind(ty) {
			TypeKind::Rec(_) => self.types.unfold(ty),
			_ => ty,
		};
		self.analysis.relation_sides.get(&(ty, name)).copied()
	}

	/// The JavaScript properties of the relation sides of an object type.
	pub(crate) fn relation_properties(&mut self, ty: TypeId) -> Vec<String> {
		if self.analysis.relation_sides.is_empty() {
			return Vec::new();
		}
		let fields = self.types.fields_of(ty).to_vec();
		let names: Vec<Sym> = fields.iter().map(|field| field.name).collect();
		let sides: Vec<Sym> = names
			.into_iter()
			.filter(|&name| {
				fields.iter().any(|field| {
					field.name == name
						&& field
							.flags
							.contains(fa_analyzer::types::FieldFlags::RELATION)
				}) || self.relation_side(ty, name).is_some()
			})
			.collect();
		sides
			.into_iter()
			.map(|name| Self::property(self.types.syms.resolve(name)))
			.collect()
	}

	/// The property naming the relation sides of an object of type `ty`
	/// (`[faSides]: $sides`), read by erased generic code (copies, printing,
	/// equality: `faSides`); `None` for a type without sides.
	pub(crate) fn sides_property(&mut self, ty: TypeId) -> Option<String> {
		let sides = self.relation_properties(ty);
		if sides.is_empty() {
			return None;
		}
		let names: Vec<String> = sides.iter().map(|side| js::string_literal(side)).collect();
		let constant = self.constant("sides", format!("[{}]", names.join(", ")));
		let key = self.helper("faSides");
		Some(format!("[{key}]: {constant}"))
	}

	/// The initial value of a relation side in a new element: `new Set()`
	/// for a side that holds several elements, `undefined` otherwise.
	pub(crate) fn empty_side(&self, field_ty: TypeId) -> &'static str {
		match self.types.kind(field_ty) {
			TypeKind::Collection(CollKind::RelationPool(_), ..) => "new Set()",
			_ => "undefined",
		}
	}

	/// Whether an object type has relation sides.
	pub(crate) fn has_relation_sides(&mut self, ty: TypeId) -> bool {
		if self.analysis.relation_sides.is_empty() {
			return false;
		}
		let fields = self.types.fields_of(ty).to_vec();
		if fields.iter().any(|field| {
			field
				.flags
				.contains(fa_analyzer::types::FieldFlags::RELATION)
		}) {
			return true;
		}
		let names: Vec<Sym> = fields.iter().map(|field| field.name).collect();
		names
			.into_iter()
			.any(|name| self.relation_side(ty, name).is_some())
	}

	/// Whether a type owns a collection of elements with relation sides: a
	/// copy of it relinks the copies (`faCopyGraph`).
	pub(crate) fn owns_related_elements(&mut self, ty: TypeId) -> bool {
		if self.analysis.relation_sides.is_empty() {
			return false;
		}
		let items: Vec<TypeId> = self
			.types
			.fields_of(ty)
			.iter()
			.filter_map(|field| match self.types.kind(field.ty) {
				// a relation pool holds links, not elements
				TypeKind::Collection(CollKind::RelationPool(_), ..) => None,
				TypeKind::Collection(_, item, _) => Some(item),
				_ => None,
			})
			.collect();
		items.into_iter().any(|item| self.has_relation_sides(item))
	}

	/// The callback a relation write or removal `n` gives the runtime to
	/// notify the sides it changes, when they are reactive
	/// (`Analysis::relation_notifies`): `faNotifyField`.
	pub(crate) fn relation_notify(&mut self, n: NodeId) -> Option<String> {
		if !self.analysis.relation_notifies.contains(&n) {
			return None;
		}
		Some(self.helper("faNotifyField").to_string())
	}

	/// The kind code of a relation for the runtime (`faUnlink`).
	fn kind_code(kind: RelationKind) -> u8 {
		match kind {
			RelationKind::OneToMany => 0,
			RelationKind::OneToOne => 1,
			RelationKind::ManyToMany => 2,
			RelationKind::Symmetric => 3,
			RelationKind::OwnerField => 4,
		}
	}

	/// The JavaScript property names of a relation's sides.
	fn side_names(&self, relation: &Relation) -> (String, String) {
		let direct = Self::property(self.types.syms.resolve(relation.direct));
		let reverse = relation.reverse.map_or_else(
			|| direct.clone(),
			|reverse| Self::property(self.types.syms.resolve(reverse)),
		);
		(direct, reverse)
	}

	/// `holder.side = value` of a relation side (`assignment` is the
	/// `Assignment` node, in `Analysis::relation_writes`): the two-sided
	/// update. Returns `false` when the assignment isn't a relation write.
	pub(crate) fn relation_assignment(
		&mut self,
		assignment: NodeId,
		target: NodeId,
		value: NodeId,
	) -> bool {
		let Some(ids) = self.analysis.relation_writes.get(&assignment).cloned() else {
			return false;
		};
		let Some(&id) = ids.first() else {
			return false;
		};
		let View::Access {
			target: holder,
			members,
		} = self.view(self.strip_groups(target))
		else {
			return false;
		};
		if members.len() != 1 {
			return false;
		}
		let relation = self.analysis.relations[id as usize].clone();
		let values = self.in_order(&[holder, value], |g, node| g.expr(node));
		let (direct, reverse) = self.side_names(&relation);
		let holder = &values[0];
		let line = match relation.kind {
			RelationKind::OwnerField | RelationKind::ManyToMany => {
				format!("{}.{direct} = {};", holder.primary_text(), values[1].text)
			}
			kind => {
				let helper = self.helper("faRelateOne").to_string();
				let many = kind == RelationKind::OneToMany;
				let notify = self
					.relation_notify(assignment)
					.map_or_else(String::new, |notify| format!(", {notify}"));
				format!(
					"{helper}({}, {}, {}, {}, {many}{notify});",
					holder.text,
					js::string_literal(&direct),
					js::string_literal(&reverse),
					values[1].text
				)
			}
		};
		self.out.line(line);
		true
	}

	/// `holder.side.add(t)`, `delete(t)`, `clear()` of a many-to-many side
	/// (`RelationPool#…` in `Analysis::relation_writes`), or a read of a
	/// relation pool (a `Set`).
	pub(crate) fn relation_pool_call(
		&mut self,
		n: NodeId,
		callee: NodeId,
		path: &str,
		arguments: &[Argument],
	) -> Expr {
		let name = path.trim_start_matches("RelationPool#");
		let write = self
			.analysis
			.relation_writes
			.get(&n)
			.and_then(|ids| ids.first().copied());
		let nodes: Vec<NodeId> = arguments
			.iter()
			.filter_map(|argument| match argument {
				Argument::Given(value) => Some(*value),
				_ => None,
			})
			.collect();
		// `holder.side.method(…)`: the holder and the side
		let side = match self.view(callee) {
			View::Access { target, members } if members.len() >= 2 => {
				let members: Vec<NodeId> = members.iter().collect();
				let side = members[members.len() - 2];
				let holder = self.access_chain(callee, target, &members[..members.len() - 2]);
				Some((holder, side))
			}
			_ => None,
		};
		if let (Some(id), Some((holder, _))) = (write, side) {
			let relation = self.analysis.relations[id as usize].clone();
			let (direct, reverse) = self.side_names(&relation);
			let holder = if nodes.is_empty() {
				holder
			} else {
				self.keep_value(holder)
			};
			let values = self.in_order(&nodes, |g, node| g.expr(node));
			let helper = match name {
				"add" => "faRelateAdd",
				"delete" => "faRelateDelete",
				_ => "faRelateClear",
			};
			let helper = self.helper(helper).to_string();
			let mut all = vec![
				holder.text,
				js::string_literal(&direct),
				js::string_literal(&reverse),
			];
			all.extend(values.into_iter().map(|value| value.text));
			all.extend(self.relation_notify(n));
			return js::call(&helper, &all);
		}
		// a read: the `Set` of the side
		let receiver = self.receiver(callee);
		let values = self.in_order(&nodes, |g, node| g.expr(node));
		match name {
			"includes" => {
				let set = receiver.primary_text();
				Expr::primary(format!("{set}.has({})", values[0].text))
			}
			_ => self.collection_method(n, &format!("Set#{name}"), receiver, values),
		}
	}

	/// After `faAdd` of a literal setting direct sides (`relation_writes`
	/// on the `add` call): link them, the element `element` being stored.
	pub(crate) fn link_added(&mut self, n: NodeId, literal: NodeId, element: &str) {
		let Some(ids) = self.analysis.relation_writes.get(&n).cloned() else {
			return;
		};
		let Some(info) = self.analysis.object(self.strip_groups(literal)).cloned() else {
			return;
		};
		for (name, value) in &info.fields {
			if !matches!(value, FieldValue::Given(_)) {
				continue;
			}
			let Some((id, RelationSide::Direct)) = self.relation_side(info.ty, *name) else {
				continue;
			};
			if !ids.contains(&id) {
				continue;
			}
			let relation = self.analysis.relations[id as usize].clone();
			let (direct, reverse) = self.side_names(&relation);
			let (direct_text, reverse_text) =
				(js::string_literal(&direct), js::string_literal(&reverse));
			let notify = self
				.relation_notify(n)
				.map_or_else(String::new, |notify| format!(", {notify}"));
			let target = self.temp();
			match relation.kind {
				RelationKind::OwnerField => {}
				RelationKind::ManyToMany => {
					// a list of references (D19): linked one by one
					let helper = self.helper("faRelateAdd");
					self.out.line(format!(
						"const {target} = {element}.{direct}; {element}.{direct} = new Set();"
					));
					self.out.line(format!(
						"for (const $item of {target}) {helper}({element}, {direct_text}, {reverse_text}, $item{notify});"
					));
				}
				kind => {
					let helper = self.helper("faRelateOne");
					let many = kind == RelationKind::OneToMany;
					self.out.line(format!(
						"const {target} = {element}.{direct}; {element}.{direct} = undefined;"
					));
					self.out.line(format!(
						"{helper}({element}, {direct_text}, {reverse_text}, {target}, {many}{notify});"
					));
				}
			}
		}
	}

	/// The unlinking table of a removal from a related collection (the
	/// call `n`, in `Analysis::relation_unlinks`): `[kind, holder, direct,
	/// reverse]` per relation and role, a module-level constant, and the
	/// owner of the collection (`undefined` for a module-level one).
	pub(crate) fn unlink_table(&mut self, n: NodeId, callee: NodeId) -> Option<(String, String)> {
		let ids = self.analysis.relation_unlinks.get(&n)?.clone();
		// the collection the receiver names, and its owner
		let (owner, collection) = match self.view(callee) {
			View::Access { target, members } => {
				let members: Vec<NodeId> = members.iter().collect();
				match members.len() {
					0 | 1 => (None, target),
					count => (
						Some((target, members[..count - 2].to_vec())),
						members[count - 2],
					),
				}
			}
			_ => return None,
		};
		let matches = |g: &Self, reference: CollectionRef| match reference {
			CollectionRef::Field { name, .. } | CollectionRef::ExternalField { name, .. } => {
				g.types.syms.resolve(name) == g.text(collection)
			}
			CollectionRef::Module { declaration } => match g.analysis.resolution(collection) {
				Resolution::Local(symbol) => {
					let decl = g.analysis.symbols.symbol(symbol).decl_node;
					decl == declaration
						|| g.analysis.symbols.symbol_of_decl(declaration) == Some(symbol)
				}
				_ => false,
			},
			CollectionRef::External { key } => {
				g.analysis.resolution(collection) == Resolution::External(key)
			}
		};
		// a module-level value of another file reached through a namespace
		// (`library.people`) has no owner
		let external = matches!(
			self.analysis.resolution(collection),
			Resolution::External(_)
		);
		let owner = owner.filter(|_| !external);
		let mut entries = Vec::new();
		for id in ids {
			let relation = self.analysis.relations[id as usize].clone();
			let (direct, reverse) = self.side_names(&relation);
			let kind = Self::kind_code(relation.kind);
			let (holder, target) = (
				matches(self, relation.holder),
				matches(self, relation.target),
			);
			let (holder, target) = if holder || target {
				(holder, target)
			} else {
				(true, true)
			};
			let entry = |role: u8| {
				format!(
					"[{kind}, {role}, {}, {}]",
					js::string_literal(&direct),
					js::string_literal(&reverse)
				)
			};
			match relation.kind {
				RelationKind::Symmetric | RelationKind::OwnerField => entries.push(entry(0)),
				_ => {
					if target {
						entries.push(entry(0));
					}
					if holder {
						entries.push(entry(1));
					}
				}
			}
		}
		let table = self.constant("links", format!("[{}]", entries.join(", ")));
		let owner = match owner {
			Some((target, prefix)) => {
				let value = self.access_chain(callee, target, &prefix);
				value.text
			}
			None => "undefined".to_string(),
		};
		Some((table, owner))
	}
}

/// The JavaScript of a removal that unlinks first: `delete(e)` and
/// `take(e)` unlink `e`; `clear()` every element; `deleteWhere(p)` each
/// element `p` keeps out.
pub(crate) fn unlinking_call(
	unlink: &str,
	table: &str,
	owner: &str,
	name: &str,
	collection: &Expr,
	values: &[Expr],
) -> (Option<String>, Vec<Expr>) {
	match name {
		"delete" | "take" => (
			Some(format!("{unlink}({}, {table}, {owner});", values[0].text)),
			values.to_vec(),
		),
		"clear" => (
			Some(format!(
				"for (const $element of {}) {unlink}($element, {table}, {owner});",
				collection.primary_text()
			)),
			Vec::new(),
		),
		"deleteWhere" => {
			let predicate = values[0].primary_text();
			let wrapped = Expr::new(
				format!(
					"($element) => {predicate}($element) && ({unlink}($element, {table}, {owner}), true)"
				),
				crate::js::Prec::Assign,
			);
			(None, vec![wrapped])
		}
		_ => (None, values.to_vec()),
	}
}
