//! A store owns only the fields retained by structural conversion. The
//! canonical ownership query still defines every terminal category.

use rustc_hash::FxHashSet;

use super::{FieldFlags, Ownership, TypeId, TypeKind, TypeStore};

pub struct StoreProjection {
	pub ownership: Ownership,
	/// A conversion must never erase this root lifetime owner, even when
	/// it occurs inside another retained field or a collection item.
	pub erased_owner: Option<TypeId>,
	/// Retaining owned data while discarding fields requires field-transfer
	/// support, including destruction of the omitted parts.
	pub needs_field_moves: bool,
}

impl TypeStore {
	pub fn store_projection(&mut self, actual: TypeId, target: TypeId) -> StoreProjection {
		if actual.is_unset() || target.is_unset() {
			return StoreProjection {
				ownership: Ownership::Unresolved,
				erased_owner: None,
				needs_field_moves: false,
			};
		}
		let mut pending = vec![(actual, target, false)];
		let mut seen = FxHashSet::default();
		let mut active = FxHashSet::default();
		let mut unique = false;
		let mut structure = false;
		let mut dependent = false;
		let mut unresolved = false;
		let mut erased_owner = None;
		let mut omitted_fields = false;
		while let Some((actual, target, leaving)) = pending.pop() {
			let pair = (actual, target);
			if leaving {
				active.remove(&pair);
				continue;
			}
			if !seen.insert(pair) {
				structure |= active.contains(&pair);
				continue;
			}
			let leaf = self.ownership_leaf(actual);
			if self.has_lifetime_boundary(actual) && !self.preserves_lifetime_owner(actual, target)
			{
				erased_owner.get_or_insert(actual);
			}
			if actual != target
				&& !matches!(
					leaf,
					Some(
						Ownership::Ordinary
							| Ownership::SharedHandle
							| Ownership::Dependent
							| Ownership::Unresolved
					)
				) {
				active.insert(pair);
				pending.push((actual, target, true));
				let mut children = Vec::new();
				match (self.kind(actual), self.kind(target)) {
					(TypeKind::Rec(_), _) => {
						if let Some(actual) = self.rec_target(actual) {
							children.push((actual, target));
						}
					}
					(_, TypeKind::Rec(_)) => {
						if let Some(target) = self.rec_target(target) {
							children.push((actual, target));
						}
					}
					(TypeKind::Union(list), _) => children
						.extend(self.list_items(list).iter().map(|&actual| (actual, target))),
					(_, TypeKind::Union(list)) => {
						let targets = self.list_items(list).to_vec();
						let selected = if targets.contains(&actual) {
							Some(actual)
						} else {
							targets.into_iter().find(|&target| {
								self.assignable(actual, target, super::super::Mode::Copy)
									.is_ok()
							})
						};
						if let Some(target) = selected {
							children.push((actual, target));
						}
					}
					(TypeKind::Object(source), TypeKind::Object(destination))
						if !self.is_resource(actual) =>
					{
						omitted_fields |= self.shape_fields(source).iter().any(|source| {
							!source
								.flags
								.intersects(FieldFlags::RELATION | FieldFlags::METHOD)
								&& !self
									.shape_fields(destination)
									.iter()
									.any(|target| target.name == source.name)
						});
						for field in self.shape_fields(destination) {
							if field
								.flags
								.intersects(FieldFlags::RELATION | FieldFlags::METHOD)
							{
								continue;
							}
							if let Some(source) = self
								.shape_fields(source)
								.iter()
								.find(|source| source.name == field.name)
							{
								children.push((source.ty, field.ty));
							}
						}
						// An empty structural projection owns no data.
						if children.is_empty() {
							continue;
						}
					}
					(TypeKind::Tuple(source), TypeKind::Tuple(destination)) => {
						children.extend(
							self.list_items(source)
								.iter()
								.copied()
								.zip(self.list_items(destination).iter().copied()),
						);
					}
					(
						TypeKind::Collection(_, source, key),
						TypeKind::Collection(_, destination, target_key),
					) => {
						structure = true;
						children.extend([(source, destination), (key, target_key)]);
					}
					(TypeKind::CArray(source, _), TypeKind::CArray(destination, _)) => {
						children.push((source, destination))
					}
					_ => {}
				}
				if !children.is_empty() {
					pending.extend(
						children
							.into_iter()
							.map(|(actual, target)| (actual, target, false)),
					);
					continue;
				}
			}
			match self.ownership(actual) {
				Ownership::Unique => unique = true,
				Ownership::Structure => structure = true,
				Ownership::Dependent => dependent = true,
				Ownership::Unresolved => unresolved = true,
				Ownership::Ordinary | Ownership::SharedHandle => {}
			}
		}
		let ownership = if unique {
			Ownership::Unique
		} else if unresolved {
			Ownership::Unresolved
		} else if dependent {
			Ownership::Dependent
		} else if structure {
			Ownership::Structure
		} else {
			Ownership::Ordinary
		};
		StoreProjection {
			ownership,
			erased_owner,
			needs_field_moves: omitted_fields
				&& matches!(ownership, Ownership::Structure | Ownership::Unique),
		}
	}
}
