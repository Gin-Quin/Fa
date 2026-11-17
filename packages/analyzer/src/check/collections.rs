//! Sets and maps (types brief §5.3, §5.4) and the collection functions that
//! every kind shares: map literals, the members of `Set`, `GrowingSet`,
//! `Map` and `GrowingMap`, `C.from(source, options…)` (§5.0.2, D59), map
//! indexing and the `(key, value)` loop pattern. Arrays are in `arrays.rs`.
//!
//! The effects of every operation are the table of `builtins::array::effects`
//! (§5.0.9); the method types carry them in `FnSig::effects`.

use std::sync::Arc;

use crate::analysis::{Argument, CallInfo, Callee, NodeFlags};
use crate::builtins::{self, array::is_removal};
use crate::check::Walker;
use crate::check::objects::MemberKind;
use crate::diagnostics::codes::types::{CL005, CL006, CL007, CL010, CL040, FN023, WCL001};
use crate::diagnostics::codes::values::E_DECL_3;
use crate::ids::{NodeId, SymbolId, TypeId};
use crate::symbols::{Resolution, SymbolKind};
use crate::syntax::{self, NodeList, View};
use crate::types::{CollKind, IntStorage, TypeKind};

impl Walker<'_> {
	// ── literals ─────────────────────────────────────────────────────────

	/// `[(key, value), …]` where a map is expected: every item is a pair
	/// (`CL005`), checked against the key and value types; two equal literal
	/// keys are `CL006` (D61).
	pub(super) fn check_map_literal(
		&mut self,
		n: NodeId,
		items: NodeList<'_>,
		kind: CollKind,
		key: TypeId,
		value: TypeId,
	) -> TypeId {
		let tree = Arc::clone(&self.tree);
		let pair = self.s.types.tuple(&[key, value]);
		let mut not_pairs = Vec::new();
		let mut keys = Vec::new();
		for item in items.iter() {
			let mut inner = item;
			while let View::Group(group) = syntax::view(&tree, inner) {
				inner = group;
			}
			match syntax::view(&tree, inner) {
				View::Tuple { items: parts } if parts.len() == 2 => {
					let (Some(key_node), Some(value_node)) = (parts.get(0), parts.get(1)) else {
						continue;
					};
					let actual = self.check(key_node, Some(key));
					self.sink(key_node, actual, key);
					self.store_closures(key_node, n);
					let actual = self.check(value_node, Some(value));
					self.sink(value_node, actual, value);
					self.store_closures(value_node, n);
					self.memo(inner, |_| pair);
					self.memo(item, |_| pair);
					keys.push(key_node);
				}
				View::Spread(_) => {
					self.unsupported(item, "spreads in maps");
				}
				_ => {
					let actual = self.check(item, Some(pair));
					if matches!(self.s.types.kind(actual), TypeKind::Tuple(list) if self.s.types.list_items(list).len() == 2)
					{
						self.sink(item, actual, pair);
					} else if !self.s.types.is_recovering(actual) {
						not_pairs.push(item);
					}
				}
			}
		}
		self.memo(n, |w| {
			for item in not_pairs {
				let diagnostic = w.diagnostic(
					&CL005,
					item,
					"the items of a map are `(key, value)` pairs, ex: `[(\"ana\", 12)]`",
				);
				w.report(diagnostic);
			}
			let mut seen: Vec<String> = Vec::new();
			for key_node in keys {
				let Some(text) = w.literal_text(key_node) else {
					continue;
				};
				if seen.contains(&text) {
					let message = format!("the key {text} is given twice");
					let diagnostic = w.diagnostic(&CL006, key_node, message);
					w.report(diagnostic);
				} else {
					seen.push(text);
				}
			}
			w.s.types.collection(kind, value, key)
		})
	}

	/// The source text of a compile-time literal (a number, a string, a
	/// boolean, a tuple of literals), for comparing literal keys.
	fn literal_text(&self, n: NodeId) -> Option<String> {
		match self.view(n) {
			View::Integer(_) | View::Float(_) | View::Boolean(_) | View::None => {
				Some(self.tree.text(n.index()).to_string())
			}
			View::String(text) => Some(format!("\"{text}\"")),
			View::Negate(inner) => self.literal_text(inner).map(|text| format!("-{text}")),
			View::Group(inner) => self.literal_text(inner),
			View::Tuple { items } => {
				let parts: Option<Vec<String>> =
					items.iter().map(|item| self.literal_text(item)).collect();
				parts.map(|parts| format!("({})", parts.join(", ")))
			}
			_ => None,
		}
	}

	// ── members ──────────────────────────────────────────────────────────

	/// A member of a set or a map value (types brief §5.3, §5.4): `size`
	/// and the methods, typed from the item and key types. `CL007` for
	/// `length`, `CL040` for a removal on a growing collection.
	pub(super) fn collection_member(
		&mut self,
		member: NodeId,
		receiver: TypeId,
		text: &str,
	) -> Option<(TypeId, MemberKind)> {
		let TypeKind::Collection(kind, item, key) = self.s.types.kind(receiver) else {
			return None;
		};
		if let CollKind::RelationPool(side) = kind {
			return self.relation_pool_member(member, receiver, side == 1, item, text);
		}
		if super::pools::pool_owner(kind).is_some() {
			return self.pool_member(member, receiver, kind, item, text);
		}
		let owner = match kind {
			CollKind::Set | CollKind::GrowingSet => "Set",
			CollKind::Map | CollKind::GrowingMap => "Map",
			_ => {
				let message = format!(
					"the members of a `{}` are not supported yet by the analyzer",
					kind.name()
				);
				let diagnostic =
					self.diagnostic(&crate::diagnostics::codes::analyzer::AN001, member, message);
				self.report(diagnostic);
				return None;
			}
		};
		if text == "length" {
			let message = format!(
				"a `{}` has a `size`, not a `length`: it has no positions",
				kind.name()
			);
			let diagnostic = self.diagnostic(&CL007, member, message);
			self.report(diagnostic);
			return None;
		}
		if self.growing_lacks(member, kind, text) {
			return None;
		}
		let builtin = builtins::by_path(&format!("{owner}#{text}"));
		let ty = match owner {
			"Set" => self.set_method_type(kind, text, item),
			_ => self.map_method_type(kind, text, key, item),
		};
		match (builtin, ty) {
			(Some(builtin), Some(ty)) => {
				self.set_resolution(member, Resolution::Builtin(builtin));
				Some((self.with_effects(ty, kind, text), MemberKind::Builtin))
			}
			_ => {
				self.no_member(member, receiver, text);
				None
			}
		}
	}

	/// `CL040`: a growing collection has no removal method. Type part.
	pub(super) fn growing_lacks(&mut self, member: NodeId, kind: CollKind, text: &str) -> bool {
		let growing = matches!(
			kind,
			CollKind::GrowingArray
				| CollKind::GrowingSet
				| CollKind::GrowingMap
				| CollKind::GrowingPool
				| CollKind::GrowingColumnar
		);
		if !growing || !is_removal(text) {
			return false;
		}
		let message = format!(
			"a `{}` never removes elements: it has no `{text}` (a `{}` has)",
			kind.name(),
			kind.name().trim_start_matches("Growing")
		);
		let diagnostic = self.diagnostic(&CL040, member, message);
		self.report(diagnostic);
		true
	}

	/// `Integer(min = 0)`: sizes and counts.
	fn natural(&mut self) -> TypeId {
		let storage = IntStorage::Default;
		self.s
			.types
			.int_range(ethnum::I256::ZERO, storage.range().hi, storage)
	}

	/// The methods of a set (types brief §5.4, D34, D37).
	fn set_method_type(&mut self, kind: CollKind, name: &str, item: TypeId) -> Option<TypeId> {
		let set = self.s.types.collection(CollKind::Set, item, TypeId::NONE);
		let optional_item = self.s.types.optional(item);
		let predicate = self.callback_with(&[item], TypeId::BOOLEAN, 0);
		Some(match name {
			"size" | "capacity" => return Some(self.natural()),
			"reserve" => {
				let natural = self.natural();
				self.method(&[natural], TypeId::VOID)
			}
			"isEmpty" => self.method(&[], TypeId::BOOLEAN),
			"includes" | "add" | "delete" => self.method(&[item], TypeId::BOOLEAN),
			"take" => self.method(&[item], optional_item),
			"deleteWhere" => self.method(&[predicate], TypeId::VOID),
			"clear" => self.method(&[], TypeId::VOID),
			"find" => self.method(&[predicate], optional_item),
			"filter" => self.method(&[predicate], set),
			"every" | "some" => self.method(&[predicate], TypeId::BOOLEAN),
			"map" => {
				let result = self.builtin_param("Set.map", "Result", TypeId::UNKNOWN);
				let transform = self.callback_with(&[item], result, 0);
				let mapped = self.s.types.array(result);
				self.method(&[transform], mapped)
			}
			"reduce" => {
				let accumulator = self.builtin_param("Set.reduce", "Accumulator", TypeId::UNKNOWN);
				let combine = self.callback_with(&[accumulator, item], accumulator, 0);
				self.method(&[combine, accumulator], accumulator)
			}
			"sum" => {
				let sum = self.array_method_type("sum", item)?;
				let ret = self.signature_of(sum)?.ret;
				self.method(&[], ret)
			}
			"union" | "intersection" | "difference" => {
				let other = self.s.types.collection(kind, item, TypeId::NONE);
				self.method(&[other], set)
			}
			"values" => self.values_method(item),
			"isSubsetOf" => {
				let other = self.s.types.collection(kind, item, TypeId::NONE);
				self.method(&[other], TypeId::BOOLEAN)
			}
			_ => return None,
		})
	}

	/// The methods of a map (types brief §5.3, D34, D36, D37). Callbacks
	/// receive `(key, value)`.
	fn map_method_type(
		&mut self,
		_kind: CollKind,
		name: &str,
		key: TypeId,
		value: TypeId,
	) -> Option<TypeId> {
		let map = self.s.types.collection(CollKind::Map, value, key);
		let optional_value = self.s.types.optional(value);
		let predicate = self.callback_with(&[key, value], TypeId::BOOLEAN, 0);
		Some(match name {
			"size" | "capacity" => return Some(self.natural()),
			"reserve" => {
				let natural = self.natural();
				self.method(&[natural], TypeId::VOID)
			}
			"isEmpty" => self.method(&[], TypeId::BOOLEAN),
			"includes" | "delete" => self.method(&[key], TypeId::BOOLEAN),
			"get" | "take" => self.method(&[key], optional_value),
			"getOrInsert" => self.method(&[key, value], value),
			"values" => self.values_method(value),
			"keys" => self.values_method(key),
			"deleteWhere" => self.method(&[predicate], TypeId::VOID),
			"clear" => self.method(&[], TypeId::VOID),
			"find" => {
				let pair = self.s.types.tuple(&[key, value]);
				let found = self.s.types.optional(pair);
				self.method(&[predicate], found)
			}
			"filter" => self.method(&[predicate], map),
			"every" | "some" => self.method(&[predicate], TypeId::BOOLEAN),
			"map" => {
				let result = self.builtin_param("Map.map", "Result", TypeId::UNKNOWN);
				let transform = self.callback_with(&[key, value], result, 0);
				let mapped = self.s.types.collection(CollKind::Map, result, key);
				self.method(&[transform], mapped)
			}
			"reduce" => {
				let accumulator = self.builtin_param("Map.reduce", "Accumulator", TypeId::UNKNOWN);
				let combine = self.callback_with(&[accumulator, key, value], accumulator, 0);
				self.method(&[combine, accumulator], accumulator)
			}
			_ => return None,
		})
	}

	// ── `from` ───────────────────────────────────────────────────────────

	/// `Set.from(source, key = …)`, `Map.from(source)`, `Set(Float).from(…)`
	/// (types brief §5.0.2, D33, D59): a copy of any collection whose
	/// elements satisfy the element type, given or inferred from the source.
	/// Options are named: `capacity`, and `key` for sets; anything else is
	/// `FN023`. `WCL001` when a literal would do. `None` when the callee is
	/// not one (`Array.from` is in `from.rs`).
	pub(super) fn check_set_or_map_from(
		&mut self,
		n: NodeId,
		callee: NodeId,
		arguments: NodeList<'_>,
		expected: Option<TypeId>,
	) -> Option<TypeId> {
		let tree = Arc::clone(&self.tree);
		let View::Access { target, members } = syntax::view(&tree, callee) else {
			return None;
		};
		let member = members.first().filter(|_| members.len() == 1)?;
		if !matches!(syntax::view(&tree, member), View::Name(name) if tree.name(name) == "from") {
			return None;
		}
		// every collection but `Array` (from.rs)
		let set_or_map = |text: &str| {
			super::type_expr::collection_kind(text).filter(|kind| *kind != CollKind::Array)
		};
		// the collection: `Set`, or an application `Set(Float)` giving the elements
		let (kind, given) = match syntax::view(&tree, target) {
			View::Name(name) if !self.is_declared(name) => {
				let kind = set_or_map(tree.name(name))?;
				(kind, None)
			}
			View::Call { callee: inner, .. } => match syntax::view(&tree, inner) {
				View::Name(name) if !self.is_declared(name) => {
					let kind = set_or_map(tree.name(name))?;
					let ty = self.check_type_expr(target);
					match self.s.types.kind(ty) {
						TypeKind::Collection(_, item, key) => (kind, Some((item, key))),
						_ => (kind, Some((TypeId::UNKNOWN, TypeId::UNKNOWN))),
					}
				}
				_ => return None,
			},
			_ => return None,
		};
		let is_map = matches!(kind, CollKind::Map | CollKind::GrowingMap);
		let is_set = matches!(kind, CollKind::Set | CollKind::GrowingSet);
		let path = match kind {
			CollKind::Map => "Map.from",
			CollKind::GrowingMap => "GrowingMap.from",
			CollKind::GrowingSet => "GrowingSet.from",
			CollKind::GrowingArray => "GrowingArray.from",
			CollKind::Pool => "Pool.from",
			CollKind::GrowingPool => "GrowingPool.from",
			CollKind::Columnar => "Columnar.from",
			CollKind::GrowingColumnar => "GrowingColumnar.from",
			CollKind::OrderedColumnar => "OrderedColumnar.from",
			_ => "Set.from",
		};
		// the element type the context gives, when no application does
		let hinted = given.or_else(|| match expected.map(|ty| self.s.types.kind(ty)) {
			Some(TypeKind::Collection(expected_kind, item, key)) if expected_kind == kind => {
				Some((item, key))
			}
			_ => None,
		});
		// the arguments: the source, then named options
		let mut source = None;
		let mut key_option = None;
		let mut capacity = None;
		let mut problems: Vec<(NodeId, String)> = Vec::new();
		for argument in arguments.iter() {
			match syntax::view(&tree, argument) {
				View::Assignment {
					target: name,
					annotation: None,
					value: Some(value),
				} if matches!(syntax::view(&tree, name), View::Name(_)) => match tree.text(name.index()) {
					"capacity" if capacity.is_none() => capacity = Some(value),
					"key" if is_set && key_option.is_none() => key_option = Some(value),
					other => {
						problems.push((name, format!("`{path}` has no option `{other}`")));
						self.check(value, None);
					}
				},
				_ if source.is_none() => source = Some(argument),
				_ => {
					problems.push((argument, format!("`{path}` takes one source collection")));
					self.check(argument, None);
				}
			}
		}
		let element_hint = hinted.map(|(item, key)| {
			if is_map {
				self.s.types.tuple(&[key, item])
			} else {
				item
			}
		});
		let source_hint = element_hint.map(|element| self.s.types.array(element));
		let source_ty = source.map(|source| self.check(source, source_hint));
		let source_element = source_ty.and_then(|ty| self.iteration_item(ty));
		if let (Some(source), Some(actual), Some(element)) = (source, source_element, element_hint)
		{
			// a list literal was already checked against the element type
			if !matches!(self.view(source), View::Array { .. }) {
				self.sink(source, actual, element);
			}
		}
		// the element type: given, or inferred from the source
		let element = match (element_hint, source_element) {
			(Some(element), _) => element,
			(None, Some(actual)) => self.join_member(actual),
			(None, None) => TypeId::UNKNOWN,
		};
		let (item, key) = if is_map {
			match self.s.types.kind(element) {
				TypeKind::Tuple(list) if self.s.types.list_items(list).len() == 2 => {
					let pair = self.s.types.list_items(list);
					(pair[1], pair[0])
				}
				_ => (TypeId::UNKNOWN, TypeId::UNKNOWN),
			}
		} else {
			(element, TypeId::NONE)
		};
		if let Some(value) = key_option {
			let result = self.builtin_param("Set.from", "Key", TypeId::UNKNOWN);
			let callback = self.callback_with(&[item], result, 0);
			self.check(value, Some(callback));
		}
		if let Some(value) = capacity {
			let natural = self.natural();
			let actual = self.check(value, Some(natural));
			self.sink(value, actual, natural);
		}
		Some(self.memo(n, |w| {
			let builtin = builtins::by_path(path).expect("a built-in `from`");
			w.set_resolution(member, Resolution::Builtin(builtin));
			w.set_flag(target, NodeFlags::TYPE_EXPR);
			for (node, message) in problems {
				let diagnostic = w.diagnostic(&FN023, node, message);
				w.report(diagnostic);
			}
			let source_ty = source_ty.unwrap_or(TypeId::UNKNOWN);
			if source.is_none() {
				let diagnostic = w.diagnostic(
					&crate::diagnostics::codes::types::FN024,
					n,
					format!("`{path}` takes the collection to copy, ex: `{path}([1, 2, 3])`"),
				);
				w.report(diagnostic);
			} else if source_element.is_none() && !w.s.types.is_recovering(source_ty) {
				let message = format!(
					"`{path}` copies a collection, not a `{}`",
					w.display(source_ty)
				);
				let diagnostic =
					w.diagnostic(&crate::diagnostics::codes::values::E_TYPE_1, n, message);
				w.report(diagnostic);
			}
			if is_map
				&& !w.s.types.is_recovering(element)
				&& !matches!(w.s.types.kind(element), TypeKind::Tuple(_))
				&& let Some(source) = source
			{
				let diagnostic = w.diagnostic(
					&CL005,
					source,
					"the items of a map are `(key, value)` pairs, ex: `Map.from([(\"ana\", 12)])`",
				);
				w.report(diagnostic);
			}
			// `let s: Set(Integer) = Set.from([1, 2])` is the literal `[1, 2]`
			if given.is_none()
				&& hinted.is_some()
				&& key_option.is_none()
				&& capacity.is_none()
				&& source.is_some_and(|source| matches!(w.view(source), View::Array { .. }))
			{
				let message = format!(
					"the type is already known here: write the list literal without `{path}`"
				);
				let diagnostic = w.diagnostic(&WCL001, n, message);
				w.report(diagnostic);
			}
			let hashed = if is_map { key } else { item };
			if (is_map || is_set) && !w.s.types.is_recovering(hashed) {
				let what = if is_map { "key" } else { "element" };
				w.check_comparable_element(n, kind, what, hashed);
			}
			if let Some(value) = key_option
				&& let Some(sig) = w.signature_of(w.type_of(value))
			{
				w.check_comparable_element(value, kind, "key", sig.ret);
			}
			let option = |given: Option<NodeId>| match given {
				Some(value) => Argument::Given(value),
				None => Argument::Default,
			};
			let mut call_arguments = vec![match source {
				Some(source) => Argument::Given(source),
				None => Argument::Missing,
			}];
			if is_set {
				call_arguments.push(option(key_option));
			}
			// `capacity`: every collection takes it (minimal representations)
			call_arguments.push(option(capacity));
			w.a.calls.insert(
				n,
				CallInfo {
					callee: Callee::Builtin(builtin),
					arguments: call_arguments,
				},
			);
			if w.s.types.is_recovering(item) || w.s.types.is_recovering(key) {
				return TypeId::UNKNOWN;
			}
			let ty = w.s.types.collection(kind, item, key);
			w.set_type(target, ty);
			ty
		}))
	}

	/// `CL010`: the elements of a set and the keys of a map are exact, even
	/// read-only (types brief §5.0.7): a `Set(Hero)` is never a
	/// `Set(Human)`. Flow part of a sink that failed; whether it reported.
	pub(super) fn inexact_collection(&mut self, n: NodeId, actual: TypeId, target: TypeId) -> bool {
		let (
			TypeKind::Collection(kind, item, key),
			TypeKind::Collection(target_kind, target_item, target_key),
		) = (self.s.types.kind(actual), self.s.types.kind(target))
		else {
			return false;
		};
		let (exact, expected, what) = match kind {
			_ if kind != target_kind => return false,
			CollKind::Set | CollKind::GrowingSet => (item, target_item, "elements"),
			CollKind::Map | CollKind::GrowingMap => (key, target_key, "keys"),
			_ => return false,
		};
		if exact == expected || !self.s.types.satisfies(exact, expected) {
			return false;
		}
		let message = format!(
			"a `{}` is never a `{}`: the {what} of a `{}` are exact; copy it with `{}.from(…)` into the expected type",
			self.display(actual),
			self.display(target),
			kind.name(),
			kind.name()
		);
		let diagnostic = self.diagnostic(&CL010, n, message);
		self.report_flow(diagnostic);
		true
	}

	/// `WCL001` for `Array.from([…])` where the context already gives the
	/// array type and no option is passed: the literal alone does (types
	/// brief §5.0.2). Called after `from.rs` typed the call.
	pub(super) fn warn_needless_array_from(
		&mut self,
		n: NodeId,
		callee: NodeId,
		arguments: NodeList<'_>,
		expected: Option<TypeId>,
	) {
		let tree = Arc::clone(&self.tree);
		let View::Access { target, .. } = syntax::view(&tree, callee) else {
			return;
		};
		let given = matches!(syntax::view(&tree, target), View::Call { .. });
		let literal_only = arguments.len() == 1
			&& arguments
				.first()
				.is_some_and(|source| matches!(syntax::view(&tree, source), View::Array { .. }));
		let expected_array = expected.is_some_and(|expected| {
			matches!(
				self.s.types.kind(expected),
				TypeKind::Collection(CollKind::Array, ..)
			)
		});
		if given || !literal_only || !expected_array || !self.reported_once.insert((n, WCL001.id)) {
			return;
		}
		let diagnostic = self.diagnostic(
			&WCL001,
			n,
			"the type is already known here: write the list literal without `Array.from`",
		);
		self.report(diagnostic);
	}

	// ── maps: indexing, loops ──────────────────────────────────────────

	/// The key type of a map, when `ty` is one.
	pub(super) fn map_key(&self, ty: TypeId) -> Option<TypeId> {
		match self.s.types.kind(ty) {
			TypeKind::Collection(CollKind::Map | CollKind::GrowingMap, _, key) => Some(key),
			_ => None,
		}
	}

	/// `map[key]` read: the flow part of the presence check. Every read may
	/// miss its key (`INDEX_CHECK`, `KeyNotFound`, by `proofs.rs`) until the
	/// three proofs of D32 (a literal map unchanged since with a literal key,
	/// an `includes(key)` guard, a `getOrInsert` reference) remove it.
	pub(super) fn check_key_presence(&mut self, n: NodeId, target: NodeId, key: NodeId) {
		// a key of another type: already reported
		let key_ty = self.type_of(key);
		if let Some(expected) = self.map_key(self.type_of(target))
			&& !self.s.types.is_recovering(key_ty)
			&& !self.s.types.satisfies(key_ty, expected)
		{
			self.clear_flag(n, NodeFlags::INDEX_CHECK | NodeFlags::CAN_FAIL);
			return;
		}
		// proven present (D32): a literal key of an unchanged literal map,
		// an `includes` guard
		if let Some(map) = self.place_of(target)
			&& let Some(key) = self.key_constant(key)
			&& self.key_proven(map, key)
		{
			self.clear_flag(n, NodeFlags::INDEX_CHECK | NodeFlags::CAN_FAIL);
			return;
		}
		self.check_index_bounds(n, target, key);
	}

	/// A map key as a compile-time identity: a literal, or an immutable local
	/// (D32). `None` for any other key.
	pub(crate) fn key_constant(&mut self, key: NodeId) -> Option<crate::ids::ConstId> {
		use crate::types::ConstValue;
		let key = self.strip_groups(key);
		let value = match self.view(key) {
			View::String(text) => {
				let text = text.to_string();
				ConstValue::String(self.s.types.syms.intern(&text))
			}
			View::Integer(value) => ConstValue::Int(value),
			View::Boolean(value) => ConstValue::Bool(value),
			View::Name(_) => {
				let symbol = self.read_local(key)?;
				if !matches!(
					self.a.symbols.symbol(symbol).kind,
					SymbolKind::Let
						| SymbolKind::Parameter { .. }
						| SymbolKind::LoopVariable { copy: true }
				) {
					return None;
				}
				ConstValue::Binding(symbol.index() as u32)
			}
			_ => return None,
		};
		Some(self.s.types.constant(value))
	}

	/// Whether the facts of a map place prove a key present.
	fn key_proven(&mut self, map: crate::ids::PlaceId, key: crate::ids::ConstId) -> bool {
		let Some(fact) = self.body.flow.fact(map) else {
			return false;
		};
		let fact = self.facts.fact_value(fact);
		self.facts.const_list_items(fact.key_present).contains(&key)
	}

	/// Adds keys to the keys proven present in a map place.
	pub(super) fn prove_keys(&mut self, map: crate::ids::PlaceId, keys: &[crate::ids::ConstId]) {
		let mut fact = self
			.body
			.flow
			.fact(map)
			.map_or(crate::flow::Fact::TOP, |fact| self.facts.fact_value(fact));
		let mut present = self.facts.const_list_items(fact.key_present).to_vec();
		for &key in keys {
			if !present.contains(&key) {
				present.push(key);
			}
		}
		fact.key_present = self.facts.const_list(&present);
		let id = self.facts.fact(fact);
		self.body.flow.set_fact(map, id);
	}

	/// The true branch of `m.includes(key)`: the key is present in `m`
	/// until `m` changes (D32). Flow part of a condition.
	pub(super) fn key_presence_condition(&mut self, n: NodeId) {
		let n = self.strip_groups(n);
		let View::Call { callee, arguments } = self.view(n) else {
			return;
		};
		let View::Access { target, members } = self.view(self.strip_groups(callee)) else {
			return;
		};
		let Some(member) = members.last().filter(|_| members.len() == 1) else {
			return;
		};
		let Resolution::Builtin(builtin) = self.a.resolution(member) else {
			return;
		};
		if builtins::get(builtin).path != "Map#includes" {
			return;
		}
		let Some(key) = arguments.first() else {
			return;
		};
		if let Some(map) = self.place_of(target)
			&& let Some(key) = self.key_constant(key)
		{
			self.prove_keys(map, &[key]);
		}
	}

	/// The keys of a map literal bound to a local, all literals (D32).
	pub(super) fn literal_map_keys(&mut self, symbol: SymbolId, value: NodeId) {
		let ty = self.a.symbols.symbol(symbol).ty;
		if self.map_key(ty).is_none() {
			return;
		}
		let View::Array { items } = self.view(self.strip_groups(value)) else {
			return;
		};
		let items: Vec<NodeId> = items.iter().collect();
		let mut keys = Vec::new();
		for item in items {
			let View::Tuple { items: parts } = self.view(self.strip_groups(item)) else {
				return;
			};
			let Some(key) = parts.first() else {
				return;
			};
			if matches!(self.view(self.strip_groups(key)), View::Name(_)) {
				return;
			}
			let Some(key) = self.key_constant(key) else {
				return;
			};
			keys.push(key);
		}
		if self.body.flow.reachable && !keys.is_empty() {
			let place = self.symbol_place(symbol);
			self.prove_keys(place, &keys);
		}
	}

	/// `for (key, value) in map`: binds the parts of the pairs a loop gives.
	/// A map's key is a read-only reference (`CL031` when written), its value
	/// a reference into the map. Type part.
	pub(super) fn bind_loop_tuple(&mut self, pattern: NodeId, items: NodeList<'_>, item: TypeId) {
		let tree = Arc::clone(&self.tree);
		let parts = match self.s.types.kind(item) {
			TypeKind::Tuple(list) => self.s.types.list_items(list).to_vec(),
			_ => vec![TypeId::UNKNOWN; items.len()],
		};
		self.memo(pattern, |w| {
			if parts.len() != items.len() && !w.s.types.is_recovering(item) {
				let message = format!(
					"a loop over this collection gives a `{}`: destructure it with {} names",
					w.display(item),
					parts.len()
				);
				let diagnostic = w.diagnostic(
					&crate::diagnostics::codes::values::E_TYPE_1,
					pattern,
					message,
				);
				w.report(diagnostic);
			}
			for (index, part) in items.iter().enumerate() {
				let ty = parts.get(index).copied().unwrap_or(TypeId::UNKNOWN);
				match syntax::view(&tree, part) {
					View::Name(name) if !name.is_empty() => {
						let symbol =
							w.declare(part, name, SymbolKind::LoopVariable { copy: false });
						w.a.symbols.symbol_mut(symbol).ty = ty;
						w.set_type(part, ty);
						if index == 0 && parts.len() == 2 {
							w.readonly_elements.insert(symbol);
						}
					}
					_ => {
						w.set_type(part, TypeId::UNKNOWN);
					}
				}
			}
			item
		});
	}

	/// Marks the variable of a loop over a set: its elements are read-only.
	pub(super) fn mark_if_readonly_element(&mut self, symbol: SymbolId, iterable: TypeId) {
		if matches!(
			self.s.types.kind(iterable),
			TypeKind::Collection(CollKind::Set | CollKind::GrowingSet, ..)
		) {
			self.readonly_elements.insert(symbol);
		}
	}

	/// Whether `n` is `map.keys()`: its items are the map's keys.
	pub(super) fn is_map_keys_call(&self, n: NodeId) -> bool {
		let n = self.strip_groups(n);
		let View::Call { callee, .. } = self.view(n) else {
			return false;
		};
		let View::Access {
			target, members, ..
		} = self.view(self.strip_groups(callee))
		else {
			return false;
		};
		members
			.last()
			.is_some_and(|member| self.tree.text(member.index()) == "keys")
			&& members.len() == 1
			&& self
				.map_key(self.type_of(self.strip_groups(target)))
				.is_some()
	}

	/// `CL031`: writing a set element or a map key (a read-only reference),
	/// instead of `E-DECL-3`. Type part.
	pub(super) fn report_readonly_element(&mut self, target: NodeId, symbol: SymbolId) -> bool {
		let readonly = self.readonly_elements.contains(&symbol)
			|| self.body.flow.ref_state(symbol).is_some_and(|state| {
				let state = self.facts.refs.get(state);
				state.readonly || state.places.iter().any(|&place| self.readonly_place(place))
			}) || self
			.reference_parameters
			.get(&symbol)
			.is_some_and(|&place| self.readonly_place(place));
		if !readonly {
			return false;
		}
		let name = self
			.s
			.types
			.syms
			.resolve(self.a.symbols.symbol(symbol).name);
		let message = format!(
			"`{name}` is a key or an element of a set: it can't be modified; delete it and add the new value"
		);
		let diagnostic = self.diagnostic(&E_DECL_3, target, message);
		self.report(diagnostic);
		true
	}
}
