//! The type relation: `assignable(S, T, mode)`, joins and literal adaptation.
//!
//! - Scalars (types brief §3.3, `design/decisions.md`): an integer type
//!   satisfies another when its range is contained, whatever the storage
//!   (`Integer` and `Int64` are mutually assignable); `Float32` satisfies
//!   `Float` (exact widening, never the reverse); `Character` satisfies
//!   `String`; a literal type satisfies its base type.
//! - Inside a collection (a view, [`Mode::View`]), the representation must not
//!   change: integer storages must be equal (`Array(Int8)`, `Array(Integer)` and
//!   `Array(Int64)` are distinct), no float widening, no `Character` → `String`.
//! - Unions and optionals: a member satisfies its union; a union satisfies a
//!   type when each of its members does.
//! - Objects, tuples and functions get the minimal structural rules here;
//!   writes through a narrower type, effects and generics are M1/M2.
//!
//! Results are cached per `(S, T, mode)`.

use ethnum::I256;

use crate::ids::{PathListId, TypeId};
use crate::types::store::TypeStore;
use crate::types::{FieldFlags, FloatBits, NominalKind, TypeFlags, TypeKind};

/// How a value is used where a type is expected.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum Mode {
	/// Read through a view (a parameter, a `use`): fields are views.
	Read,
	/// Copied into a new place (`let`, `mutable`, a field of a new object): values may convert.
	Copy,
	/// Viewed inside an aggregate, without conversion: storages must match.
	View,
	/// Written through on these paths (M2: treated as [`Mode::Exact`] for now).
	Write(PathListId),
	/// The same type.
	Exact,
}

/// Why a type doesn't satisfy another.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum Reason {
	/// Unrelated types.
	Mismatch,
	/// An integer range not contained in the target's.
	Range,
	/// The same values with another representation inside an aggregate (`CV005`).
	Storage,
	/// A field of the target is missing.
	MissingField,
	/// A field of the target has another type, or is `readonly` in the source only.
	Field,
	/// Not the same number of items or parameters.
	Arity,
	/// Another declaration of a nominal type.
	Nominal,
}

impl TypeStore {
	/// Whether a value of type `source` is accepted where `target` is expected.
	pub fn assignable(&mut self, source: TypeId, target: TypeId, mode: Mode) -> Result<(), Reason> {
		if source == target {
			return Ok(());
		}
		let key = (source, target, mode);
		if let Some(&result) = self.assign_cache.get(&key) {
			return result;
		}
		let incomplete = self.incomplete_count();
		let result = self.relate(source, target, mode);
		// a result that relied on a coinductive assumption is not final, nor
		// one that read a type not complete yet (P9: the cache is never cleared)
		if self.rec_assumptions.is_empty() && self.incomplete_count() == incomplete {
			self.assign_cache.insert(key, result);
		}
		result
	}

	/// `assignable(source, target, Read).is_ok()`.
	pub fn satisfies(&mut self, source: TypeId, target: TypeId) -> bool {
		self.assignable(source, target, Mode::Read).is_ok()
	}

