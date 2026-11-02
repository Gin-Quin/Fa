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

	/// A list literal whose type is a set or a map: `new Set([…])`,
	/// `new Map([[key, value], …])`; `None` for an array.
	pub(crate) fn collection_literal(&mut self, n: NodeId, items: NodeList<'_>) -> Option<Expr> {
		let TypeKind::Collection(kind, item, key) = self.types.kind(self.type_of(n)) else {
			return None;
		};
		let class = match kind {
			CollKind::Set | CollKind::GrowingSet => self.set_class(item),
			CollKind::Map | CollKind::GrowingMap => self.map_class(key),
			_ => return None,
		};
		if items.is_empty() {
			return Some(Expr::primary(format!("new {class}()")));
		}
		let values = self.array_literal(items);
		Some(Expr::primary(format!("new {class}({})", values.text)))
	}

	/// The class of the sets of an element type.
	fn set_class(&mut self, item: TypeId) -> String {
		if self.is_primitive(item) {
			"Set".to_string()
		} else {
			self.helper("FaSet").to_string()
		}
	}

	/// The class of the maps of a key type.
	fn map_class(&mut self, key: TypeId) -> String {
		if self.is_primitive(key) {
			"Map".to_string()
		} else {
			self.helper("FaHashMap").to_string()
		}
	}

	/// `Array.from(source)`, `Set.from(source, key = …)`, `Map.from(source)`:
	/// a new collection of the call's type with the source's items (the
	/// capacity is a hint JavaScript doesn't take).
	pub(crate) fn collection_from(&mut self, n: NodeId, path: &str, arguments: &[Argument]) -> Expr {
		let Some(Argument::Given(source)) = arguments.first() else {
			return self.unsupported(n, &format!("`{path}` without a source"));
		};
		let source = *source;
		let key = match (path, arguments.get(1)) {
			("Set.from", Some(Argument::Given(key))) => Some(*key),
			_ => None,
		};
		let nodes: Vec<NodeId> = std::iter::once(source).chain(key).collect();
		let values = self.in_order(&nodes, |g, node| g.expr(node));
		let source_ty = self.type_of(source);
		let ty = self.type_of(n);
		let item = match self.types.kind(ty) {
			TypeKind::Collection(_, item, _) => item,
			_ => TypeId::UNKNOWN,
		};
		// the items read from a place are copied (values are never shared)
		let shared = self.place_root(source).is_some() && self.is_composite(item);
		let items = match self.types.kind(source_ty) {
			TypeKind::Collection(CollKind::Array | CollKind::GrowingArray, ..)
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
					TypeKind::Collection(CollKind::Array | CollKind::GrowingArray, ..)
				);
				if fresh || shared {
					items
				} else {
					js::call("Array.from", &[items.text])
				}
			}
			"Set.from" => match key.map(|_| values[1].clone()) {
				Some(key) => {
					let class = self.helper("FaSet");
					Expr::primary(format!(
						"new {class}({}, {{ key: {} }})",
						items.text,
						key.at(Prec::Assign)
					))
				}
				None => {
					let class = self.set_class(item);
					Expr::primary(format!("new {class}({})", items.text))
				}
			},
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
