//! Substitution of type parameters and unification (types brief §3.11):
//! instantiating a generic signature or type, and collecting the bindings a
//! call infers.

use rustc_hash::FxHashMap;

use crate::ids::{ListId, NominalId, ParamId, TypeId};
use crate::types::store::TypeStore;
use crate::types::{FieldEntry, FnSig, NominalInfo, ParamSig, TypeFlags, TypeKind};

/// A substitution: type parameters and the types that replace them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Bindings {
	/// The parameters, in declaration order.
	pub params: Vec<ParamId>,
	/// What each one is bound to (`UNSET`: not bound yet).
	pub types: Vec<TypeId>,
}

impl Bindings {
	/// Unbound parameters.
	pub fn new(params: &[ParamId]) -> Self {
		Self {
			params: params.to_vec(),
			types: vec![TypeId::UNSET; params.len()],
		}
	}

	/// The binding of a parameter, when it is one of these and bound.
	pub fn get(&self, param: ParamId) -> Option<TypeId> {
		let index = self.params.iter().position(|&p| p == param)?;
		let ty = self.types[index];
		(!ty.is_unset()).then_some(ty)
	}

	/// Whether every parameter is bound.
	pub fn complete(&self) -> bool {
		self.types.iter().all(|ty| !ty.is_unset())
	}
}

/// What unification found for a parameter.
#[derive(Clone, Debug, Default)]
pub struct Candidates {
	/// Per parameter (same order as the bindings), the types found, in order.
	pub found: Vec<Vec<TypeId>>,
}

impl TypeStore {
	/// The type with the bound parameters replaced (the unbound ones stay).
	pub fn substitute(&mut self, ty: TypeId, bindings: &Bindings) -> TypeId {
		if !self.flags(ty).contains(TypeFlags::HAS_PARAM) && !self.has_nominal_params(ty) {
			return ty;
		}
		let mut cache = FxHashMap::default();
		self.substitute_in(ty, bindings, &mut cache)
	}

	fn has_nominal_params(&self, ty: TypeId) -> bool {
		match self.kind(ty) {
			TypeKind::NamedUnion(nominal, _) | TypeKind::Member(nominal, _) => {
				let args = self.nominal(nominal).args;
				self.list_items(args)
					.iter()
					.any(|&arg| self.flags(arg).contains(TypeFlags::HAS_PARAM))
			}
			_ => false,
		}
	}