	fn relate(&mut self, source: TypeId, target: TypeId, mode: Mode) -> Result<(), Reason> {
		use TypeKind as K;
		let (s, t) = (self.kind(source), self.kind(target));
		if matches!(s, K::Never | K::Unknown) || matches!(t, K::Unknown) {
			return Ok(());
		}
		// recursive types: equality is coinductive (types brief §3.6), so a
		// pair met again while relating the unfoldings is assumed related
		if matches!(s, K::Rec(_)) || matches!(t, K::Rec(_)) {
			if self.rec_assumptions.contains(&(source, target)) {
				return Ok(());
			}
			let (unfolded_source, unfolded_target) = (self.unfold(source), self.unfold(target));
			let unset =
				|ty: TypeId, unfolded: TypeId| ty == unfolded && matches!(self.kind(ty), K::Rec(_));
			if unset(source, unfolded_source) || unset(target, unfolded_target) {
				self.note_incomplete();
			}
			if (unfolded_source, unfolded_target) == (source, target) {
				return Err(Reason::Mismatch);
			}
			self.rec_assumptions.push((source, target));
			let result = self.assignable(unfolded_source, unfolded_target, mode);
			self.rec_assumptions.pop();
			return result;
		}
		if matches!(mode, Mode::Exact | Mode::Write(_)) {
			return Err(Reason::Mismatch);
		}
		// a value of a type parameter is any type satisfying its constraint
		if let K::Param(param) = s {
			if let K::Union(list) = t
				&& self.list_items(list).contains(&source)
			{
				return Ok(());
			}
			let constraint = self.param_constraint(param);
			if constraint.is_unset() || constraint == crate::ids::TypeId::UNKNOWN {
				return Err(Reason::Mismatch);
			}
			return self.assignable(constraint, target, mode);
		}
		// unions: each source member satisfies the target; a member satisfies its union
		// (index loops: the member list is borrowed from the store, which the recursion mutates)
		if let K::Union(list) = s {
			for index in 0..self.list_items(list).len() {
				let member = self.list_items(list)[index];
				self.assignable(member, target, mode)?;
			}
			return Ok(());
		}
		if let K::Union(list) = t {
			let mut best = Reason::Mismatch;
			for index in 0..self.list_items(list).len() {
				let member = self.list_items(list)[index];
				match self.assignable(source, member, mode) {
					Ok(()) => return Ok(()),
					Err(reason @ (Reason::Range | Reason::Storage)) => best = reason,
					Err(_) => {}
				}
			}
			// a named union whose members are each in the union
			// (`IndexError` in `IndexError.IndexError | DivisionByZero.DivisionByZero`)
			if let K::NamedUnion(nominal, mask) = s {
				let count = self.sym_list(self.nominal(nominal).members).len() as u16;
				let mut all = true;
				let indices: Vec<u16> = (0..count)
					.filter(|&index| self.mask_has(mask, index))
					.collect();
				for index in indices {
					let member = self.member(nominal, index);
					if self.assignable(member, target, mode).is_err() {
						all = false;
						break;
					}
				}
				if all {
					return Ok(());
				}
			}
			return Err(best);
		}
		let view = mode == Mode::View;
		match (s, t) {
			(K::Int(a), K::Int(b)) => {
				let (a, b) = (*self.int(a), *self.int(b));
				if view && a.storage != b.storage {
					return Err(Reason::Storage);
				}
				if b.interval().contains_interval(a.interval()) {
					Ok(())
				} else {
					Err(Reason::Range)
				}
			}
			(K::BoolLit(_), K::Boolean)
			| (K::BigIntLit(_), K::BigInt)
			| (K::StringLit(_), K::String) => Ok(()),
			(K::FloatLit(a, _), K::Float(b)) if a == b => Ok(()),
			(
				K::Float(FloatBits::F32) | K::FloatLit(FloatBits::F32, _),
				K::Float(FloatBits::F64),
			) => {
				if view {
					Err(Reason::Storage)
				} else {
					Ok(())
				}
			}
			(K::Character, K::String) => {
				if view {
					Err(Reason::Storage)
				} else {
					Ok(())
				}
			}
			(K::Member(a, index), K::NamedUnion(b, mask))
			| (K::EnumMember(a, index), K::Enum(b, mask))
				if a == b =>
			{
				if self.mask_has(mask, index) {
					Ok(())
				} else {
					Err(Reason::Mismatch)
				}
			}
			(K::NamedUnion(a, sub), K::NamedUnion(b, sup)) | (K::Enum(a, sub), K::Enum(b, sup))
				if a == b =>
			{
				if self.mask_subset(sub, sup) {
					Ok(())
				} else {
					Err(Reason::Mismatch)
				}
			}
			(K::Member(nominal, _) | K::NamedUnion(nominal, _), K::ErrorTop) => {
				if self.nominal(nominal).kind == NominalKind::Errors {
					Ok(())
				} else {
					Err(Reason::Mismatch)
				}
			}
			// a member with object data satisfies the type of its data (D4)
			(K::Member(nominal, index), K::Object(_)) => {
				self.note_payloads(nominal);
				let payload = self.payload(nominal, index);
				match self.kind(payload) {
					K::Object(_) => self.assignable(payload, target, mode),
					_ => Err(Reason::Mismatch),
				}
			}
			// a member defined by a literal accepts that literal directly
			(_, K::NamedUnion(nominal, _) | K::Member(nominal, _))
				if self.literal_member(source, target, nominal).is_some() =>
			{
				Ok(())
			}
			(K::Tuple(a), K::Tuple(b)) => {
				let (a, b) = (self.list_items(a).to_vec(), self.list_items(b).to_vec());
				if a.len() != b.len() {
					return Err(Reason::Arity);
				}
				let item_mode = if mode == Mode::Copy {
					Mode::Copy
				} else {
					Mode::View
				};
				for (item, expected) in a.into_iter().zip(b) {
					self.assignable(item, expected, item_mode)?;
				}
				Ok(())
			}
			(K::Object(a), K::Object(b)) => self.relate_objects(a, b, mode),
			(K::Collection(kind_a, item_a, key_a), K::Collection(kind_b, item_b, key_b)) => {
				if kind_a != kind_b {
					return Err(Reason::Mismatch);
				}
				// set elements and map keys are exact (CL010); other items are views
				let item_mode = if matches!(
					kind_a,
					crate::types::CollKind::Set | crate::types::CollKind::GrowingSet
				) {
					Mode::Exact
				} else {
					Mode::View
				};
				self.assignable(key_a, key_b, Mode::Exact)?;
				self.assignable(item_a, item_b, item_mode)
			}
			(K::Function(a), K::Function(b)) => self.relate_functions(a, b),
			(K::Ref(target_a, path_a), K::Ref(target_b, path_b)) if path_a == path_b => {
				self.assignable(target_a, target_b, Mode::View)
			}
			(K::Iterator(item_a, path_a), K::Iterator(item_b, path_b)) if path_a == path_b => {
				self.assignable(item_a, item_b, Mode::View)
			}
			(K::Task(result_a, throws_a), K::Task(result_b, throws_b)) => {
				self.assignable(result_a, result_b, Mode::View)?;
				self.assignable(throws_a, throws_b, Mode::Read)
			}
			_ => Err(Reason::Mismatch),
		}
	}

