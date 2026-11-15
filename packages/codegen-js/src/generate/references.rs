//! References, requirements and escaping closures (advanced brief §4,
//! §5.11, §6.6, §9.7; `values/declaring-values.md`, implementation section).
//!
//! - `use x = place` of an object or a collection is the JavaScript
//!   reference to it: `const x = place`; `mutable use` is a `let`, which
//!   `reuse` reassigns. A whole write through it (`x = value`) is done in
//!   place (`faAssign`), so that every reference keeps seeing the value.
//! - `use x = place` of a primitive place (a number, a string, a union…)
//!   has no JavaScript equivalent: the container and the key are evaluated
//!   once (a local name stays a name: re-reading it is the same Fa place),
//!   and each use of `x` is the place (`player.score`, `items[$t0]`,
//!   `counts.get($t1)`, written with `counts.set`). A `mutable use` of one is
//!   a pair of locals `x$o` and `x$k`, which `reuse` reassigns; `reuse x =
//!   none` empties the container. A whole local has no container: the pair
//!   holds an accessor cell of the local (`{ get $v() …, set $v(…) … }`).
//! - D60: a whole write of an object or a collection into a place rooted at
//!   a value some reference designates is an in-place write (`faAssign`),
//!   so that the reference sees the new value; union and optional places
//!   are replaced.
//! - Requirements: one hidden parameter per requirement, after the regular
//!   ones, in the order of `FnSig::requires`; a call passes, per
//!   `Analysis::requirement_args`, the temporary of a `provide` (a `const`
//!   evaluated once at the statement), the provided place read again at the
//!   call (a primitive place), or its own hidden parameter.
//! - An escaping closure (`Analysis::escapes`) copies the captures that may
//!   change after its creation into `const`s, declared just before it.

use std::collections::HashSet;

use fa_analyzer::analysis::{Argument, Callee, FnInfo, NodeFlags, RequirementArg};
use fa_analyzer::builtins;
use fa_analyzer::ids::{NodeId, SymbolId, TypeId};
use fa_analyzer::symbols::{Resolution, SymbolKind};
use fa_analyzer::syntax::View;
use fa_analyzer::types::{CollKind, TypeKind};

use crate::generate::{Dest, Generator, Kind};
use crate::js::{self, Expr};

/// How a reference is held.
#[derive(Clone, Debug)]
pub(crate) enum Reference {
	/// An object or a collection: its JavaScript name holds it.
	Object,
	/// A reseatable reference to a primitive place: the `let` locals of its
	/// container and key.
	Pair {
		container: String,
		key: String,
		map: bool,
		optional: bool,
	},
}

/// What designates a primitive place, evaluated once.
enum Key {
	/// The whole place, as written (a local name, an `export mutable` value),
	/// with its map slot when it is itself a value of a map.
	Whole(String, Option<(String, String)>),
	/// A field (a constant property) of the container.
	Field(String),
	/// An item of an array.
	Index(Expr),
	/// A value of a map.
	Entry(Expr),
	/// A value computed once (a slice, a character: strings are immutable).
	Value(Expr),
}

/// A place designated by a reference: its container and key.
struct Parts {
	container: Option<Expr>,
	key: Key,
	/// The container is `undefined` when the reference is `none` (`get`).
	optional: bool,
}

