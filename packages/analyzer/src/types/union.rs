//! Anonymous unions: normalization (types brief §3.4.1, decision D1) and
//! distinguishability (§3.4.2, `TY040`).

use smallvec::SmallVec;

use crate::ids::{NominalId, TypeId};
use crate::types::relate::Mode;
use crate::types::store::TypeStore;
use crate::types::{IntType, Interval, NominalKind, RuntimeKind, TypeKind};

impl TypeStore {
	/// The normalized union of `members`, run on every `|`, join, difference
	/// and substitution:
	///
	/// 1. flatten nested anonymous unions (a named union is one member);
	/// 2. drop `Never` (a union with `Unknown` is `Unknown`);
	/// 3. absorption, non-object members only (D1): drop a literal, an integer
	///    range, `None`, an enumeration or named-union member, a `Fields` type
	///    or a collection that satisfies another member without changing
	///    representation ([`Mode::View`]: `Integer | Int64` keeps both);
	/// 4. merge integer ranges of the same storage that touch or overlap;
	/// 5. merge members of one named union or enumeration into its masked form;
	/// 6. sort by id and dedupe. No member is `Never`, one is that member.
	pub fn union(&mut self, members: &[TypeId]) -> TypeId {
		let mut scratch = std::mem::take(&mut self.scratch);
		scratch.clear();
		// 1–2. flatten, drop Never
		for &member in members {
			match self.kind(member) {
				TypeKind::Union(list) => scratch.extend_from_slice(self.list_items(list)),
				TypeKind::Never => {}
				TypeKind::Unknown => {
					self.scratch = scratch;
					return TypeId::UNKNOWN;
				}
				_ => scratch.push(member),
			}
		}
		scratch.sort_unstable();
		scratch.dedup();
		// 5. masked forms first, so that absorption sees whole subsets
		self.merge_nominal_members(&mut scratch);
		// 3. absorption of non-object members
		let mut index = 0;
		while index < scratch.len() {
			let member = scratch[index];
			let absorbable = self.is_absorbable(member);
			let absorbed = absorbable
				&& (0..scratch.len()).any(|other| {
					other != index
						&& scratch[other] != member
						&& self.assignable(member, scratch[other], Mode::View).is_ok()
				});
			if absorbed {
				scratch.remove(index);
			} else {
				index += 1;
			}
		}
		// 4. merge touching ranges of one storage
		self.merge_ranges(&mut scratch);
		// 6. sort, dedupe
		scratch.sort_unstable();
		scratch.dedup();
		let result = match scratch.len() {
			0 => TypeId::NEVER,
			1 => scratch[0],
			_ => {
				let list = self.list(&scratch);
				self.intern(TypeKind::Union(list))
			}
		};
		self.scratch = scratch;
		result
	}

	/// The members of a type seen as a union: its list for an anonymous union, itself otherwise.
	pub fn union_members<'a>(&'a self, ty: &'a TypeId) -> &'a [TypeId] {
		match self.kind(*ty) {
			TypeKind::Union(list) => self.list_items(list),
			_ => std::slice::from_ref(ty),
		}
	}

	/// Whether a member may be absorbed by another one (decision D1): never an
	/// object, a tuple or a function.
	fn is_absorbable(&self, ty: TypeId) -> bool {
		match self.kind(ty) {
			TypeKind::Int(_)
			| TypeKind::BoolLit(_)
			| TypeKind::FloatLit(..)
			| TypeKind::BigIntLit(_)
			| TypeKind::StringLit(_)
			| TypeKind::None
			| TypeKind::EnumMember(..)
			| TypeKind::Enum(..)
			| TypeKind::Member(..)
			| TypeKind::NamedUnion(..)
			| TypeKind::Fields(_)
			| TypeKind::Collection(..) => true,
			// the scalar types themselves: `Character | String` keeps both (distinguishability decides)
			_ => false,
		}
	}

	fn merge_ranges(&mut self, members: &mut Vec<TypeId>) {
		let mut ranges: SmallVec<[IntType; 8]> = SmallVec::new();
		members.retain(|&member| match self.kind(member) {
			TypeKind::Int(id) => {
				ranges.push(*self.int(id));
				false
			}
			_ => true,
		});
		if ranges.is_empty() {
			return;
		}
		ranges.sort_by_key(|range| (range.storage, range.min));
		let mut merged: SmallVec<[IntType; 8]> = SmallVec::new();
		for range in ranges {
			match merged.last_mut() {
				Some(last)
					if last.storage == range.storage
						&& last.interval().touches(range.interval()) =>
				{
					let hull: Interval = last.interval().hull(range.interval());
					last.min = hull.lo;
					last.max = hull.hi;
				}
				_ => merged.push(range),
			}
		}
		for range in merged {
			members.push(self.int_type(range));
		}
	}

