//! The merge of a finished unit (design/parallel-pipeline.md §1.3): the
//! structures of a view that its interface reaches are deep-copied into the
//! global store, children first, deduplicated by the global indexes; the
//! local ids are remapped to global ones. The rest of the view dies with it.
//!
//! One merge runs at a time (the global store's merge lock); the views of
//! other units keep reading the global store meanwhile. A global entry never
//! holds a local id (checked on every insert in debug builds).

use std::sync::MutexGuard;

use rustc_hash::FxHashMap;

use crate::ids::{
	ConstId, DeclKey, EffectSetId, IntId, ListId, MaskId, ModuleId, NominalId, ParamId,
	ParamListId, PathId, PathListId, RecId, ReqListId, ShapeId, SigId, StepListId, Sym, SymListId,
	TypeId,
};
use crate::types::global::is_local;
use crate::types::store::{TypeEntry, TypeStore};
use crate::types::{
	ConstValue, DeclKeyData, EffectSet, FieldEntry, FnSig, NominalInfo, ParamInfo, ParamSig, Path,
	PathRoot, PathStep, RecInfo, Requirement, Shape, TypeKind,
};

/// Copies the structures of a view into the global store (see the module
/// documentation). Created by [`TypeStore::merger`]; its methods return the
/// global id of a local one (a global id is returned as is).
pub struct Merger<'v> {
	view: &'v TypeStore,
	_lock: MutexGuard<'v, ()>,
	types: FxHashMap<u32, u32>,
	syms: FxHashMap<u32, u32>,
	lists: FxHashMap<u32, u32>,
	sym_lists: FxHashMap<u32, u32>,
	shapes: FxHashMap<u32, u32>,
	sigs: FxHashMap<u32, u32>,
	paths: FxHashMap<u32, u32>,
	effects: FxHashMap<u32, u32>,
	consts: FxHashMap<u32, u32>,
	nominals: FxHashMap<u32, u32>,
	decl_keys: FxHashMap<u32, u32>,
	modules: FxHashMap<u32, u32>,
	params: FxHashMap<u32, u32>,
	recs: FxHashMap<u32, u32>,
	/// The module a declared name of an object type is recorded for.
	module: Option<ModuleId>,
}

impl TypeStore {
	/// A merger of this view into the global store; `module` is the module
	/// the names of object types declared in this view are recorded for.
	/// Waits for the merge running, if any.
	pub fn merger(&self, module: Option<ModuleId>) -> Merger<'_> {
		let lock = self
			.global
			.merging
			.lock()
			.unwrap_or_else(|poisoned| poisoned.into_inner());
		Merger {
			view: self,
			_lock: lock,
			types: FxHashMap::default(),
			syms: FxHashMap::default(),
			lists: FxHashMap::default(),
			sym_lists: FxHashMap::default(),
			shapes: FxHashMap::default(),
			sigs: FxHashMap::default(),
			paths: FxHashMap::default(),
			effects: FxHashMap::default(),
			consts: FxHashMap::default(),
			nominals: FxHashMap::default(),
			decl_keys: FxHashMap::default(),
			modules: FxHashMap::default(),
			params: FxHashMap::default(),
			recs: FxHashMap::default(),
			module,
		}
	}
}

/// A debug check: what goes into the global store holds no local id.
#[inline]
fn global(id: u32) -> u32 {
	debug_assert!(!is_local(id), "a local id reached the global store");
	id
}