impl Generator<'_> {
	// ── `use`, `mutable use`, `reuse` ────────────────────────────────────

	/// Whether references to values of a type are the JavaScript values
	/// themselves: objects and collections (optional or not).
	fn is_object_reference(&self, ty: TypeId) -> bool {
		let ty = self.without_none(ty);
		matches!(
			self.types.kind(ty),
			TypeKind::Object(_)
				| TypeKind::Bytes
				| TypeKind::Collection(..)
				| TypeKind::Tuple(_)
				| TypeKind::Range
		)
	}

	/// The type without its `none` member (`T?` → `T`).
	pub(crate) fn without_none(&self, ty: TypeId) -> TypeId {
		if let TypeKind::Union(list) = self.types.kind(ty) {
			let others: Vec<TypeId> = self
				.types
				.list_items(list)
				.iter()
				.copied()
				.filter(|&member| member != TypeId::NONE)
				.collect();
			if let [only] = others.as_slice() {
				return *only;
			}
		}
		ty
	}

	/// `use x = place`, `mutable use x = place`.
	pub(crate) fn use_declaration(&mut self, n: NodeId, value: NodeId, mutable: bool) {
		let Some(symbol) = self.analysis.symbols.symbol_of_decl(n) else {
			return;
		};
		let ty = self.analysis.symbols.symbol(symbol).ty;
		let module_value = self.is_module_value(symbol);
		if self.is_object_reference(ty) {
			let value = match self.view(self.strip_reference(value)) {
				View::Index { target, index } => self
					.native_slice(self.strip_reference(value), target, index, true)
					.unwrap_or_else(|| self.expr(value)),
				_ => self.expr(value),
			};
			let name = self.declare(symbol);
			let keyword = match (module_value, mutable) {
				(true, _) => "",
				(false, true) => "let ",
				(false, false) => "const ",
			};
			self.out.line(format!("{keyword}{name} = {};", value.text));
			self.references.insert(symbol, Reference::Object);
			return;
		}
		let Some(parts) = self.reference_parts(value, !mutable) else {
			return;
		};
		if let Key::Value(value) = parts.key {
			let name = self.declare(symbol);
			let keyword = match (module_value, mutable) {
				(true, _) => "",
				(false, true) => "let ",
				(false, false) => "const ",
			};
			self.out.line(format!("{keyword}{name} = {};", value.text));
			return;
		}
		// a reference read by functions: its module-level pair, prepared
		// before the functions were generated (`prepare_module_references`)
		if let Some(Reference::Pair {
			container: container_name,
			key: key_name,
			..
		}) = self.references.get(&symbol).cloned()
		{
			let Some((container, key, _)) = self.pair_of(&parts) else {
				self.unsupported(n, "this reference");
				return;
			};
			self.out.line(format!("{container_name} = {container};"));
			self.out.line(format!("{key_name} = {key};"));
			return;
		}
		if !mutable {
			self.alias_parts(symbol, parts);
			return;
		}
		let Some((container, key, map)) = self.pair_of(&parts) else {
			self.unsupported(n, "this `mutable use`");
			return;
		};
		let wanted = self.symbol_name(symbol);
		let container_name = self.scopes.declare(&format!("{wanted}$o"));
		let key_name = self.scopes.declare(&format!("{wanted}$k"));
		self.out.line(format!(
			"let {container_name} = {container}, {key_name} = {key};"
		));
		let reference = Reference::Pair {
			container: container_name,
			key: key_name,
			map,
			optional: parts.optional || self.may_be_none(ty),
		};
		self.alias_pair(symbol, &reference);
		self.references.insert(symbol, reference);
	}

	/// The Fa name of a symbol.
	fn symbol_name(&self, symbol: SymbolId) -> String {
		self.types
			.syms
			.resolve(self.analysis.symbols.symbol(symbol).name)
			.to_string()
	}

	/// Whether a module-level reference read by functions has a JavaScript
	/// name of its own: an object reference, or a value computed once.
	pub(crate) fn module_reference_has_name(&self, symbol: SymbolId) -> bool {
		let info = self.analysis.symbols.symbol(symbol);
		if self.is_object_reference(info.ty) {
			return true;
		}
		let View::Use { value, .. } = self.view(info.decl_node) else {
			return true;
		};
		match self.view(self.strip_reference(value)) {
			View::Index { target, index } => {
				self.type_of(index) == TypeId::RANGE
					|| self.kind(self.type_of(target)) == Kind::Text
			}
			_ => false,
		}
	}

	/// The references of the module read by functions (module values),
	/// prepared before the functions are generated: a primitive place gets
	/// module-level `let`s for its container and key, which its declaration
	/// assigns; a whole local is an alias of that local's name.
	pub(crate) fn prepare_module_references(&mut self) {
		let mut symbols: Vec<SymbolId> = self
			.module_values
			.iter()
			.copied()
			.filter(|&symbol| {
				matches!(
					self.analysis.symbols.symbol(symbol).kind,
					SymbolKind::Use | SymbolKind::MutableUse
				) && !self.module_reference_has_name(symbol)
			})
			.collect();
		symbols.sort();
		for symbol in symbols {
			let info = self.analysis.symbols.symbol(symbol);
			let (ty, decl, kind) = (info.ty, info.decl_node, info.kind);
			let View::Use { value, .. } = self.view(decl) else {
				continue;
			};
			let value = self.strip_reference(value);
			let (map, optional) = match self.view(value) {
				View::Name(_) if kind == SymbolKind::Use => {
					if let Resolution::Local(source) = self.analysis.resolution(value) {
						let name = self.name_of(source);
						self.alias(symbol, name);
					}
					continue;
				}
				// a `mutable use` of a whole local: a pair holding its cell
				View::Name(_) => (false, false),
				View::Index { target, .. } => {
					let map = matches!(
						self.types.kind(self.type_of(target)),
						TypeKind::Collection(CollKind::Map | CollKind::GrowingMap, ..)
					);
					(map, false)
				}
				View::Call { .. } => {
					let path = match self.analysis.call(value).map(|call| call.callee) {
						Some(Callee::Builtin(builtin)) => builtins::get(builtin).path,
						_ => "",
					};
					(
						path.starts_with("Map#") || path.starts_with("Pool#"),
						!matches!(path, "Map#getOrInsert" | "Pool#add"),
					)
				}
				_ => (false, false),
			};
			let wanted = self.symbol_name(symbol);
			let container = self.scopes.declare_global(&format!("{wanted}$o"));
			let key = self.scopes.declare_global(&format!("{wanted}$k"));
			self.module_slots.push(container.clone());
			self.module_slots.push(key.clone());
			let reference = Reference::Pair {
				container,
				key,
				map,
				optional: optional || self.may_be_none(ty),
			};
			self.alias_pair(symbol, &reference);
			self.references.insert(symbol, reference);
		}
	}

	/// Makes a symbol an alias of a place designated by `parts`.
	fn alias_parts(&mut self, symbol: SymbolId, parts: Parts) {
		let dot = if parts.optional { "?." } else { "." };
		let container = parts.container.map(|container| container.primary_text());
		let container = container.unwrap_or_default();
		match parts.key {
			Key::Whole(text, map) => {
				if let Some(map) = map {
					self.map_aliases.insert(symbol, map);
				}
				self.alias(symbol, text);
			}
			Key::Field(property) => self.alias(symbol, format!("{container}{dot}{property}")),
			Key::Index(index) => {
				let access = if parts.optional { "?.[" } else { "[" };
				self.alias(symbol, format!("{container}{access}{}]", index.text));
			}
			Key::Entry(key) => {
				self.alias(symbol, format!("{container}{dot}get({})", key.text));
				self.map_aliases.insert(symbol, (container, key.text));
			}
			Key::Value(_) => {}
		}
	}

	/// Makes a symbol the alias of the place of a `mutable use` pair.
	fn alias_pair(&mut self, symbol: SymbolId, reference: &Reference) {
		let Reference::Pair {
			container,
			key,
			map,
			optional,
		} = reference
		else {
			return;
		};
		let dot = if *optional { "?." } else { "." };
		if *map {
			self.alias(symbol, format!("{container}{dot}get({key})"));
			self.map_aliases
				.insert(symbol, (container.clone(), key.clone()));
		} else {
			let access = if *optional { "?.[" } else { "[" };
			self.alias(symbol, format!("{container}{access}{key}]"));
		}
	}

	/// `reuse x = place` (or `none`): the `mutable use` designates another place.
	pub(crate) fn reuse_statement(&mut self, n: NodeId, value: NodeId) {
		let Resolution::Local(symbol) = self.analysis.resolution(n) else {
			self.unsupported(n, "this `reuse`");
			return;
		};
		match self.references.get(&symbol).cloned() {
			Some(Reference::Object) => {
				let value = match self.view(self.strip_reference(value)) {
					View::Index { target, index } => self
						.native_slice(self.strip_reference(value), target, index, true)
						.unwrap_or_else(|| self.expr(value)),
					_ => self.expr(value),
				};
				let name = self.name_of(symbol);
				self.out.line(format!("{name} = {};", value.text));
			}
			Some(Reference::Pair { container, key, .. }) => {
				if matches!(self.view(self.strip_groups(value)), View::None) {
					self.out.line(format!("{container} = undefined;"));
					return;
				}
				let Some(parts) = self.reference_parts(value, false) else {
					return;
				};
				let Some((new_container, new_key, _)) = self.pair_of(&parts) else {
					self.unsupported(value, "this place in a `reuse`");
					return;
				};
				self.out.line(format!("{container} = {new_container};"));
				self.out.line(format!("{key} = {new_key};"));
			}
			None => {
				self.unsupported(n, "this `reuse`");
			}
		}
	}

	/// The container and the key of the primitive place a reference
	/// designates, evaluated once (`keep_names`: a local name stays itself,
	/// for a `use`; a `mutable use` keeps every part, since its locals are
	/// assigned).
	fn reference_parts(&mut self, value: NodeId, keep_names: bool) -> Option<Parts> {
		let node = self.strip_reference(value);
		let simple = |container, key| Parts {
			container,
			key,
			optional: false,
		};
		match self.view(node) {
			View::Name(_) => {
				if let Resolution::Local(source) = self.analysis.resolution(node) {
					// `use y = x` of a `mutable use x`: what `x` designates now
					if let Some(Reference::Pair {
						container,
						key,
						map,
						optional,
					}) = self.references.get(&source).cloned()
					{
						let container = self.keep_value_part(Expr::name(container, false));
						let key = self.keep_value_part(Expr::name(key, false));
						let key = if map {
							Key::Entry(key)
						} else {
							Key::Index(key)
						};
						return Some(Parts {
							container: Some(container),
							key,
							optional,
						});
					}
					let text = self.name_of(source);
					let map = self.map_aliases.get(&source).cloned();
					return Some(simple(None, Key::Whole(text, map)));
				}
				let place = self.place(node, true)?;
				Some(simple(None, Key::Whole(place.text, place.map)))
			}
			View::Access { target, members } => {
				let members: Vec<NodeId> = members.iter().collect();
				let (&last, prefix) = members.split_last()?;
				let property = match self.analysis.resolution(last) {
					Resolution::Field(shape, index) => {
						let field = self.types.shape_fields(shape)[index as usize];
						Self::property(self.types.syms.resolve(field.name))
					}
					Resolution::Builtin(builtin) => builtins::get(builtin)
						.path
						.strip_prefix("Range#")?
						.to_string(),
					_ => {
						self.unsupported(node, "a reference to this member");
						return None;
					}
				};
				let container = self.access_chain(node, target, prefix);
				let path = prefix.iter().all(|&member| {
					matches!(self.analysis.resolution(member), Resolution::Field(..))
				}) && self.is_plain_local(target);
				let container = if path && keep_names {
					container
				} else {
					self.keep_value_part(container)
				};
				Some(simple(Some(container), Key::Field(property)))
			}
			View::Index { target, index } => {
				let target_ty = self.type_of(target);
				if self.type_of(index) == TypeId::RANGE || self.kind(target_ty) == Kind::Text {
					let value = self
						.native_slice(node, target, index, true)
						.unwrap_or_else(|| self.expr(node));
					return Some(simple(None, Key::Value(value)));
				}
				let values = self.in_order(&[target, index], |g, part| g.expr(part));
				let keep_container = !(keep_names && self.is_plain_local(target));
				let container = if keep_container {
					self.keep_value_part(values[0].clone())
				} else {
					values[0].clone()
				};
				let key = self.keep_value_part(values[1].clone());
				if let TypeKind::Collection(CollKind::Map | CollKind::GrowingMap, _, key_ty) =
					self.types.kind(target_ty)
				{
					if self.has(node, NodeFlags::INDEX_CHECK) {
						// `KeyNotFound` now, when the key is absent
						let check = self.map_index(node, container.clone(), key.clone(), key_ty);
						self.out.line(format!("{};", check.text));
					}
					return Some(simple(Some(container), Key::Entry(key)));
				}
				let key = if self.index_needs_normalizing(node, index) {
					let helper = self.helper("faNormalizeIndex");
					let temp = self.temp();
					self.out.line(format!(
						"const {temp} = {helper}({}.length, {});",
						container.primary_text(),
						key.text
					));
					Expr::name(temp, true)
				} else {
					key
				};
				Some(simple(Some(container), Key::Index(key)))
			}
			View::Call { callee, .. } => self.call_reference_parts(node, callee),
			_ => {
				self.unsupported(node, "a reference to this value");
				None
			}
		}
	}

	/// The parts of a reference returned by `map.get(key)`,
	/// `map.getOrInsert(key, value)`, `array.get(index)` or
	/// `array.find(predicate)` (a primitive value: the place, `none` when
	/// absent).
	fn call_reference_parts(&mut self, node: NodeId, callee: NodeId) -> Option<Parts> {
		let path = match self.analysis.call(node).map(|info| info.callee) {
			Some(Callee::Builtin(builtin)) => builtins::get(builtin).path,
			_ => "",
		};
		let arguments: Vec<NodeId> = self
			.analysis
			.call(node)
			.map(|info| {
				info.arguments
					.iter()
					.filter_map(|argument| match argument {
						Argument::Given(value) => Some(*value),
						_ => None,
					})
					.collect()
			})
			.unwrap_or_default();
		let value_pool = matches!(path, "Pool#find" | "Pool#add")
			&& self.is_value_pool(self.receiver_type(callee));
		if !value_pool
			&& !matches!(
				path,
				"Map#get" | "Map#getOrInsert" | "Array#get" | "Array#find"
			) {
			self.unsupported(node, "a reference to a primitive value returned by a call");
			return None;
		}
		let receiver = self.receiver(callee);
		let receiver = self.keep_value_part(receiver);
		let values = self.in_order(&arguments, |g, argument| g.expr(argument));
		// an element of a pool of values: the pool and its slot
		if path == "Pool#find" {
			let (slot, container) = (self.temp(), self.temp());
			let pool = receiver.primary_text();
			self.out.line(format!(
				"const {slot} = {pool}.slotOf({});",
				values.first()?.text
			));
			self.out.line(format!(
				"const {container} = {slot} !== undefined ? {pool} : undefined;"
			));
			return Some(Parts {
				container: Some(Expr::name(container, true)),
				key: Key::Entry(Expr::name(slot, true)),
				optional: true,
			});
		}
		if path == "Pool#add" {
			let slot = self.temp();
			self.out.line(format!(
				"const {slot} = {}.add({});",
				receiver.primary_text(),
				values.first()?.text
			));
			return Some(Parts {
				container: Some(receiver),
				key: Key::Entry(Expr::name(slot, true)),
				optional: false,
			});
		}
		if path == "Array#find" {
			// the item the predicate finds first: its index, `none` without one
			let (position, container) = (self.temp(), self.temp());
			let array = receiver.primary_text();
			self.out.line(format!(
				"const {position} = {array}.findIndex({});",
				values.first()?.text
			));
			self.out.line(format!(
				"const {container} = {position} >= 0 ? {array} : undefined;"
			));
			return Some(Parts {
				container: Some(Expr::name(container, true)),
				key: Key::Index(Expr::name(position, true)),
				optional: true,
			});
		}
		let key = self.keep_value_part(values.first()?.clone());
		match path {
			"Map#getOrInsert" => {
				let helper = self.helper("faMapGetOrInsert");
				self.out.line(format!(
					"{helper}({}, {}, {});",
					receiver.text,
					key.text,
					values.get(1)?.text
				));
				Some(Parts {
					container: Some(receiver),
					key: Key::Entry(key),
					optional: false,
				})
			}
			"Map#get" => {
				let temp = self.temp();
				self.out.line(format!(
					"const {temp} = {0}.has({1}) ? {0} : undefined;",
					receiver.primary_text(),
					key.text
				));
				Some(Parts {
					container: Some(Expr::name(temp, true)),
					key: Key::Entry(key),
					optional: true,
				})
			}
			_ => {
				let (position, container) = (self.temp(), self.temp());
				let array = receiver.primary_text();
				self.out.line(format!(
					"const {position} = {0} < 0 ? {0} + {array}.length : {0};",
					key.text
				));
				self.out.line(format!(
					"const {container} = {position} >= 0 && {position} < {array}.length ? {array} : undefined;"
				));
				Some(Parts {
					container: Some(Expr::name(container, true)),
					key: Key::Index(Expr::name(position, true)),
					optional: true,
				})
			}
		}
	}

	/// The slot of the element of a pool of values a reference designates
	/// (`use element = pool.find(…)`), for `delete` and `take`.
	pub(crate) fn reference_slot(&self, n: NodeId) -> Option<String> {
		let n = self.strip_reference(n);
		let Resolution::Local(symbol) = self.analysis.resolution(n) else {
			return None;
		};
		match self.references.get(&symbol) {
			Some(Reference::Pair { key, map: true, .. }) => Some(key.clone()),
			_ => self.map_aliases.get(&symbol).map(|(_, key)| key.clone()),
		}
	}

	/// The node of a place without its parentheses and `try`.
	fn strip_reference(&self, n: NodeId) -> NodeId {
		let mut current = n;
		while let View::Group(inner) | View::Try(inner) = self.view(current) {
			current = inner;
		}
		current
	}

	/// Whether a node is a local name read as is (re-reading it reads the
	/// same Fa place).
	fn is_plain_local(&self, n: NodeId) -> bool {
		let n = self.strip_groups(n);
		matches!(self.view(n), View::Name(_))
			&& match self.analysis.resolution(n) {
				Resolution::Local(symbol) => {
					!self.aliases.contains_key(&symbol)
						&& self.analysis.symbols.symbol(symbol).kind != SymbolKind::MutableUse
				}
				_ => false,
			}
	}

	/// A part of a place evaluated once: stable, or kept in a temporary.
	fn keep_value_part(&mut self, value: Expr) -> Expr {
		if value.stable {
			return value;
		}
		let temp = self.temp();
		self.out.line(format!("const {temp} = {};", value.text));
		Expr::name(temp, true)
	}

	// ── in-place writes (D60) ────────────────────────────────────────────

	/// Whether a whole write into a place must keep the object it holds:
	/// a reference to an object or a collection, or a place rooted at a
	/// value a reference designates (not a value of a map, which `set`
	/// replaces, nor a union or an optional place).
	pub(crate) fn writes_in_place(&mut self, target: NodeId) -> bool {
		let ty = self.type_of(target);
		if !matches!(
			self.types.kind(ty),
			TypeKind::Object(_) | TypeKind::Collection(..)
		) {
			return false;
		}
		match self.view(target) {
			View::Name(_) => match self.analysis.resolution(target) {
				Resolution::Local(symbol) => {
					matches!(self.references.get(&symbol), Some(Reference::Object))
						|| self.referenced_roots().contains(&symbol)
				}
				_ => false,
			},
			View::Index { target: map, .. }
				if matches!(
					self.types.kind(self.type_of(map)),
					TypeKind::Collection(CollKind::Map | CollKind::GrowingMap, ..)
				) =>
			{
				false
			}
			View::Access { .. } | View::Index { .. } => {
				self.place_root(target).is_some_and(|root| {
					matches!(self.references.get(&root), Some(Reference::Object))
						|| self.referenced_roots().contains(&root)
				})
			}
			_ => false,
		}
	}

	/// `place = value` in place: `faAssign(place, value)` (the value is
	/// copied by `faAssign` where it keeps parts of it).
	pub(crate) fn assign_in_place(&mut self, target: NodeId, value: NodeId) {
		let lifted = self.needs_statements(value);
		let Some(place) = self.place(target, !lifted) else {
			self.unsupported(target, "this assignment target");
			return;
		};
		let value = if lifted {
			let temp = self.temp();
			self.out.line(format!("let {temp};"));
			self.into(value, Dest::Assign(temp.clone()));
			Expr::name(temp, true)
		} else {
			self.expr(value)
		};
		let helper = self.helper("faAssign");
		self.out
			.line(format!("{helper}({}, {});", place.text, value.text));
	}

	/// The roots of the places designated by the references of the file
	/// (`use`, `mutable use`, `reuse`, `provide`, `for` variables over
	/// collections of objects).
	fn referenced_roots(&mut self) -> &HashSet<SymbolId> {
		if self.referenced_roots.is_none() {
			let mut roots = HashSet::new();
			for index in 0..self.tree.nodes.len() {
				let node = NodeId::new(index);
				let value = match self.view(node) {
					View::Use { value, .. } | View::Reuse { value, .. } => value,
					View::Provide { value, .. } => value,
					_ => continue,
				};
				let value = self.strip_reference(value);
				let root = match self.view(value) {
					View::Call { callee, .. } => match self.view(callee) {
						View::Access { target, .. } => self.place_root(target),
						_ => None,
					},
					_ => self.place_root(value),
				};
				roots.extend(root);
			}
			self.referenced_roots = Some(roots);
		}
		self.referenced_roots.as_ref().expect("computed")
	}

	// ── requirements ─────────────────────────────────────────────────────

	/// `provide Key = value`: a temporary evaluated once, or nothing for a
	/// primitive place (each call reads it).
	pub(crate) fn provide_statement(&mut self, n: NodeId, ty: NodeId, value: NodeId) {
		let place = matches!(
			self.view(self.strip_groups(value)),
			View::Name(_) | View::Access { .. } | View::Index { .. }
		);
		if place && !self.is_composite(self.type_of(value)) {
			self.provided.insert(n, None);
			return;
		}
		let value = self.expr(value);
		let type_name = self.text(ty);
		let mut wanted: String = type_name
			.chars()
			.take_while(|c| c.is_alphanumeric())
			.collect();
		if let Some(first) = wanted.get(0..1) {
			wanted = first.to_lowercase() + &wanted[1..];
		}
		let name = self.scopes.declare(&format!("{wanted}$p"));
		self.out.line(format!("const {name} = {};", value.text));
		self.provided.insert(n, Some(name));
	}

	/// The hidden arguments of a call for the callee's requirements.
	pub(crate) fn requirement_arguments(&mut self, call: NodeId) -> Vec<String> {
		let Some(arguments) = self.analysis.requirement_args.get(&call).cloned() else {
			return Vec::new();
		};
		arguments
			.into_iter()
			.map(|argument| match argument {
				RequirementArg::Provided(provide) => match self.provided.get(&provide).cloned() {
					Some(Some(name)) => name,
					_ => match self.view(provide) {
						View::Provide { value, .. } => self.expr(value).text,
						_ => "undefined".to_string(),
					},
				},
				RequirementArg::Own(key) => self
					.requirement_scopes
					.iter()
					.rev()
					.flatten()
					.find(|(own, _)| *own == key)
					.map_or_else(|| "undefined".to_string(), |(_, name)| name.clone()),
				RequirementArg::Missing => "undefined".to_string(),
			})
			.collect()
	}

	/// Declares the hidden parameters of a function's requirements (after
	/// its regular parameters): their names, in the order of
	/// `FnSig::requires`. The requirement symbols of the body take them.
	pub(crate) fn declare_requirements(&mut self, info: &FnInfo) -> Vec<(TypeId, String)> {
		let Some(sig) = info.sig else {
			return Vec::new();
		};
		let requires = self.types.sig(sig).requires;
		let requirements = self.types.requirements(requires).to_vec();
		let mut declared = Vec::new();
		for (index, requirement) in requirements.iter().enumerate() {
			let symbol = self
				.analysis
				.symbols
				.symbols()
				.iter()
				.position(|symbol| {
					symbol.scope == info.scope
						&& symbol.kind
							== SymbolKind::Requirement {
								index: index as u16,
							}
				})
				.map(SymbolId::new);
			let name = match symbol {
				Some(symbol) => self.declare(symbol),
				None => {
					let wanted = self.types.syms.resolve(requirement.name).to_string();
					self.scopes.declare(&wanted)
				}
			};
			declared.push((requirement.ty, name));
		}
		declared
	}

	// ── escaping closures ────────────────────────────────────────────────

	/// Before an escaping closure: copies of the captures that may change
	/// after its creation, which the closure reads instead. Returns the
	/// aliases to restore after the closure is generated.
	pub(crate) fn copy_captures(&mut self, info: &FnInfo) -> Vec<(SymbolId, Option<String>)> {
		if !self.analysis.escapes.contains(&info.lambda) {
			return Vec::new();
		}
		let captures = info.captures.clone();
		self.copy_captured(&captures)
	}

	/// Copies of the local values `symbols` that may change later, which a
	/// closure being generated reads instead (see
	/// [`Generator::copy_captures`]). A `reactive` value is always copied,
	/// even a module-level one: it is captured by the closures that don't
	/// track it (the cleanup of a `watch` block). Returns the aliases to
	/// restore.
	pub(crate) fn copy_captured(
		&mut self,
		symbols: &[SymbolId],
	) -> Vec<(SymbolId, Option<String>)> {
		self.copy_values(symbols, false)
	}

	/// [`Generator::copy_captured`]; `module_values`: the module-level ones
	/// too (a detached `start do` copies the top-level values it uses, on
	/// every target: `design/decisions.md`).
	pub(crate) fn copy_values(
		&mut self,
		symbols: &[SymbolId],
		module_values: bool,
	) -> Vec<(SymbolId, Option<String>)> {
		let mut saved = Vec::new();
		for &symbol in symbols {
			let kind = self.analysis.symbols.symbol(symbol).kind;
			if kind == SymbolKind::Derived {
				continue;
			}
			let changes = self.is_mutable_place(symbol)
				|| self.aliases.contains_key(&symbol)
				|| kind == SymbolKind::Use;
			let reactive = kind == SymbolKind::Reactive;
			if !changes || (self.is_module_value(symbol) && !reactive && !module_values) {
				continue;
			}
			let ty = self.analysis.symbols.symbol(symbol).ty;
			let current = Expr::name(self.name_of(symbol), true);
			let value = if self.is_composite(ty) {
				self.copy_value(current, ty)
			} else {
				current
			};
			let wanted = self
				.types
				.syms
				.resolve(self.analysis.symbols.symbol(symbol).name)
				.to_string();
			let name = self.scopes.declare(&format!("{wanted}$copy"));
			// a task may change its copy
			let keyword = if module_values { "let" } else { "const" };
			self.out.line(format!("{keyword} {name} = {};", value.text));
			saved.push((symbol, self.aliases.insert(symbol, name)));
		}
		saved
	}

	/// Restores the aliases [`Generator::copy_captures`] replaced.
	pub(crate) fn restore_captures(&mut self, saved: Vec<(SymbolId, Option<String>)>) {
		for (symbol, alias) in saved {
			match alias {
				Some(alias) => self.aliases.insert(symbol, alias),
				None => self.aliases.remove(&symbol),
			};
		}
	}

	/// Whether the read of a local at a node is its last use, as a copy
	/// (`NodeFlags::MOVE`): the value is moved, not copied (D15).
	pub(crate) fn is_move(&self, node: NodeId) -> bool {
		fa_analyzer::lower::copies::is_move(self.tree, self.analysis, node)
	}
}

impl Generator<'_> {
	/// The container, the key (a JavaScript expression) and whether it is
	/// a map's, of a place designated by `parts` (`None` for a value). A
	/// whole local (or another whole place, written as is) has no
	/// container: an accessor cell stands for it, whose `$v` reads and
	/// writes the place (`{ get $v() { return a; }, set $v($value) { a =
	/// $value; } }`), so that a `mutable use` pair can designate it.
	fn pair_of(&mut self, parts: &Parts) -> Option<(String, String, bool)> {
		if let Key::Whole(text, map) = &parts.key {
			if let Some((container, key)) = map {
				return Some((container.clone(), key.clone(), true));
			}
			let cell = format!(
				"{{ get $v() {{ return {text}; }}, set $v($value) {{ {text} = $value; }} }}"
			);
			return Some((cell, js::string_literal("$v"), false));
		}
		let container = parts.container.as_ref()?.text.clone();
		let (key, map) = match &parts.key {
			Key::Field(property) => (js::string_literal(property), false),
			Key::Index(index) => (index.text.clone(), false),
			Key::Entry(key) => (key.text.clone(), true),
			Key::Whole(..) | Key::Value(_) => return None,
		};
		Some((container, key, map))
	}
}