	fn relate_objects(
		&mut self,
		source: crate::ids::ShapeId,
		target: crate::ids::ShapeId,
		mode: Mode,
	) -> Result<(), Reason> {
		let (source_shape, target_shape) = (self.shape(source), self.shape(target));
		if let Some(nominal) = target_shape.nominal
			&& source_shape.nominal != Some(nominal)
		{
			return Err(Reason::Nominal);
		}
		let source_fields = self.shape_fields(source).to_vec();
		let target_fields = self.shape_fields(target).to_vec();
		let field_mode = if mode == Mode::Copy {
			Mode::Copy
		} else {
			Mode::View
		};
		// required methods are functions of the actual type, checked by the
		// walker where it knows the declarations (GN013)
		for expected in target_fields.iter().filter(|field| {
			!field
				.flags
				.intersects(FieldFlags::PRIVATE | FieldFlags::METHOD)
		}) {
			let Some(actual) = source_fields
				.iter()
				.find(|field| field.name == expected.name)
			else {
				return Err(Reason::MissingField);
			};
			if mode != Mode::Copy
				&& actual.flags.contains(FieldFlags::READONLY)
				&& !expected.flags.contains(FieldFlags::READONLY)
			{
				return Err(Reason::Field);
			}
			self.assignable(actual.ty, expected.ty, field_mode)
				.map_err(|_| Reason::Field)?;
		}
		Ok(())
	}

