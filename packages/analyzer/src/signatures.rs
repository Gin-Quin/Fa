//! The names visible at a function expression, separate from structural
//! function types. References keep recursive named types finite and portable.

use std::sync::Arc;

use crate::ids::{DeclKey, Sym};
use crate::types::TypeStore;
use crate::types::merge::Merger;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SignatureNames {
	Function {
		parameters: Vec<Option<Sym>>,
		/// Defaults belong to declarations, never to a structural function type.
		defaults: Vec<bool>,
		/// Present for a residual `with` signature, including inherited bindings.
		bound: Option<Vec<Sym>>,
		returns: Option<Arc<SignatureNames>>,
	},
	Object(Vec<(Sym, Arc<SignatureNames>)>),
	Tuple(Vec<Option<Arc<SignatureNames>>>),
	Collection(Arc<SignatureNames>),
	Reference(DeclKey),
}

impl SignatureNames {
	pub fn merged(&self, merger: &mut Merger<'_>) -> Arc<Self> {
		Arc::new(match self {
			Self::Function {
				parameters,
				defaults,
				bound,
				returns,
			} => Self::Function {
				parameters: parameters
					.iter()
					.map(|name| name.map(|name| merger.sym(name)))
					.collect(),
				defaults: defaults.clone(),
				bound: bound
					.as_ref()
					.map(|names| names.iter().map(|&name| merger.sym(name)).collect()),
				returns: returns.as_ref().map(|names| names.merged(merger)),
			},
			Self::Object(fields) => Self::Object(
				fields
					.iter()
					.map(|(name, names)| (merger.sym(*name), names.merged(merger)))
					.collect(),
			),
			Self::Tuple(items) => Self::Tuple(
				items
					.iter()
					.map(|names| names.as_ref().map(|names| names.merged(merger)))
					.collect(),
			),
			Self::Collection(item) => Self::Collection(item.merged(merger)),
			Self::Reference(key) => Self::Reference(merger.decl_key(*key)),
		})
	}

	pub fn stable_hash(&self, types: &TypeStore) -> u128 {
		fn fold(hash: u128, part: u128) -> u128 {
			hash.wrapping_mul(0x100000001b3).wrapping_add(part)
		}
		let name = |name| {
			types
				.syms
				.resolve(name)
				.bytes()
				.fold(1u128, |hash, byte| fold(hash, u128::from(byte)))
		};
		match self {
			Self::Function {
				parameters,
				defaults,
				bound,
				returns,
			} => {
				let hash = parameters
					.iter()
					.fold(1, |hash, item| fold(hash, item.map_or(0, name)));
				let hash = defaults
					.iter()
					.fold(hash, |hash, &default| fold(hash, u128::from(default)));
				let hash = fold(
					hash,
					bound.as_ref().map_or(0, |names| {
						names.iter().fold(1, |hash, &item| fold(hash, name(item)))
					}),
				);
				fold(
					hash,
					returns.as_ref().map_or(0, |names| names.stable_hash(types)),
				)
			}
			Self::Object(fields) => {
				let mut fields: Vec<_> = fields
					.iter()
					.map(|(field, names)| (name(*field), names.stable_hash(types)))
					.collect();
				fields.sort_unstable();
				fields
					.into_iter()
					.fold(2, |hash, (name, names)| fold(fold(hash, name), names))
			}
			Self::Tuple(items) => items.iter().fold(3, |hash, names| {
				fold(
					hash,
					names.as_ref().map_or(0, |names| names.stable_hash(types)),
				)
			}),
			Self::Collection(item) => fold(4, item.stable_hash(types)),
			Self::Reference(key) => fold(5, types.decl_hash(*key)),
		}
	}

	pub(crate) fn references(&self, out: &mut Vec<DeclKey>) {
		match self {
			Self::Function {
				returns: Some(names),
				..
			}
			| Self::Collection(names) => names.references(out),
			Self::Object(fields) => {
				for (_, names) in fields {
					names.references(out);
				}
			}
			Self::Tuple(items) => {
				for names in items.iter().flatten() {
					names.references(out);
				}
			}
			Self::Reference(key) => out.push(*key),
			_ => {}
		}
	}
}