	fn merge_nominal_members(&mut self, members: &mut Vec<TypeId>) {
		let mut groups: SmallVec<[MaskGroup; 2]> = SmallVec::new();
		members.retain(|&member| {
			let (nominal, is_enum, words) = match self.kind(member) {
				TypeKind::Member(nominal, index) => (nominal, false, single_bit(index)),
				TypeKind::EnumMember(nominal, index) => (nominal, true, single_bit(index)),
				TypeKind::NamedUnion(nominal, mask) => {
					(nominal, false, self.mask_bits(mask).into())
				}
				TypeKind::Enum(nominal, mask) => (nominal, true, self.mask_bits(mask).into()),
				_ => return true,
			};
			match groups
				.iter_mut()
				.find(|(n, e, _)| *n == nominal && *e == is_enum)
			{
				Some((_, _, bits)) => {
					if bits.len() < words.len() {
						bits.resize(words.len(), 0);
					}
					for (bit, word) in bits.iter_mut().zip(&words) {
						*bit |= word;
					}
				}
				None => groups.push((nominal, is_enum, words)),
			}
			false
		});
		for (nominal, is_enum, words) in groups {
			let mask = self.mask(&words);
			members.push(if is_enum {
				self.enumeration(nominal, mask)
			} else {
				self.named_union(nominal, mask)
			});
		}
	}

	/// The first pair of members of a union that can't be told apart at
	/// runtime (`TY040`, types brief §3.4.2), if any. Run on every union written
	/// by the user and on every join without expected type.
	pub fn indistinguishable_pair(&self, union: TypeId) -> Option<(TypeId, TypeId)> {
		let members = match self.kind(union) {
			TypeKind::Union(list) => self.list_items(list),
			_ => return None,
		};
		for (i, &a) in members.iter().enumerate() {
			for &b in &members[i + 1..] {
				if !self.distinguishable(a, b) {
					return Some((a, b));
				}
			}
		}
		None
	}

	/// Whether two members of a union can be told apart at runtime.
	pub fn distinguishable(&self, a: TypeId, b: TypeId) -> bool {
		let (kind_a, kind_b) = (self.runtime_kind(a), self.runtime_kind(b));
		if kind_a != kind_b {
			return true;
		}
		if self.is_literal(a) && self.is_literal(b) {
			return a != b;
		}
		let (ta, tb) = (self.kind(a), self.kind(b));
		match (ta, tb) {
			(TypeKind::Int(x), TypeKind::Int(y))
				if self.int(x).interval().is_disjoint(self.int(y).interval()) =>
			{
				true
			}
			(TypeKind::EnumMember(x, i), TypeKind::EnumMember(y, j)) => x == y && i != j,
			_ if kind_a == RuntimeKind::Object => {
				let nominal_a = self.is_tagged(ta);
				let nominal_b = self.is_tagged(tb);
				if nominal_a && nominal_b {
					return true;
				}
				match (ta, tb) {
					(TypeKind::Object(x), TypeKind::Object(y)) => {
						self.objects_distinguishable(x, y)
					}
					_ => false,
				}
			}
			_ => false,
		}
	}

	/// Members and subsets of named unions carry a global tag (types brief D8).
	fn is_tagged(&self, kind: TypeKind) -> bool {
		match kind {
			TypeKind::Member(nominal, _) | TypeKind::NamedUnion(nominal, _) => {
				matches!(
					self.nominal(nominal).kind,
					NominalKind::Union | NominalKind::Errors
				)
			}
			_ => false,
		}
	}

	/// Each has a field the other lacks, or both have a field whose types are
	/// disjoint literal types (a discriminant).
	fn objects_distinguishable(&self, a: crate::ids::ShapeId, b: crate::ids::ShapeId) -> bool {
		let (fields_a, fields_b) = (self.shape_fields(a), self.shape_fields(b));
		let lacks = |left: &[crate::types::FieldEntry], right: &[crate::types::FieldEntry]| {
			left.iter()
				.any(|field| !right.iter().any(|other| other.name == field.name))
		};
		if lacks(fields_a, fields_b) && lacks(fields_b, fields_a) {
			return true;
		}
		fields_a.iter().any(|field| {
			fields_b.iter().any(|other| {
				other.name == field.name
					&& field.ty != other.ty
					&& self.is_literal_union(field.ty)
					&& self.is_literal_union(other.ty)
					&& self.literal_sets_disjoint(field.ty, other.ty)
			})
		})
	}

