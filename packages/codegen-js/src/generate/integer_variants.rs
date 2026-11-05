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
//! fixpoint over the calls of the file. An exported function exports every
//! variant (a module is generated without its callers, whose calls bind a
//! numeric-union parameter to one member of its constraint), and a call of
//! another module's function imports the one it calls.

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
	/// Of generic parameters, those constrained by a union of
	/// JavaScript-number integers and `Float`.
	fn numeric_params_of(&self, generics: &[ParamId]) -> Vec<ParamId> {
		generics
			.iter()
			.copied()
			.filter(|&param| !self.integer_members(param).is_empty())
			.collect()
	}

	/// The integer storages of the constraint of a numeric-union parameter
	/// (empty for any other parameter).
	fn integer_members(&self, param: ParamId) -> Vec<IntStorage> {
		let constraint = self.types.param(param).constraint;
		let members: Vec<TypeId> = match self.types.kind(constraint) {
			TypeKind::Union(list) => self.types.list_items(list).to_vec(),
			_ => return Vec::new(),
		};
		let kinds: Vec<Kind> = members.iter().map(|&member| self.kind(member)).collect();
		let numbers = kinds.iter().all(|kind| match kind {
			Kind::Int(storage) => !storage.is_js_bigint(),
			Kind::Float(FloatBits::F64) => true,
			_ => false,
		});
		if !numbers {
			return Vec::new();
		}
		let mut storages: Vec<IntStorage> = kinds
			.into_iter()
			.filter_map(|kind| match kind {
				Kind::Int(storage) => Some(storage),
				_ => None,
			})
			.collect();
		storages.sort();
		storages.dedup();
		storages
	}

	/// The generic parameters of a function declaration of this file.
	fn generics_of(&self, declaration: NodeId) -> Vec<ParamId> {
		let info = self
			.analysis
			.function(declaration)
			.or_else(|| match self.view(declaration) {
				View::Function { value, .. } | View::ExportFunction { value, .. } => {
					self.analysis.function(value)
				}
				_ => None,
			});
		info.map(|info| info.generics.clone()).unwrap_or_default()
	}

	/// The variant a call (or generic application) at `site` of a function
	/// with these generic parameters calls, under the bindings of the variant
	/// being generated: `None` when no numeric-union parameter is bound to an
	/// integer type. The instantiation ends with the function's parameters,
	/// in declaration order (`check/generics.rs`, `declaration_order`).
	pub(crate) fn variant_for(&self, site: NodeId, generics: &[ParamId]) -> Option<Variant> {
		let numeric = self.numeric_params_of(generics);
		if numeric.is_empty() {
			return None;
		}
		let list = self
			.analysis
			.instantiation(site)
			.or_else(|| match self.view(site) {
				View::Call { callee, .. } => self.analysis.instantiation(self.strip_groups(callee)),
				_ => None,
			})?;
		let arguments = self.types.list_items(list).to_vec();
		let offset = arguments.len().checked_sub(generics.len())?;
		let mut variant = Vec::new();
		for (index, &param) in generics.iter().enumerate() {
			if !numeric.contains(&param) {
				continue;
			}
			let argument = arguments[offset + index];
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

	/// The variant a call of a function declaration of this file calls.
	pub(crate) fn call_variant(&self, site: NodeId, declaration: NodeId) -> Option<Variant> {
		self.variant_for(site, &self.generics_of(declaration))
	}

	/// The suffix of a variant's name: per numeric-union parameter, in
	/// order, `i` and its storage, or `f` (`$idefault_f`).
	pub(crate) fn suffix_of(&self, generics: &[ParamId], variant: &Variant) -> String {
		let parts: Vec<String> = self
			.numeric_params_of(generics)
			.iter()
			.map(
				|param| match variant.iter().find(|(bound, _)| bound == param) {
					Some((_, storage)) => format!("i{}", format!("{storage:?}").to_lowercase()),
					None => "f".to_string(),
				},
			)
			.collect();
		format!("${}", parts.join("_"))
	}

	/// The suffix of a variant of a function declaration of this file.
	pub(crate) fn variant_suffix(&self, declaration: NodeId, variant: &Variant) -> String {
		self.suffix_of(&self.generics_of(declaration), variant)
	}

	/// Every variant of an exported function: other modules may call any
	/// (their calls bind a numeric-union parameter to one member of its
	/// constraint), at most 16.
	fn exported_variants(&self, declaration: NodeId) -> Vec<Variant> {
		let generics = self.generics_of(declaration);
		let mut variants: Vec<Variant> = vec![Vec::new()];
		for param in self.numeric_params_of(&generics) {
			let mut next = Vec::new();
			for variant in &variants {
				// the parameter left a float
				next.push(variant.clone());
				for storage in self.integer_members(param) {
					let mut with = variant.clone();
					with.push((param, storage));
					with.sort();
					next.push(with);
				}
			}
			next.truncate(16);
			variants = next;
		}
		variants.retain(|variant| !variant.is_empty());
		variants
	}

	/// The declaration of another module a callee names (a name, a path
	/// through namespaces, a generic application of one).
	pub(crate) fn external_of_callee(
		&self,
		callee: NodeId,
	) -> Option<crate::generate::modules::External> {
		let callee = self.strip_groups(callee);
		match self.view(callee) {
			View::Name(_) => self.external(self.analysis.resolution(callee)),
			View::Access { members, .. } => {
				self.external(self.analysis.resolution(members.last()?))
			}
			View::Call { callee: inner, .. } => self.external_of_callee(inner),
			_ => None,
		}
	}

	/// The variant of a function of another module a call calls, as the
	/// suffix of its export name.
	pub(crate) fn external_variant_suffix(&self, site: NodeId, callee: NodeId) -> Option<String> {
		let callee = self.strip_groups(callee);
		let named = match self.view(callee) {
			View::Name(_) => callee,
			View::Access { members, .. } => members.last()?,
			View::Call { callee: inner, .. } => return self.external_variant_suffix(site, inner),
			_ => return None,
		};
		let resolution = self.analysis.resolution(named);
		let (module, decl) = self.session.external_node(resolution)?;
		let interface = self.session.interface(module)?;
		let exported = interface
			.export
			.iter()
			.chain(interface.members.values())
			.find(|exported| exported.decl == decl)?;
		let generics: Vec<ParamId> = self
			.types
			.list_items(exported.generics)
			.iter()
			.filter_map(|&generic| match self.types.kind(generic) {
				TypeKind::Param(param) => Some(param),
				_ => None,
			})
			.collect();
		let variant = self.variant_for(site, &generics)?;
		Some(self.suffix_of(&generics, &variant))
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
					if !self
						.numeric_params_of(&self.generics_of(declaration))
						.is_empty() =>
				{
					Some((call, declaration))
				}
				_ => None,
			})
			.collect();
		let mut variants: HashMap<NodeId, BTreeSet<Variant>> = HashMap::new();
		// an exported function: every variant (other modules call them)
		if let Some(program) = self.program {
			let mut exported: Vec<NodeId> = program.exports.keys().copied().collect();
			exported.sort();
			for declaration in exported {
				for variant in self.exported_variants(declaration) {
					variants.entry(declaration).or_default().insert(variant);
				}
			}
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