	fn substitute_in(
		&mut self,
		ty: TypeId,
		bindings: &Bindings,
		cache: &mut FxHashMap<TypeId, TypeId>,
	) -> TypeId {
		if !self.flags(ty).contains(TypeFlags::HAS_PARAM) && !self.has_nominal_params(ty) {
			return ty;
		}
		if let Some(&done) = cache.get(&ty) {
			return done;
		}
		let result = match self.kind(ty) {
			TypeKind::Param(param) => bindings.get(param).unwrap_or(ty),
			TypeKind::Union(list) => {
				let members = self.list_items(list).to_vec();
				let members: Vec<TypeId> = members
					.into_iter()
					.map(|member| self.substitute_in(member, bindings, cache))
					.collect();
				self.union(&members)
			}
			TypeKind::Tuple(list) => {
				let items = self.list_items(list).to_vec();
				let items: Vec<TypeId> = items
					.into_iter()
					.map(|item| self.substitute_in(item, bindings, cache))
					.collect();
				self.tuple(&items)
			}
			TypeKind::Object(shape) => {
				let nominal = self.shape(shape).nominal;
				let fields = self.shape_fields(shape).to_vec();
				let fields: Vec<FieldEntry> = fields
					.into_iter()
					.map(|field| FieldEntry {
						ty: self.substitute_in(field.ty, bindings, cache),
						..field
					})
					.collect();
				self.object(&fields, nominal)
			}
			TypeKind::Function(sig) => {
				let sig = self.sig(sig);
				let params = self.params(sig.params).to_vec();
				let params: Vec<ParamSig> = params
					.into_iter()
					.map(|param| ParamSig {
						ty: self.substitute_in(param.ty, bindings, cache),
						..param
					})
					.collect();
				let ret = self.substitute_in(sig.ret, bindings, cache);
				let throws = self.substitute_in(sig.throws, bindings, cache);
				let params = self.param_list(&params);
				self.function(FnSig {
					params,
					ret,
					throws,
					..sig
				})
			}
			TypeKind::Collection(kind, item, key) => {
				let item = self.substitute_in(item, bindings, cache);
				let key = self.substitute_in(key, bindings, cache);
				self.collection(kind, item, key)
			}
			TypeKind::Iterator(item, path) => {
				let item = self.substitute_in(item, bindings, cache);
				self.intern(TypeKind::Iterator(item, path))
			}
			TypeKind::Ref(target, path) => {
				let target = self.substitute_in(target, bindings, cache);
				self.intern(TypeKind::Ref(target, path))
			}
			TypeKind::Task(result, throws) => {
				let result = self.substitute_in(result, bindings, cache);
				let throws = self.substitute_in(throws, bindings, cache);
				self.intern(TypeKind::Task(result, throws))
			}
			TypeKind::NamedUnion(nominal, mask) => {
				let instance = self.substitute_nominal(nominal, bindings, cache);
				self.named_union(instance, mask)
			}
			TypeKind::Member(nominal, index) => {
				let instance = self.substitute_nominal(nominal, bindings, cache);
				self.member(instance, index)
			}
			// a recursive reference of a generic declaration: the reference
			// of the instance (its target is the declaration's, substituted)
			TypeKind::Rec(rec) => {
				let info = self.rec_info(rec);
				let args = self.list_items(info.args).to_vec();
				let args: Vec<TypeId> = args
					.into_iter()
					.map(|arg| self.substitute_in(arg, bindings, cache))
					.collect();
				let name = self.syms.resolve(info.name).to_string();
				self.recursive_reference(info.key, &name, &args, info.object)
			}
			_ => ty,
		};
		cache.insert(ty, result);
		result
	}

	/// The instance of a generic named union with substituted arguments,
	/// with the data of its members substituted too.
	fn substitute_nominal(
		&mut self,
		nominal: NominalId,
		bindings: &Bindings,
		cache: &mut FxHashMap<TypeId, TypeId>,
	) -> NominalId {
		let info = self.nominal(nominal);
		let args = self.list_items(info.args).to_vec();
		let args: Vec<TypeId> = args
			.into_iter()
			.map(|arg| self.substitute_in(arg, bindings, cache))
			.collect();
		let list = self.list(&args);
		self.nominal_instance(info, list)
	}

	/// The instance of a generic nominal type (a named union) for `args`:
	/// the data of its members are those of the declaration, substituted.
	pub fn nominal_instance(&mut self, info: NominalInfo, args: ListId) -> NominalId {
		let instance = self.intern_nominal(NominalInfo { args, ..info });
		if self.has_payloads(instance) {
			return instance;
		}
		let Some((generic, params)) = self.generic_nominal(info.key) else {
			return instance;
		};
		let count = self.sym_list(info.members).len() as u16;
		let bindings = Bindings {
			params,
			types: self.list_items(args).to_vec(),
		};
		// the payloads may name the instance itself (recursive unions)
		self.set_payloads(instance, vec![TypeId::UNSET; count as usize]);
		let payloads: Vec<TypeId> = (0..count)
			.map(|index| {
				let payload = self.payload(generic, index);
				if payload.is_unset() {
					payload
				} else {
					self.substitute(payload, &bindings)
				}
			})
			.collect();
		self.set_payloads(instance, payloads);
		instance
	}

	/// Unifies a declared type that may contain the parameters of `bindings`
	/// with an actual type, collecting the candidate of each parameter.
	pub fn unify(
		&mut self,
		declared: TypeId,
		actual: TypeId,
		bindings: &Bindings,
		out: &mut Candidates,
	) {
		if out.found.len() < bindings.params.len() {
			out.found.resize(bindings.params.len(), Vec::new());
		}
		self.unify_in(declared, actual, bindings, out, 0);
	}

