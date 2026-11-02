//! Objects, arrays, named unions, enumerations and optionals (values brief
//! §9, §11; types brief §3.2.7, §3.4.6, §3.5.3; `design/decisions.md`).
//!
//! - An object is an object literal with its fields in the order of its
//!   type's declaration (one hidden class per type), its values computed in
//!   the order they are written; a private field `#x` is the property `$p_x`.
//! - The functions of a type are module-level functions `Type$name(self, …)`.
//! - A member of a named union is its data object with a `$t` tag first
//!   (`{ $t: "Shape.Circle", radius: 1 }`), `{ $t, v }` for other data, a
//!   frozen module-level singleton `{ $t }` without data, and the literal
//!   itself for a literal member. The tag is the qualified name of the member.
//! - A member of an enumeration is its underlying number; a table of names
//!   per enumeration prints it.
//! - An array is a JavaScript array; an index the analyzer didn't prove goes
//!   through `faIndex` (`IndexError`).
//! - `none` is `undefined`: `?.` and `??` are JavaScript's.
//! - A composite value read from a place is copied where Fa's value
//!   semantics need it (a `mutable` destination, or a mutable source).

use fa_analyzer::analysis::{FieldValue, NodeFlags};
use fa_analyzer::builtins;
use fa_analyzer::ids::{NodeId, NominalId, SymbolId, TypeId};
use fa_analyzer::symbols::{Resolution, SymbolKind};
use fa_analyzer::syntax::{NodeList, View};
use fa_analyzer::types::{CollKind, ConstValue, NominalKind, TypeKind};

use crate::generate::{Dest, Generator, Kind};
use crate::js::{self, Expr, Prec};

/// An assignable place, its container evaluated once.
pub(crate) struct Place {
	/// The JavaScript of the place (`point.x`, `items[$t0]`, `count`).
	pub(crate) text: String,
	/// The type of the values it holds.
	pub(crate) ty: TypeId,
}

