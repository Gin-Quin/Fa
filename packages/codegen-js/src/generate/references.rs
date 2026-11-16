//! References, requirements and escaping closures (advanced brief §4,
//! §5.11, §6.6, §9.7; `values/declaring-values.md`, implementation section).
//!
//! - `use x = place` of an object or a collection is the JavaScript
//!   reference to it: `const x = place`; `mutable use` is a `let`, which
//!   `reuse` reassigns. A whole write through it (`x = value`) is done in
//!   place (`faAssign`), so that every reference keeps seeing the value.
//! - Primitive places use cached accessor cells. A local alias and every call
//!   forward the same carrier; `mutable use` reseats its carrier. Pool cells
//!   retain the slot index for delete/take and retire with that slot. Generic
//!   references carry either that primitive cell or the bare object itself.
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
	/// An erased item: either the object itself or a primitive cell.
	Cell { carrier: String, erased: bool },
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
	/// A borrowed result retains the designated place instead of copying data.
	pub(crate) fn returns_reference(&self) -> bool {
		self.current_function()
			.and_then(|function| self.analysis.function(function))
			.and_then(|info| info.sig)
			.is_some_and(|sig| self.types.sig(sig).reference.is_some())
	}

	pub(crate) fn call_returns_reference(&self, node: NodeId) -> bool {
		let View::Call { callee, .. } = self.view(node) else {
			return false;
		};
		matches!(self.types.kind(self.type_of(callee)), TypeKind::Function(sig) if self.types.sig(sig).reference.is_some())
	}

	pub(crate) fn reference_parameter(&self, call: NodeId, value: NodeId) -> bool {
		let View::Call { callee, .. } = self.view(call) else {
			return false;
		};
		let TypeKind::Function(sig) = self.types.kind(self.type_of(callee)) else {
			return false;
		};
		let params = self.types.params(self.types.sig(sig).params);
		let Some(info) = self.analysis.call(call) else {
			return false;
		};
		let skip = info.arguments.len().saturating_sub(params.len());
		params
			.iter()
			.zip(info.arguments.iter().skip(skip))
			.any(|(param, argument)| {
				param.reference.is_some()
					&& matches!(argument, Argument::Given(node) if *node == value)
			})
	}

	pub(crate) fn bind_reference_parameter(&mut self, info: &FnInfo, position: usize, name: &str) {
		let Some(sig) = info.sig else {
			return;
		};
		if !self
			.types
			.params(self.types.sig(sig).params)
			.get(position)
			.is_some_and(|param| param.reference.is_some())
		{
			return;
		}
		let param = &info.params[position];
		let ty = self.analysis.symbols.symbol(param.symbol).ty;
		if self.is_object_reference(ty) {
			self.references.insert(param.symbol, Reference::Object);
			return;
		}
		self.reference_cells.insert(param.symbol, name.to_string());
		if matches!(self.types.kind(self.without_none(ty)), TypeKind::Param(_)) {
			self.bind_erased_reference(param.symbol, name);
			return;
		}
		let access = if self.may_be_none(ty) { "?." } else { "." };
		self.alias(param.symbol, format!("{name}{access}$v"));
	}

	fn bind_erased_reference(&mut self, symbol: SymbolId, carrier: &str) {
		let read = self.helper("faReferenceValue");
		self.alias(symbol, format!("{read}({carrier})"));
		self.reference_cells.insert(symbol, carrier.to_string());
		self.references.insert(
			symbol,
			Reference::Cell {
				carrier: carrier.to_string(),
				erased: true,
			},
		);
	}

	/// Produce the address carrier before reading the designated value.
	pub(crate) fn reference_cell(&mut self, value: NodeId) -> Expr {
		let node = self.strip_reference(value);
		if matches!(self.view(node), View::None) {
			return Expr::constant("undefined", crate::js::Prec::Primary);
		}
		if let Resolution::Local(symbol) = self.analysis.resolution(node)
			&& let Some(cell) = self.reference_cells.get(&symbol)
		{
			return Expr::name(cell.clone(), true);
		}
		if self.call_returns_reference(node) {
			let View::Call { callee, arguments } = self.view(node) else {
				unreachable!()
			};
			return self.call(node, callee, arguments);
		}
		if let Some(cell) = self.generic_pool_reference(node) {
			return cell;
		}
		if self.is_object_reference(self.type_of(node)) {
			return self.expr(node);
		}
		let Some(parts) = self.reference_parts(node, false) else {
			return self.unsupported(node, "this returned reference place");
		};
		self.cell_from_parts(node, &parts)
	}

	fn cell_from_parts(&mut self, node: NodeId, parts: &Parts) -> Expr {
		let Some((container, key, map)) = self.pair_of(&parts) else {
			return self.unsupported(node, "this returned reference place");
		};
		let container = self.keep_value_part(Expr::primary(container));
		let key = self.keep_value_part(Expr::primary(key));
		let borrow = self.helper("faBorrowSlot");
		let erased = matches!(
			self.types.kind(self.without_none(self.type_of(node))),
			TypeKind::Param(_)
		);
		let cell = format!(
			"{borrow}({}, {}, {map}, {erased})",
			container.text, key.text
		);
		if parts.optional {
			Expr::primary(format!(
				"({} === undefined ? undefined : {cell})",
				container.text
			))
		} else {
			Expr::primary(cell)
		}
	}

	/// An erased Pool.add returns either an object or a primitive slot.
	/// Dispatch once at insertion and retain the exact target in the cell.
	fn generic_pool_reference(&mut self, node: NodeId) -> Option<Expr> {
		if let View::Call { callee, .. } = self.view(node)
			&& matches!(self.analysis.call(node).map(|call| call.callee), Some(Callee::Builtin(id)) if builtins::get(id).path == "Pool#find")
			&& matches!(self.types.kind(self.receiver_type(callee)), TypeKind::Collection(_, item, _) if matches!(self.types.kind(item), TypeKind::Param(_)))
		{
			let receiver = self.receiver(callee);
			let receiver = self.keep_value_part(receiver);
			let predicate = self.analysis.call(node)?.arguments.iter().find_map(|argument| match argument { Argument::Given(node) => Some(*node), _ => None })?;
			let predicate = self.expr(predicate);
			let find = self.helper("faFindReference");
			return Some(js::call(find, &[receiver.text, predicate.text]));
		}
		if let View::Call { callee, .. } = self.view(node)
			&& matches!(self.analysis.call(node).map(|call| call.callee), Some(Callee::Builtin(id)) if builtins::get(id).path == "Pool#add")
			&& matches!(self.types.kind(self.receiver_type(callee)), TypeKind::Collection(_, item, _) if matches!(self.types.kind(item), TypeKind::Param(_)))
		{
			let receiver = self.receiver(callee);
			let receiver = self.keep_value_part(receiver).primary_text();
			let argument = self
				.analysis
				.call(node)
				.and_then(|call| call.arguments.first())
				.and_then(|arg| {
					if let Argument::Given(node) = arg {
						Some(*node)
					} else {
						None
					}
				})
				.expect("Pool.add argument");
			let data = self.expr(argument);
			let data = self.copy_stored(argument, data);
			let data = self.keep_value_part(data).text;
			let result = self.temp();
			let borrow = self.helper("faBorrowSlot");
			let add = self.helper("faAdd");
			let expression = format!(
				"({receiver}[Symbol.toStringTag] === \"fa:ValuePool\" ? {borrow}({receiver}, {receiver}.add({data}), true, true) : {add}({receiver}, {data}))"
			);

			self.out.line(format!("const {result} = {expression};"));
			return Some(Expr::name(result, true));
		}
		None
	}

	// ── `use`, `mutable use`, `reuse` ────────────────────────────────────

	/// Whether references to values of a type are the JavaScript values
	/// themselves: objects and collections (optional or not).
	pub(crate) fn is_object_reference(&self, ty: TypeId) -> bool {
		let ty = self.without_none(ty);
		matches!(
			self.types.kind(ty),
			TypeKind::Object(_) | TypeKind::Collection(..) | TypeKind::Tuple(_) | TypeKind::Range
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
		if matches!(self.types.kind(self.without_none(ty)), TypeKind::Param(_)) {
			let carrier = self.reference_cell(value);
			let name = self.declare(symbol);
			let keyword = if module_value {
				""
			} else if mutable {
				"let "
			} else {
				"const "
			};
			self.out
				.line(format!("{keyword}{name} = {};", carrier.text));
			self.bind_erased_reference(symbol, &name);
			return;
		}
		if self.is_object_reference(ty) {
			let value = self.expr(value);
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
		let node = self.strip_reference(value);
		let existing = self.call_returns_reference(node)
			|| matches!(self.analysis.resolution(node), Resolution::Local(source) if self.reference_cells.contains_key(&source));
		if existing {
			let carrier = self.reference_cell(value);
			self.cell_declaration(symbol, carrier, mutable, ty);
			return;
		}
		let Some(parts) = self.reference_parts(value, false) else {
			return;
		};
		if let Key::Value(value) = parts.key {
			let name = self.declare(symbol);
			let keyword = if module_value {
				""
			} else if mutable {
				"let "
			} else {
				"const "
			};
			self.out.line(format!("{keyword}{name} = {};", value.text));
			return;
		}
		let carrier = self.cell_from_parts(value, &parts);
		self.cell_declaration(symbol, carrier, mutable, ty);
	}

	fn cell_declaration(&mut self, symbol: SymbolId, carrier: Expr, mutable: bool, ty: TypeId) {
		let module_value = self.is_module_value(symbol);
		let name = self.declare(symbol);
		let keyword = if module_value {
			""
		} else if mutable {
			"let "
		} else {
			"const "
		};
		self.out
			.line(format!("{keyword}{name} = {};", carrier.text));
		self.bind_primitive_reference(symbol, &name, ty);
	}

	fn bind_primitive_reference(&mut self, symbol: SymbolId, name: &str, ty: TypeId) {
		let access = if self.may_be_none(ty) { "?." } else { "." };
		self.alias(symbol, format!("{name}{access}$v"));
		self.reference_cells.insert(symbol, name.to_string());
		self.references.insert(
			symbol,
			Reference::Cell {
				carrier: name.to_string(),
				erased: false,
			},
		);
	}

	/// Prepare carrier names before functions that read module references.
	pub(crate) fn prepare_module_references(&mut self) {
		let mut symbols: Vec<_> = self
			.module_values
			.iter()
			.copied()
			.filter(|&symbol| {
				matches!(
					self.analysis.symbols.symbol(symbol).kind,
					SymbolKind::Use | SymbolKind::MutableUse
				)
			})
			.collect();
		symbols.sort();
		for symbol in symbols {
			let ty = self.analysis.symbols.symbol(symbol).ty;
			let name = self.name_of(symbol);
			if self.is_object_reference(ty) {
				self.references.insert(symbol, Reference::Object);
			} else if matches!(self.types.kind(self.without_none(ty)), TypeKind::Param(_)) {
				self.bind_erased_reference(symbol, &name);
			} else {
				let decl = self.analysis.symbols.symbol(symbol).decl_node;
				let value = match self.view(decl) {
					View::Use { value, .. } => self.strip_reference(value),
					_ => continue,
				};
				// Slices and characters are immutable values rather than writable slots.
				if let View::Index { target, .. } = self.view(value)
					&& self.kind(self.type_of(target)) == Kind::Text
				{
					continue;
				}
				self.bind_primitive_reference(symbol, &name, ty);
			}
		}
	}

	/// `reuse x = place` (or `none`): the `mutable use` designates another place.
	pub(crate) fn reuse_statement(&mut self, n: NodeId, value: NodeId) {
		let Resolution::Local(symbol) = self.analysis.resolution(n) else {
			self.unsupported(n, "this `reuse`");
			return;
		};
		match self.references.get(&symbol).cloned() {
			Some(Reference::Cell { carrier, .. }) => {
				let value = self.reference_cell(value);
				self.out.line(format!("{carrier} = {};", value.text));
			}
			Some(Reference::Object) => {
				let value = self.expr(value);
				let name = self.name_of(symbol);
				self.out.line(format!("{name} = {};", value.text));
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
					let value = self.expr(node);
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
		if self.call_returns_reference(node) {
			let View::Call { arguments, .. } = self.view(node) else {
				return None;
			};
			let cell = self.call(node, callee, arguments);
			let cell = self.keep_value_part(cell);
			return Some(Parts {
				container: Some(cell),
				key: Key::Field("$v".into()),
				optional: self.may_be_none(self.type_of(node)),
			});
		}
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
		if let Some(cell) = self.reference_cells.get(&symbol) {
			return Some(format!("{cell}.$k"));
		}
		self.map_aliases.get(&symbol).map(|(_, key)| key.clone())
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