	fn is_literal_union(&self, ty: TypeId) -> bool {
		match self.kind(ty) {
			TypeKind::Union(list) => self
				.list_items(list)
				.iter()
				.all(|&member| self.is_literal(member)),
			TypeKind::EnumMember(..) => true,
			_ => self.is_literal(ty),
		}
	}

	fn literal_sets_disjoint(&self, a: TypeId, b: TypeId) -> bool {
		let members = |ty: TypeId| -> SmallVec<[TypeId; 4]> {
			match self.kind(ty) {
				TypeKind::Union(list) => self.list_items(list).into(),
				_ => smallvec::smallvec![ty],
			}
		};
		let (a, b) = (members(a), members(b));
		a.iter().all(|x| !b.contains(x))
	}
}

/// The members of one named union (or enumeration, the flag) found in a union.
type MaskGroup = (NominalId, bool, SmallVec<[u64; 2]>);

fn single_bit(index: u16) -> SmallVec<[u64; 2]> {
	let index = index as usize;
	let mut words: SmallVec<[u64; 2]> = smallvec::smallvec![0; index / 64 + 1];
	words[index / 64] = 1 << (index % 64);
	words
}

#[cfg(test)]
mod tests {
	use ethnum::I256;

	use super::*;
	use crate::ids::{ListId, Sym};
	use crate::types::{CollKind, FieldEntry, FieldFlags, FloatBits, IntStorage, NominalInfo};

	fn n(value: i128) -> I256 {
		I256::from(value)
	}

	fn nominal(
		store: &mut TypeStore,
		name: &str,
		kind: NominalKind,
		members: &[&str],
	) -> NominalId {
		let module = store.module("main");
		let key = store.decl_key(module, &[name], 0);
		let names: Vec<Sym> = members
			.iter()
			.map(|member| store.syms.intern(member))
			.collect();
		let members = store.intern_sym_list(&names);
		store.intern_nominal(NominalInfo {
			key,
			args: ListId::EMPTY,
			kind,
			members,
		})
	}

	fn object(store: &mut TypeStore, fields: &[(&str, TypeId)]) -> TypeId {
		let fields: Vec<FieldEntry> = fields
			.iter()
			.map(|&(name, ty)| FieldEntry {
				name: store.syms.intern(name),
				ty,
				flags: FieldFlags::empty(),
			})
			.collect();
		store.object(&fields, None)
	}

	#[test]
	fn degenerate_unions() {
		let mut store = TypeStore::new();
		assert_eq!(store.union(&[]), TypeId::NEVER);
		assert_eq!(store.union(&[TypeId::NEVER]), TypeId::NEVER);
		assert_eq!(store.union(&[TypeId::STRING]), TypeId::STRING);
		assert_eq!(
			store.union(&[TypeId::STRING, TypeId::NEVER]),
			TypeId::STRING
		);
		assert_eq!(
			store.union(&[TypeId::STRING, TypeId::STRING]),
			TypeId::STRING
		);
		assert_eq!(
			store.union(&[TypeId::STRING, TypeId::UNKNOWN]),
			TypeId::UNKNOWN
		);
	}

	#[test]
	fn unions_are_flattened_sorted_and_deduped() {
		let mut store = TypeStore::new();
		let a = store.union(&[TypeId::STRING, TypeId::INTEGER]);
		let b = store.union(&[TypeId::INTEGER, TypeId::STRING]);
		assert_eq!(a, b);
		let optional = store.optional(a);
		let nested = store.union(&[TypeId::NONE, b]);
		assert_eq!(optional, nested);
		assert_eq!(
			store.union_members(&optional),
			&[TypeId::NONE, TypeId::INTEGER, TypeId::STRING]
		);
		// T?? is T?
		let twice = store.optional(optional);
		assert_eq!(twice, optional);
		// Never? is None
		assert_eq!(store.optional(TypeId::NEVER), TypeId::NONE);
	}

	#[test]
	fn literals_are_absorbed_by_their_base_type() {
		let mut store = TypeStore::new();
		let hello = store.string_literal("hello");
		assert_eq!(store.union(&[hello, TypeId::STRING]), TypeId::STRING);
		assert_eq!(
			store.union(&[TypeId::TRUE, TypeId::BOOLEAN]),
			TypeId::BOOLEAN
		);
		let five = store.int_literal(n(5));
		assert_eq!(store.union(&[five, TypeId::INTEGER]), TypeId::INTEGER);
		let half = store.float_literal(FloatBits::F64, 0.5);
		assert_eq!(store.union(&[half, TypeId::FLOAT]), TypeId::FLOAT);
		// None is absorbed by an optional member only (flattened first)
		let maybe = store.optional(TypeId::INTEGER);
		assert_eq!(store.union(&[TypeId::NONE, maybe]), maybe);
		// two different literals stay
		let world = store.string_literal("world");
		let both = store.union(&[hello, world]);
		assert!(matches!(store.kind(both), TypeKind::Union(_)));
	}