impl Generator<'_> {
	// ── objects ──────────────────────────────────────────────────────────

	/// The JavaScript property of a field (`#secret` → `$p_secret`).
	pub(crate) fn property(name: &str) -> String {
		match name.strip_prefix('#') {
			Some(private) => format!("$p_{private}"),
			None => name.to_string(),
		}
	}

	/// An object literal (typed or not, or a member of a named union built
	/// with braces): the fields in declaration order.
	pub(crate) fn object_literal(&mut self, n: NodeId) -> Expr {
		let Some(info) = self.analysis.object(n).cloned() else {
			return self.unsupported(n, "this object literal");
		};
		let spreads = info
			.fields
			.iter()
			.any(|(_, value)| matches!(value, FieldValue::Spread(_)));
		let values = if spreads {
			self.spread_values(&info.fields)
		} else {
			// the given values, in the order they are written
			let slots: Vec<Option<NodeId>> = info
				.fields
				.iter()
				.map(|(_, value)| match value {
					FieldValue::Given(node) => Some(*node),
					FieldValue::Default(_) | FieldValue::None | FieldValue::Spread(_) => None,
				})
				.collect();
			self.ordered_values(&slots, |g, node| {
				let value = g.expr(node);
				g.copy_stored(node, value)
			})
		};
		let mut parts = Vec::with_capacity(info.fields.len() + 1);
		let mut error = None;
		if let Some(member) = info.member
			&& let TypeKind::Member(nominal, index) = self.types.kind(member)
		{
			if self.is_errors(nominal) {
				error = Some((nominal, index));
			} else {
				let tag = js::string_literal(&self.member_qualified_name(nominal, index));
				parts.push(format!("$t: {tag}"));
			}
		}
		let mut fields = Vec::with_capacity(info.fields.len());
		for ((name, value), given) in info.fields.iter().zip(values) {
			let property = Self::property(self.types.syms.resolve(*name));
			let value = match (value, given) {
				(_, Some(given)) => given,
				(FieldValue::Default(default), None) => self.expr(*default),
				_ => Expr::constant("undefined", Prec::Primary),
			};
			parts.push(format!("{property}: {}", value.at(Prec::Assign)));
			fields.push((property, value));
		}
		if let Some((nominal, index)) = error {
			// an error: its class, the fields in the order of its constructor
			let order = self.error_fields(nominal, index);
			let values = order
				.iter()
				.map(|field| {
					fields.iter().find(|(name, _)| name == field).map_or_else(
						|| "undefined".to_string(),
						|(_, value)| value.at(Prec::Assign),
					)
				})
				.collect();
			return self.new_error(nominal, index, values);
		}
		if parts.is_empty() {
			return Expr::primary("{}");
		}
		Expr::primary(format!("{{ {} }}", parts.join(", ")))
	}

	/// The values of the fields of an object literal with spreads
	/// (`{ ...defaults, port = 8080 }`): the spread values and the written
	/// ones evaluated once, in source order, then each field read from its
	/// source (`$t0.host`).
	fn spread_values(
		&mut self,
		fields: &[(fa_analyzer::ids::Sym, FieldValue)],
	) -> Vec<Option<Expr>> {
		let mut nodes: Vec<NodeId> = Vec::new();
		for (_, value) in fields {
			if let FieldValue::Given(node) | FieldValue::Spread(node) = value
				&& !nodes.contains(node)
			{
				nodes.push(*node);
			}
		}
		nodes.sort_by_key(|node| self.tree.spans[node.index()].start);
		let spread_nodes: Vec<NodeId> = fields
			.iter()
			.filter_map(|(_, value)| match value {
				FieldValue::Spread(node) => Some(*node),
				_ => None,
			})
			.collect();
		let values = self.in_order(&nodes, |g, node| {
			let value = g.expr(node);
			if spread_nodes.contains(&node) {
				value
			} else {
				g.copy_stored(node, value)
			}
		});
		// every value before the object is built: the source order is kept
		let values: Vec<Expr> = values.into_iter().map(|value| self.keep(value)).collect();
		fields
			.iter()
			.map(|(name, value)| match value {
				FieldValue::Given(node) => {
					let index = nodes.iter().position(|found| found == node)?;
					Some(values[index].clone())
				}
				FieldValue::Spread(node) => {
					let index = nodes.iter().position(|found| found == node)?;
					let property = Self::property(self.types.syms.resolve(*name));
					let read =
						Expr::primary(format!("{}.{property}", values[index].primary_text()));
					let ty = self
						.types
						.fields_of(self.type_of(*node))
						.iter()
						.find(|field| field.name == *name)
						.map_or(TypeId::UNKNOWN, |field| field.ty);
					if self.is_composite(ty) && self.place_root(*node).is_some() {
						Some(self.copy_value(read, ty))
					} else {
						Some(read)
					}
				}
				FieldValue::Default(_) | FieldValue::None => None,
			})
			.collect()
	}

	/// Values for slots (`None`: no value), evaluated in the order they are
	/// written in the source, returned in slot order; when the two orders
	/// differ, the values with effects go through temporaries first.
	pub(crate) fn ordered_values(
		&mut self,
		slots: &[Option<NodeId>],
		mut f: impl FnMut(&mut Self, NodeId) -> Expr,
	) -> Vec<Option<Expr>> {
		let mut order: Vec<(usize, usize)> = slots
			.iter()
			.enumerate()
			.filter_map(|(index, slot)| {
				slot.map(|node| (self.tree.spans[node.index()].start, index))
			})
			.collect();
		order.sort();
		let nodes: Vec<NodeId> = order
			.iter()
			.map(|&(_, index)| slots[index].expect("a value"))
			.collect();
		let values = self.in_order(&nodes, &mut f);
		let reordered = order.windows(2).any(|pair| pair[0].1 > pair[1].1);
		let mut by_slot: Vec<Option<Expr>> = vec![None; slots.len()];
		for (&(_, index), value) in order.iter().zip(values) {
			let value = if reordered && !value.stable {
				let temp = self.temp();
				self.out.line(format!("const {temp} = {};", value.text));
				Expr::name(temp, true)
			} else {
				value
			};
			by_slot[index] = Some(value);
		}
		by_slot
	}

	/// `target.member…` (fields, members of named unions and enumerations,
	/// functions of types, properties of strings and arrays).
	pub(crate) fn member_access(
		&mut self,
		n: NodeId,
		target: NodeId,
		members: NodeList<'_>,
	) -> Expr {
		// a constant computed by the analyzer (`"abc".length`)
		let ty = self.type_of(n);
		if let Some(int) = self.types.as_int(ty)
			&& let Some(value) = int.literal()
			&& matches!(self.view(self.strip_groups(target)), View::String(_))
		{
			return Expr::constant(value.to_string(), Prec::Primary);
		}
		let members: Vec<NodeId> = members.iter().collect();
		// a constant of the number API (`Int8.maxValue`, `Float.nan`)
		if let [member] = members.as_slice()
			&& let Resolution::Builtin(builtin) = self.analysis.resolution(*member)
			&& let Some(constant) = self.number_constant_member(n, builtins::get(builtin).path)
		{
			return constant;
		}
		self.access_chain(n, target, &members)
	}

	/// `target.a.b…` for the given members (all of an access, or all but the
	/// last for the receiver of a method).
	pub(crate) fn access_chain(&mut self, n: NodeId, target: NodeId, members: &[NodeId]) -> Expr {
		// `library.math.add`, `counter.count`: through namespaces (modules.rs)
		if let Some((mut value, used)) = self.namespace_prefix(target, members) {
			for &member in &members[used..] {
				value = self.apply_member(value, member, false);
			}
			return value;
		}
		let mut members = members.iter().copied();
		let mut value = if self.is_type_name(target) {
			// `Shape.Circle`, `Point.origin`, `Color.members`, `Self.create`
			let Some(first) = members.next() else {
				return self.unsupported(n, "a type as a value");
			};
			self.type_member(target, first)
		} else if let Resolution::Builtin(builtin) = self.analysis.resolution(target) {
			let path = builtins::get(builtin).path;
			// a constant of the number API (`Float.nan` in `Float.nan.isNaN()`)
			if let Some(&first) = members.clone().collect::<Vec<_>>().first()
				&& let Resolution::Builtin(constant) = self.analysis.resolution(first)
				&& let Some(value) =
					self.number_constant_member(first, builtins::get(constant).path)
			{
				members.next();
				let mut value = value;
				for member in members {
					value = self.apply_member(value, member, false);
				}
				return value;
			}
			if path != "arguments" {
				return self.unsupported(n, &format!("`{path}` as a value"));
			}
			self.use_arguments()
		} else {
			self.expr(target)
		};
		for member in members {
			value = self.apply_member(value, member, false);
		}
		value
	}

	/// Whether a node names a type (`Point`, `Self`).
	pub(crate) fn is_type_name(&self, n: NodeId) -> bool {
		match self.analysis.resolution(n) {
			Resolution::Local(symbol) => {
				self.analysis.symbols.symbol(symbol).kind == SymbolKind::Type
			}
			_ => matches!(self.view(n), View::Name(_)) && self.text(n) == "Self",
		}
	}

	/// The first member after a type name: a member of a named union or
	/// enumeration, a function of the type, `members` of an enumeration.
	fn type_member(&mut self, target: NodeId, member: NodeId) -> Expr {
		match self.analysis.resolution(member) {
			Resolution::Member(nominal, index) => self.nominal_member_value(member, nominal, index),
			Resolution::Local(function) => {
				let name = self.name_of(function);
				Expr::name(name, true)
			}
			Resolution::Builtin(builtin) if builtins::get(builtin).path == "Enum.members" => {
				let denoted = match self.analysis.resolution(target) {
					Resolution::Local(symbol) => self.analysis.symbols.symbol(symbol).ty,
					_ => self.type_of(target),
				};
				match self.types.kind(denoted) {
					TypeKind::Enum(nominal, _) | TypeKind::EnumMember(nominal, _) => {
						let name = self.enum_members(nominal);
						Expr::name(name, true)
					}
					_ => self.unsupported(member, "`members`"),
				}
			}
			_ => self.unsupported(member, "this member of a type"),
		}
	}

	/// `value.member` for one member (`optional`: after `?.`).
	pub(crate) fn apply_member(&mut self, value: Expr, member: NodeId, optional: bool) -> Expr {
		let dot = if optional { "?." } else { "." };
		match self.analysis.resolution(member) {
			Resolution::Field(shape, index) => {
				let field = self.types.shape_fields(shape)[index as usize];
				let property = Self::property(self.types.syms.resolve(field.name));
				Expr::primary(format!("{}{dot}{property}", value.primary_text()))
					.with_purity(value.pure, false)
			}
			Resolution::Builtin(builtin) => match builtins::get(builtin).path {
				"String#length" if !optional => {
					let helper = self.helper("faStringLength");
					js::call(helper, &[value.text])
				}
				"Array#length" => Expr::primary(format!("{}{dot}length", value.primary_text()))
					.with_purity(value.pure, false),
				"Set#size" | "Map#size" => {
					Expr::primary(format!("{}{dot}size", value.primary_text()))
						.with_purity(value.pure, false)
				}
				path @ ("Range#start" | "Range#end" | "Range#includesEnd") => {
					let field = path.trim_start_matches("Range#");
					Expr::primary(format!("{}{dot}{field}", value.primary_text()))
						.with_purity(value.pure, false)
				}
				path => self.unsupported(member, &format!("`{path}` as a value")),
			},
			// a field common to the members of an anonymous union
			Resolution::Unresolved if matches!(self.view(member), View::Name(_)) => {
				let property = Self::property(self.text(member));
				Expr::primary(format!("{}{dot}{property}", value.primary_text()))
					.with_purity(value.pure, false)
			}
			_ => self.unsupported(member, "this member"),
		}
	}

	/// The value of a member of a named union or enumeration without data.
	pub(crate) fn nominal_member_value(
		&mut self,
		n: NodeId,
		nominal: NominalId,
		index: u16,
	) -> Expr {
		let info = self.types.nominal(nominal);
		match info.kind {
			NominalKind::Enum => self.enum_value(nominal, index),
			NominalKind::Union => {
				let payload = self.types.payload(nominal, index);
				if payload.is_unset() {
					let name = self.member_singleton(nominal, index);
					return Expr::name(name, true);
				}
				match self.literal_of(payload) {
					Some(literal) => literal,
					None => self.unsupported(n, "a member holding data, without its data"),
				}
			}
			NominalKind::Errors => self.new_error(nominal, index, Vec::new()),
			_ => self.unsupported(n, "this member"),
		}
	}

	/// `Shape.Labelled("north")`: `{ $t: "Shape.Labelled", v: "north" }`.
	pub(crate) fn member_with_data(&mut self, member: TypeId, data: NodeId) -> Expr {
		let TypeKind::Member(nominal, index) = self.types.kind(member) else {
			return self.unsupported(data, "this member");
		};
		let value = self.expr(data);
		let value = self.copy_stored(data, value);
		if self.is_errors(nominal) {
			return self.new_error(nominal, index, vec![value.text]);
		}
		let tag = js::string_literal(&self.member_qualified_name(nominal, index));
		Expr::primary(format!("{{ $t: {tag}, $v: {} }}", value.at(Prec::Assign)))
	}

	/// The qualified name of a member: `Shape.Circle`.
	pub(crate) fn member_qualified_name(&self, nominal: NominalId, index: u16) -> String {
		let info = self.types.nominal(nominal);
		format!(
			"{}.{}",
			self.types.decl_name(info.key),
			self.types.member_name(nominal, index)
		)
	}

	/// The module-level frozen singleton of a member without data.
	fn member_singleton(&mut self, nominal: NominalId, index: u16) -> String {
		let qualified = self.member_qualified_name(nominal, index);
		let name = qualified.replace('.', "$");
		let code = format!(
			"Object.freeze({{ $t: {} }})",
			js::string_literal(&qualified)
		);
		self.constant(&name, code)
	}

	/// The underlying number of an enumeration member: its pinned value, or
	/// its position.
	pub(crate) fn enum_value(&mut self, nominal: NominalId, index: u16) -> Expr {
		match self.pinned_value(nominal, index) {
			Some(value) => Expr::constant(value, Prec::Primary),
			None => Expr::constant(index.to_string(), Prec::Primary),
		}
	}

	/// The value pinned to a member of an enumeration (`archived = 9`),
	/// read from its declaration: the analysis doesn't record it.
	fn pinned_value(&self, nominal: NominalId, index: u16) -> Option<String> {
		let (&node, _) = self.analysis.type_decls.iter().find(|(_, decl)| {
			matches!(
				self.types.kind(decl.ty),
				TypeKind::Enum(found, _) | TypeKind::EnumMember(found, _) if found == nominal
			)
		})?;
		let View::TypeDecl { value, .. } = self.view(node) else {
			return None;
		};
		let members = match self.view(value) {
			View::TypedObject { members, .. } | View::Object { items: members } => members,
			_ => return None,
		};
		let member = members.get(index as usize)?;
		match self.view(member) {
			View::Assignment {
				value: Some(pinned),
				..
			} => match self.view(self.strip_groups(pinned)) {
				View::Integer(value) => Some(value.to_string()),
				_ => None,
			},
			_ => None,
		}
	}

	/// The module-level table of the names of an enumeration's members, by value.
	pub(crate) fn enum_names(&mut self, nominal: NominalId) -> String {
		let info = self.types.nominal(nominal);
		let count = self.types.sym_list(info.members).len();
		let name = format!("{}$names", self.types.decl_name(info.key));
		let mut entries = Vec::new();
		let mut contiguous = true;
		for index in 0..count as u16 {
			let value = self.enum_value(nominal, index).text;
			if value != index.to_string() {
				contiguous = false;
			}
			entries.push((
				value,
				js::string_literal(self.types.member_name(nominal, index)),
			));
		}
		let code = if contiguous {
			let names: Vec<String> = entries.into_iter().map(|(_, name)| name).collect();
			format!("[{}]", names.join(", "))
		} else {
			let pairs: Vec<String> = entries
				.into_iter()
				.map(|(value, name)| format!("{value}: {name}"))
				.collect();
			format!("{{ {} }}", pairs.join(", "))
		};
		self.constant(&name, code)
	}

	/// The module-level array of the members of an enumeration.
	fn enum_members(&mut self, nominal: NominalId) -> String {
		let info = self.types.nominal(nominal);
		let count = self.types.sym_list(info.members).len();
		let name = format!("{}$members", self.types.decl_name(info.key));
		let values: Vec<String> = (0..count as u16)
			.map(|index| self.enum_value(nominal, index).text)
			.collect();
		let code = format!("Object.freeze([{}])", values.join(", "));
		self.constant(&name, code)
	}

	/// The value of a literal type (a literal member of a named union).
	pub(crate) fn literal_of(&self, ty: TypeId) -> Option<Expr> {
		match self.types.kind(ty) {
			TypeKind::StringLit(sym) => Some(Expr::constant(
				js::string_literal(self.types.syms.resolve(sym)),
				Prec::Primary,
			)),
			TypeKind::BoolLit(value) => Some(Expr::constant(value.to_string(), Prec::Primary)),
			TypeKind::Int(id) => {
				let int = self.types.int(id);
				let value = int.literal()?;
				let text = if int.storage.is_js_bigint() {
					format!("{value}n")
				} else {
					value.to_string()
				};
				Some(Expr::constant(text, Prec::Primary))
			}
			TypeKind::FloatLit(_, constant) => match self.types.const_value(constant) {
				ConstValue::Float(bits) => Some(js::float_literal(f64::from_bits(bits))),
				_ => None,
			},
			TypeKind::None => Some(Expr::constant("undefined", Prec::Primary)),
			_ => None,
		}
	}

	// ── arrays ───────────────────────────────────────────────────────────

	/// `[a, b, c]`.
	pub(crate) fn array_literal(&mut self, items: NodeList<'_>) -> Expr {
		let nodes: Vec<NodeId> = items.iter().collect();
		let values = self.in_order(&nodes, |g, node| {
			let value = g.expr(node);
			g.copy_stored(node, value)
		});
		let texts: Vec<String> = values.iter().map(|value| value.at(Prec::Assign)).collect();
		Expr::primary(format!("[{}]", texts.join(", ")))
	}

	/// A range as a value: a `FaRange` (a `for` loop over a range literal
	/// is a counted loop and builds none).
	pub(crate) fn range_value(
		&mut self,
		start: Option<NodeId>,
		end: Option<NodeId>,
		inclusive: bool,
	) -> Expr {
		let bounds: Vec<NodeId> = start.into_iter().chain(end).collect();
		let mut values = self.in_order(&bounds, |g, node| g.expr(node)).into_iter();
		let start = start.map_or("undefined".to_string(), |_| {
			values.next().expect("a start").text
		});
		let end = end.map_or("undefined".to_string(), |_| {
			values.next().expect("an end").text
		});
		let class = self.helper("FaRange");
		Expr::primary(format!("new {class}({start}, {end}, {inclusive})"))
	}

	/// `target[a..b]`: a slice of an array (a copy) or of a string.
	fn slice(&mut self, target: NodeId, range: NodeId) -> Option<Expr> {
		let helper = |g: &mut Self| {
			if g.kind(g.type_of(target)) == Kind::Text {
				g.helper("faStringSlice")
			} else {
				g.helper("faArraySlice")
			}
		};
		let View::Range {
			start,
			end,
			inclusive,
		} = self.view(self.strip_groups(range))
		else {
			if self.type_of(range) != TypeId::RANGE {
				return None;
			}
			// a `Range` value: its fields are the bounds
			let values = self.in_order(&[target, range], |g, node| g.expr(node));
			let range = self.keep(values[1].clone());
			let bounds = range.primary_text();
			let helper = helper(self);
			return Some(js::call(
				helper,
				&[
					values[0].text.clone(),
					format!("{bounds}.start"),
					format!("{bounds}.end"),
					format!("{bounds}.includesEnd"),
				],
			));
		};
		let mut nodes = vec![target];
		nodes.extend(start);
		nodes.extend(end);
		let mut values = self.in_order(&nodes, |g, node| g.expr(node)).into_iter();
		let container = values.next().expect("a target").text;
		let start = start.map_or("undefined".to_string(), |_| {
			values.next().expect("a start").text
		});
		let end = end.map_or("undefined".to_string(), |_| {
			values.next().expect("an end").text
		});
		let helper = if self.kind(self.type_of(target)) == Kind::Text {
			self.helper("faStringSlice")
		} else {
			self.helper("faArraySlice")
		};
		Some(js::call(
			helper,
			&[container, start, end, inclusive.to_string()],
		))
	}

	/// `target[index]`: an array item or a character of a string.
	pub(crate) fn index(&mut self, n: NodeId, target: NodeId, index: NodeId) -> Expr {
		if let Some(slice) = self.slice(target, index) {
			return slice;
		}
		let values = self.in_order(&[target, index], |g, node| g.expr(node));
		let (container, position) = (&values[0], &values[1]);
		if let TypeKind::Collection(CollKind::Map | CollKind::GrowingMap, _, key) =
			self.types.kind(self.type_of(target))
		{
			return self.map_index(n, container.clone(), position.clone(), key);
		}
		let checked = self.has(n, NodeFlags::INDEX_CHECK);
		match self.kind(self.type_of(target)) {
			Kind::Text => {
				// the grapheme at a position: a helper, checked or not
				let helper = self.helper("faStringIndex");
				js::call(helper, &[container.text.clone(), position.text.clone()])
			}
			_ if checked => {
				let helper = self.helper("faIndex");
				js::call(helper, &[container.text.clone(), position.text.clone()])
			}
			_ => Expr::primary(format!("{}[{}]", container.primary_text(), position.text)),
		}
	}

	// ── optionals ────────────────────────────────────────────────────────

	/// `target?.a?.b`: JavaScript's optional chain. The parser keeps the
	/// members of a chain in one node, without saying which ones follow a
	/// `?.`: every member is read with `?.`, which is the same where a
	/// member is never `none` (the chain stops at its first `none`).
	pub(crate) fn optional_access(
		&mut self,
		_n: NodeId,
		target: NodeId,
		members: NodeList<'_>,
	) -> Expr {
		let mut value = self.expr(target);
		for member in members.iter() {
			value = self.apply_member(value, member, true);
		}
		value
	}

	/// `a ?? b ?? c`: an operand is evaluated only when the ones before are
	/// `none`.
	pub(crate) fn fallback(&mut self, operands: &[NodeId]) -> Expr {
		let Some((&first, rest)) = operands.split_first() else {
			return Expr::constant("undefined", Prec::Primary);
		};
		let mut result = self.expr(first);
		for &operand in rest {
			let indent = self.out.indentation() + 1;
			let (lines, value) = self.capture_at(indent, |g| g.expr(operand));
			if lines.is_empty() {
				result = js::binary(&result, "??", &value, Prec::Nullish);
				continue;
			}
			let temp = self.temp();
			self.out.line(format!("let {temp} = {};", result.text));
			self.out.open(format!("if ({temp} === undefined)"));
			self.out.append(lines);
			self.out.line(format!("{temp} = {};", value.text));
			self.out.close("");
			result = Expr::name(temp, true);
		}
		result
	}

	// ── types ────────────────────────────────────────────────────────────

	/// `type Name = …`: its functions, as module-level functions.
	pub(crate) fn type_declaration(&mut self, n: NodeId) {
		let Some(decl) = self.analysis.type_decl(n).cloned() else {
			return;
		};
		for function in decl.functions {
			let node = self.analysis.symbols.symbol(function).decl_node;
			self.out.blank();
			self.function_declaration(node);
		}
	}

	/// The JavaScript names of the functions of the types declared in a
	/// list of statements: `Type$function`.
	pub(crate) fn type_function_names(&mut self, statements: &[NodeId]) {
		for &statement in statements {
			let Some(decl) = self.analysis.type_decl(statement).cloned() else {
				continue;
			};
			let type_name = self
				.types
				.syms
				.resolve(self.analysis.symbols.symbol(decl.symbol).name)
				.to_string();
			for function in decl.functions {
				let name = self
					.types
					.syms
					.resolve(self.analysis.symbols.symbol(function).name)
					.to_string();
				self.declare_as(function, &format!("{type_name}${name}"));
			}
		}
	}

	// ── places and writes ────────────────────────────────────────────────

	/// An assignable place: a name, a field or an array item, its container
	/// and index evaluated once (temporaries before the statement, unless
	/// `direct`: the value needs no statement, so local names can't change
	/// before the write).
	pub(crate) fn place(&mut self, target: NodeId, direct: bool) -> Option<Place> {
		let target = self.strip_groups(target);
		match self.view(target) {
			View::Name(_) => {
				let ty = self.type_of(target);
				// an `export mutable` value of another file (modules.rs)
				if let Some(text) = self.external_place(target) {
					return Some(Place { text, ty });
				}
				let Resolution::Local(symbol) = self.analysis.resolution(target) else {
					return None;
				};
				let text = self.name_of(symbol);
				let ty = self.analysis.symbols.symbol(symbol).ty;
				Some(Place { text, ty })
			}
			View::Access {
				target: container,
				members,
			} if {
				let members: Vec<NodeId> = members.iter().collect();
				self.namespace_prefix_place(container, &members).is_some()
			} =>
			{
				let members: Vec<NodeId> = members.iter().collect();
				self.namespace_prefix_place(container, &members)
			}
			View::Access {
				target: container,
				members,
			} => {
				let last = members.last()?;
				let mut value = self.stable_value(container);
				let count = members.len();
				for member in members.iter().take(count - 1) {
					value = self.apply_member(value, member, false);
				}
				let (shape, index) = match self.analysis.resolution(last) {
					Resolution::Field(shape, index) => (shape, index),
					Resolution::Builtin(builtin) => {
						let field = builtins::get(builtin).path.strip_prefix("Range#")?;
						return Some(Place {
							text: format!("{}.{field}", value.primary_text()),
							ty: self.type_of(last),
						});
					}
					_ => return None,
				};
				let field = self.types.shape_fields(shape)[index as usize];
				let property = Self::property(self.types.syms.resolve(field.name));
				Some(Place {
					text: format!("{}.{property}", value.primary_text()),
					ty: field.ty,
				})
			}
			View::Index {
				target: container,
				index,
			} => {
				let values = self.in_order(&[container, index], |g, node| g.expr(node));
				let (container_value, mut position) = if direct {
					(
						self.keep_place_part(container, values[0].clone()),
						self.keep_place_part(index, values[1].clone()),
					)
				} else {
					(self.keep(values[0].clone()), self.keep(values[1].clone()))
				};
				if self.has(target, NodeFlags::INDEX_CHECK) {
					let helper = self.helper("faNormalizeIndex");
					let temp = self.temp();
					self.out.line(format!(
						"const {temp} = {helper}({}.length, {});",
						container_value.primary_text(),
						position.text
					));
					position = Expr::name(temp, true);
				}
				Some(Place {
					text: format!("{}[{}]", container_value.primary_text(), position.text),
					ty: self.type_of(target),
				})
			}
			_ => None,
		}
	}

	/// The container or the index of a place: a read of a local name stays
	/// itself (nothing between the read and the write of the place can
	/// assign it), anything else is kept once.
	fn keep_place_part(&mut self, node: NodeId, value: Expr) -> Expr {
		let node = self.strip_groups(node);
		let local = matches!(self.view(node), View::Name(_))
			&& match self.analysis.resolution(node) {
				Resolution::Local(symbol) => {
					!self.is_module_value(symbol) && !self.aliases.contains_key(&symbol)
				}
				_ => false,
			};
		if local && value.pure {
			return value;
		}
		self.keep(value)
	}

	/// A value evaluated once: stable, or kept in a temporary.
	fn keep(&mut self, value: Expr) -> Expr {
		if value.stable {
			return value;
		}
		let temp = self.temp();
		self.out.line(format!("const {temp} = {};", value.text));
		Expr::name(temp, true)
	}

	/// The value of a container: a name stays itself (a later read of the
	/// same object is the same object), anything else is kept once.
	fn stable_value(&mut self, container: NodeId) -> Expr {
		let value = self.expr(container);
		if matches!(self.view(self.strip_groups(container)), View::Name(_)) {
			return value;
		}
		self.keep(value)
	}

	/// `target .= { field = value, … }`: every value first, then the writes.
	pub(crate) fn update_fields(&mut self, target: NodeId, value: NodeId) {
		let View::Object { items } = self.view(value) else {
			self.unsupported(value, "this `.=`");
			return;
		};
		let container = self.stable_value(target);
		let mut fields = Vec::new();
		for item in items.iter() {
			match self.view(item) {
				View::Assignment {
					target: name,
					value: Some(value),
					..
				} => fields.push((self.text(name).to_string(), value)),
				View::Name(_) => fields.push((self.text(item).to_string(), item)),
				_ => {
					self.unsupported(item, "this field of `.=`");
					return;
				}
			}
		}
		let nodes: Vec<NodeId> = fields.iter().map(|&(_, value)| value).collect();
		let values = self.in_order(&nodes, |g, node| {
			let value = g.expr(node);
			g.copy_stored(node, value)
		});
		// all values before the first write (`p .= { x = p.y, y = p.x }` swaps)
		let values: Vec<Expr> = if values.len() > 1 {
			values.into_iter().map(|value| self.keep(value)).collect()
		} else {
			values
		};
		for ((name, _), value) in fields.iter().zip(values) {
			self.out.line(format!(
				"{}.{} = {};",
				container.primary_text(),
				Self::property(name),
				value.text
			));
		}
	}

	// ── copies ───────────────────────────────────────────────────────────

	/// Whether values of a type are JavaScript objects that Fa copies.
	pub(crate) fn is_composite(&self, ty: TypeId) -> bool {
		match self.types.kind(ty) {
			TypeKind::Object(_)
			| TypeKind::Tuple(_)
			| TypeKind::Collection(..)
			| TypeKind::NamedUnion(..)
			| TypeKind::Member(..)
			| TypeKind::Range => true,
			TypeKind::Union(list) => self
				.types
				.list_items(list)
				.iter()
				.any(|&member| self.is_composite(member)),
			_ => false,
		}
	}

	/// The symbol at the root of a place expression, if the node reads a place.
	pub(crate) fn place_root(&self, n: NodeId) -> Option<SymbolId> {
		match self.view(n) {
			View::Group(inner) => self.place_root(inner),
			View::Name(_) => match self.analysis.resolution(n) {
				Resolution::Local(symbol) => Some(symbol),
				_ => None,
			},
			View::Access { target, .. } | View::Index { target, .. } => self.place_root(target),
			_ => None,
		}
	}

	/// A composite value read from a place, stored into a mutable
	/// destination (or from a mutable place): a copy, so that the two never
	/// share their contents. Fresh values (literals, call results) are not
	/// copied.
	pub(crate) fn copy_into(
		&mut self,
		node: NodeId,
		value: Expr,
		destination_mutable: bool,
	) -> Expr {
		let ty = self.type_of(node);
		if !self.is_composite(ty) {
			return value;
		}
		let Some(root) = self.place_root(node) else {
			return value;
		};
		let source_mutable = self.is_mutable_place(root);
		if !destination_mutable && !source_mutable {
			return value;
		}
		self.copy_of(value, ty)
	}

	/// Whether the place rooted at a symbol may change later (its parts
	/// are copied when bound elsewhere).
	pub(crate) fn is_mutable_place(&self, root: SymbolId) -> bool {
		!self.is_constant(root)
			|| matches!(
				self.analysis.symbols.symbol(root).kind,
				SymbolKind::LoopVariable { copy: false }
			)
	}

	/// A copy of a composite value of a type.
	pub(crate) fn copy_value(&mut self, value: Expr, ty: TypeId) -> Expr {
		self.copy_of(value, ty)
	}

	/// A value stored inside another one (an item, a field): always a
	/// copy when read from a place.
	pub(crate) fn copy_stored(&mut self, node: NodeId, value: Expr) -> Expr {
		self.copy_into(node, value, true)
	}

	/// A copy of a value of a type: `slice()` for an array of primitives,
	/// `faCopy` otherwise.
	fn copy_of(&mut self, value: Expr, ty: TypeId) -> Expr {
		let primitive_items = match self.types.kind(ty) {
			TypeKind::Collection(CollKind::Array, item, _) => !self.is_composite(item),
			TypeKind::Tuple(list) => self
				.types
				.list_items(list)
				.iter()
				.all(|&item| !self.is_composite(item)),
			_ => false,
		};
		if primitive_items {
			return Expr::primary(format!("{}.slice()", value.primary_text()));
		}
		if let Some(function) = self.copy_function(ty) {
			return js::call(&function, &[value.text]);
		}
		let helper = self.helper("faCopy");
		js::call(helper, &[value.text])
	}

	/// The generated copy function of a type (`copy$Point`): an object
	/// literal with the fields in declaration order (one hidden class), the
	/// composite fields copied by their own functions; `None` for the types
	/// copied by `faCopy` (unions, maps, sets).
	fn copy_function(&mut self, ty: TypeId) -> Option<String> {
		if let Some(name) = self.copies.get(&ty) {
			// empty while being generated: a recursive type, copied by `faCopy`
			return (!name.is_empty()).then(|| name.clone());
		}
		let (wanted, body) = match self.types.kind(ty) {
			TypeKind::Object(_) => {
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
				// the declaration order, when the type is declared
				let order: Vec<String> = self
					.analysis
					.type_decls
					.values()
					.find(|decl| decl.ty == ty)
					.map(|decl| {
						decl.fields
							.iter()
							.map(|field| Self::property(self.types.syms.resolve(field.name)))
							.collect()
					})
					.unwrap_or_else(|| fields.iter().map(|(name, _)| name.clone()).collect());
				let mut parts = Vec::new();
				self.copies.insert(ty, String::new());
				for name in order {
					let Some(&(_, field_ty)) = fields.iter().find(|(field, _)| *field == name)
					else {
						continue;
					};
					let read = Expr::primary(format!("value.{name}"));
					let copied = if self.is_composite(field_ty) {
						match self.copy_field(read, field_ty) {
							Some(copied) => copied,
							None => {
								self.copies.remove(&ty);
								return None;
							}
						}
					} else {
						read
					};
					parts.push(format!("{name}: {}", copied.text));
				}
				self.copies.remove(&ty);
				let wanted = match self.types.type_name(ty) {
					Some(name) => format!("copy${name}"),
					None => "copy$object".to_string(),
				};
				let object = if parts.is_empty() {
					"{}".to_string()
				} else {
					format!("{{ {} }}", parts.join(", "))
				};
				(wanted, format!("(value) => ({object})"))
			}
			TypeKind::Range => {
				let class = self.helper("FaRange");
				(
					"copy$Range".to_string(),
					format!("(value) => new {class}(value.start, value.end, value.includesEnd)"),
				)
			}
			TypeKind::Collection(CollKind::Array, item, _) if self.is_composite(item) => {
				let item_copy = self.copy_function(item)?;
				let wanted = format!("{item_copy}$array");
				(wanted, format!("(value) => value.map({item_copy})"))
			}
			_ => return None,
		};
		let name = self.constant(&wanted, body);
		self.copies.insert(ty, name.clone());
		Some(name)
	}

	/// The copy function of a type, for a value of another type stored as
	/// it (its fields only).
	pub(crate) fn narrowing_copy(&mut self, ty: TypeId) -> Option<String> {
		if let TypeKind::Collection(_, item, _) = self.types.kind(ty) {
			if !matches!(self.types.kind(item), TypeKind::Object(_)) {
				return None;
			}
			let item_copy = self.copy_function(item)?;
			return Some(self.constant(
				&format!("{item_copy}$array"),
				format!("(value) => value.map({item_copy})"),
			));
		}
		self.copy_function(ty)
	}

	/// The `==` of two values of an object type: its fields only (the
	/// operands may be of wider types, structural typing), in declaration
	/// order; `None` for other types.
	pub(crate) fn equality_function(&mut self, ty: TypeId) -> Option<String> {
		if !matches!(self.types.kind(ty), TypeKind::Object(_)) {
			return None;
		}
		if let Some(name) = self.equalities.get(&ty) {
			return (!name.is_empty()).then(|| name.clone());
		}
		self.equalities.insert(ty, String::new());
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
		let mut tests = Vec::new();
		for (name, field_ty) in fields {
			let test = match self.repr(field_ty) {
				crate::generate::Repr::Other => match self.equality_function(field_ty) {
					Some(equals) => format!("{equals}(a.{name}, b.{name})"),
					None => {
						let helper = self.helper("faEquals");
						format!("{helper}(a.{name}, b.{name})")
					}
				},
				_ => format!("a.{name} === b.{name}"),
			};
			tests.push(test);
		}
		let body = if tests.is_empty() {
			"true".to_string()
		} else {
			tests.join(" && ")
		};
		let wanted = match self.types.type_name(ty) {
			Some(name) => format!("equals${name}"),
			None => "equals$object".to_string(),
		};
		let name = self.constant(&wanted, format!("(a, b) => {body}"));
		self.equalities.insert(ty, name.clone());
		Some(name)
	}

	/// The copy of a composite field read as `value.name`.
	fn copy_field(&mut self, read: Expr, ty: TypeId) -> Option<Expr> {
		match self.types.kind(ty) {
			TypeKind::Collection(CollKind::Array, item, _) if !self.is_composite(item) => {
				Some(Expr::primary(format!("{}.slice()", read.text)))
			}
			TypeKind::Union(list) => {
				// an optional composite: `none` stays `none`
				let members = self.types.list_items(list).to_vec();
				let [first, second] = members.as_slice() else {
					return None;
				};
				let inner = if *first == TypeId::NONE {
					*second
				} else if *second == TypeId::NONE {
					*first
				} else {
					return None;
				};
				let copied = self.copy_field(read.clone(), inner)?;
				Some(Expr::new(
					format!("{} === undefined ? undefined : {}", read.text, copied.text),
					Prec::Conditional,
				))
			}
			_ => {
				let function = self.copy_function(ty)?;
				Some(js::call(&function, &[read.text]))
			}
		}
	}

	/// `into` for an assignment of a composite value: the copy rule.
	#[allow(dead_code)]
	pub(crate) fn dest_of(&self, name: String) -> Dest {
		Dest::Assign(name)
	}
}
