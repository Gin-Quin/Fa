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
	/// The JavaScript function of the `destroy` of a resource type: of this
	/// file, or imported from the file declaring the type.
	fn destroy_call(&mut self, ty: TypeId) -> Option<String> {
		let TypeKind::Object(shape) = self.types.kind(ty) else {
			return None;
		};
		let nominal = self.types.shape(shape).nominal?;
		let key = self.types.nominal(nominal).key;
		let module = self.types.decl_key_data(key).module;
		if self.types.module_path(module) == self.session.module(self.file) {
			let declaration = self
				.analysis
				.type_decls
				.values()
				.find(|decl| decl.ty == ty)?;
			let destroy = declaration.functions.iter().copied().find(|&function| {
				self.types
					.syms
					.resolve(self.analysis.symbols.symbol(function).name)
					== "destroy"
			})?;
			return Some(self.name_of(destroy));
		}
		// another file's: by its interface (the type's functions, their export names)
		let interface = self.session.interface(module)?;
		let destroy = interface.type_method(self.types, ty, "destroy")?.decl;
		let export = interface.export_names.get(&destroy)?.clone();
		let path = crate::generate::modules::js_path(self.types.module_path(module));
		Some(self.import(&path, &export))
	}

	/// The generated drop function of a type holding a resource
	/// (`drop$Holder`), `None` for a type holding none. Its name is declared
	/// before its body, which a recursive type calls.
	pub(crate) fn drop_function(&mut self, ty: TypeId) -> Option<String> {
		let ty = self.types.unfold(ty);
		if !self.types.contains_resource(ty) {
			return None;
		}
		if let Some(name) = self.drops.get(&ty) {
			return Some(name.clone());
		}
		let wanted = self
			.types
			.type_name(ty)
			.map_or("drop$value".to_string(), |name| format!("drop${name}"));
		let name = self.scopes.declare_global(&wanted);
		self.drops.insert(ty, name.clone());
		// a part this backend can't walk is not destroyed (design/todo.md)
		let lines = self.drop_body(ty).unwrap_or_default();
		self.constants.push((
			name.clone(),
			format!(
				"(value) => {{ if (value === undefined) return; {} }}",
				lines.join(" ")
			),
		));
		Some(name)
	}

	/// The statements of the drop function of a type.
	fn drop_body(&mut self, ty: TypeId) -> Option<Vec<String>> {
		match self.types.kind(ty) {
			TypeKind::Object(_) => {
				let mut lines = Vec::new();
				if self.types.is_resource(ty) {
					let destroy = self.destroy_call(ty)?;
					lines.push(format!("{destroy}(value);"));
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
					}
				}
				Some(lines)
			}
			TypeKind::Tuple(list) => {
				let items = self.types.list_items(list).to_vec();
				let mut lines = Vec::new();
				for (index, item) in items.into_iter().enumerate().rev() {
					if let Some(drop) = self.drop_function(item) {
						lines.push(format!("{drop}(value[{index}]);"));
					}
				}
				Some(lines)
			}
			TypeKind::Collection(kind, item, key) => {
				let item_drop = self.drop_function(item);
				let line = match kind {
					CollKind::Map | CollKind::GrowingMap => {
						let mut each = Vec::new();
						if let Some(drop) = self.drop_function(key) {
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
					| CollKind::GrowingPool
					| CollKind::OrderedColumnar => {
						let drop = item_drop?;
						format!("for (const item of value) {drop}(item);")
					}
					_ => return None,
				};
				Some(vec![line])
			}
			// a member of a named union: by its tag
			TypeKind::NamedUnion(nominal, _) | TypeKind::Member(nominal, _) => {
				let only = match self.types.kind(ty) {
					TypeKind::Member(_, index) => Some(index),
					_ => None,
				};
				let count = self
					.types
					.sym_list(self.types.nominal(nominal).members)
					.len() as u16;
				let mut lines = Vec::new();
				for index in (0..count).filter(|&index| only.is_none_or(|only| only == index)) {
					let payload = self.types.payload(nominal, index);
					if payload.is_unset() {
						continue;
					}
					let Some(drop) = self.drop_function(payload) else {
						continue;
					};
					let tag =
						crate::js::string_literal(&self.member_qualified_name(nominal, index));
					// an object payload is the member's object, other data its `$v`
					let data = if matches!(self.types.kind(payload), TypeKind::Object(_)) {
						"value"
					} else {
						"value.$v"
					};
					lines.push(format!("if (value.$t === {tag}) {drop}({data});"));
				}
				Some(lines)
			}
			// an optional value (`none` holds nothing), a union of objects told
			// apart like the analyzer does (a field the others lack, a literal
			// discriminant)
			TypeKind::Union(list) => {
				let members: Vec<TypeId> = self
					.types
					.list_items(list)
					.iter()
					.copied()
					.filter(|&member| member != TypeId::NONE)
					.collect();
				let mut lines = Vec::new();
				for &member in &members {
					let Some(drop) = self.drop_function(member) else {
						continue;
					};
					let others: Vec<TypeId> = members
						.iter()
						.copied()
						.filter(|&other| other != member)
						.collect();
					let test = self.member_test(member, &others)?;
					match test {
						Some(test) => lines.push(format!("if ({test}) {drop}(value);")),
						None => lines.push(format!("{drop}(value);")),
					}
				}
				Some(lines)
			}
			_ => None,
		}
	}

	/// The JavaScript test telling a value of `member` apart from the values
	/// of the `others` of its union: `Some(None)` when it needs none, `None`
	/// when there is no such test.
	fn member_test(&self, member: TypeId, others: &[TypeId]) -> Option<Option<String>> {
		let is_object = |ty: TypeId| {
			matches!(
				self.types.kind(ty),
				TypeKind::Object(_)
					| TypeKind::Tuple(_)
					| TypeKind::Collection(..)
					| TypeKind::NamedUnion(..)
					| TypeKind::Member(..)
			)
		};
		if others.is_empty() {
			return Some(None);
		}
		if !others.iter().any(|&other| is_object(other)) {
			return Some(Some("typeof value === \"object\"".to_string()));
		}
		match self.types.kind(member) {
			TypeKind::NamedUnion(nominal, _) | TypeKind::Member(nominal, _) => {
				let prefix = format!("{}.", self.types.decl_name(self.types.nominal(nominal).key));
				Some(Some(format!(
					"value?.$t?.startsWith({}) === true",
					crate::js::string_literal(&prefix)
				)))
			}
			TypeKind::Object(shape) => {
				let fields = self.types.shape_fields(shape);
				let other_fields = |name| {
					others
						.iter()
						.filter_map(move |&other| match self.types.kind(other) {
							TypeKind::Object(other_shape) => Some(
								self.types
									.shape_fields(other_shape)
									.iter()
									.find(|field| field.name == name)
									.map(|field| field.ty),
							),
							_ => None,
						})
				};
				// a field every other object lacks
				if let Some(field) = fields
					.iter()
					.find(|field| other_fields(field.name).all(|other| other.is_none()))
				{
					let name = crate::js::string_literal(self.types.syms.resolve(field.name));
					return Some(Some(format!(
						"typeof value === \"object\" && Object.hasOwn(value, {name})"
					)));
				}
				// a literal discriminant
				let field = fields.iter().find(|field| {
					self.literal_of(field.ty).is_some()
						&& other_fields(field.name).all(|other| {
							other.is_none_or(|other| {
								other != field.ty && self.literal_of(other).is_some()
							})
						})
				})?;
				let literal = self.literal_of(field.ty)?;
				let name = Self::property(self.types.syms.resolve(field.name));
				Some(Some(format!(
					"typeof value === \"object\" && value.{name} === {}",
					literal.text
				)))
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

	/// A field read out of a new value holding a resource (`open().handle`,
	/// `Handle { descriptor = 4 }.descriptor`): the value is dropped once
	/// the field is read, but the field when it moves out (it holds a
	/// resource itself), like the native drop of a temporary.
	pub(crate) fn temporary_access(
		&mut self,
		n: NodeId,
		target: NodeId,
		members: &[NodeId],
	) -> Option<Expr> {
		let inner = self.strip_groups(target);
		if !matches!(
			self.view(inner),
			View::Call { .. } | View::TypedObject { .. } | View::Object { .. } | View::Tuple { .. }
		) || !members
			.iter()
			.all(|&member| matches!(self.analysis.resolution(member), Resolution::Field(..)))
		{
			return None;
		}
		let drop = self.drop_function(self.type_of(target))?;
		let value = self.expr(target);
		let temp = self.inline_temp();
		let result = self.inline_temp();
		let mut read = Expr::name(temp.clone(), false);
		for &member in members {
			read = self.apply_member(read, member, false);
		}
		let moved = if self.types.contains_resource(self.type_of(n)) {
			format!("{} = undefined, ", read.text)
		} else {
			String::new()
		};
		// dropped at the end of the statement, like natively
		let drop = match self.guard_frames.last_mut() {
			Some(frame) => {
				frame.push(format!("{drop}({temp});"));
				String::new()
			}
			None => format!("{drop}({temp}), "),
		};
		Some(Expr::new(
			format!(
				"({temp} = {}, {result} = {}, {moved}{drop}{result})",
				value.at(Prec::Assign),
				read.text
			),
			Prec::Primary,
		))
	}

	/// The receiver of a method call when it is a new value holding a
	/// resource (`open(1).describe()`): kept in a temporary, and the
	/// statement dropping it after the call.
	pub(crate) fn temporary_receiver(
		&mut self,
		callee: NodeId,
		receiver: Expr,
	) -> (Expr, Option<String>) {
		let target = match self.view(callee) {
			View::Access { target, members } if members.len() == 1 => self.strip_groups(target),
			_ => return (receiver, None),
		};
		if !matches!(
			self.view(target),
			View::Call { .. } | View::TypedObject { .. } | View::Object { .. } | View::Tuple { .. }
		) {
			return (receiver, None);
		}
		let Some(drop) = self.drop_function(self.type_of(target)) else {
			return (receiver, None);
		};
		let temp = self.inline_temp();
		let receiver = Expr::new(
			format!("({temp} = {})", receiver.at(Prec::Assign)),
			Prec::Primary,
		);
		// dropped at the end of the statement, like natively
		if let Some(frame) = self.guard_frames.last_mut() {
			frame.push(format!("{drop}({temp});"));
			return (receiver, None);
		}
		(receiver, Some(format!("{drop}({temp})")))
	}

	/// The result of a call whose receiver is dropped after it.
	pub(crate) fn after_call_drop(
		&mut self,
		node: NodeId,
		result: Expr,
		drop: Option<String>,
	) -> Expr {
		let Some(drop) = drop else {
			return result;
		};
		if self.call_returns_reference(node)
			&& !self.is_object_reference(self.type_of(node))
			&& self.reference_call_layout(node) == fa_analyzer::analysis::ReferenceLayout::General
		{
			let result_temp = self.inline_temp();
			let key_temp = self.inline_temp();
			let key = self.helper("faReferenceKey");
			self.reference_call_keys
				.insert(node, Expr::name(key_temp.clone(), true));
			return Expr::new(
				format!(
					"({result_temp} = {}, {key_temp} = {key}, {drop}, {result_temp})",
					result.at(Prec::Assign)
				),
				Prec::Primary,
			);
		}
		let temp = self.inline_temp();
		Expr::new(
			format!("({temp} = {}, {drop}, {temp})", result.at(Prec::Assign)),
			Prec::Primary,
		)
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