	#[test]
	fn objects_are_never_absorbed() {
		let mut store = TypeStore::new();
		let named = object(&mut store, &[("name", TypeId::STRING)]);
		let person = object(
			&mut store,
			&[("name", TypeId::STRING), ("age", TypeId::INTEGER)],
		);
		let union = store.union(&[named, person]);
		assert_eq!(store.union_members(&union).len(), 2);
		// … and the pair is indistinguishable: TY040
		assert!(store.indistinguishable_pair(union).is_some());
		let pair = store.tuple(&[TypeId::INT8]);
		let wide = store.tuple(&[TypeId::INTEGER]);
		let tuples = store.union(&[pair, wide]);
		assert_eq!(store.union_members(&tuples).len(), 2);
	}

	#[test]
	fn collections_are_absorbed() {
		let mut store = TypeStore::new();
		let natural = store.int_range(n(0), IntStorage::Default.range().hi, IntStorage::Default);
		let naturals = store.array(natural);
		let integers = store.array(TypeId::INTEGER);
		assert_eq!(store.union(&[naturals, integers]), integers);
		let strings = store.collection(CollKind::Set, TypeId::STRING, TypeId::NONE);
		let union = store.union(&[integers, strings]);
		assert_eq!(store.union_members(&union).len(), 2);
	}

	#[test]
	fn touching_ranges_merge() {
		let mut store = TypeStore::new();
		let low = store.int_range(n(0), n(5), IntStorage::Default);
		let high = store.int_range(n(6), n(9), IntStorage::Default);
		let expected = store.int_range(n(0), n(9), IntStorage::Default);
		assert_eq!(store.union(&[low, high]), expected);
		// a literal union 0 | 1 | 2 is one range type
		let literals: Vec<TypeId> = (0..3).map(|value| store.int_literal(n(value))).collect();
		let range = store.int_range(n(0), n(2), IntStorage::Default);
		assert_eq!(store.union(&literals), range);
		// gaps stay
		let far = store.int_range(n(20), n(29), IntStorage::Default);
		let gapped = store.union(&[low, far]);
		assert_eq!(store.union_members(&gapped), &[low, far]);
		assert!(store.indistinguishable_pair(gapped).is_none());
		// ranges of different storages don't merge (absorption may still apply)
		let small = store.int_range(n(10), n(12), IntStorage::I8);
		let mixed = store.union(&[low, small]);
		assert_eq!(store.union_members(&mixed).len(), 2);
		// overlapping
		let a = store.int_range(n(0), n(10), IntStorage::I16);
		let b = store.int_range(n(5), n(20), IntStorage::I16);
		let hull = store.int_range(n(0), n(20), IntStorage::I16);
		assert_eq!(store.union(&[a, b]), hull);
		// a range contained in a full type is absorbed
		assert_eq!(store.union(&[small, TypeId::INT8]), TypeId::INT8);
	}

	#[test]
	fn named_union_members_merge_into_masks() {
		let mut store = TypeStore::new();
		let shape = nominal(
			&mut store,
			"Shape",
			NominalKind::Union,
			&["Circle", "Square", "Point"],
		);
		let circle = store.member(shape, 0);
		let square = store.member(shape, 1);
		let point = store.member(shape, 2);
		let two = store.union(&[circle, square]);
		assert!(matches!(store.kind(two), TypeKind::NamedUnion(..)));
		let all = store.union(&[two, point]);
		assert_eq!(all, store.nominal_type(shape));
		assert_eq!(store.union(&[circle, circle]), circle);
		// with another member type, the masked form is one member
		let mixed = store.union(&[circle, square, TypeId::STRING]);
		let mut expected = vec![TypeId::STRING, two];
		expected.sort();
		assert_eq!(store.union_members(&mixed), expected.as_slice());
		let color = nominal(&mut store, "Color", NominalKind::Enum, &["red", "green"]);
		let red = store.intern(TypeKind::EnumMember(color, 0));
		let green = store.intern(TypeKind::EnumMember(color, 1));
		assert_eq!(store.union(&[red, green]), store.nominal_type(color));
	}

	#[test]
	fn errors_are_absorbed_by_error() {
		let mut store = TypeStore::new();
		let api = nominal(
			&mut store,
			"ApiError",
			NominalKind::Errors,
			&["Timeout", "Network"],
		);
		let timeout = store.member(api, 0);
		assert_eq!(store.union(&[timeout, TypeId::ERROR]), TypeId::ERROR);
	}