	fn relate_functions(
		&mut self,
		source: crate::ids::SigId,
		target: crate::ids::SigId,
	) -> Result<(), Reason> {
		let (mut source, target) = (self.sig(source), self.sig(target));
		// generic function types (phase 2 of "Generic functions carry their
		// list on the function value"): a generic function satisfies a generic
		// type with as many parameters when each constraint of the type
		// satisfies the function's (the function accepts at least what the type
		// accepts), its signature read with the type's parameters
		let source_generics = self.list_items(source.generics).to_vec();
		let target_generics = self.list_items(target.generics).to_vec();
		if !source_generics.is_empty() || !target_generics.is_empty() {
			let renamed = self.generic_renaming(&source_generics, &target_generics)?;
			let function = self.function(source);
			let renamed = self.substitute(function, &renamed);
			let TypeKind::Function(renamed) = self.kind(renamed) else {
				return Err(Reason::Mismatch);
			};
			source = self.sig(renamed);
		}
		let source_params = self.params(source.params).to_vec();
		let target_params = self.params(target.params).to_vec();
		let fewest = target_params
			.len()
			.saturating_sub(target.ignorable as usize);
		if source_params.len() > target_params.len() || source_params.len() < fewest {
			return Err(Reason::Arity);
		}
		for (actual, expected) in source_params.iter().zip(&target_params) {
			if actual.variadic != expected.variadic {
				return Err(Reason::Arity);
			}
			// parameters are contravariant
			self.assignable(expected.ty, actual.ty, Mode::Read)?;
			// The stored function must not consume a Task through an argument
			// that its visible contract borrows (notably a Resource view).
			if self.takes_task_parameter(actual.ty) && !self.takes_task_parameter(expected.ty) {
				return Err(Reason::Mismatch);
			}
		}
		if target.ret != TypeId::VOID {
			self.assignable(source.ret, target.ret, Mode::Read)?;
		}
		self.assignable(source.throws, target.throws, Mode::Read)?;
		if source.is_async && !target.is_async {
			return Err(Reason::Mismatch);
		}
		let target_requirements = self.requirements(target.requires).to_vec();
		let source_requirements = self.requirements(source.requires).to_vec();
		if !source_requirements.iter().all(|requirement| {
			target_requirements
				.iter()
				.any(|target| target.ty == requirement.ty)
		}) {
			return Err(Reason::Mismatch);
		}
		if source.effects != target.effects {
			let (mutates, invalidates) = self.effects(source.effects);
			if mutates
				.iter()
				.any(|&path| !crate::effects::set_covers(self, target.effects, path, false))
				|| invalidates
					.iter()
					.any(|&path| !crate::effects::set_covers(self, target.effects, path, true))
			{
				return Err(Reason::Mismatch);
			}
		}
		Ok(())
	}

	/// The bindings that read a generic function's parameters as a generic
	/// function type's (`source` and `target` are their `Param` types): as many
	/// parameters, and the constraint of each parameter of the type satisfies
	/// the function's (it can't be narrower than the type's).
	pub fn generic_renaming(
		&mut self,
		source: &[TypeId],
		target: &[TypeId],
	) -> Result<crate::types::Bindings, Reason> {
		if source.len() != target.len() {
			return Err(Reason::Arity);
		}
		let mut params = Vec::with_capacity(source.len());
		for (&source, &target) in source.iter().zip(target) {
			let (TypeKind::Param(source), TypeKind::Param(target_param)) =
				(self.kind(source), self.kind(target))
			else {
				return Err(Reason::Mismatch);
			};
			if self.param(source).value != self.param(target_param).value {
				return Err(Reason::Mismatch);
			}
			let wanted = self.param_constraint(source);
			if !wanted.is_unset() && wanted != TypeId::UNKNOWN {
				let given = self.param_constraint(target_param);
				if given.is_unset() || given == TypeId::UNKNOWN || !self.satisfies(given, wanted) {
					return Err(Reason::Field);
				}
			}
			params.push(source);
		}
		let mut bindings = crate::types::Bindings::new(&params);
		bindings.types = target.to_vec();
		Ok(bindings)
	}

	/// The type of a value that is one of `a` or `b` with no expected type
	/// (branches of an `if`, a `when`, a `catch`): the normalized union of the
	/// widened types (values brief D-29/D-98). The caller checks the result
	/// for `TY040` and `E-IF-2` (`TY006`).
	pub fn join(&mut self, a: TypeId, b: TypeId) -> TypeId {
		let (a, b) = (self.widen(a), self.widen(b));
		self.union(&[a, b])
	}