impl Merger<'_> {
	/// The global id of a type.
	pub fn ty(&mut self, ty: TypeId) -> TypeId {
		if !is_local(ty.0) {
			return ty;
		}
		if let Some(&found) = self.types.get(&ty.0) {
			return TypeId(found);
		}
		let entry = *self.view.types.resolve(ty.0);
		let kind = match entry.kind {
			TypeKind::Rec(rec) => {
				// a cycle goes through its reference: memoized before its target
				let (global, target) = self.rec_shell(rec);
				let kind = TypeKind::Rec(global);
				let id = self.insert_type(kind, entry);
				self.types.insert(ty.0, id.0);
				if let Some((key, args)) = target {
					self.finish_rec(global, rec, key, args, id);
				}
				return id;
			}
			kind => self.kind(kind),
		};
		let id = self.insert_type(kind, entry);
		self.types.insert(ty.0, id.0);
		self.after_type(ty, id);
		id
	}

	fn insert_type(&mut self, kind: TypeKind, entry: TypeEntry) -> TypeId {
		TypeId(self.view.global.types.insert(TypeEntry {
			kind,
			flags: entry.flags,
			stable: entry.stable,
		}))
	}

	/// What a type carries besides its kind: the declared name of an object
	/// type, the data of the members of a nominal.
	fn after_type(&mut self, local: TypeId, global: TypeId) {
		if let Some(&name) = self.view.names.get(&local)
			&& let Some(module) = self.module
		{
			let name = self.sym(name);
			let module = self.module(module);
			let mut late = self.late();
			let names = late.names.entry(global).or_default();
			if !names.iter().any(|(other, _, _)| *other == module) {
				names.push((module, 0, name));
			}
		}
	}

	fn late(&self) -> std::sync::RwLockWriteGuard<'_, crate::types::store::GlobalLate> {
		self.view
			.global
			.late
			.write()
			.unwrap_or_else(|poisoned| poisoned.into_inner())
	}

	fn kind(&mut self, kind: TypeKind) -> TypeKind {
		use TypeKind as K;
		match kind {
			K::Int(id) => K::Int(self.int(id)),
			K::FloatLit(bits, value) => K::FloatLit(bits, self.constant(value)),
			K::BigIntLit(value) => K::BigIntLit(self.constant(value)),
			K::StringLit(sym) => K::StringLit(self.sym(sym)),
			K::Object(shape) => K::Object(self.shape(shape)),
			K::Tuple(list) => K::Tuple(self.list(list)),
			K::Union(list) => K::Union(self.list(list)),
			K::NamedUnion(nominal, mask) => K::NamedUnion(self.nominal(nominal), self.mask(mask)),
			K::Enum(nominal, mask) => K::Enum(self.nominal(nominal), self.mask(mask)),
			K::Member(nominal, index) => K::Member(self.nominal(nominal), index),
			K::EnumMember(nominal, index) => K::EnumMember(self.nominal(nominal), index),
			K::CStruct(nominal) => K::CStruct(self.nominal(nominal)),
			K::Pointer(target) => K::Pointer(self.ty(target)),
			K::CArray(item, count) => K::CArray(self.ty(item), count),
			K::Function(sig) => K::Function(self.sig(sig)),
			K::Collection(collection, item, key) => {
				K::Collection(collection, self.ty(item), self.ty(key))
			}
			K::Iterator(item, path) => K::Iterator(self.ty(item), self.path(path)),
			K::Ref(target, path) => K::Ref(self.ty(target), self.path(path)),
			K::Task(result, throws) => K::Task(self.ty(result), self.ty(throws)),
			K::Param(param) => K::Param(self.param(param)),
			K::ImplicitParam(owner, index) => K::ImplicitParam(self.decl_key(owner), index),
			K::Const(value) => K::Const(self.constant(value)),
			K::Rec(_) => unreachable!("merged by `ty`"),
			other => other,
		}
	}

	/// The global symbol of a symbol.
	pub fn sym(&mut self, sym: Sym) -> Sym {
		if !is_local(sym.0) {
			return sym;
		}
		if let Some(&found) = self.syms.get(&sym.0) {
			return Sym(found);
		}
		let text = self.view.syms.resolve(sym);
		let id = self.view.global.syms.insert(text.as_bytes());
		self.syms.insert(sym.0, id);
		Sym(id)
	}

	fn int(&mut self, id: IntId) -> IntId {
		if !is_local(id.0) {
			return id;
		}
		IntId(self.view.global.ints.insert(*self.view.int(id)))
	}

	/// The global id of a list of types.
	pub fn list(&mut self, list: ListId) -> ListId {
		if !is_local(list.0) {
			return list;
		}
		if let Some(&found) = self.lists.get(&list.0) {
			return ListId(found);
		}
		let items: Vec<TypeId> = self.view.list_items(list).to_vec();
		let items: Vec<TypeId> = items.into_iter().map(|item| self.ty(item)).collect();
		debug_assert!(items.iter().all(|item| !is_local(item.0)));
		let id = self.view.global.lists.insert(&items);
		self.lists.insert(list.0, id);
		ListId(id)
	}

	fn sym_list(&mut self, list: SymListId) -> SymListId {
		if !is_local(list.0) {
			return list;
		}
		if let Some(&found) = self.sym_lists.get(&list.0) {
			return SymListId(found);
		}
		let items: Vec<Sym> = self.view.sym_list(list).to_vec();
		let items: Vec<Sym> = items.into_iter().map(|item| self.sym(item)).collect();
		let id = self.view.global.sym_lists.insert(&items);
		self.sym_lists.insert(list.0, id);
		SymListId(id)
	}

	fn shape(&mut self, shape: ShapeId) -> ShapeId {
		if !is_local(shape.0) {
			return shape;
		}
		if let Some(&found) = self.shapes.get(&shape.0) {
			return ShapeId(found);
		}
		let data = self.view.shape(shape);
		let fields: Vec<FieldEntry> = self.view.shape_fields(shape).to_vec();
		let fields: Vec<FieldEntry> = fields
			.into_iter()
			.map(|field| FieldEntry {
				name: self.sym(field.name),
				ty: self.ty(field.ty),
				flags: field.flags,
			})
			.collect();
		let fields = global(self.view.global.field_lists.insert(&fields));
		let nominal = data.nominal.map(|nominal| self.nominal(nominal));
		let id = self.view.global.shapes.insert(Shape { fields, nominal });
		self.shapes.insert(shape.0, id);
		ShapeId(id)
	}

	fn mask(&mut self, mask: MaskId) -> MaskId {
		if !is_local(mask.0) {
			return mask;
		}
		MaskId(self.view.global.masks.insert(self.view.mask_bits(mask)))
	}

	fn constant(&mut self, value: ConstId) -> ConstId {
		if !is_local(value.0) {
			return value;
		}
		if let Some(&found) = self.consts.get(&value.0) {
			return ConstId(found);
		}
		let data = self.view.consts.resolve(value.0).map(|data| match data {
			ConstValue::String(sym) => ConstValue::String(self.sym(sym)),
			ConstValue::Character(sym) => ConstValue::Character(self.sym(sym)),
			ConstValue::BigInt(sym) => ConstValue::BigInt(self.sym(sym)),
			ConstValue::EnumMember(nominal, index) => {
				ConstValue::EnumMember(self.nominal(nominal), index)
			}
			other => other,
		});
		let id = self.view.global.consts.insert(data);
		self.consts.insert(value.0, id);
		ConstId(id)
	}

	/// The global id of a nominal type (its member data with it).
	pub fn nominal(&mut self, nominal: NominalId) -> NominalId {
		if !is_local(nominal.0) {
			return nominal;
		}
		if let Some(&found) = self.nominals.get(&nominal.0) {
			return NominalId(found);
		}
		let info = self.view.nominal(nominal);
		let merged = NominalInfo {
			key: self.decl_key(info.key),
			args: self.list(info.args),
			kind: info.kind,
			members: self.sym_list(info.members),
		};
		let id = NominalId(self.view.global.nominals.insert(merged));
		self.nominals.insert(nominal.0, id.0);
		// its member data (they may name the nominal: it is memoized above)
		let set = self.view.global_has_payloads(id);
		if !set && let Some(payloads) = self.view.payloads.get(&nominal) {
			let payloads: Vec<TypeId> = payloads.clone();
			let payloads: Vec<TypeId> = payloads
				.into_iter()
				.map(|payload| {
					if payload.is_unset() {
						payload
					} else {
						self.ty(payload)
					}
				})
				.collect();
			self.late().payloads.entry(id).or_insert(payloads);
		}
		id
	}

	/// The global id of a declaration key.
	pub fn decl_key(&mut self, key: DeclKey) -> DeclKey {
		if !is_local(key.0) {
			return key;
		}
		if let Some(&found) = self.decl_keys.get(&key.0) {
			return DeclKey(found);
		}
		let data = self.view.decl_key_data(key);
		let merged = DeclKeyData {
			module: self.module(data.module),
			names: self.sym_list(data.names),
			disambiguator: data.disambiguator,
		};
		let id = self.view.global.decl_keys.insert(merged);
		self.decl_keys.insert(key.0, id);
		DeclKey(id)
	}

	/// The global id of a module.
	pub fn module(&mut self, module: ModuleId) -> ModuleId {
		if !is_local(module.0) {
			return module;
		}
		if let Some(&found) = self.modules.get(&module.0) {
			return ModuleId(found);
		}
		let path = *self.view.modules.resolve(module.0);
		let path = self.sym(path);
		let id = self.view.global.modules.insert(path);
		self.modules.insert(module.0, id);
		ModuleId(id)
	}

	fn sig(&mut self, sig: SigId) -> SigId {
		if !is_local(sig.0) {
			return sig;
		}
		if let Some(&found) = self.sigs.get(&sig.0) {
			return SigId(found);
		}
		let data = self.view.sig(sig);
		let merged = FnSig {
			params: self.param_list(data.params),
			ret: self.ty(data.ret),
			throws: self.ty(data.throws),
			requires: self.req_list(data.requires),
			effects: self.effect_set(data.effects),
			..data
		};
		let id = self.view.global.sigs.insert(merged);
		self.sigs.insert(sig.0, id);
		SigId(id)
	}

	fn param_list(&mut self, list: ParamListId) -> ParamListId {
		if !is_local(list.0) {
			return list;
		}
		let params: Vec<ParamSig> = self.view.params(list).to_vec();
		let params: Vec<ParamSig> = params
			.into_iter()
			.map(|param| ParamSig {
				ty: self.ty(param.ty),
				..param
			})
			.collect();
		ParamListId(self.view.global.param_lists.insert(&params))
	}

	fn req_list(&mut self, list: ReqListId) -> ReqListId {
		if !is_local(list.0) {
			return list;
		}
		let requirements: Vec<Requirement> = self.view.requirements(list).to_vec();
		let requirements: Vec<Requirement> = requirements
			.into_iter()
			.map(|requirement| Requirement {
				name: self.sym(requirement.name),
				ty: self.ty(requirement.ty),
			})
			.collect();
		ReqListId(self.view.global.req_lists.insert(&requirements))
	}

	fn effect_set(&mut self, set: EffectSetId) -> EffectSetId {
		if !is_local(set.0) {
			return set;
		}
		if let Some(&found) = self.effects.get(&set.0) {
			return EffectSetId(found);
		}
		let data = *self.view.effects.resolve(set.0);
		let merged = EffectSet {
			mutates: self.path_list(data.mutates),
			invalidates: self.path_list(data.invalidates),
		};
		let id = self.view.global.effects.insert(merged);
		self.effects.insert(set.0, id);
		EffectSetId(id)
	}

	fn path_list(&mut self, list: PathListId) -> PathListId {
		if !is_local(list.0) {
			return list;
		}
		let paths: Vec<PathId> = self.view.path_lists.resolve(list.0).to_vec();
		let paths: Vec<PathId> = paths.into_iter().map(|path| self.path(path)).collect();
		PathListId(self.view.global.path_lists.insert(&paths))
	}

	fn path(&mut self, path: PathId) -> PathId {
		if !is_local(path.0) {
			return path;
		}
		if let Some(&found) = self.paths.get(&path.0) {
			return PathId(found);
		}
		let (data, steps) = self.view.path_data(path);
		let steps: Vec<PathStep> = steps.to_vec();
		let steps: Vec<PathStep> = steps
			.into_iter()
			.map(|step| match step {
				PathStep::Field(name) => PathStep::Field(self.sym(name)),
				PathStep::Private(name) => PathStep::Private(self.sym(name)),
				PathStep::IndexConst(value) => PathStep::IndexConst(self.constant(value)),
				other => other,
			})
			.collect();
		let steps = StepListId(self.view.global.step_lists.insert(&steps));
		let root = match data.root {
			PathRoot::ModuleValue(key) => PathRoot::ModuleValue(self.decl_key(key)),
			other => other,
		};
		let id = self.view.global.paths.insert(Path { root, steps });
		self.paths.insert(path.0, id);
		PathId(id)
	}

	/// The global id of a type parameter (an identity: one copy per local
	/// parameter, its bounds set once they are merged).
	pub fn param(&mut self, param: ParamId) -> ParamId {
		if !is_local(param.0) {
			return param;
		}
		if let Some(&found) = self.params.get(&param.0) {
			return ParamId(found);
		}
		let slot = self.view.global.params.push(std::sync::OnceLock::new());
		let id = global(slot as u32);
		self.params.insert(param.0, id);
		let info = self.view.param(param);
		let merged = ParamInfo {
			owner: self.decl_key(info.owner),
			name: self.sym(info.name),
			constraint: self.ty(info.constraint),
			default: self.ty(info.default),
			..info
		};
		let _ = self.view.global.params.get(slot).set(merged);
		ParamId(id)
	}

	/// A global recursive reference for a local one: an existing one (the
	/// same declaration and arguments), or a new shell whose target is set
	/// by [`Merger::finish_rec`] (its declaration key and arguments).
	fn rec_shell(&mut self, rec: RecId) -> (RecId, Option<(DeclKey, ListId)>) {
		if let Some(&found) = self.recs.get(&rec.0) {
			return (RecId(found), None);
		}
		let info = self.view.rec_info(rec);
		let key = self.decl_key(info.key);
		let args = self.list(info.args);
		let existing = self.late().rec_by_args.get(&(key, args)).copied();
		if let Some(existing) = existing
			&& let TypeKind::Rec(found) = self.view.global.types.get(existing.0).kind
		{
			self.recs.insert(rec.0, found.0);
			return (found, None);
		}
		let slot = self.view.global.recs.push(std::sync::OnceLock::new());
		self.recs.insert(rec.0, slot as u32);
		(RecId(slot as u32), Some((key, args)))
	}

	/// Sets the target of a new global recursive reference.
	fn finish_rec(&mut self, global: RecId, local: RecId, key: DeclKey, args: ListId, ty: TypeId) {
		let info = self.view.rec_info(local);
		let merged = RecInfo {
			key,
			name: self.sym(info.name),
			args,
			target: self.ty(info.target),
			object: info.object,
		};
		let _ = self.view.global.recs.get(global.0 as usize).set(merged);
		self.late().rec_by_args.entry((key, args)).or_insert(ty);
	}

	/// Merges the generic declarations of the view (their nominals and
	/// recursive references): the units that read it instantiate them.
	pub fn generic_declarations(&mut self) {
		let generics: Vec<(DeclKey, (NominalId, Vec<ParamId>))> = self
			.view
			.generic_nominals
			.iter()
			.map(|(&key, value)| (key, value.clone()))
			.collect();
		for (key, (nominal, params)) in generics {
			let key = self.decl_key(key);
			let nominal = self.nominal(nominal);
			let params: Vec<ParamId> = params.into_iter().map(|param| self.param(param)).collect();
			self.late()
				.generic_nominals
				.entry(key)
				.or_insert((nominal, params));
		}
		let recursive: Vec<(DeclKey, (TypeId, Vec<ParamId>))> = self
			.view
			.rec_params
			.iter()
			.map(|(&key, value)| (key, value.clone()))
			.collect();
		for (key, (generic, params)) in recursive {
			let key = self.decl_key(key);
			let generic = self.ty(generic);
			let params: Vec<ParamId> = params.into_iter().map(|param| self.param(param)).collect();
			self.late()
				.rec_params
				.entry(key)
				.or_insert((generic, params));
		}
	}
}
