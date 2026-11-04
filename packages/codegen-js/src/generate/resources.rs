//! The resources of foreign code on JavaScript (a type with `destroy`):
//! destroyed at the points where the native backend drops them
//! (`design/decisions.md`, "A type holding a task is move-only").
//!
//! - **Drop functions**: a generated function per type holding a resource
//!   (`drop$Holder`): the resource's `destroy`, then the fields, items,
//!   keys and values holding one. A field moved out of its object is
//!   `undefined` (never a resource, which is an object), which they skip.
//! - **Owners**: a local or a parameter (not `self`) holding a resource is
//!   dropped at the end of its block (`finally`), unless it moved out: a
//!   local moved somewhere has a drop flag (`holder$owned`), cleared where
//!   it moves; a local never moved has none.
//! - **Moves**: a field moved out of a local (`Analysis::moved_reads`)
//!   leaves `undefined` in its object; a replaced value is dropped before
//!   the assignment stores the new one.

use fa_analyzer::ids::{NodeId, SymbolId, TypeId};
use fa_analyzer::symbols::{Resolution, SymbolKind};
use fa_analyzer::syntax::View;
use fa_analyzer::types::{CollKind, TypeKind};

use crate::generate::Generator;
use crate::js::{Expr, Prec};

impl Generator<'_> {
	/// The `destroy` function of a resource type declared in this file.
	fn destroy_of(&self, ty: TypeId) -> Option<SymbolId> {
		if !self.types.is_resource(ty) {
			return None;
		}
		let declaration = self
			.analysis
			.type_decls
			.values()
			.find(|decl| decl.ty == ty)?;
		declaration.functions.iter().copied().find(|&function| {
			self.types
				.syms
				.resolve(self.analysis.symbols.symbol(function).name)
				== "destroy"
		})
	}

	/// The generated drop function of a type holding a resource
	/// (`drop$Holder`), `None` for a type holding none (or one this backend
	/// can't walk: a recursive type, a union of several objects).
	pub(crate) fn drop_function(&mut self, ty: TypeId) -> Option<String> {
		if !self.types.contains_resource(ty) {
			return None;
		}
		if let Some(name) = self.drops.get(&ty) {
			return (!name.is_empty()).then(|| name.clone());
		}
		self.drops.insert(ty, String::new());
		let body = self.drop_body(ty);
		self.drops.remove(&ty);
		let (wanted, lines) = body?;
		let code = format!(
			"(value) => {{ if (value === undefined) return; {} }}",
			lines.join(" ")
		);
		let name = self.constant(&wanted, code);
		self.drops.insert(ty, name.clone());
		Some(name)
	}

	/// The name and the statements of the drop function of a type.
	fn drop_body(&mut self, ty: TypeId) -> Option<(String, Vec<String>)> {
		let type_name = |g: &Self| {
			g.types
				.type_name(ty)
				.map_or("drop$value".to_string(), |name| format!("drop${name}"))
		};
		match self.types.kind(ty) {
			TypeKind::Object(_) => {
				let mut lines = Vec::new();
				if let Some(destroy) = self.destroy_of(ty) {
					lines.push(format!("{}(value);", self.name_of(destroy)));
				} else if self.types.is_resource(ty) {
					// a resource of another file: its `destroy` isn't reachable here
					return None;
				}
				let fields: Vec<(String, TypeId)> = self
					.types
					.fields_of(ty)
					.iter()
					.map(|field| {
						(
							Self::property(self.types.syms.resolve(field.name)),
							field.ty,
						)
					})
					.collect();
				for (name, field_type) in fields.into_iter().rev() {
					if let Some(drop) = self.drop_function(field_type) {
						lines.push(format!("{drop}(value.{name});"));
					} else if self.types.contains_resource(field_type) {
						return None;
					}
				}
				Some((type_name(self), lines))
			}
			TypeKind::Tuple(list) => {
				let items = self.types.list_items(list).to_vec();
				let mut lines = Vec::new();
				for (index, item) in items.into_iter().enumerate().rev() {
					if let Some(drop) = self.drop_function(item) {
						lines.push(format!("{drop}(value[{index}]);"));
					} else if self.types.contains_resource(item) {
						return None;
					}
				}
				Some(("drop$tuple".to_string(), lines))
			}
			TypeKind::Collection(kind, item, key) => {
				let item_drop = self.drop_function(item);
				let line = match kind {
					CollKind::Map | CollKind::GrowingMap => {
						let key_drop = self.drop_function(key);
						if item_drop.is_none() && self.types.contains_resource(item)
							|| key_drop.is_none() && self.types.contains_resource(key)
						{
							return None;
						}
						let mut each = Vec::new();
						if let Some(drop) = key_drop {
							each.push(format!("{drop}(key);"));
						}
						if let Some(drop) = item_drop {
							each.push(format!("{drop}(item);"));
						}
						format!("for (const [key, item] of value) {{ {} }}", each.join(" "))
					}
					CollKind::Array
					| CollKind::GrowingArray
					| CollKind::Set
					| CollKind::GrowingSet
					| CollKind::Pool
					| CollKind::GrowingPool => {
						let drop = item_drop?;
						format!("for (const item of value) {drop}(item);")
					}
					_ => return None,
				};
				Some(("drop$items".to_string(), vec![line]))
			}
			// an optional value: `none` holds nothing
			TypeKind::Union(list) => {
				let members = self.types.list_items(list).to_vec();
				let holders: Vec<TypeId> = members
					.iter()
					.copied()
					.filter(|&member| self.types.contains_resource(member))
					.collect();
				let others_none = members
					.iter()
					.all(|&member| holders.contains(&member) || member == TypeId::NONE);
				let [holder] = holders.as_slice() else {
					return None;
				};
				if !others_none {
					return None;
				}
				let drop = self.drop_function(*holder)?;
				Some(("drop$optional".to_string(), vec![format!("{drop}(value);")]))
			}
			_ => None,
		}
	}

	/// The local or parameter that a declaration (or a parameter) makes
	/// own a value holding a resource, dropped at the end of its block.
	pub(crate) fn owned_resource(&self, n: NodeId) -> Option<SymbolId> {
		if !matches!(self.view(n), View::Declaration { .. }) {
			return None;
		}
		let symbol = self.analysis.symbols.symbol_of_decl(n)?;
		self.owns_resource(symbol).then_some(symbol)
	}

	/// Whether a local or a parameter owns a value holding a resource.
	pub(crate) fn owns_resource(&self, symbol: SymbolId) -> bool {
		let info = self.analysis.symbols.symbol(symbol);
		matches!(
			info.kind,
			SymbolKind::Let | SymbolKind::Mutable | SymbolKind::Parameter { .. }
		) && self.types.contains_resource(info.ty)
	}

	/// Whether a local moves out somewhere (as a whole): it needs a drop flag.
	fn moves_out(&self, symbol: SymbolId) -> bool {
		(0..self.tree.nodes.len()).any(|index| {
			let node = NodeId::new(index);
			matches!(self.view(node), View::Name(_))
				&& self.analysis.resolution(node) == Resolution::Local(symbol)
				&& self.moves_whole(node)
		})
	}

	/// Whether the read of a local at a node moves it out.
	fn moves_whole(&self, node: NodeId) -> bool {
		self.analysis.moved_reads.contains(&node) || self.is_move(node)
	}

	/// Starts owning a local or a parameter holding a resource: its drop
	/// function, and its drop flag (declared here) when it moves out
	/// somewhere. The statement that drops it: `if (flag) drop(local);`.
	pub(crate) fn own_resource(&mut self, symbol: SymbolId) -> Option<String> {
		let ty = self.analysis.symbols.symbol(symbol).ty;
		let drop = self.drop_function(ty)?;
		let name = self.name_of(symbol);
		if !self.moves_out(symbol) {
			return Some(format!("{drop}({name});"));
		}
		let flag = self.scopes.declare(&format!("{name}$owned"));
		self.out.line(format!("let {flag} = true;"));
		self.resource_flags.insert(symbol, flag.clone());
		Some(format!("if ({flag}) {drop}({name});"))
	}

	/// The read of a local that moves it out: its drop flag cleared.
	pub(crate) fn moving_read(&self, node: NodeId, symbol: SymbolId, read: Expr) -> Expr {
		match self.resource_flags.get(&symbol) {
			Some(flag) if self.moves_whole(node) => {
				Expr::new(format!("({flag} = false, {})", read.text), Prec::Primary)
			}
			_ => read,
		}
	}

	/// A field holding a resource moved out of its local (`holder.handle`):
	/// `undefined` left in its object, which its drop skips.
	pub(crate) fn field_moved_out(
		&mut self,
		n: NodeId,
		target: NodeId,
		members: &[NodeId],
	) -> Option<Expr> {
		if !self.analysis.moved_reads.contains(&n) || !self.types.contains_resource(self.type_of(n))
		{
			return None;
		}
		let (&last, before) = members.split_last()?;
		let Resolution::Field(..) = self.analysis.resolution(last) else {
			return None;
		};
		let container = if before.is_empty() {
			self.expr(target)
		} else {
			self.access_chain(n, target, before)
		};
		let name = Self::property(self.text(last));
		let temp = self.inline_temp();
		let container = container.primary_text();
		Some(Expr::new(
			format!("({temp} = {container}.{name}, {container}.{name} = undefined, {temp})"),
			Prec::Primary,
		))
	}

	/// Before `target = value` replaces a value holding a resource: the
	/// statement dropping the old value (but what moved out of it), and the
	/// one setting its drop flag again.
	pub(crate) fn drop_replaced(&mut self, target: NodeId) -> Option<(String, Option<String>)> {
		let ty = self.type_of(target);
		let drop = self.drop_function(ty)?;
		let flag = match self.view(self.strip_groups(target)) {
			View::Name(_) => match self.analysis.resolution(self.strip_groups(target)) {
				Resolution::Local(symbol) => self.resource_flags.get(&symbol).cloned(),
				_ => None,
			},
			_ => None,
		};
		Some((drop, flag))
	}
}
