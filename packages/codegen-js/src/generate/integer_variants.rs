//! Integer variants of generic functions (design/decisions.md, "Integer
//! instances of numeric-union parameters on JavaScript").
//!
//! A generic function is one JavaScript function, its arithmetic on a
//! parameter constrained by a union of integers and `Float`
//! (`<Value: Integer | Float>`) computed like `Float` (`number_kind`). When
//! a call of this file instantiates such a parameter with an integer type,
//! the function gets an integer variant, `name$i<storage>`, generated with
//! the parameter computed as that integer (overflow checks, integer
//! division), and the call calls it, as an instance would natively. Calls
//! inside a variant follow its bindings, so the variants are found by a
//! fixpoint over the calls of the file. A call of another module's
//! generic function calls its one function (a module is generated without
//! its callers).

use std::collections::{BTreeSet, HashMap};

use fa_analyzer::analysis::Callee;
use fa_analyzer::ids::{NodeId, ParamId, TypeId};
use fa_analyzer::syntax::View;
use fa_analyzer::types::{FloatBits, IntStorage, TypeKind};

use crate::generate::{Generator, Kind};

/// The integer storage of each numeric-union parameter of a variant, by
/// parameter (sorted).
pub(crate) type Variant = Vec<(ParamId, IntStorage)>;

impl Generator<'_> {
	/// The parameters of a function declaration constrained by a union of
	/// JavaScript-number integers and `Float` (its own generics).
	fn numeric_union_params(&self, declaration: NodeId) -> Vec<ParamId> {
		let Some(info) = self.analysis.function(declaration) else {
			return Vec::new();
		};
		info.generics
			.iter()
			.copied()
			.filter(|&param| {
				let constraint = self.types.param(param).constraint;
				let members: Vec<TypeId> = match self.types.kind(constraint) {
					TypeKind::Union(list) => self.types.list_items(list).to_vec(),
					_ => return false,
				};
				let kinds: Vec<Kind> = members.iter().map(|&member| self.kind(member)).collect();
				kinds.iter().any(|kind| matches!(kind, Kind::Int(_)))
					&& kinds.iter().all(|kind| match kind {
						Kind::Int(storage) => !storage.is_js_bigint(),
						Kind::Float(FloatBits::F64) => true,
						_ => false,
					})
			})
			.collect()
	}

	/// The variant a call of `declaration` at `site` (the call, or its
	/// generic application) calls, under the bindings of the variant being
	/// generated: `None` when no numeric-union parameter is bound to an
	/// integer type.
	pub(crate) fn call_variant(&self, site: NodeId, declaration: NodeId) -> Option<Variant> {
		let numeric = self.numeric_union_params(declaration);
		if numeric.is_empty() {
			return None;
		}
		let generics = self.analysis.function(declaration)?.generics.clone();
		let list = self
			.analysis
			.instantiation(site)
			.or_else(|| match self.view(site) {
				View::Call { callee, .. } => self.analysis.instantiation(self.strip_groups(callee)),
				_ => None,
			})?;
		let arguments = self.types.list_items(list).to_vec();
		let mut variant = Vec::new();
		for (index, &param) in generics.iter().enumerate() {
			if !numeric.contains(&param) {
				continue;
			}
			let Some(&argument) = arguments.get(index) else {
				continue;
			};
			let storage = match self.types.kind(argument) {
				TypeKind::Param(bound) => self.numeric_bindings.get(&bound).copied(),
				_ => match self.kind(argument) {
					Kind::Int(storage) if !storage.is_js_bigint() => Some(storage),
					_ => None,
				},
			};
			if let Some(storage) = storage {
				variant.push((param, storage));
			}
		}
		variant.sort();
		(!variant.is_empty()).then_some(variant)
	}

	/// The suffix of a variant's name: `$i` and its storages.
	pub(crate) fn variant_suffix(variant: &Variant) -> String {
		let storages: Vec<String> = variant
			.iter()
			.map(|(_, storage)| format!("{storage:?}").to_lowercase())
			.collect();
		format!("$i{}", storages.join("_"))
	}

	/// Finds the integer variants the calls of the file need (a fixpoint:
	/// the calls inside a variant follow its bindings).
	pub(crate) fn find_integer_variants(&mut self) {
		let mut calls: Vec<(NodeId, NodeId)> = self
			.analysis
			.calls
			.iter()
			.filter_map(|(&call, info)| match info.callee {
				Callee::Function(declaration)
					if !self.numeric_union_params(declaration).is_empty() =>
				{
					Some((call, declaration))
				}
				_ => None,
			})
			.collect();
		if calls.is_empty() {
			return;
		}
		calls.sort();
		// the generic functions with numeric-union parameters, by span
		let mut owners: Vec<NodeId> = calls.iter().map(|&(_, declaration)| declaration).collect();
		owners.sort();
		owners.dedup();
		let enclosing = |this: &Self, call: NodeId| -> Option<NodeId> {
			let span = this.tree.spans[call.index()];
			owners
				.iter()
				.copied()
				.filter(|owner| {
					let owner_span = this.tree.spans[owner.index()];
					owner_span.start <= span.start && span.end <= owner_span.end
				})
				.min_by_key(|owner| {
					let owner_span = this.tree.spans[owner.index()];
					owner_span.end - owner_span.start
				})
		};
		let mut variants: HashMap<NodeId, BTreeSet<Variant>> = HashMap::new();
		loop {
			let mut changed = false;
			for &(call, declaration) in &calls {
				let mut contexts: Vec<Variant> = vec![Vec::new()];
				if let Some(owner) = enclosing(self, call)
					&& let Some(found) = variants.get(&owner)
				{
					contexts.extend(found.iter().cloned());
				}
				for context in contexts {
					self.numeric_bindings = context.into_iter().collect();
					if let Some(variant) = self.call_variant(call, declaration) {
						changed |= variants.entry(declaration).or_default().insert(variant);
					}
				}
			}
			self.numeric_bindings.clear();
			if !changed {
				break;
			}
		}
		self.integer_variants = variants
			.into_iter()
			.map(|(declaration, set)| (declaration, set.into_iter().collect()))
			.collect();
	}
}
