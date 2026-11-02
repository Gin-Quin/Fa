//! Arrays (types brief §5.1): literals, the item type of an iteration. The
//! members (`length`, `push`) and indexing are in `objects.rs`.

use crate::check::Walker;
use crate::diagnostics::codes::types::TY006;
use crate::diagnostics::codes::values::E_DECL_9;
use crate::ids::ParamId;
use crate::ids::{NodeId, TypeId};
use crate::syntax::{NodeList, View};
use crate::types::{CollKind, TypeKind};
use crate::types::{FnSig, ParamInfo, ParamSig};

/// The methods of arrays that change their array (types brief §5.0.9): they
/// need a mutable receiver (`EFF007`).
pub(super) fn mutates_array(method: &str) -> bool {
	matches!(
		method,
		"push"
			| "insert"
			| "delete"
			| "take" | "pop"
			| "clear" | "sortInPlace"
			| "reverseInPlace"
			| "deleteWhere"
	)
}

impl Walker<'_> {
	/// The type of the member `name` of an `Array(item)` (types brief §5.1,
	/// §5.0.6): `length`, the methods, with the generic ones (`map`,
	/// `reduce`, `sortInPlace`) using parameters inferred at the call.
	pub(super) fn array_method_type(&mut self, name: &str, item: TypeId) -> Option<TypeId> {
		let natural = {
			let storage = crate::types::IntStorage::Default;
			self.s
				.types
				.int_range(ethnum::I256::ZERO, storage.range().hi, storage)
		};
		let array = self.s.types.array(item);
		let optional_item = self.s.types.optional(item);
		let predicate = self.callback(&[item, TypeId::INTEGER], TypeId::BOOLEAN);
		let ty = match name {
			"length" => return Some(natural),
			"get" => self.method(&[TypeId::INTEGER], optional_item),
			"push" => self.method(&[item], TypeId::VOID),
			"insert" => self.method(&[TypeId::INTEGER, item], TypeId::VOID),
			"delete" => self.method(&[TypeId::INTEGER], TypeId::VOID),
			"take" => self.method(&[TypeId::INTEGER], item),
			"pop" => self.method(&[], optional_item),
			"clear" | "reverseInPlace" => self.method(&[], TypeId::VOID),
			"reverse" => self.method(&[], array),
			"isEmpty" => self.method(&[], TypeId::BOOLEAN),
			"includes" => self.method(&[item], TypeId::BOOLEAN),
			"indexOf" => {
				let index = self.s.types.optional(natural);
				self.method(&[item], index)
			}
			"find" => self.method(&[predicate], optional_item),
			"filter" => self.method(&[predicate], array),
			"every" | "some" => self.method(&[predicate], TypeId::BOOLEAN),
			"deleteWhere" => {
				let predicate = self.callback(&[item], TypeId::BOOLEAN);
				self.method(&[predicate], TypeId::VOID)
			}
			"map" => {
				let result = self.builtin_param("Array.map", "Result", TypeId::UNKNOWN);
				let transform = self.callback(&[item, TypeId::INTEGER], result);
				let mapped = self.s.types.array(result);
				self.method(&[transform], mapped)
			}
			"reduce" => {
				let accumulator =
					self.builtin_param("Array.reduce", "Accumulator", TypeId::UNKNOWN);
				let combine = self.callback(&[accumulator, item, TypeId::INTEGER], accumulator);
				self.method(&[combine, accumulator], accumulator)
			}
			"sort" | "sortInPlace" => {
				let numbers = self.s.types.union(&[TypeId::INTEGER, TypeId::FLOAT]);
				let order = self.builtin_param("Array.sort", "Order", numbers);
				let compare = self.callback(&[item, item], order);
				let ret = if name == "sort" { array } else { TypeId::VOID };
				self.method(&[compare], ret)
			}
			"sum" => {
				// numbers only (`CL024`): the base numeric type of the items
				match self.s.types.kind(item) {
					TypeKind::Int(_) | TypeKind::Float(_) => {
						let base = self.s.types.widen(item);
						let base = match self.s.types.kind(base) {
							TypeKind::Int(id) => TypeId::of_storage(self.s.types.int(id).storage),
							_ => base,
						};
						self.method(&[], base)
					}
					_ => return None,
				}
			}
			_ => return None,
		};
		Some(ty)
	}

	/// A method type `(params) => ret`.
	fn method(&mut self, params: &[TypeId], ret: TypeId) -> TypeId {
		let sig = self.s.types.simple_sig(params, ret, TypeId::NEVER);
		self.s.types.function(sig)
	}

	/// A callback type: what a lambda argument is checked against. A function
	/// given for it may leave out the trailing `index` (D36).
	fn callback(&mut self, params: &[TypeId], ret: TypeId) -> TypeId {
		let ignorable = u8::from(params.last() == Some(&TypeId::INTEGER) && params.len() > 1);
		let params: Vec<ParamSig> = params
			.iter()
			.map(|&ty| ParamSig {
				ty,
				variadic: false,
			})
			.collect();
		let base = self.s.types.simple_sig(&[], ret, TypeId::NEVER);
		let params = self.s.types.param_list(&params);
		self.s.types.function(FnSig {
			params,
			ignorable,
			..base
		})
	}

	/// The type parameter of a generic built-in method (`Result` of `map`),
	/// created once per file.
	fn builtin_param(
		&mut self,
		owner: &'static str,
		name: &'static str,
		constraint: TypeId,
	) -> TypeId {
		// `sort()` without a comparator: the natural order, an `Integer`
		let default = if owner == "Array.sort" {
			TypeId::INTEGER
		} else {
			TypeId::UNSET
		};
		if let Some(&ty) = self.builtin_params.get(owner) {
			return ty;
		}
		let module = self.s.types.module("prelude");
		let key = self.s.types.decl_key(module, &[owner], 0);
		let name = self.s.types.syms.intern(name);
		let param: ParamId = self.s.types.add_param(ParamInfo {
			owner: key,
			index: 0,
			name,
			constraint,
			default,
		});
		let ty = self.s.types.intern(TypeKind::Param(param));
		self.builtin_params.insert(owner, ty);
		ty
	}

	/// `[a, b, c]`: an `Array` of the expected item type, or of the join of
	/// the widened item types (D52). `[]` without context is an
	/// `Array(Never)`, which a declaration can't hold (`E-DECL-11`).
	pub(super) fn check_array_literal(
		&mut self,
		n: NodeId,
		items: NodeList<'_>,
		expected: Option<TypeId>,
	) -> TypeId {
		let expected_item = expected.and_then(|expected| self.expected_item(expected));
		let mut types = Vec::with_capacity(items.len());
		for item in items.iter() {
			if let View::Spread(_) = self.view(item) {
				self.unsupported(item, "spreads in arrays");
				types.push((item, TypeId::UNKNOWN));
				continue;
			}
			let ty = self.check(item, expected_item);
			if let Some(expected_item) = expected_item {
				self.sink(item, ty, expected_item);
			}
			types.push((item, ty));
		}
		self.memo(n, |w| {
			if let Some(item) = expected_item {
				return w.s.types.array(item);
			}
			if types.is_empty() {
				return w.s.types.array(TypeId::NEVER);
			}
			if types
				.iter()
				.any(|&(_, ty)| w.s.types.is_recovering(ty) || ty.is_unset())
			{
				return TypeId::UNKNOWN;
			}
			if let Some(&(item, _)) = types.iter().find(|&&(_, ty)| ty == TypeId::VOID) {
				let diagnostic = w.diagnostic(
					&E_DECL_9,
					item,
					"this expression has no value (`Void`): an array can't hold it",
				);
				w.report(diagnostic);
				return TypeId::UNKNOWN;
			}
			let integer = types
				.iter()
				.any(|&(_, ty)| w.s.types.is_literal(ty) && w.s.types.as_int(ty).is_some());
			let float = types
				.iter()
				.any(|&(_, ty)| matches!(w.s.types.kind(ty), TypeKind::FloatLit(..)));
			if integer && float {
				let diagnostic = w.diagnostic(
					&TY006,
					n,
					"integers and floats are mixed: write the integers as floats (`1.0`), or annotate the array",
				);
				w.report(diagnostic);
				return TypeId::UNKNOWN;
			}
			let widened: Vec<TypeId> = types.iter().map(|&(_, ty)| w.join_member(ty)).collect();
			let item = w.s.types.union(&widened);
			w.check_distinguishable(n, item);
			w.s.types.array(item)
		})
	}

	/// A type as a member of a join: widened, and a member of a named union
	/// or an enumeration widens to the whole type (D53).
	pub(super) fn join_member(&mut self, ty: TypeId) -> TypeId {
		match self.s.types.kind(ty) {
			TypeKind::Member(nominal, _) | TypeKind::EnumMember(nominal, _) => {
				self.s.types.nominal_type(nominal)
			}
			_ => self.s.types.widen(ty),
		}
	}

	/// The item type an expected type gives to the items of a list literal.
	fn expected_item(&self, expected: TypeId) -> Option<TypeId> {
		match self.s.types.kind(expected) {
			TypeKind::Collection(CollKind::Array, item, _) => Some(item),
			TypeKind::Union(list) => self.s.types.list_items(list).iter().find_map(|&member| {
				match self.s.types.kind(member) {
					TypeKind::Collection(CollKind::Array, item, _) => Some(item),
					_ => None,
				}
			}),
			_ => None,
		}
	}

	/// The type of the values a `for` loop over a value of type `ty` gives
	/// (`for item in items`): the items of an array, the characters of a
	/// string. `None` for a value that can't be iterated (yet).
	pub fn iteration_item(&self, ty: TypeId) -> Option<TypeId> {
		match self.s.types.kind(ty) {
			TypeKind::Collection(CollKind::Array, item, _) => Some(item),
			TypeKind::String | TypeKind::StringLit(_) => Some(TypeId::CHARACTER),
			TypeKind::Unknown => Some(TypeId::UNKNOWN),
			_ => None,
		}
	}
}
