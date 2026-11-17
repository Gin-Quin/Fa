//! Implicit conversions preserve the inside of every structure. Only
//! literal/range widening within one primitive leaves item storage unchanged.

use rustc_hash::{FxHashMap, FxHashSet};

use super::{FieldFlags, TypeId, TypeKind, TypeStore};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Primitive {
	Integer(super::super::IntStorage),
	Float(super::super::FloatBits),
	String,
	Boolean,
	BigInt,
}

type ItemParts = Vec<Vec<(TypeId, TypeId)>>;

enum ItemCheck {
	Compare(TypeId, TypeId),
	Finish(TypeId, TypeId, ItemParts),
}

impl TypeStore {
	/// Whether item types differ beyond the one language-level storage
	/// exception. This is independent of a backend's selected layout.
	pub fn same_structure_item(&self, actual: TypeId, target: TypeId) -> bool {
		let mut results = FxHashMap::default();
		let mut pending = vec![ItemCheck::Compare(actual, target)];
		while let Some(check) = pending.pop() {
			match check {
				ItemCheck::Compare(actual, target) => {
					if results.contains_key(&(actual, target)) {
						continue;
					}
					match self.structure_item_parts(actual, target) {
						Err(result) => {
							results.insert((actual, target), result);
						}
						Ok(parts) => {
							pending.push(ItemCheck::Finish(actual, target, parts.clone()));
							pending.extend(
								parts
									.into_iter()
									.flatten()
									.map(|(a, b)| ItemCheck::Compare(a, b)),
							);
						}
					}
				}
				ItemCheck::Finish(actual, target, parts) => {
					let same = parts
						.iter()
						.all(|choices| choices.iter().any(|pair| results.get(pair) == Some(&true)));
					results.insert((actual, target), same);
				}
			}
		}
		results[&(actual, target)]
	}

	/// Every group must match; a union member can match any alternative.
	/// Recursive identities are opaque here, so the dependency graph is a
	/// DAG and an explicit postorder stack suffices even for deeply nested
	/// optional tuples. No product types are constructed while comparing alternatives.
	fn structure_item_parts(&self, actual: TypeId, target: TypeId) -> Result<ItemParts, bool> {
		if actual == target
			|| self
				.primitive_item(actual)
				.is_some_and(|kind| Some(kind) == self.primitive_item(target))
		{
			return Err(true);
		}
		let pairs = match (self.kind(actual), self.kind(target)) {
			(TypeKind::Union(left), TypeKind::Union(right)) => {
				let (left, right) = (self.list_items(left), self.list_items(right));
				return Ok(left
					.iter()
					.map(|&a| right.iter().map(|&b| (a, b)).collect())
					.chain(
						right
							.iter()
							.map(|&b| left.iter().map(|&a| (a, b)).collect()),
					)
					.collect());
			}
			(TypeKind::Union(items), _) => self
				.list_items(items)
				.iter()
				.map(|&item| (item, target))
				.collect(),
			(_, TypeKind::Union(items)) => self
				.list_items(items)
				.iter()
				.map(|&item| (actual, item))
				.collect(),
			(TypeKind::Tuple(left), TypeKind::Tuple(right)) => {
				let (left, right) = (self.list_items(left), self.list_items(right));
				if left.len() != right.len() {
					return Err(false);
				}
				left.iter().copied().zip(right.iter().copied()).collect()
			}
			(TypeKind::Object(left), TypeKind::Object(right)) => {
				if self.shape(left).nominal != self.shape(right).nominal {
					return Err(false);
				}
				let (left, right) = (self.shape_fields(left), self.shape_fields(right));
				if left.len() != right.len() {
					return Err(false);
				}
				let mut pairs = Vec::with_capacity(left.len());
				for field in left {
					let Some(other) = right.iter().find(|other| other.name == field.name) else {
						return Err(false);
					};
					if field.flags != other.flags {
						return Err(false);
					}
					pairs.push((field.ty, other.ty));
				}
				pairs
			}
			(
				TypeKind::Collection(kind, item, key),
				TypeKind::Collection(other, other_item, other_key),
			) if kind == other => vec![(item, other_item), (key, other_key)],
			(TypeKind::CArray(item, size), TypeKind::CArray(other, other_size))
				if size == other_size =>
			{
				vec![(item, other)]
			}
			_ => return Err(false),
		};
		Ok(pairs.into_iter().map(|pair| vec![pair]).collect())
	}