	fn unify_in(
		&mut self,
		declared: TypeId,
		actual: TypeId,
		bindings: &Bindings,
		out: &mut Candidates,
		depth: u32,
	) {
		if depth > 32
			|| actual.is_unset()
			|| !self.flags(declared).contains(TypeFlags::HAS_PARAM)
				&& !self.has_nominal_params(declared)
		{
			return;
		}
		match (self.kind(declared), self.kind(actual)) {
			(TypeKind::Param(param), _) => {
				if let Some(index) = bindings.params.iter().position(|&p| p == param)
					&& !matches!(self.kind(actual), TypeKind::Never)
				{
					out.found[index].push(actual);
				}
			}
			(
				TypeKind::Collection(kind, item, key),
				TypeKind::Collection(actual_kind, actual_item, actual_key),
			) if kind == actual_kind => {
				self.unify_in(item, actual_item, bindings, out, depth + 1);
				self.unify_in(key, actual_key, bindings, out, depth + 1);
			}
			(TypeKind::Tuple(a), TypeKind::Tuple(b)) => {
				let (a, b) = (self.list_items(a).to_vec(), self.list_items(b).to_vec());
				for (declared, actual) in a.into_iter().zip(b) {
					self.unify_in(declared, actual, bindings, out, depth + 1);
				}
			}
			(TypeKind::Object(a), TypeKind::Object(b)) => {
				let declared_fields = self.shape_fields(a).to_vec();
				let actual_fields = self.shape_fields(b).to_vec();
				for field in declared_fields {
					if let Some(other) = actual_fields.iter().find(|other| other.name == field.name)
					{
						self.unify_in(field.ty, other.ty, bindings, out, depth + 1);
					}
				}
			}
			(TypeKind::Object(_), TypeKind::Member(nominal, index)) => {
				let payload = self.payload(nominal, index);
				self.unify_in(declared, payload, bindings, out, depth + 1);
			}
			(TypeKind::Function(a), TypeKind::Function(b)) => {
				let (a, b) = (self.sig(a), self.sig(b));
				let a_params = self.params(a.params).to_vec();
				let b_params = self.params(b.params).to_vec();
				for (declared, actual) in a_params.iter().zip(&b_params) {
					self.unify_in(declared.ty, actual.ty, bindings, out, depth + 1);
				}
				self.unify_in(a.ret, b.ret, bindings, out, depth + 1);
			}
			(
				TypeKind::NamedUnion(a, _) | TypeKind::Member(a, _),
				TypeKind::NamedUnion(b, _) | TypeKind::Member(b, _),
			) => {
				let (a, b) = (self.nominal(a), self.nominal(b));
				if a.key == b.key {
					let (a, b) = (
						self.list_items(a.args).to_vec(),
						self.list_items(b.args).to_vec(),
					);
					for (declared, actual) in a.into_iter().zip(b) {
						self.unify_in(declared, actual, bindings, out, depth + 1);
					}
				}
			}
			(TypeKind::Union(list), _) => {
				// `Item?` against `Integer?` or `Integer`: the members with parameters
				// take what the other members don't cover
				let members = self.list_items(list).to_vec();
				let actual_members: Vec<TypeId> = self.union_members(&actual).to_vec();
				let fixed: Vec<TypeId> = members
					.iter()
					.copied()
					.filter(|&member| !self.flags(member).contains(TypeFlags::HAS_PARAM))
					.collect();
				let rest: Vec<TypeId> = actual_members
					.into_iter()
					.filter(|&member| !fixed.iter().any(|&fixed| self.satisfies(member, fixed)))
					.collect();
				let open: Vec<TypeId> = members
					.into_iter()
					.filter(|&member| self.flags(member).contains(TypeFlags::HAS_PARAM))
					.collect();
				if let ([open], false) = (open.as_slice(), rest.is_empty()) {
					let rest = self.union(&rest);
					self.unify_in(*open, rest, bindings, out, depth + 1);
				}
			}
			_ => {}
		}
	}
}