	/// The type a literal takes where `expected` is expected (values brief §2.2),
	/// or `None` when it doesn't adapt. Integer literals adapt to an integer
	/// type containing them, to a float type representing them exactly, to
	/// `BigInt`; float literals to `Float32` (when finite after rounding) and
	/// `Float`. String literals adapting to `Character` (one grapheme cluster)
	/// and literals adapting to a union member are decided by the walk (M1).
	pub fn adapt_literal(&mut self, literal: TypeId, expected: TypeId) -> Option<TypeId> {
		use TypeKind as K;
		match (self.kind(literal), self.kind(expected)) {
			(K::Int(id), K::Int(target)) => {
				let value = self.int(id).literal()?;
				let target = *self.int(target);
				target
					.interval()
					.contains(value)
					.then(|| self.int_range(value, value, target.storage))
			}
			(K::Int(id), K::Float(bits)) => {
				let value = self.int(id).literal()?;
				exactly_representable(value, bits).then(|| self.float_literal(bits, value.as_f64()))
			}
			(K::Int(id), K::BigInt) => {
				let value = self.int(id).literal()?;
				let text = self.syms.intern(&value.to_string());
				let constant = self.constant(crate::types::ConstValue::BigInt(text));
				Some(self.intern(K::BigIntLit(constant)))
			}
			// a literal adapts to a type parameter when it adapts to every
			// member of its union constraint (D19): `mutable total: Value = 0`
			(_, K::Param(param)) if self.is_literal(literal) => {
				let constraint = self.param_constraint(param);
				if constraint.is_unset() || constraint == crate::ids::TypeId::UNKNOWN {
					return None;
				}
				let members = self.union_members(&constraint).to_vec();
				members
					.into_iter()
					.all(|member| self.adapt_literal(literal, member).is_some())
					.then_some(expected)
			}
			(K::FloatLit(FloatBits::F64, constant), K::Float(FloatBits::F32)) => {
				let crate::types::ConstValue::Float(bits) = self.const_value(constant) else {
					return None;
				};
				let value = f64::from_bits(bits);
				let rounded = value as f32;
				(rounded.is_finite() || !value.is_finite())
					.then(|| self.float_literal(FloatBits::F32, rounded as f64))
			}
			_ => self
				.assignable(literal, expected, Mode::Read)
				.is_ok()
				.then_some(literal),
		}
	}

	/// The member of `target` (a named union or one of its members) defined
	/// by the literal type `literal`, if any: `"hello"` for `Hello = "hello"`.
	pub fn literal_member(
		&self,
		literal: TypeId,
		target: TypeId,
		nominal: crate::ids::NominalId,
	) -> Option<u16> {
		if !self.is_literal(literal) {
			return None;
		}
		let count = self.sym_list(self.nominal(nominal).members).len() as u16;
		self.note_payloads(nominal);
		(0..count).find(|&index| {
			self.payload(nominal, index) == literal
				&& match self.kind(target) {
					TypeKind::NamedUnion(_, mask) => self.mask_has(mask, index),
					TypeKind::Member(_, member) => member == index,
					_ => false,
				}
		})
	}

	/// Whether diagnostics about this type should be suppressed (it contains `Unknown`).
	pub fn is_recovering(&self, ty: TypeId) -> bool {
		self.flags(ty).contains(TypeFlags::HAS_UNKNOWN)
	}
}