	#[test]
	fn distinguishability_matrix() {
		let mut store = TypeStore::new();
		let check = |store: &mut TypeStore, members: &[TypeId]| {
			let union = store.union(members);
			store.indistinguishable_pair(union).is_none()
		};
		// different runtime kinds
		assert!(check(&mut store, &[TypeId::INTEGER, TypeId::STRING]));
		assert!(check(&mut store, &[TypeId::INTEGER, TypeId::NONE]));
		assert!(check(
			&mut store,
			&[TypeId::BOOLEAN, TypeId::STRING, TypeId::NONE]
		));
		// number kind: TY040
		assert!(!check(&mut store, &[TypeId::INTEGER, TypeId::FLOAT]));
		assert!(!check(&mut store, &[TypeId::FLOAT32, TypeId::PERCENTAGE]));
		// Integer and Int64 are a number and a bigint
		assert_eq!(store.runtime_kind(TypeId::INT64), RuntimeKind::BigInt);
		assert!(store.distinguishable(TypeId::INTEGER, TypeId::INT64));
		// string kind
		assert!(!check(&mut store, &[TypeId::STRING, TypeId::CHARACTER]));
		let a = store.string_literal("a");
		let b = store.string_literal("b");
		assert!(check(&mut store, &[a, b]));
		// a literal and a non-literal of the same kind
		let zero = store.int_literal(n(0));
		assert!(!check(&mut store, &[zero, TypeId::FLOAT]));
		// disjoint ranges
		let low = store.int_range(n(0), n(9), IntStorage::Default);
		let high = store.int_range(n(20), n(29), IntStorage::Default);
		assert!(check(&mut store, &[low, high]));
		let overlapping = store.int_range(n(5), n(9), IntStorage::I8);
		assert!(!check(&mut store, &[high, overlapping, TypeId::FLOAT]));
		// arrays and tuples are all JavaScript arrays
		let integers = store.array(TypeId::INTEGER);
		let strings = store.array(TypeId::STRING);
		let tuple = store.tuple(&[TypeId::INTEGER]);
		assert!(!check(&mut store, &[integers, strings]));
		assert!(!check(&mut store, &[tuple, integers]));
		let set = store.collection(CollKind::Set, TypeId::INTEGER, TypeId::NONE);
		assert!(check(&mut store, &[set, integers]));
		// enumerations are numbers
		let color = nominal(&mut store, "Color", NominalKind::Enum, &["red", "green"]);
		let light = nominal(&mut store, "Light", NominalKind::Enum, &["on", "off"]);
		let color_type = store.nominal_type(color);
		let light_type = store.nominal_type(light);
		assert!(!check(&mut store, &[color_type, TypeId::INTEGER]));
		assert!(!check(&mut store, &[color_type, light_type]));
		// named unions are tagged
		let shape = nominal(
			&mut store,
			"Shape",
			NominalKind::Union,
			&["Circle", "Square"],
		);
		let other = nominal(&mut store, "Other", NominalKind::Union, &["A", "B"]);
		let shape_type = store.nominal_type(shape);
		let other_type = store.nominal_type(other);
		assert!(check(&mut store, &[shape_type, other_type]));
		// objects: a field each lacks, or a literal discriminant
		let circle = object(&mut store, &[("radius", TypeId::FLOAT)]);
		let square = object(&mut store, &[("side", TypeId::FLOAT)]);
		assert!(check(&mut store, &[circle, square]));
		let named = object(&mut store, &[("name", TypeId::STRING)]);
		let person = object(
			&mut store,
			&[("name", TypeId::STRING), ("age", TypeId::INTEGER)],
		);
		assert!(!check(&mut store, &[named, person]));
		let kind_circle = store.string_literal("circle");
		let kind_square = store.string_literal("square");
		let tagged_circle = object(
			&mut store,
			&[("kind", kind_circle), ("size", TypeId::FLOAT)],
		);
		let tagged_square = object(
			&mut store,
			&[("kind", kind_square), ("size", TypeId::FLOAT)],
		);
		assert!(check(&mut store, &[tagged_circle, tagged_square]));
		// functions are one kind
		let sig = store.simple_sig(&[], TypeId::VOID, TypeId::NEVER);
		let f = store.function(sig);
		let sig = store.simple_sig(&[TypeId::INTEGER], TypeId::VOID, TypeId::NEVER);
		let g = store.function(sig);
		assert!(!check(&mut store, &[f, g]));
		assert!(check(&mut store, &[f, TypeId::NONE]));
	}
}