	fn primitive_item(&self, ty: TypeId) -> Option<Primitive> {
		let mut pending = vec![ty];
		let mut primitive = None;
		while let Some(ty) = pending.pop() {
			let next = match self.kind(ty) {
				TypeKind::Union(list) => {
					pending.extend(self.list_items(list));
					continue;
				}
				TypeKind::Int(range) => Primitive::Integer(self.int(range).storage),
				TypeKind::Float(bits) | TypeKind::FloatLit(bits, _) => Primitive::Float(bits),
				TypeKind::String | TypeKind::StringLit(_) => Primitive::String,
				TypeKind::Boolean | TypeKind::BoolLit(_) => Primitive::Boolean,
				TypeKind::BigInt | TypeKind::BigIntLit(_) => Primitive::BigInt,
				_ => return None,
			};
			if primitive.is_some_and(|primitive| primitive != next) {
				return None;
			}
			primitive = Some(next);
		}
		primitive
	}

	/// The first changed item or recursive part, for a MEM004 diagnostic.
	/// The value's own fields may project or convert bounded data; crossing
	/// a collection/recursive boundary requires an explicit converting copy.
	pub fn structure_conversion(
		&mut self,
		actual: TypeId,
		target: TypeId,
	) -> Option<(TypeId, TypeId)> {
		let mut pending = vec![(actual, target)];
		let mut seen = FxHashSet::default();
		while let Some((actual, target)) = pending.pop() {
			if actual.is_unset()
				|| target.is_unset()
				|| actual == target
				|| !seen.insert((actual, target))
			{
				continue;
			}
			let (source, destination) = (self.unfold(actual), self.unfold(target));
			if source == destination || self.union_members(&target).contains(&actual) {
				continue;
			}
			match (self.kind(actual), self.kind(target)) {
				(TypeKind::Param(_) | TypeKind::ImplicitParam(..) | TypeKind::Unknown, _)
				| (_, TypeKind::Param(_) | TypeKind::ImplicitParam(..) | TypeKind::Unknown) => continue,
				(TypeKind::Union(source), _) => pending.extend(
					self.list_items(source)
						.iter()
						.map(|&actual| (actual, target)),
				),
				(_, TypeKind::Union(destination)) => {
					let targets = self.list_items(destination).to_vec();
					if let Some(target) = targets.into_iter().find(|&target| {
						self.assignable(actual, target, super::super::Mode::Copy)
							.is_ok()
					}) {
						pending.push((actual, target));
					}
				}
				(TypeKind::Rec(_), _) | (_, TypeKind::Rec(_)) => {
					if !self
						.flags(actual)
						.contains(super::super::TypeFlags::HAS_PARAM)
						&& !self
							.flags(target)
							.contains(super::super::TypeFlags::HAS_PARAM)
					{
						return Some((actual, target));
					}
				}
				(
					TypeKind::Collection(kind, item, key),
					TypeKind::Collection(other, target_item, target_key),
				) if kind == other => {
					for (actual, target) in [(item, target_item), (key, target_key)] {
						if self
							.flags(actual)
							.contains(super::super::TypeFlags::HAS_PARAM)
							|| self
								.flags(target)
								.contains(super::super::TypeFlags::HAS_PARAM)
						{
							continue;
						}
						if !self.same_structure_item(actual, target) {
							return Some((actual, target));
						}
					}
				}
				(TypeKind::Object(source), TypeKind::Object(destination)) => {
					for field in self.shape_fields(destination) {
						if field
							.flags
							.intersects(FieldFlags::METHOD | FieldFlags::RELATION)
						{
							continue;
						}
						if let Some(actual) = self
							.shape_fields(source)
							.iter()
							.find(|actual| actual.name == field.name)
						{
							pending.push((actual.ty, field.ty));
						}
					}
				}
				(TypeKind::Tuple(source), TypeKind::Tuple(destination)) => pending.extend(
					self.list_items(source)
						.iter()
						.copied()
						.zip(self.list_items(destination).iter().copied()),
				),
				(TypeKind::CArray(source, size), TypeKind::CArray(destination, other))
					if size == other =>
				{
					pending.push((source, destination))
				}
				(TypeKind::NamedUnion(source, mask), TypeKind::NamedUnion(destination, _))
					if self.nominal(source).key == self.nominal(destination).key =>
				{
					for index in 0..self.sym_list(self.nominal(source).members).len() as u16 {
						if self.mask_has(mask, index) {
							pending.push((
								self.payload(source, index),
								self.payload(destination, index),
							));
						}
					}
				}
				(
					TypeKind::Member(source, index),
					TypeKind::Member(destination, _) | TypeKind::NamedUnion(destination, _),
				) if self.nominal(source).key == self.nominal(destination).key => pending.push((
					self.payload(source, index),
					self.payload(destination, index),
				)),
				(TypeKind::Member(source, index), TypeKind::Object(_)) => {
					pending.push((self.payload(source, index), target))
				}
				_ => {}
			}
		}
		None
	}
}
