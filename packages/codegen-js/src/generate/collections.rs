//! Sets, maps and `from` (types brief §5.0.2, §5.3, §5.4).
//!
//! A set or a map whose elements (keys) are primitive values is a
//! JavaScript `Set` or `Map`: their SameValueZero equality is Fa's `==` on
//! numbers, strings and booleans. Other elements (objects, tuples) go in the
//! runtime's `FaSet` and `FaHashMap`, which compare them with Fa's `==`, and
//! so does a set with a `key` function. Both kinds have the same interface,
//! so the code using a collection doesn't depend on its kind.

use fa_analyzer::analysis::{Argument, NodeFlags};
use fa_analyzer::ids::{NodeId, TypeId};
use fa_analyzer::syntax::{NodeList, View};
use fa_analyzer::types::{CollKind, TypeKind};

use crate::generate::{Generator, Repr};
use crate::js::{self, Expr, Prec};

impl Generator<'_> {
	/// Whether values of a type are compared by a JavaScript `Set` or `Map`
	/// as Fa compares them: numbers, strings, booleans, bigints.
	pub(crate) fn is_primitive(&self, ty: TypeId) -> bool {
		match self.types.kind(ty) {
			TypeKind::Union(list) => self
				.types
				.list_items(list)
				.iter()
				.all(|&member| self.is_primitive(member)),
			TypeKind::Enum(..) | TypeKind::EnumMember(..) => true,
			_ => matches!(
				self.repr(ty),
				Repr::Number | Repr::String | Repr::Boolean | Repr::BigInt | Repr::None
			),
		}
	}

	/// Whether a type is a pool of values that are not objects
	/// (`Pool<Integer>`): a `FaValuePool` of the runtime, whose elements
	/// have slots (references.rs).
	pub(crate) fn is_value_pool(&self, ty: TypeId) -> bool {
		matches!(
			self.types.kind(ty),
			TypeKind::Collection(CollKind::Pool | CollKind::GrowingPool | CollKind::Columnar | CollKind::GrowingColumnar, item, _)
				if !self.is_object_reference(item)
		)
	}

	/// A list literal whose type is a set or a map: `new Set([…])`,
	/// `new Map([[key, value], …])`; `None` for an array.
	pub(crate) fn collection_literal(&mut self, n: NodeId, items: NodeList<'_>) -> Option<Expr> {
		let TypeKind::Collection(kind, item, key) = self.types.kind(self.type_of(n)) else {
			return None;
		};
		let class = match kind {
			CollKind::Set | CollKind::GrowingSet => self.set_class(item),
			_ if is_pool(kind) && self.is_value_pool(self.type_of(n)) => {
				self.helper("FaValuePool").to_string()
			}
			_ if is_pool(kind) => "Set".to_string(),
			CollKind::Map | CollKind::GrowingMap => self.map_class(key),
			_ => return None,
		};
		if items.is_empty() {
			return Some(Expr::primary(format!("new {class}()")));
		}
		let values = if matches!(kind, CollKind::RelationPool(_)) {
			// a relation side holds references to the elements: never copies (D19)
			let nodes: Vec<NodeId> = items.iter().collect();
			let values = self.in_order(&nodes, |g, node| g.expr(node));
			let texts: Vec<String> = values.iter().map(|value| value.at(Prec::Assign)).collect();
			Expr::primary(format!("[{}]", texts.join(", ")))
		} else {
			self.array_literal(items)
		};
		Some(Expr::primary(format!("new {class}({})", values.text)))
	}

	/// The class of the sets of an element type.
	pub(crate) fn set_class(&mut self, item: TypeId) -> String {
		if self.is_primitive(item) {
			"Set".to_string()
		} else {
			self.helper("FaSet").to_string()
		}
	}

	/// The class of the maps of a key type.
	pub(crate) fn map_class(&mut self, key: TypeId) -> String {
		if self.is_primitive(key) {
			"Map".to_string()
		} else {
			self.helper("FaHashMap").to_string()
		}
	}

	/// `collection.reserve(additional)`, `text.reserve(additional)`: the
	/// engine manages the memory, so nothing but the argument is evaluated
	/// (minimal representations: `reserve` does nothing on JavaScript).
	pub(crate) fn reserve_call(&mut self, callee: NodeId, arguments: &[Argument]) -> Expr {
		let receiver = self.receiver(callee);
		let additional = match arguments.first() {
			Some(Argument::Given(value)) => Some(self.expr(*value)),
			_ => None,
		};
		let mut effects: Vec<Expr> = std::iter::once(receiver)
			.chain(additional)
			.filter(|value| !value.pure)
			.collect();
		match effects.len() {
			0 => Expr::constant("undefined", Prec::Primary),
			1 => effects.remove(0),
			_ => {
				let texts: Vec<String> =
					effects.iter().map(|value| value.at(Prec::Assign)).collect();
				Expr::new(format!("({})", texts.join(", ")), Prec::Primary)
			}
		}
	}

	/// The values of the arguments of a `from` without its capacity, a
	/// storage hint that JavaScript doesn't take. The hint is still
	/// evaluated, where it is written: the arguments written before it are
	/// first kept in temporaries when it has effects.
	fn before_capacity(&mut self, nodes: &[NodeId], capacity: Option<NodeId>) -> Vec<Expr> {
		let Some(capacity) = capacity else {
			return self.in_order(nodes, |g, node| g.expr(node));
		};
		let mut written: Vec<(usize, NodeId)> = nodes.iter().copied().enumerate().collect();
		written.push((nodes.len(), capacity));
		written.sort_by_key(|(_, node)| self.tree.span(node.index()).start);
		let ordered: Vec<NodeId> = written.iter().map(|(_, node)| *node).collect();
		let mut values: Vec<Option<Expr>> = self
			.in_order(&ordered, |g, node| g.expr(node))
			.into_iter()
			.map(Some)
			.collect();
		let at = written
			.iter()
			.position(|(index, _)| *index == nodes.len())
			.expect("the capacity");
		let hint = values[at].take().expect("the capacity");
		if !hint.pure {
			for earlier in values[..at].iter_mut().flatten() {
				if !earlier.stable {
					let temp = self.temp();
					self.out.line(format!("const {temp} = {};", earlier.text));
					*earlier = Expr::name(temp, true);
				}
			}
			self.out.line(format!("void ({});", hint.text));
		}
		let mut result: Vec<Option<Expr>> = vec![None; nodes.len()];
		for ((index, _), value) in written.iter().zip(values) {
			if let Some(value) = value {
				result[*index] = Some(value);
			}
		}
		result
			.into_iter()
			.map(|value| value.expect("a value"))
			.collect()
	}

	/// `String.from(text, capacity = n)`: the text itself (a JavaScript
	/// string is an immutable value), after the capacity is evaluated.
	pub(crate) fn string_from(&mut self, arguments: &[Argument]) -> Expr {
		let Some(Argument::Given(text)) = arguments.first() else {
			return Expr::constant("\"\"", Prec::Primary);
		};
		let capacity = match arguments.get(1) {
			Some(Argument::Given(capacity)) => Some(*capacity),
			_ => None,
		};
		self.before_capacity(&[*text], capacity).remove(0)
	}

	/// `Array.from(source)`, `Set.from(source)`, `Map.from(source)`:
	/// a new collection of the call's type with the source's items (the
	/// capacity is a hint JavaScript doesn't take: `before_capacity`).
	pub(crate) fn collection_from(
		&mut self,
		n: NodeId,
		path: &str,
		arguments: &[Argument],
	) -> Expr {
		let Some(Argument::Given(source)) = arguments.first() else {
			return self.internal_error(
				n,
				"the checked collection constructor has no source argument",
			);
		};
		let source = *source;
		let capacity = match arguments.get(1) {
			Some(Argument::Given(capacity)) => Some(*capacity),
			_ => None,
		};
		let values = self.before_capacity(&[source], capacity);
		let source_ty = self.type_of(source);
		let ty = self.type_of(n);
		let item = match self.types.kind(ty) {
			TypeKind::Collection(_, item, _) => item,
			_ => TypeId::UNKNOWN,
		};
		// the items read from a place are copied (values are never shared)
		let shared = self.place_root(source).is_some() && self.is_composite(item);
		let items = match self.types.kind(source_ty) {
			TypeKind::Collection(
				CollKind::Array | CollKind::InlineArray | CollKind::GrowingArray,
				..,
			)
			| TypeKind::Collection(CollKind::Set | CollKind::GrowingSet, ..)
			| TypeKind::Collection(CollKind::Map | CollKind::GrowingMap, ..) => values[0].clone(),
			_ => {
				let helper = self.helper("faIterable");
				js::call(helper, std::slice::from_ref(&values[0].text))
			}
		};
		let items = if shared {
			let helper = self.helper("faCopy");
			Expr::primary(format!("Array.from({}, {helper})", items.text))
		} else {
			items
		};
		match path {
			"Array.from" => {
				let fresh = matches!(
					self.view(self.strip_groups(source)),
					View::Array { .. } | View::Tuple { .. }
				) && matches!(
					self.types.kind(source_ty),
					TypeKind::Collection(
						CollKind::Array | CollKind::InlineArray | CollKind::GrowingArray,
						..
					)
				);
				if fresh || shared {
					items
				} else {
					js::call("Array.from", &[items.text])
				}
			}
			// a pool of values, a pool or a columnar of objects (a `Set`)
			"Pool.from" if self.is_value_pool(ty) => {
				let class = self.helper("FaValuePool");
				Expr::primary(format!("new {class}({})", items.text))
			}
			"Pool.from" => Expr::primary(format!("new Set({})", items.text)),
			"Set.from" => {
				let class = self.set_class(item);
				Expr::primary(format!("new {class}({})", items.text))
			}
			_ => {
				let key_ty = match self.types.kind(ty) {
					TypeKind::Collection(_, _, key) => key,
					_ => TypeId::UNKNOWN,
				};
				let class = self.map_class(key_ty);
				Expr::primary(format!("new {class}({})", items.text))
			}
		}
	}

	/// A method of a set or a map, on its receiver (callbacks of maps take
	/// `(key, value)`).
	pub(crate) fn collection_method(
		&mut self,
		n: NodeId,
		path: &str,
		receiver: Expr,
		values: Vec<Expr>,
	) -> Expr {
		let target = receiver.primary_text();
		let all = |rest: &[Expr]| {
			let mut all = vec![receiver.text.clone()];
			all.extend(rest.iter().map(|value| value.text.clone()));
			all
		};
		let method = |name: &str, rest: &[Expr]| {
			let rest: Vec<String> = rest.iter().map(|value| value.text.clone()).collect();
			Expr::primary(format!("{target}.{name}({})", rest.join(", ")))
		};
		let entries = Expr::constant("true", Prec::Primary);
		match path {
			"Set#isEmpty" | "Map#isEmpty" => {
				Expr::new(format!("{target}.size === 0"), Prec::Equality)
			}
			"Set#includes" | "Map#includes" => method("has", &values),
			"Set#delete" | "Map#delete" | "Map#get" => {
				method(path.split_once('#').map_or("", |(_, name)| name), &values)
			}
			"Set#clear" | "Map#clear" => method("clear", &[]),
			"Set#add" => {
				let helper = self.helper("faSetAdd");
				js::call(helper, &all(&values))
			}
			"Set#take" => {
				let helper = self.helper("faSetTake");
				js::call(helper, &all(&values))
			}
			"Map#take" => {
				let helper = self.helper("faMapTake");
				js::call(helper, &all(&values))
			}
			"Map#getOrInsert" => {
				let helper = self.helper("faMapGetOrInsert");
				js::call(helper, &all(&values))
			}
			"Set#deleteWhere" | "Map#deleteWhere" => {
				let helper = self.helper("faDeleteWhere");
				js::call(helper, &all(&values))
			}
			"Set#find" => {
				let helper = self.helper("faFind");
				js::call(helper, &all(&values))
			}
			"Map#find" => {
				let helper = self.helper("faMapFind");
				js::call(helper, &all(&values))
			}
			"Set#filter" => {
				let helper = self.helper("faSetFilter");
				js::call(helper, &all(&values))
			}
			"Map#filter" => {
				let helper = self.helper("faMapFilter");
				js::call(helper, &all(&values))
			}
			"Set#map" => {
				let function = values
					.first()
					.map_or_else(String::new, |value| value.text.clone());
				Expr::primary(format!("Array.from({}, {function})", receiver.text))
			}
			"Map#map" => {
				let helper = self.helper("faMapMap");
				js::call(helper, &all(&values))
			}
			"Set#some" | "Set#every" | "Map#some" | "Map#every" => {
				let helper = if path.ends_with("some") {
					self.helper("faSome")
				} else {
					self.helper("faEvery")
				};
				let mut rest = values.clone();
				if path.starts_with("Map") {
					rest.push(entries);
				}
				js::call(helper, &all(&rest))
			}
			"Set#reduce" | "Map#reduce" => {
				let helper = self.helper("faReduce");
				let mut rest = values.clone();
				if path.starts_with("Map") {
					rest.push(entries);
				}
				js::call(helper, &all(&rest))
			}
			"Set#sum" => {
				let item = match self.types.kind(self.type_of(n)) {
					TypeKind::Int(id) if self.types.int(id).storage.is_js_bigint() => "0n",
					TypeKind::BigInt => "0n",
					_ => "0",
				};
				let helper = self.helper("faReduce");
				js::call(
					helper,
					&[
						receiver.text.clone(),
						"(a, b) => a + b".to_string(),
						item.to_string(),
					],
				)
			}
			"Set#union" | "Set#intersection" | "Set#difference" | "Set#isSubsetOf" => {
				let helper = match path {
					"Set#union" => "faSetUnion",
					"Set#intersection" => "faSetIntersection",
					"Set#difference" => "faSetDifference",
					_ => "faSetIsSubsetOf",
				};
				let helper = self.helper(helper);
				js::call(helper, &all(&values))
			}
			_ => self.unsupported(n, &format!("`{path}`")),
		}
	}

	/// A method of a pool or a columnar (types brief §5.5, §5.6;
	/// `collections/pools.md`). On JavaScript, a `Pool` or a `Columnar` of
	/// objects is a `Set` of these objects: a reference is the object, which
	/// keeps its identity until deleted; an `OrderedColumnar` is an array of
	/// objects (its methods are the array's). Pools of primitive values use
	/// stable slots (`FaValuePool`), without boxing object pools.
	pub(crate) fn pool_call(
		&mut self,
		n: NodeId,
		callee: NodeId,
		path: &str,
		arguments: &[Argument],
	) -> Expr {
		let (owner, name) = path.split_once('#').unwrap_or((path, ""));
		let receiver_type = self.receiver_type(callee);
		if self.is_value_pool(receiver_type) {
			return self.value_pool_call(n, callee, name, arguments);
		}
		if owner == "OrderedColumnar" && name != "add" {
			return self.array_method(n, callee, &format!("Array#{name}"), arguments);
		}

		let receiver = self.receiver(callee);
		let nodes: Vec<NodeId> = arguments
			.iter()
			.filter_map(|argument| match argument {
				Argument::Given(value) => Some(*value),
				_ => None,
			})
			.collect();
		// `add` stores a copy of a value read from a place; the other
		// methods take the element itself (a reference)
		let (lines, values) = self.capture(|g| {
			g.in_order(&nodes, |g, node| {
				let value = g.expr(node);
				if name == "add" {
					g.copy_stored(node, value)
				} else {
					value
				}
			})
		});
		let receiver = self.keep_before(receiver, &lines);
		self.out.append(lines);
		// a removal from a related collection unlinks first (relations.rs)
		let values = match self.unlink_table(n, callee) {
			Some((table, owner)) => {
				let values: Vec<Expr> = values
					.into_iter()
					.map(|value| self.keep_value(value))
					.collect();
				let unlink = self.helper("faUnlink");
				// reactive sides: the other elements' sides notify (relations.rs)
				let owner = match self.relation_notify(n) {
					Some(notify) => format!("{owner}, {notify}"),
					None => owner,
				};
				let (before, values) = crate::generate::relations::unlinking_call(
					unlink, &table, &owner, name, &receiver, &values,
				);
				if let Some(before) = before {
					self.out.line(before);
				}
				values
			}
			None => values,
		};
		let mut all = vec![receiver.text.clone()];
		all.extend(values.iter().map(|value| value.text.clone()));
		match name {
			"add" if self.analysis.relation_writes.contains_key(&n) => {
				// a literal setting direct sides: linked once stored
				let helper = self.helper("faAdd");
				let element = self.temp();
				self.out.line(format!(
					"const {element} = {};",
					js::call(helper, &all).text
				));
				if let Some(literal) = nodes.first() {
					self.link_added(n, *literal, &element);
				}
				Expr::name(element, true)
			}
			"add" => {
				let helper = self.helper("faAdd");
				js::call(helper, &all)
			}
			"includes" => {
				let helper = self.helper("faArrayIncludes");
				js::call(helper, &all)
			}
			// `clear()` after unlinking: the elements are removed one by one
			_ => self.collection_method(n, &format!("Set#{name}"), receiver, values),
		}
	}

	/// A method of a pool of values (`FaValuePool`): `delete` and `take`
	/// take the slot of the reference they are given; the other methods
	/// read the values.
	fn value_pool_call(
		&mut self,
		n: NodeId,
		callee: NodeId,
		name: &str,
		arguments: &[Argument],
	) -> Expr {
		let receiver = self.receiver(callee);
		let nodes: Vec<NodeId> = arguments
			.iter()
			.filter_map(|argument| match argument {
				Argument::Given(value) => Some(*value),
				_ => None,
			})
			.collect();
		if let ("delete" | "take", Some(&element)) = (name, nodes.first()) {
			let (lines, slot) = self.capture(|g| g.reference_slot(element));
			let receiver = self.keep_before(receiver, &lines);
			self.out.append(lines);
			let Some(slot) = slot else {
				return self.unsupported(n, "removing this element without a retained pool slot");
			};
			return Expr::primary(format!("{}.{name}({})", receiver.primary_text(), slot.text));
		}
		let (lines, values) = self.capture(|g| g.in_order(&nodes, |g, node| g.expr(node)));
		let receiver = self.keep_before(receiver, &lines);
		self.out.append(lines);
		let texts: Vec<String> = values.iter().map(|value| value.text.clone()).collect();
		let method = |method: &str| {
			Expr::primary(format!(
				"{}.{method}({})",
				receiver.primary_text(),
				texts.join(", ")
			))
		};
		match name {
			"add" => method("addValue"),
			"deleteWhere" | "filter" | "clear" => method(name),
			"includes" => {
				let helper = self.helper("faArrayIncludes");
				let array = format!("Array.from({})", receiver.text);
				js::call(helper, &[array, texts.first().cloned().unwrap_or_default()])
			}
			_ => self.collection_method(n, &format!("Set#{name}"), receiver, values),
		}
	}

	/// `map[key]`: `KeyNotFound` unless the analyzer proved the key present.
	pub(crate) fn map_index(&mut self, n: NodeId, map: Expr, key: Expr, key_ty: TypeId) -> Expr {
		if !self.has(n, NodeFlags::INDEX_CHECK) {
			return Expr::primary(format!("{}.get({})", map.primary_text(), key.text));
		}
		let helper = self.helper("faMapIndex");
		let mut arguments = vec![map.text, key.text];
		if let Some(formatter) = self.formatter(key_ty) {
			arguments.push(formatter);
		}
		js::call(helper, &arguments)
	}
}

/// Whether a collection is a pool or an unordered columnar: a JavaScript
/// `Set` of objects.
pub(crate) fn is_pool(kind: CollKind) -> bool {
	matches!(
		kind,
		CollKind::Pool
			| CollKind::GrowingPool
			| CollKind::Columnar
			| CollKind::GrowingColumnar
			| CollKind::RelationPool(_)
	)
}