/// Whether an integer is exactly representable in a float type (values brief D-20).
fn exactly_representable(value: I256, bits: FloatBits) -> bool {
	let magnitude = value.unsigned_abs();
	if magnitude == 0 {
		return true;
	}
	let significant = 256 - magnitude.leading_zeros() - magnitude.trailing_zeros();
	let (mantissa, max_exponent) = match bits {
		FloatBits::F32 => (24, 128),
		FloatBits::F64 => (53, 1024),
	};
	significant <= mantissa && 256 - magnitude.leading_zeros() <= max_exponent
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::ids::{ListId, Sym};
	use crate::types::{CollKind, FieldEntry, IntStorage, NominalInfo};

	fn n(value: i128) -> I256 {
		I256::from(value)
	}

	fn field(store: &mut TypeStore, name: &str, ty: TypeId) -> FieldEntry {
		FieldEntry {
			name: store.syms.intern(name),
			ty,
			flags: FieldFlags::empty(),
		}
	}

	fn ok(store: &mut TypeStore, source: TypeId, target: TypeId) -> bool {
		store.assignable(source, target, Mode::Read).is_ok()
	}

	#[test]
	fn identical_types_and_special_types() {
		let mut store = TypeStore::new();
		assert!(ok(&mut store, TypeId::STRING, TypeId::STRING));
		assert!(ok(&mut store, TypeId::NEVER, TypeId::STRING));
		assert!(ok(&mut store, TypeId::UNKNOWN, TypeId::INT8));
		assert!(ok(&mut store, TypeId::INT8, TypeId::UNKNOWN));
		assert!(!ok(&mut store, TypeId::STRING, TypeId::NEVER));
		assert!(!ok(&mut store, TypeId::VOID, TypeId::INTEGER));
		assert!(!ok(&mut store, TypeId::NONE, TypeId::INTEGER));
		assert!(!ok(&mut store, TypeId::INTEGER, TypeId::BOOLEAN));
	}

	#[test]
	fn integer_ranges() {
		let mut store = TypeStore::new();
		let small = store.int_range(n(0), n(10), IntStorage::Default);
		let five = store.int_literal(n(5));
		let big = store.int_literal(n(300));
		assert!(ok(&mut store, five, small));
		assert!(ok(&mut store, small, TypeId::INTEGER));
		assert!(!ok(&mut store, TypeId::INTEGER, small));
		assert!(ok(&mut store, TypeId::INT8, TypeId::INT16));
		assert!(!ok(&mut store, TypeId::INT16, TypeId::INT8));
		assert_eq!(
			store.assignable(big, TypeId::UINT8, Mode::Read),
			Err(Reason::Range)
		);
		assert!(!ok(&mut store, TypeId::INT8, TypeId::UINT8));
		assert!(ok(&mut store, TypeId::UINT32, TypeId::INTEGER));
		assert!(!ok(&mut store, TypeId::UINT64, TypeId::INTEGER));
		assert!(ok(&mut store, TypeId::UINT64, TypeId::INT128));
		assert!(!ok(&mut store, TypeId::INT128, TypeId::UINT128));
	}

	#[test]
	fn integer_and_int64_are_assignable_as_scalars_only() {
		let mut store = TypeStore::new();
		assert!(ok(&mut store, TypeId::INTEGER, TypeId::INT64));
		assert!(ok(&mut store, TypeId::INT64, TypeId::INTEGER));
		let integers = store.array(TypeId::INTEGER);
		let int64s = store.array(TypeId::INT64);
		let int8s = store.array(TypeId::INT8);
		assert_eq!(
			store.assignable(integers, int64s, Mode::Read),
			Err(Reason::Storage)
		);
		assert_eq!(
			store.assignable(int64s, integers, Mode::Copy),
			Err(Reason::Storage)
		);
		assert!(!ok(&mut store, int8s, integers));
		// a ranged refinement with the same storage is a valid view
		let naturals = {
			let natural =
				store.int_range(n(0), IntStorage::Default.range().hi, IntStorage::Default);
			store.array(natural)
		};
		assert!(ok(&mut store, naturals, integers));
		assert!(!ok(&mut store, integers, naturals));
	}

	#[test]
	fn floats_widen_only_from_float32() {
		let mut store = TypeStore::new();
		assert!(ok(&mut store, TypeId::FLOAT32, TypeId::FLOAT));
		assert!(!ok(&mut store, TypeId::FLOAT, TypeId::FLOAT32));
		assert!(!ok(&mut store, TypeId::INTEGER, TypeId::FLOAT));
		assert!(!ok(&mut store, TypeId::FLOAT, TypeId::INTEGER));
		assert!(!ok(&mut store, TypeId::PERCENTAGE, TypeId::FLOAT));
		assert!(!ok(&mut store, TypeId::BIG_INT, TypeId::INTEGER));
		let half = store.float_literal(FloatBits::F64, 0.5);
		assert!(ok(&mut store, half, TypeId::FLOAT));
		assert!(!ok(&mut store, half, TypeId::FLOAT32));
		let floats32 = store.array(TypeId::FLOAT32);
		let floats = store.array(TypeId::FLOAT);
		assert!(!ok(&mut store, floats32, floats));
	}

	#[test]
	fn characters_and_literals() {
		let mut store = TypeStore::new();
		assert!(ok(&mut store, TypeId::CHARACTER, TypeId::STRING));
		assert!(!ok(&mut store, TypeId::STRING, TypeId::CHARACTER));
		let hello = store.string_literal("hello");
		let goodbye = store.string_literal("goodbye");
		assert!(ok(&mut store, hello, TypeId::STRING));
		assert!(!ok(&mut store, goodbye, hello));
		assert!(ok(&mut store, TypeId::TRUE, TypeId::BOOLEAN));
		assert!(!ok(&mut store, TypeId::TRUE, TypeId::FALSE));
		let characters = store.array(TypeId::CHARACTER);
		let strings = store.array(TypeId::STRING);
		assert!(!ok(&mut store, characters, strings));
	}

	#[test]
	fn unions_and_optionals() {
		let mut store = TypeStore::new();
		let maybe = store.optional(TypeId::INTEGER);
		assert!(ok(&mut store, TypeId::INTEGER, maybe));
		assert!(ok(&mut store, TypeId::NONE, maybe));
		assert!(ok(&mut store, TypeId::INT8, maybe));
		assert!(!ok(&mut store, maybe, TypeId::INTEGER));
		assert!(!ok(&mut store, TypeId::STRING, maybe));
		let text_or_number = store.union(&[TypeId::STRING, TypeId::INTEGER]);
		let wider = store.union(&[TypeId::STRING, TypeId::INTEGER, TypeId::NONE]);
		assert!(ok(&mut store, maybe, wider));
		assert!(!ok(&mut store, maybe, text_or_number));
		let a = store.string_literal("a");
		let b = store.string_literal("b");
		let ab = store.union(&[a, b]);
		assert!(ok(&mut store, a, ab));
		assert!(ok(&mut store, ab, TypeId::STRING));
		assert!(!ok(&mut store, TypeId::STRING, ab));
		let big = store.int_literal(n(1000));
		let small_or_text = store.union(&[TypeId::INT8, TypeId::STRING]);
		assert_eq!(
			store.assignable(big, small_or_text, Mode::Read),
			Err(Reason::Range)
		);
	}

	#[test]
	fn named_unions_and_errors() {
		let mut store = TypeStore::new();
		let module = store.module("main");
		let key = store.decl_key(module, &["ApiError"], 0);
		let names: Vec<Sym> = ["Timeout", "Network"]
			.iter()
			.map(|name| store.syms.intern(name))
			.collect();
		let members = store.intern_sym_list(&names);
		let errors = store.intern_nominal(NominalInfo {
			key,
			args: ListId::EMPTY,
			kind: NominalKind::Errors,
			members,
		});
		let all = store.nominal_type(errors);
		let timeout = store.member(errors, 0);
		assert!(ok(&mut store, timeout, all));
		assert!(!ok(&mut store, all, timeout));
		assert!(ok(&mut store, timeout, TypeId::ERROR));
		assert!(ok(&mut store, all, TypeId::ERROR));
		let shape_key = store.decl_key(module, &["Shape"], 0);
		let shape = store.intern_nominal(NominalInfo {
			key: shape_key,
			args: ListId::EMPTY,
			kind: NominalKind::Union,
			members,
		});
		let circle = store.member(shape, 0);
		assert!(!ok(&mut store, circle, TypeId::ERROR));
		assert!(!ok(&mut store, circle, all));
	}

	#[test]
	fn objects_are_structural() {
		let mut store = TypeStore::new();
		let name = field(&mut store, "name", TypeId::STRING);
		let age = field(&mut store, "age", TypeId::INTEGER);
		let small_age = field(&mut store, "age", TypeId::INT8);
		let person = store.object(&[name, age], None);
		let named = store.object(&[name], None);
		let small_person = store.object(&[name, small_age], None);
		assert!(ok(&mut store, person, named));
		assert_eq!(
			store.assignable(named, person, Mode::Read),
			Err(Reason::MissingField)
		);
		// a field read through a view keeps its storage; a copy converts
		assert_eq!(
			store.assignable(small_person, person, Mode::Read),
			Err(Reason::Field)
		);
		assert!(store.assignable(small_person, person, Mode::Copy).is_ok());
		let readonly_name = FieldEntry {
			flags: FieldFlags::READONLY,
			..name
		};
		let frozen = store.object(&[readonly_name], None);
		assert!(ok(&mut store, named, frozen));
		assert!(!ok(&mut store, frozen, named));
		assert!(store.assignable(frozen, named, Mode::Copy).is_ok());
	}

	#[test]
	fn tuples_collections_functions() {
		let mut store = TypeStore::new();
		let pair = store.tuple(&[TypeId::INT8, TypeId::STRING]);
		let wide = store.tuple(&[TypeId::INTEGER, TypeId::STRING]);
		let single = store.tuple(&[TypeId::INTEGER]);
		assert!(!ok(&mut store, pair, wide));
		assert!(store.assignable(pair, wide, Mode::Copy).is_ok());
		assert_eq!(
			store.assignable(pair, single, Mode::Read),
			Err(Reason::Arity)
		);
		let array = store.array(TypeId::STRING);
		let growing = store.collection(CollKind::GrowingArray, TypeId::STRING, TypeId::NONE);
		assert!(!ok(&mut store, array, growing));
		let sig = store.simple_sig(&[TypeId::INTEGER], TypeId::INT8, TypeId::NEVER);
		let narrow = store.function(sig);
		let sig = store.simple_sig(&[TypeId::INT8], TypeId::INTEGER, TypeId::NEVER);
		let wide_fn = store.function(sig);
		assert!(ok(&mut store, narrow, wide_fn));
		assert!(!ok(&mut store, wide_fn, narrow));
	}

	#[test]
	fn exact_mode_requires_identity() {
		let mut store = TypeStore::new();
		assert!(
			store
				.assignable(TypeId::INT8, TypeId::INT8, Mode::Exact)
				.is_ok()
		);
		assert!(
			store
				.assignable(TypeId::INT8, TypeId::INT16, Mode::Exact)
				.is_err()
		);
		assert!(
			store
				.assignable(TypeId::NEVER, TypeId::INT16, Mode::Exact)
				.is_ok()
		);
		let ints = store.collection(CollKind::Set, TypeId::INT8, TypeId::NONE);
		let wider = store.collection(CollKind::Set, TypeId::INT16, TypeId::NONE);
		assert!(!ok(&mut store, ints, wider));
	}

	#[test]
	fn joins_widen_literals() {
		let mut store = TypeStore::new();
		let yes = store.string_literal("yes");
		let zero = store.int_literal(n(0));
		let joined = store.join(yes, zero);
		let expected = store.union(&[TypeId::STRING, TypeId::INTEGER]);
		assert_eq!(joined, expected);
		let one = store.int_literal(n(1));
		assert_eq!(store.join(one, zero), TypeId::INTEGER);
		assert_eq!(store.join(TypeId::NEVER, yes), TypeId::STRING);
		let maybe = store.join(TypeId::NONE, one);
		assert_eq!(maybe, store.optional(TypeId::INTEGER));
	}

	#[test]
	fn literal_adaptation() {
		let mut store = TypeStore::new();
		let value = store.int_literal(n(100));
		let adapted = store.adapt_literal(value, TypeId::INT8).unwrap();
		assert_eq!(store.as_int(adapted).unwrap().storage, IntStorage::I8);
		let big = store.int_literal(n(300));
		assert_eq!(store.adapt_literal(big, TypeId::UINT8), None);
		assert!(store.adapt_literal(big, TypeId::FLOAT).is_some());
		assert!(store.adapt_literal(big, TypeId::BIG_INT).is_some());
		let unrepresentable = store.int_literal(n(16_777_217));
		assert_eq!(store.adapt_literal(unrepresentable, TypeId::FLOAT32), None);
		assert!(
			store
				.adapt_literal(unrepresentable, TypeId::FLOAT)
				.is_some()
		);
		let power = store.int_literal(n(1 << 60));
		assert!(store.adapt_literal(power, TypeId::FLOAT32).is_some());
		let tenth = store.float_literal(FloatBits::F64, 0.1);
		assert!(store.adapt_literal(tenth, TypeId::FLOAT32).is_some());
		let huge = store.float_literal(FloatBits::F64, 1e39);
		assert_eq!(store.adapt_literal(huge, TypeId::FLOAT32), None);
		assert_eq!(store.adapt_literal(tenth, TypeId::INTEGER), None);
		assert_eq!(store.adapt_literal(value, TypeId::PERCENTAGE), None);
	}

	#[test]
	fn representability() {
		assert!(exactly_representable(n(16_777_216), FloatBits::F32));
		assert!(!exactly_representable(n(16_777_217), FloatBits::F32));
		assert!(exactly_representable(n(-16_777_215), FloatBits::F32));
		assert!(exactly_representable(n(1 << 53), FloatBits::F64));
		assert!(!exactly_representable(n((1 << 53) + 1), FloatBits::F64));
		assert!(exactly_representable(n(0), FloatBits::F32));
	}

	#[test]
	fn results_are_cached() {
		let mut store = TypeStore::new();
		let before = store.assign_cache.len();
		assert!(ok(&mut store, TypeId::INT8, TypeId::INTEGER));
		assert!(ok(&mut store, TypeId::INT8, TypeId::INTEGER));
		assert_eq!(store.assign_cache.len(), before + 1);
	}
}
