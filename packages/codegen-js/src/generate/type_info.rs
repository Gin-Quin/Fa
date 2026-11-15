//! Type witnesses shared by every erased generic ABI, including unused binders.
//! Task ownership follows the static type, never the current value's contents.

use std::collections::{BTreeSet, HashSet};

use fa_analyzer::ids::{ParamId, TypeId};
use fa_analyzer::types::TypeKind;

use super::Generator;
use super::generics::Hidden;

impl Generator<'_> {
	pub(crate) fn type_info(&mut self, ty: TypeId) -> Option<String> {
		if let TypeKind::Param(param) = self.types.kind(ty) {
			return self.type_parameter_info(param);
		}
		let owns = self.task_ownership_condition(ty)?;
		let collections = self.collection_ownership_condition(ty)?;
		Some(format!(
			"{{ ownsTask: {owns}, hasCollection: {collections} }}"
		))
	}

	pub(crate) fn type_parameter_info(&self, param: ParamId) -> Option<String> {
		self.hidden_names
			.get(&Hidden::TypeInfo(param))
			.cloned()
			.or_else(|| {
				let expected = self.types.param(param);
				self.hidden_names.iter().find_map(|(entry, name)| {
					let Hidden::TypeInfo(actual) = *entry else {
						return None;
					};
					let actual = self.types.param(actual);
					(actual.owner == expected.owner && actual.index == expected.index)
						.then(|| name.clone())
				})
			})
	}

	pub(crate) fn collection_ownership_condition(&self, ty: TypeId) -> Option<String> {
		let mut pending = vec![ty];
		let mut seen = HashSet::new();
		let mut conditions = BTreeSet::new();
		while let Some(ty) = pending.pop() {
			if !seen.insert(ty) || self.types.is_resource(ty) {
				continue;
			}
			match self.types.kind(ty) {
				TypeKind::Collection(..) => return Some("true".into()),
				TypeKind::Param(param) if !self.types.param(param).value => {
					conditions.insert(format!(
						"{}.hasCollection",
						self.type_parameter_info(param)?
					));
				}
				TypeKind::Rec(_) => pending.extend(self.types.rec_target(ty)),
				TypeKind::Object(_) => {
					pending.extend(self.types.fields_of(ty).iter().map(|field| field.ty))
				}
				TypeKind::Tuple(list) | TypeKind::Union(list) => {
					pending.extend(self.types.list_items(list))
				}
				TypeKind::Member(nominal, index) => {
					pending.push(self.types.payload(nominal, index))
				}
				TypeKind::NamedUnion(nominal, mask) => {
					let count = self
						.types
						.sym_list(self.types.nominal(nominal).members)
						.len() as u16;
					pending.extend(
						(0..count)
							.filter(|&index| self.types.mask_has(mask, index))
							.map(|index| self.types.payload(nominal, index)),
					);
				}
				_ => {}
			}
		}
		Some(if conditions.is_empty() {
			"false".into()
		} else {
			conditions.into_iter().collect::<Vec<_>>().join(" || ")
		})
	}

	pub(crate) fn task_ownership_condition(&self, ty: TypeId) -> Option<String> {
		if self.types.owns_unique(ty) {
			return Some("true".to_string());
		}
		let mut pending = vec![ty];
		let mut seen = HashSet::new();
		let mut conditions = BTreeSet::new();
		while let Some(ty) = pending.pop() {
			if !seen.insert(ty) {
				continue;
			}
			match self.types.kind(ty) {
				TypeKind::Task(..) => return Some("true".to_string()),
				TypeKind::Param(param) if !self.types.param(param).value => {
					conditions.insert(format!("{}.ownsTask", self.type_parameter_info(param)?));
				}
				TypeKind::Rec(_) => {
					if let Some(target) = self.types.rec_target(ty) {
						pending.push(target);
					}
				}
				TypeKind::CArray(item, _) => pending.push(item),
				TypeKind::Collection(_, item, key) => pending.extend([item, key]),
				TypeKind::Tuple(list) | TypeKind::Union(list) => {
					pending.extend(self.types.list_items(list).iter().copied())
				}
				TypeKind::Object(_) => {
					pending.extend(self.types.fields_of(ty).iter().map(|field| field.ty))
				}
				TypeKind::NamedUnion(nominal, mask) => {
					let count = self
						.types
						.sym_list(self.types.nominal(nominal).members)
						.len() as u16;
					pending.extend(
						(0..count)
							.filter(|&index| self.types.mask_has(mask, index))
							.map(|index| self.types.payload(nominal, index)),
					);
				}
				TypeKind::Member(nominal, index) => {
					pending.push(self.types.payload(nominal, index))
				}
				_ => {}
			}
		}
		Some(if conditions.is_empty() {
			"false".to_string()
		} else {
			conditions.into_iter().collect::<Vec<_>>().join(" || ")
		})
	}
}
