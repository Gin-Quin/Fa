//! The [`TypeStore`]: interning, predefined ids, per-type flags, stable hashes.

use std::sync::{Arc, RwLock};

use ethnum::I256;
use rustc_hash::{FxHashMap, FxHashSet};

use crate::ids::{
	ConstId, DeclKey, EffectSetId, IntId, ListId, MaskId, ModuleId, NominalId, ParamId,
	ParamListId, PathId, PathListId, RecId, ReqListId, ShapeId, SigId, SpellingId, StepListId, Sym,
	SymListId, TypeId,
};
use crate::types::global::{self, AppendVec, GlobalSlices, GlobalValues, is_local};
use crate::types::layer::{RecordLayer, SliceLayer, Syms, ValueLayer};
use crate::types::relate::{Mode, Reason};
use crate::types::subst::Bindings;
use crate::types::{
	CollKind, ConstValue, DeclKeyData, EffectSet, FieldEntry, FieldFlags, FloatBits, FnSig,
	IntStorage, IntType, Interval, NominalInfo, NominalKind, ParamInfo, ParamSig, Path, PathStep,
	RecInfo, Requirement, RuntimeKind, Shape, Spelling, TypeFlags, TypeKind,
};

/// The predefined types, interned first at fixed indices: hot checks are integer compares.
impl TypeId {
	pub const NEVER: TypeId = TypeId(1);
	pub const VOID: TypeId = TypeId(2);
	pub const NONE: TypeId = TypeId(3);
	/// The built-in `Error`.
	pub const ERROR: TypeId = TypeId(4);
	pub const UNKNOWN: TypeId = TypeId(5);
	pub const BOOLEAN: TypeId = TypeId(6);
	pub const TRUE: TypeId = TypeId(7);
	pub const FALSE: TypeId = TypeId(8);
	/// `Integer`: the range of `Int64`, default storage.
	pub const INTEGER: TypeId = TypeId(9);
	pub const INT8: TypeId = TypeId(10);
	pub const INT16: TypeId = TypeId(11);
	pub const INT32: TypeId = TypeId(12);
	/// `Int64` written explicitly.
	pub const INT64: TypeId = TypeId(13);
	pub const INT128: TypeId = TypeId(14);
	pub const UINT8: TypeId = TypeId(15);
	/// `Byte` is `UInt8`.
	pub const BYTE: TypeId = TypeId::UINT8;
	pub const UINT16: TypeId = TypeId(16);
	pub const UINT32: TypeId = TypeId(17);
	pub const UINT64: TypeId = TypeId(18);
	pub const UINT128: TypeId = TypeId(19);
	/// `Float` (`Float64`).
	pub const FLOAT: TypeId = TypeId(20);
	pub const FLOAT32: TypeId = TypeId(21);
	pub const BIG_INT: TypeId = TypeId(22);
	pub const PERCENTAGE: TypeId = TypeId(23);
	pub const STRING: TypeId = TypeId(24);
	pub const CHARACTER: TypeId = TypeId(25);
	pub const RANGE: TypeId = TypeId(26);

	/// The full-range type of an integer storage.
	pub fn of_storage(storage: IntStorage) -> TypeId {
		match storage {
			IntStorage::Default => TypeId::INTEGER,
			IntStorage::I8 => TypeId::INT8,
			IntStorage::I16 => TypeId::INT16,
			IntStorage::I32 => TypeId::INT32,
			IntStorage::I64 => TypeId::INT64,
			IntStorage::I128 => TypeId::INT128,
			IntStorage::U8 => TypeId::UINT8,
			IntStorage::U16 => TypeId::UINT16,
			IntStorage::U32 => TypeId::UINT32,
			IntStorage::U64 => TypeId::UINT64,
			IntStorage::U128 => TypeId::UINT128,
		}
	}
}

/// A type: its kind (its identity), its flags and its stable hash.
#[derive(Copy, Clone, Debug)]
pub struct TypeEntry {
	pub kind: TypeKind,
	pub flags: TypeFlags,
	pub stable: u128,
}

impl TypeEntry {
	/// An entry to look a kind up with (flags and hash are not compared).
	fn probe(kind: TypeKind) -> Self {
		Self {
			kind,
			flags: TypeFlags::empty(),
			stable: 0,
		}
	}
}

impl PartialEq for TypeEntry {
	fn eq(&self, other: &Self) -> bool {
		self.kind == other.kind
	}
}

impl Eq for TypeEntry {}

impl std::hash::Hash for TypeEntry {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		self.kind.hash(state);
	}
}

/// The number of shards of the late-bound data of the global types.
const LATE_SHARDS: usize = 16;

/// The late-bound data of the global types, one shard of it (set by the
/// merges of finished units, `merge.rs`; read-only for the views). An entry
/// is in the shard of its key ([`GlobalStore::late`]).
#[derive(Default)]
pub struct GlobalLate {
	pub(crate) payloads: FxHashMap<NominalId, Vec<TypeId>>,
	/// The declared names of object types, with the module and offset of
	/// their declarations: a view shows the first one among the modules it
	/// may read (deterministic whatever the order of the merges).
	pub(crate) names: FxHashMap<TypeId, Vec<(ModuleId, u32, Sym)>>,
	/// The instances of named generic object types: the generic type's
	/// name and the type arguments, with the module that made them (a view
	/// reads the first among the modules it may read, as for `names`).
	pub(crate) applications: FxHashMap<TypeId, Vec<(ModuleId, Sym, ListId)>>,
	pub(crate) generic_nominals: FxHashMap<DeclKey, (NominalId, Vec<ParamId>)>,
	/// The recursive reference of each declaration and arguments, once its
	/// target is set (what the views read).
	pub(crate) rec_by_args: FxHashMap<(DeclKey, ListId), TypeId>,
	/// The recursive reference of each declaration and arguments, from the
	/// moment a merge creates it (its target may not be set yet): concurrent
	/// merges of the same reference share it (`merge.rs`).
	pub(crate) rec_shells: FxHashMap<(DeclKey, ListId), RecId>,
	pub(crate) rec_params: FxHashMap<DeclKey, (TypeId, Vec<ParamId>)>,
}

/// What every view of the type store shares (design/parallel-pipeline.md
/// §1.2): append-only tables, read without locks, written only by the
/// merges of finished units (which run concurrently: an insert locks one
/// shard of one table).
#[derive(Default)]
pub struct GlobalStore {
	pub(crate) types: Arc<GlobalValues<TypeEntry>>,
	pub(crate) syms: Arc<GlobalSlices<u8>>,
	pub(crate) lists: Arc<GlobalSlices<TypeId>>,
	pub(crate) sym_lists: Arc<GlobalSlices<Sym>>,
	pub(crate) ints: Arc<GlobalValues<IntType>>,
	pub(crate) field_lists: Arc<GlobalSlices<FieldEntry>>,
	pub(crate) shapes: Arc<GlobalValues<Shape>>,
	pub(crate) param_lists: Arc<GlobalSlices<ParamSig>>,
	pub(crate) req_lists: Arc<GlobalSlices<Requirement>>,
	pub(crate) sigs: Arc<GlobalValues<FnSig>>,
	pub(crate) step_lists: Arc<GlobalSlices<PathStep>>,
	pub(crate) paths: Arc<GlobalValues<Path>>,
	pub(crate) path_lists: Arc<GlobalSlices<PathId>>,
	pub(crate) effects: Arc<GlobalValues<EffectSet>>,
	pub(crate) masks: Arc<GlobalSlices<u64>>,
	pub(crate) consts: Arc<GlobalValues<Option<ConstValue>>>,
	pub(crate) nominals: Arc<GlobalValues<NominalInfo>>,
	pub(crate) modules: Arc<GlobalValues<Sym>>,
	pub(crate) decl_keys: Arc<GlobalValues<DeclKeyData>>,
	pub(crate) params: Arc<AppendVec<std::sync::OnceLock<ParamInfo>>>,
	pub(crate) spellings: Arc<GlobalValues<Spelling>>,
	pub(crate) recs: Arc<AppendVec<std::sync::OnceLock<RecInfo>>>,
	/// The late-bound data, in shards ([`GlobalStore::late`]).
	late: [RwLock<GlobalLate>; LATE_SHARDS],
}

impl GlobalStore {
	/// The shard of the late-bound data that holds the entry of `key`.
	#[inline]
	pub(crate) fn late(&self, key: &impl std::hash::Hash) -> &RwLock<GlobalLate> {
		&self.late[(global::hash_of(key) >> 60) as usize % LATE_SHARDS]
	}

	/// A global store with the predefined types at their fixed ids.
	pub fn new() -> Arc<Self> {
		let store = GlobalStore::default();
		// index 0 of every list table: the empty list
		store.lists.insert(&[]);
		store.sym_lists.insert(&[]);
		store.field_lists.insert(&[]);
		store.param_lists.insert(&[]);
		store.req_lists.insert(&[]);
		store.step_lists.insert(&[]);
		store.path_lists.insert(&[]);
		store.masks.insert(&[]);
		// ConstId::NONE and EffectSetId::EMPTY
		store.consts.insert(None);
		store.effects.insert(EffectSet {
			mutates: PathListId(0),
			invalidates: PathListId(0),
		});
		let store = Arc::new(store);
		// the predefined types, interned by a view, then made global
		let mut view = TypeStore::view(&store);
		// index 0: UNSET, never returned by interning
		store
			.types
			.push_unindexed(TypeEntry::probe(TypeKind::Unknown));
		let predefined = [
			(TypeId::NEVER, TypeKind::Never),
			(TypeId::VOID, TypeKind::Void),
			(TypeId::NONE, TypeKind::None),
			(TypeId::ERROR, TypeKind::ErrorTop),
			(TypeId::UNKNOWN, TypeKind::Unknown),
			(TypeId::BOOLEAN, TypeKind::Boolean),
			(TypeId::TRUE, TypeKind::BoolLit(true)),
			(TypeId::FALSE, TypeKind::BoolLit(false)),
		];
		for (id, kind) in predefined {
			assert_eq!(view.intern_global(kind), id);
		}
		for storage in IntStorage::ALL {
			let int = IntId(store.ints.insert(IntType::full(storage)));
			assert_eq!(
				view.intern_global(TypeKind::Int(int)),
				TypeId::of_storage(storage)
			);
		}
		let scalars = [
			(TypeId::FLOAT, TypeKind::Float(FloatBits::F64)),
			(TypeId::FLOAT32, TypeKind::Float(FloatBits::F32)),
			(TypeId::BIG_INT, TypeKind::BigInt),
			(TypeId::PERCENTAGE, TypeKind::Percentage),
			(TypeId::STRING, TypeKind::String),
			(TypeId::CHARACTER, TypeKind::Character),
			(TypeId::RANGE, TypeKind::Range),
		];
		for (id, kind) in scalars {
			assert_eq!(view.intern_global(kind), id);
		}
		store
	}
}

/// The `width` low bits set (a `Fields` type of `width` flags has them all).
pub fn fields_mask(width: u32) -> u128 {
	if width >= 128 {
		u128::MAX
	} else {
		(1u128 << width) - 1
	}
}

/// Interns a module path into the global store (the modules of a parallel
/// run, before any unit: their ids are global in every view).
pub fn intern_module(global: &GlobalStore, path: &str) -> ModuleId {
	let sym = Sym(global.syms.insert(path.as_bytes()));
	ModuleId(global.modules.insert(sym))
}

/// A view of the type store: the API of the walk (design/parallel-pipeline.md
/// §1.3). It reads the global store, and interns what is new locally (ids
/// with [`LOCAL`] set): the structures of one unit, which die with it unless
/// a merge copies them (`merge.rs`). Equal structures get equal ids in a
/// view, so `TypeId ==` is type equality.
pub struct TypeStore {
	/// The global store.
	pub(crate) global: Arc<GlobalStore>,
	pub(crate) types: ValueLayer<TypeEntry>,
	/// Identifiers and strings.
	pub syms: Syms,
	pub(crate) lists: SliceLayer<TypeId>,
	pub(crate) sym_lists: SliceLayer<Sym>,
	pub(crate) ints: ValueLayer<IntType>,
	pub(crate) field_lists: SliceLayer<FieldEntry>,
	pub(crate) shapes: ValueLayer<Shape>,
	pub(crate) param_lists: SliceLayer<ParamSig>,
	pub(crate) req_lists: SliceLayer<Requirement>,
	pub(crate) sigs: ValueLayer<FnSig>,
	pub(crate) step_lists: SliceLayer<PathStep>,
	pub(crate) paths: ValueLayer<Path>,
	pub(crate) path_lists: SliceLayer<PathId>,
	pub(crate) effects: ValueLayer<EffectSet>,
	pub(crate) masks: SliceLayer<u64>,
	pub(crate) consts: ValueLayer<Option<ConstValue>>,
	pub(crate) nominals: ValueLayer<NominalInfo>,
	pub(crate) modules: ValueLayer<Sym>,
	pub(crate) decl_keys: ValueLayer<DeclKeyData>,
	pub(crate) params: RecordLayer<ParamInfo>,
	pub(crate) spellings: ValueLayer<Spelling>,
	/// The data of each member of a named union or `Errors` type (`UNSET`:
	/// none) declared in this view.
	pub(crate) payloads: FxHashMap<NominalId, Vec<TypeId>>,
	/// The name an object type was first declared with in this view
	/// (display only: the type stays structural).
	pub(crate) names: FxHashMap<TypeId, Sym>,
	/// The instances of named generic object types made in this view
	/// (`Pair<Integer, String>`): the generic type's name and the type
	/// arguments, so that a substitution of an instance names its result.
	pub(crate) applications: FxHashMap<TypeId, (Sym, ListId)>,
	/// The declaration of each generic nominal type, by its key: the nominal
	/// whose payloads use the parameters, and the parameters.
	pub(crate) generic_nominals: FxHashMap<DeclKey, (NominalId, Vec<ParamId>)>,
	/// The recursive references, by `RecId`.
	pub(crate) recs: RecordLayer<RecInfo>,
	/// The recursive reference of each declaration and arguments.
	pub(crate) rec_by_args: FxHashMap<(DeclKey, ListId), TypeId>,
	/// The parameters of each generic recursive declaration, by its key.
	pub(crate) rec_params: FxHashMap<DeclKey, (TypeId, Vec<ParamId>)>,
	/// The modules whose declared names of global types this view shows
	/// (`None`: all of them).
	pub(crate) visible: Option<Arc<FxHashSet<ModuleId>>>,
	/// The pairs of recursive references assumed related while relating
	/// their unfoldings (coinduction).
	pub(crate) rec_assumptions: Vec<(TypeId, TypeId)>,
	pub(crate) assign_cache: FxHashMap<(TypeId, TypeId, Mode), Result<(), Reason>>,
	/// Counts the reads of incomplete types (payloads not set yet, a
	/// recursive reference without its target, a parameter whose bounds are
	/// not resolved yet): a relation that read one is not cached (P9), so
	/// that the cache is never cleared.
	pub(crate) incomplete_reads: std::sync::atomic::AtomicU32,
	/// The parameters declared whose bounds are not set yet.
	pub(crate) pending_bounds: FxHashSet<ParamId>,
	pub(crate) scratch: Vec<TypeId>,
}

impl Default for TypeStore {
	fn default() -> Self {
		Self::new()
	}
}

impl TypeStore {
	/// A store of its own: a new global store (the predefined types) and a
	/// view of it.
	pub fn new() -> Self {
		Self::view(&GlobalStore::new())
	}

	/// A new view of a global store.
	pub fn view(global: &Arc<GlobalStore>) -> Self {
		TypeStore {
			global: Arc::clone(global),
			types: ValueLayer::new(Arc::clone(&global.types)),
			syms: Syms::new(Arc::clone(&global.syms)),
			lists: SliceLayer::new(Arc::clone(&global.lists)),
			sym_lists: SliceLayer::new(Arc::clone(&global.sym_lists)),
			ints: ValueLayer::new(Arc::clone(&global.ints)),
			field_lists: SliceLayer::new(Arc::clone(&global.field_lists)),
			shapes: ValueLayer::new(Arc::clone(&global.shapes)),
			param_lists: SliceLayer::new(Arc::clone(&global.param_lists)),
			req_lists: SliceLayer::new(Arc::clone(&global.req_lists)),
			sigs: ValueLayer::new(Arc::clone(&global.sigs)),
			step_lists: SliceLayer::new(Arc::clone(&global.step_lists)),
			paths: ValueLayer::new(Arc::clone(&global.paths)),
			path_lists: SliceLayer::new(Arc::clone(&global.path_lists)),
			effects: ValueLayer::new(Arc::clone(&global.effects)),
			masks: SliceLayer::new(Arc::clone(&global.masks)),
			consts: ValueLayer::new(Arc::clone(&global.consts)),
			nominals: ValueLayer::new(Arc::clone(&global.nominals)),
			modules: ValueLayer::new(Arc::clone(&global.modules)),
			decl_keys: ValueLayer::new(Arc::clone(&global.decl_keys)),
			params: RecordLayer::new(Arc::clone(&global.params)),
			spellings: ValueLayer::new(Arc::clone(&global.spellings)),
			payloads: FxHashMap::default(),
			names: FxHashMap::default(),
			applications: FxHashMap::default(),
			generic_nominals: FxHashMap::default(),
			recs: RecordLayer::new(Arc::clone(&global.recs)),
			rec_by_args: FxHashMap::default(),
			rec_params: FxHashMap::default(),
			visible: None,
			rec_assumptions: Vec::new(),
			assign_cache: FxHashMap::default(),
			incomplete_reads: std::sync::atomic::AtomicU32::new(0),
			pending_bounds: FxHashSet::default(),
			scratch: Vec::new(),
		}
	}

	/// The global store this view reads.
	pub fn global(&self) -> &Arc<GlobalStore> {
		&self.global
	}

	/// Restricts the declared names of global types this view shows to the
	/// declarations of some modules (a unit: the modules it may read).
	pub fn set_visible(&mut self, modules: Option<Arc<FxHashSet<ModuleId>>>) {
		self.visible = modules;
	}

	/// Interns a type whose children are all global into the global store
	/// (the predefined types; merges).
	pub(crate) fn intern_global(&mut self, kind: TypeKind) -> TypeId {
		let probe = TypeEntry::probe(kind);
		if let Some(id) = self.types.global.find(global::hash_of(&probe), &probe) {
			return TypeId(id);
		}
		let flags = self.compute_flags(kind);
		let stable = self.compute_stable_hash(kind);
		TypeId(self.types.global.insert(TypeEntry {
			kind,
			flags,
			stable,
		}))
	}

	// ── interning ────────────────────────────────────────────────────────

	/// The id of a type, interned on first use with its flags and stable hash.
	///
	/// Unions must go through [`TypeStore::union`], which normalizes them.
	pub fn intern(&mut self, kind: TypeKind) -> TypeId {
		let probe = TypeEntry::probe(kind);
		if let Some(id) = self.types.find(&probe) {
			return TypeId(id);
		}
		let flags = self.compute_flags(kind);
		let stable = self.compute_stable_hash(kind);
		TypeId(self.types.intern(TypeEntry {
			kind,
			flags,
			stable,
		}))
	}

	/// The number of types the view reaches (global ones, `UNSET`
	/// included, and its own).
	pub fn len(&self) -> usize {
		self.types.len()
	}

	/// The number of types interned by this view only.
	pub fn local_len(&self) -> usize {
		self.types.local_len()
	}

	/// Always false: the predefined types exist.
	pub fn is_empty(&self) -> bool {
		self.types.is_empty()
	}

	/// The kind of a type.
	#[inline]
	pub fn kind(&self, ty: TypeId) -> TypeKind {
		self.types.resolve(ty.0).kind
	}

	/// The flags of a type.
	#[inline]
	pub fn flags(&self, ty: TypeId) -> TypeFlags {
		self.types.resolve(ty.0).flags
	}

	/// The runtime kind of a type's values (types brief §3.4.2).
	#[inline]
	pub fn runtime_kind(&self, ty: TypeId) -> RuntimeKind {
		self.flags(ty).runtime_kind()
	}

	/// A 128-bit structural hash computed from names and children, never from
	/// ids: identical across sessions (incremental cache, canonical union order).
	#[inline]
	pub fn stable_hash(&self, ty: TypeId) -> u128 {
		self.types.resolve(ty.0).stable
	}

	// ── constructors ─────────────────────────────────────────────────────

	/// An integer type.
	pub fn int_type(&mut self, int: IntType) -> TypeId {
		let id = IntId(self.ints.intern(int));
		self.intern(TypeKind::Int(id))
	}

	/// The integer type `min..=max` of a storage.
	pub fn int_range(&mut self, min: I256, max: I256, storage: IntStorage) -> TypeId {
		self.int_type(IntType { min, max, storage })
	}

	/// The integer literal type `value` with storage `Integer`.
	pub fn int_literal(&mut self, value: I256) -> TypeId {
		self.int_range(value, value, IntStorage::Default)
	}

	/// A boolean literal type.
	pub fn bool_literal(&self, value: bool) -> TypeId {
		if value { TypeId::TRUE } else { TypeId::FALSE }
	}

	/// A string literal type.
	/// A key set (`Fields { foo, nested { bar } }`): its keys, each a name and
	/// its children (empty for a leaf).
	pub fn fields_type(&mut self, keys: &[(String, TypeId)]) -> TypeId {
		let items: Vec<TypeId> = keys
			.iter()
			.map(|(name, children)| {
				let name = self.string_literal(name);
				if children.is_unset() {
					name
				} else {
					self.tuple(&[name, *children])
				}
			})
			.collect();
		let list = self.list(&items);
		self.intern(TypeKind::Fields(list))
	}

	/// The keys of a key set: each name and its children's key set (`UNSET`
	/// for a leaf).
	pub fn fields_keys(&self, ty: TypeId) -> Vec<(String, TypeId)> {
		let TypeKind::Fields(list) = self.kind(ty) else {
			return Vec::new();
		};
		self.list_items(list)
			.iter()
			.filter_map(|&item| match self.kind(item) {
				TypeKind::StringLit(name) => {
					Some((self.syms.resolve(name).to_string(), TypeId::UNSET))
				}
				TypeKind::Tuple(pair) => match self.list_items(pair) {
					[name, children] => match self.kind(*name) {
						TypeKind::StringLit(name) => {
							Some((self.syms.resolve(name).to_string(), *children))
						}
						_ => None,
					},
					_ => None,
				},
				_ => None,
			})
			.collect()
	}

	/// The leaves of a `Fields` type, in bit order (declaration order, the
	/// members of a group in its place): their dotted paths (`nested.baz1`).
	pub fn fields_leaves(&self, ty: TypeId) -> Vec<String> {
		let mut leaves = Vec::new();
		self.collect_leaves(ty, "", &mut leaves, 0);
		leaves
	}

	fn collect_leaves(&self, ty: TypeId, prefix: &str, leaves: &mut Vec<String>, depth: u32) {
		if depth > 32 {
			return;
		}
		for (name, children) in self.fields_keys(ty) {
			let path = if prefix.is_empty() {
				name
			} else {
				format!("{prefix}.{name}")
			};
			if children.is_unset() {
				leaves.push(path);
			} else {
				self.collect_leaves(children, &path, leaves, depth + 1);
			}
		}
	}

	/// The names a `Fields` value accepts in its JSON form, with their bits:
	/// every flag and every group, as dotted paths, a group before its
	/// members (`nested`, `nested.a`, `nested.b`).
	pub fn fields_names(&self, ty: TypeId) -> Vec<(String, u128)> {
		let mut names = Vec::new();
		self.collect_names(ty, "", 0, &mut names, 0);
		names
	}

	fn collect_names(
		&self,
		ty: TypeId,
		prefix: &str,
		mut offset: u32,
		names: &mut Vec<(String, u128)>,
		depth: u32,
	) {
		if depth > 32 {
			return;
		}
		for (name, children) in self.fields_keys(ty) {
			let path = if prefix.is_empty() {
				name
			} else {
				format!("{prefix}.{name}")
			};
			let width = if children.is_unset() {
				1
			} else {
				self.fields_width(children)
			};
			names.push((
				path.clone(),
				fields_mask(width).checked_shl(offset).unwrap_or(0),
			));
			if !children.is_unset() {
				self.collect_names(children, &path, offset, names, depth + 1);
			}
			offset += width;
		}
	}

	/// The number of leaves (bits) of a `Fields` type.
	pub fn fields_width(&self, ty: TypeId) -> u32 {
		self.fields_width_of_kind(self.kind(ty))
	}

	fn fields_width_of_kind(&self, kind: TypeKind) -> u32 {
		let TypeKind::Fields(list) = kind else {
			return 0;
		};
		self.list_items(list)
			.iter()
			.map(|&item| match self.kind(item) {
				TypeKind::Tuple(pair) => match self.list_items(pair) {
					[_, children] => self.fields_width(*children),
					_ => 0,
				},
				_ => 1,
			})
			.sum()
	}

	/// The bits of a key path of a `Fields` type (`["nested", "baz1"]`): one
	/// bit for a leaf, the bits of all its leaves for a group. `None` when a
	/// name is not a key of its level.
	pub fn fields_path_bits(&self, ty: TypeId, path: &[&str]) -> Option<u128> {
		let (first, rest) = path.split_first()?;
		let mut offset = 0u32;
		for (name, children) in self.fields_keys(ty) {
			let width = if children.is_unset() {
				1
			} else {
				self.fields_width(children)
			};
			if name == *first {
				let bits = if rest.is_empty() {
					fields_mask(width)
				} else if children.is_unset() {
					return None;
				} else {
					self.fields_path_bits(children, rest)?
				};
				return Some(bits.checked_shl(offset).unwrap_or(0));
			}
			offset += width;
		}
		None
	}

	/// The storage of a `Fields` type: the smallest unsigned integer of 8,
	/// 16, 32, 64 or 128 bits holding its bits.
	pub fn fields_storage_bits(&self, ty: TypeId) -> u32 {
		match self.fields_width(ty) {
			0..=8 => 8,
			9..=16 => 16,
			17..=32 => 32,
			33..=64 => 64,
			_ => 128,
		}
	}

	pub fn string_literal(&mut self, text: &str) -> TypeId {
		let sym = self.syms.intern(text);
		self.intern(TypeKind::StringLit(sym))
	}

	/// A float literal type.
	pub fn float_literal(&mut self, bits: FloatBits, value: f64) -> TypeId {
		let value = match bits {
			FloatBits::F32 => value as f32 as f64,
			FloatBits::F64 => value,
		};
		let constant = self.constant(ConstValue::Float(value.to_bits()));
		self.intern(TypeKind::FloatLit(bits, constant))
	}

	/// A tuple type.
	pub fn tuple(&mut self, items: &[TypeId]) -> TypeId {
		let list = self.list(items);
		self.intern(TypeKind::Tuple(list))
	}

	/// `T?`: the union of `T` and `None`.
	pub fn optional(&mut self, ty: TypeId) -> TypeId {
		self.union(&[ty, TypeId::NONE])
	}

	/// An object type. Fields are sorted by name bytes.
	pub fn object(&mut self, fields: &[FieldEntry], nominal: Option<NominalId>) -> TypeId {
		let mut sorted: smallvec::SmallVec<[FieldEntry; 8]> = fields.into();
		sorted.sort_by(|a, b| self.syms.resolve(a.name).cmp(self.syms.resolve(b.name)));
		let fields = self.field_lists.intern(&sorted);
		let shape = ShapeId(self.shapes.intern(Shape { fields, nominal }));
		self.intern(TypeKind::Object(shape))
	}

	/// An object type whose fields keep their declaration order: a
	/// `CStruct`, whose layout is C's (fields in declaration order). Nominal,
	/// so the order never meets the sorted order of a structural type.
	pub fn object_in_order(&mut self, fields: &[FieldEntry], nominal: NominalId) -> TypeId {
		let fields = self.field_lists.intern(fields);
		let shape = ShapeId(self.shapes.intern(Shape {
			fields,
			nominal: Some(nominal),
		}));
		self.intern(TypeKind::Object(shape))
	}

	/// A function type.
	pub fn function(&mut self, sig: FnSig) -> TypeId {
		let sig = SigId(self.sigs.intern(sig));
		self.intern(TypeKind::Function(sig))
	}

	/// A collection type. `key` is `NONE` except for maps.
	pub fn collection(&mut self, kind: CollKind, item: TypeId, key: TypeId) -> TypeId {
		self.intern(TypeKind::Collection(kind, item, key))
	}

	/// `Array(item)`.
	pub fn array(&mut self, item: TypeId) -> TypeId {
		self.collection(CollKind::Array, item, TypeId::NONE)
	}

	/// `Iterator(item)`: the iterator of a local collection (`values()`),
	/// whose collection the walk tracks (`check/iterators.rs`).
	pub fn pointer(&mut self, target: TypeId) -> TypeId {
		self.intern(TypeKind::Pointer(target))
	}

	/// `CArray(item, count)`.
	pub fn c_array(&mut self, item: TypeId, count: u32) -> TypeId {
		self.intern(TypeKind::CArray(item, count))
	}

	/// The type of the target of a pointer (`Pointer(T)`, or `Pointer(T)?`).
	pub fn pointer_target(&self, ty: TypeId) -> Option<TypeId> {
		match self.kind(ty) {
			TypeKind::Pointer(target) => Some(target),
			TypeKind::Union(list) => {
				let items = self.list_items(list);
				if items.len() == 2 && items.contains(&TypeId::NONE) {
					let other = if items[0] == TypeId::NONE {
						items[1]
					} else {
						items[0]
					};
					self.pointer_target(other)
				} else {
					None
				}
			}
			_ => None,
		}
	}

	/// Whether this object type declares `destroy` (not an owned child).
	pub fn is_resource(&self, ty: TypeId) -> bool {
		match self.kind(ty) {
			TypeKind::Object(shape) => self
				.shape(shape)
				.nominal
				.is_some_and(|nominal| self.nominal(nominal).kind == NominalKind::Resource),
			_ => false,
		}
	}

	/// Destructors of the possible actual root values, through aliases and
	/// selected union payloads. Ordinary data fields are not root boundaries.
	fn resource_roots(&self, ty: TypeId) -> Vec<NominalId> {
		self.lifetime_roots(ty)
			.into_iter()
			.filter_map(|ty| {
				let TypeKind::Object(shape) = self.kind(ty) else {
					return None;
				};
				self.shape(shape).nominal
			})
			.collect()
	}

	/// A possible root destructor prevents field extraction until narrowing.
	pub fn has_destroy_boundary(&self, ty: TypeId) -> bool {
		!self.resource_roots(ty).is_empty()
	}

	/// Lifting a whole resource into a union preserves its written owner;
	/// projecting fields into an unrelated structural value does not.
	pub fn preserves_resource_owner(&self, actual: TypeId, target: TypeId) -> bool {
		let targets = self.resource_roots(target);
		self.resource_roots(actual).iter().all(|&actual| {
			let actual = self.nominal(actual);
			targets.iter().any(|&target| {
				let target = self.nominal(target);
				actual.key == target.key && actual.args == target.args
			})
		})
	}

	/// Root lifetime owners, including built-ins, are never erased by a
	/// structural projection. Owned fields may be projected separately.
	fn lifetime_roots(&self, ty: TypeId) -> Vec<TypeId> {
		let mut roots = Vec::new();
		let mut pending = vec![ty];
		let mut seen = rustc_hash::FxHashSet::default();
		while let Some(ty) = pending.pop() {
			if ty.is_unset() || !seen.insert(ty) {
				continue;
			}
			match self.kind(ty) {
				TypeKind::Object(_) if self.is_resource(ty) => roots.push(ty),
				TypeKind::Sync(..) | TypeKind::Subscription | TypeKind::Task(..)
					if self.is_unique(ty) =>
				{
					roots.push(ty)
				}
				TypeKind::Rec(_) => pending.extend(self.rec_target(ty)),
				TypeKind::Union(list) => pending.extend(self.list_items(list)),
				TypeKind::Member(nominal, index) => pending.push(self.payload(nominal, index)),
				TypeKind::NamedUnion(nominal, mask) => {
					for index in 0..self.sym_list(self.nominal(nominal).members).len() as u16 {
						if self.mask_has(mask, index) {
							pending.push(self.payload(nominal, index));
						}
					}
				}
				_ => {}
			}
		}
		roots
	}

	pub fn has_lifetime_boundary(&self, ty: TypeId) -> bool {
		!self.lifetime_roots(ty).is_empty()
	}

	pub fn preserves_lifetime_owner(&self, actual: TypeId, target: TypeId) -> bool {
		let targets = self.lifetime_roots(target);
		self.lifetime_roots(actual).iter().all(|&actual| {
			if self.is_resource(actual) {
				return targets.iter().any(|&target| {
					self.is_resource(target) && self.preserves_resource_owner(actual, target)
				});
			}
			targets
				.iter()
				.any(|&target| match (self.kind(actual), self.kind(target)) {
					(TypeKind::Sync(actual, _), TypeKind::Sync(target, _)) => actual == target,
					(TypeKind::Subscription, TypeKind::Subscription) => true,
					(TypeKind::Task(..), TypeKind::Task(..)) => true,
					_ => false,
				})
		})
	}

	/// Whether owned data contains a resource whose bytes may all be zero.
	/// These values need drop flags rather than a zeroed-slot convention.
	pub fn contains_resource(&self, ty: TypeId) -> bool {
		self.owned_any(ty, Self::is_resource)
	}

	/// `Iterator(item)`.
	pub fn iterator(&mut self, item: TypeId) -> TypeId {
		let path = self.path(crate::types::PathRoot::SelfParam, &[]);
		self.intern(TypeKind::Iterator(item, path))
	}

	/// Whether a value of type `ty` holds an iterator: an iterator, or a
	/// collection, an object, a tuple or a union holding one (`CL030`).
	pub fn contains_iterator(&self, ty: TypeId) -> bool {
		self.contains_iterator_depth(ty, 0)
	}

	fn contains_iterator_depth(&self, ty: TypeId, depth: u32) -> bool {
		if depth > 16 {
			return false;
		}
		match self.kind(ty) {
			TypeKind::Iterator(..) => true,
			TypeKind::Collection(_, item, key) => {
				self.contains_iterator_depth(item, depth + 1)
					|| self.contains_iterator_depth(key, depth + 1)
			}
			TypeKind::Tuple(list) | TypeKind::Union(list) => self
				.list_items(list)
				.iter()
				.any(|&item| self.contains_iterator_depth(item, depth + 1)),
			TypeKind::Object(shape) => self
				.shape_fields(shape)
				.iter()
				.any(|field| self.contains_iterator_depth(field.ty, depth + 1)),
			_ => false,
		}
	}

	/// A member of a named union or `Errors` type.
	pub fn member(&mut self, nominal: NominalId, index: u16) -> TypeId {
		self.intern(TypeKind::Member(nominal, index))
	}

	/// A subset of the members of a named union: a single member becomes a
	/// [`TypeKind::Member`], an empty mask `Never`.
	pub fn named_union(&mut self, nominal: NominalId, mask: MaskId) -> TypeId {
		self.masked(nominal, mask, false)
	}

	/// A subset of the members of an enumeration (see [`TypeStore::named_union`]).
	pub fn enumeration(&mut self, nominal: NominalId, mask: MaskId) -> TypeId {
		self.masked(nominal, mask, true)
	}

	fn masked(&mut self, nominal: NominalId, mask: MaskId, is_enum: bool) -> TypeId {
		let bits = self.mask_bits(mask);
		let count: u32 = bits.iter().map(|word| word.count_ones()).sum();
		if count == 0 {
			return TypeId::NEVER;
		}
		if count == 1 {
			let index = bits
				.iter()
				.enumerate()
				.find(|(_, word)| **word != 0)
				.map(|(i, word)| i as u32 * 64 + word.trailing_zeros())
				.expect("one bit");
			let index = index as u16;
			return self.intern(if is_enum {
				TypeKind::EnumMember(nominal, index)
			} else {
				TypeKind::Member(nominal, index)
			});
		}
		self.intern(if is_enum {
			TypeKind::Enum(nominal, mask)
		} else {
			TypeKind::NamedUnion(nominal, mask)
		})
	}

	/// The declared type of a named union, `Errors` type or enumeration: all its members.
	pub fn nominal_type(&mut self, nominal: NominalId) -> TypeId {
		let info = self.nominal(nominal);
		let count = self.sym_list(info.members).len();
		let mask = self.full_mask(count);
		match info.kind {
			NominalKind::Enum => self.intern(TypeKind::Enum(nominal, mask)),
			NominalKind::CStruct => self.intern(TypeKind::CStruct(nominal)),
			_ => self.intern(TypeKind::NamedUnion(nominal, mask)),
		}
	}

	// ── side tables ──────────────────────────────────────────────────────

	/// An interned list of types.
	pub fn list(&mut self, items: &[TypeId]) -> ListId {
		ListId(self.lists.intern(items))
	}

	/// The types of a list.
	#[inline]
	pub fn list_items(&self, list: ListId) -> &[TypeId] {
		self.lists.resolve(list.0)
	}

	/// An interned list of symbols.
	pub fn sym_list(&self, list: SymListId) -> &[Sym] {
		self.sym_lists.resolve(list.0)
	}

	/// Interns a list of symbols.
	pub fn intern_sym_list(&mut self, items: &[Sym]) -> SymListId {
		SymListId(self.sym_lists.intern(items))
	}

	/// The integer type of an [`IntId`].
	#[inline]
	pub fn int(&self, id: IntId) -> &IntType {
		self.ints.resolve(id.0)
	}

	/// The integer type of a type, if it is one.
	pub fn as_int(&self, ty: TypeId) -> Option<&IntType> {
		match self.kind(ty) {
			TypeKind::Int(id) => Some(self.int(id)),
			_ => None,
		}
	}

	/// The interval of an integer type.
	pub fn interval_of(&self, ty: TypeId) -> Option<Interval> {
		self.as_int(ty).map(IntType::interval)
	}

	/// A shape.
	pub fn shape(&self, shape: ShapeId) -> Shape {
		*self.shapes.resolve(shape.0)
	}

	/// The fields of a shape, sorted by name.
	pub fn shape_fields(&self, shape: ShapeId) -> &[FieldEntry] {
		self.field_lists.resolve(self.shape(shape).fields)
	}

	/// The fields of an object type (empty for other types).
	pub fn fields_of(&self, ty: TypeId) -> &[FieldEntry] {
		let ty = self.unfold(ty);
		match self.kind(ty) {
			TypeKind::Object(shape) => self.shape_fields(shape),
			_ => &[],
		}
	}

	/// A signature.
	pub fn sig(&self, sig: SigId) -> FnSig {
		*self.sigs.resolve(sig.0)
	}

	/// Interns a parameter list.
	pub fn param_list(&mut self, params: &[ParamSig]) -> ParamListId {
		ParamListId(self.param_lists.intern(params))
	}

	/// The parameters of a list.
	pub fn params(&self, list: ParamListId) -> &[ParamSig] {
		self.param_lists.resolve(list.0)
	}

	/// Interns a requirement list.
	pub fn req_list(&mut self, requirements: &[Requirement]) -> ReqListId {
		ReqListId(self.req_lists.intern(requirements))
	}

	/// The requirements of a list.
	pub fn requirements(&self, list: ReqListId) -> &[Requirement] {
		self.req_lists.resolve(list.0)
	}

	/// The signature of a simple function: parameters, return type and errors, no effects.
	pub fn simple_sig(&mut self, params: &[TypeId], ret: TypeId, throws: TypeId) -> FnSig {
		let params: smallvec::SmallVec<[ParamSig; 4]> = params
			.iter()
			.map(|&ty| ParamSig {
				ty,
				variadic: false,
			})
			.collect();
		FnSig {
			params: self.param_list(&params),
			ret,
			throws,
			requires: ReqListId(0),
			effects: EffectSetId::EMPTY,
			stored: 0,
			ignorable: 0,
			generics: ListId::EMPTY,
			is_async: false,
		}
	}

	/// Interns an effect path.
	pub fn path(&mut self, root: crate::types::PathRoot, steps: &[PathStep]) -> PathId {
		let steps = StepListId(self.step_lists.intern(steps));
		PathId(self.paths.intern(Path { root, steps }))
	}

	/// An effect path and its steps.
	pub fn path_data(&self, path: PathId) -> (Path, &[PathStep]) {
		let data = *self.paths.resolve(path.0);
		(data, self.step_lists.resolve(data.steps.0))
	}

	/// Interns an effect set (normalization of covered paths is M2).
	pub fn effect_set(&mut self, mutates: &[PathId], invalidates: &[PathId]) -> EffectSetId {
		let mutates = PathListId(self.path_lists.intern(mutates));
		let invalidates = PathListId(self.path_lists.intern(invalidates));
		EffectSetId(self.effects.intern(EffectSet {
			mutates,
			invalidates,
		}))
	}

	/// The `mutates` and `invalidates` paths of an effect set.
	pub fn effects(&self, set: EffectSetId) -> (&[PathId], &[PathId]) {
		let data = self.effects.resolve(set.0);
		(
			self.path_lists.resolve(data.mutates.0),
			self.path_lists.resolve(data.invalidates.0),
		)
	}

	/// Interns a member bitset (trailing zero words are dropped).
	pub fn mask(&mut self, words: &[u64]) -> MaskId {
		let used = words
			.iter()
			.rposition(|&word| word != 0)
			.map_or(0, |last| last + 1);
		MaskId(self.masks.intern(&words[..used]))
	}

	/// The mask with the first `count` members.
	pub fn full_mask(&mut self, count: usize) -> MaskId {
		let mut words: smallvec::SmallVec<[u64; 2]> = smallvec::smallvec![0; count.div_ceil(64)];
		for index in 0..count {
			words[index / 64] |= 1 << (index % 64);
		}
		self.mask(&words)
	}

	/// The mask of a single member.
	pub fn single_mask(&mut self, index: u16) -> MaskId {
		let index = index as usize;
		let mut words: smallvec::SmallVec<[u64; 2]> = smallvec::smallvec![0; index / 64 + 1];
		words[index / 64] = 1 << (index % 64);
		self.mask(&words)
	}

	/// The words of a mask.
	pub fn mask_bits(&self, mask: MaskId) -> &[u64] {
		self.masks.resolve(mask.0)
	}

	/// Whether member `index` is in a mask.
	pub fn mask_has(&self, mask: MaskId, index: u16) -> bool {
		let index = index as usize;
		self.mask_bits(mask)
			.get(index / 64)
			.is_some_and(|word| word & (1 << (index % 64)) != 0)
	}

	/// The members of `a` in `b`, and those not in `b`.
	pub fn mask_split(&mut self, a: MaskId, b: MaskId) -> (MaskId, MaskId) {
		let (both, rest): (smallvec::SmallVec<[u64; 2]>, smallvec::SmallVec<[u64; 2]>) = {
			let (a, b) = (self.mask_bits(a), self.mask_bits(b));
			a.iter()
				.enumerate()
				.map(|(i, word)| {
					let other = b.get(i).copied().unwrap_or(0);
					(word & other, word & !other)
				})
				.unzip()
		};
		(self.mask(&both), self.mask(&rest))
	}

	/// The union of two masks.
	pub fn mask_union(&mut self, a: MaskId, b: MaskId) -> MaskId {
		let words: smallvec::SmallVec<[u64; 2]> = {
			let (a, b) = (self.mask_bits(a), self.mask_bits(b));
			(0..a.len().max(b.len()))
				.map(|i| a.get(i).copied().unwrap_or(0) | b.get(i).copied().unwrap_or(0))
				.collect()
		};
		self.mask(&words)
	}

	/// A named union, an enumeration or one of their members as a mask of
	/// its nominal type: `(nominal, mask, is_enum)`.
	pub fn nominal_mask(&mut self, ty: TypeId) -> Option<(NominalId, MaskId, bool)> {
		match self.kind(ty) {
			TypeKind::NamedUnion(nominal, mask) => Some((nominal, mask, false)),
			TypeKind::Enum(nominal, mask) => Some((nominal, mask, true)),
			TypeKind::Member(nominal, index) => Some((nominal, self.single_mask(index), false)),
			TypeKind::EnumMember(nominal, index) => Some((nominal, self.single_mask(index), true)),
			_ => None,
		}
	}

	/// Whether every member of `a` is in `b`.
	pub fn mask_subset(&self, a: MaskId, b: MaskId) -> bool {
		let (a, b) = (self.mask_bits(a), self.mask_bits(b));
		a.iter()
			.enumerate()
			.all(|(i, word)| word & !b.get(i).copied().unwrap_or(0) == 0)
	}

	/// Interns a compile-time value.
	pub fn constant(&mut self, value: ConstValue) -> ConstId {
		ConstId(self.consts.intern(Some(value)))
	}

	/// A compile-time value. Panics on [`ConstId::NONE`].
	pub fn const_value(&self, id: ConstId) -> ConstValue {
		self.consts
			.resolve(id.0)
			.expect("ConstId::NONE has no value")
	}

	/// Interns a nominal type.
	pub fn intern_nominal(&mut self, info: NominalInfo) -> NominalId {
		NominalId(self.nominals.intern(info))
	}

	/// A nominal type.
	pub fn nominal(&self, nominal: NominalId) -> NominalInfo {
		*self.nominals.resolve(nominal.0)
	}

	/// An upper bound on the distinct named members visible to this view,
	/// including members that no expression has used yet.
	pub fn nominal_member_count(&self) -> usize {
		let global = self.nominals.global.len();
		(0..global)
			.map(|index| index as u32)
			.chain((0..self.nominals.local_len()).map(|index| index as u32 | super::global::LOCAL))
			.map(|id| self.sym_list(self.nominal(NominalId(id)).members).len())
			.sum()
	}

	/// Sets the data of the members of a named union or `Errors` type, in
	/// member order (`UNSET` for a member without data). Set once, after the
	/// nominal is interned, so that a member may refer to its union. The
	/// data of a global nominal are already set (by the unit that declared
	/// it): they are kept.
	pub fn set_payloads(&mut self, nominal: NominalId, payloads: Vec<TypeId>) {
		if !is_local(nominal.0) && self.global_late(&nominal).payloads.contains_key(&nominal) {
			return;
		}
		self.payloads.insert(nominal, payloads);
	}

	/// The shard of the late-bound data of the global types that holds the
	/// entry of `key`.
	fn global_late(
		&self,
		key: &impl std::hash::Hash,
	) -> std::sync::RwLockReadGuard<'_, GlobalLate> {
		self.global
			.late(key)
			.read()
			.unwrap_or_else(|poisoned| poisoned.into_inner())
	}

	/// Gives an object type the name it is declared with, for messages and
	/// hover (the first declaration wins; the type stays structural).
	pub fn name_type(&mut self, ty: TypeId, name: &str) {
		if !matches!(self.kind(ty), TypeKind::Object(_) | TypeKind::Fields(_)) {
			return;
		}
		let name = self.syms.intern(name);
		self.names.entry(ty).or_insert(name);
	}

	/// Records an instance of a named generic object type (`Pair<Integer,
	/// String>`): its generic type's name and its type arguments.
	pub fn record_application(&mut self, ty: TypeId, name: &str, arguments: &[TypeId]) {
		if !matches!(self.kind(ty), TypeKind::Object(_)) {
			return;
		}
		let name = self.syms.intern(name);
		let arguments = self.list(arguments);
		self.applications.entry(ty).or_insert((name, arguments));
	}

	/// The generic type's name and the type arguments of an instance of a
	/// named generic object type, if `ty` is one.
	pub fn application(&self, ty: TypeId) -> Option<(Sym, ListId)> {
		if let Some(&found) = self.applications.get(&ty) {
			return Some(found);
		}
		if is_local(ty.0) {
			return None;
		}
		let late = self.global_late(&ty);
		let applications = late.applications.get(&ty)?;
		applications
			.iter()
			.filter(|(module, _, _)| {
				self.visible
					.as_ref()
					.is_none_or(|visible| visible.contains(module))
			})
			.min_by(|(a, _, _), (b, _, _)| self.module_path(*a).cmp(self.module_path(*b)))
			.map(|&(_, name, arguments)| (name, arguments))
	}

	/// The declared name of an object type, if any: the one of this view,
	/// else the first among the modules the view may read.
	pub fn type_name(&self, ty: TypeId) -> Option<&str> {
		if let Some(&name) = self.names.get(&ty) {
			return Some(self.syms.resolve(name));
		}
		if is_local(ty.0) {
			return None;
		}
		let late = self.global_late(&ty);
		let names = late.names.get(&ty)?;
		let (_, _, name) = names
			.iter()
			.filter(|(module, _, _)| {
				self.visible
					.as_ref()
					.is_none_or(|visible| visible.contains(module))
			})
			.min_by(|(a, a_offset, _), (b, b_offset, _)| {
				(self.module_path(*a), a_offset).cmp(&(self.module_path(*b), b_offset))
			})?;
		// global symbols live as long as the store
		Some(self.syms.resolve(*name))
	}

	/// Whether the data of the members of a global nominal type are set.
	pub(crate) fn global_has_payloads(&self, nominal: NominalId) -> bool {
		self.global_late(&nominal).payloads.contains_key(&nominal)
	}

	/// Whether the data of the members of a nominal type are set.
	pub fn has_payloads(&self, nominal: NominalId) -> bool {
		self.payloads.contains_key(&nominal)
			|| (!is_local(nominal.0) && self.global_late(&nominal).payloads.contains_key(&nominal))
	}

	/// Records a generic nominal type: its declaration (with its parameters
	/// as arguments) and its parameters.
	pub fn set_generic_nominal(&mut self, key: DeclKey, nominal: NominalId, params: Vec<ParamId>) {
		self.generic_nominals.insert(key, (nominal, params));
	}

	/// The declaration of a generic nominal type and its parameters.
	pub fn generic_nominal(&self, key: DeclKey) -> Option<(NominalId, Vec<ParamId>)> {
		if let Some(found) = self.generic_nominals.get(&key) {
			return Some(found.clone());
		}
		if is_local(key.0) {
			return None;
		}
		self.global_late(&key).generic_nominals.get(&key).cloned()
	}

	// ── recursive types (types brief §3.6) ───────────────────────────────

	/// The reference to the recursive declaration `key` with these type
	/// arguments: one `TypeKind::Rec` per declaration and arguments. Its
	/// flags are the coinductive ones (comparable unless a non-recursive part
	/// isn't) until [`TypeStore::set_rec_target`].
	pub fn recursive_reference(
		&mut self,
		key: DeclKey,
		name: &str,
		args: &[TypeId],
		object: bool,
	) -> TypeId {
		let args = self.list(args);
		if let Some(&ty) = self.rec_by_args.get(&(key, args)) {
			return ty;
		}
		if !is_local(key.0)
			&& !is_local(args.0)
			&& let Some(&ty) = self.global_late(&(key, args)).rec_by_args.get(&(key, args))
		{
			return ty;
		}
		let name = self.syms.intern(name);
		let rec = RecId(self.recs.push(RecInfo {
			key,
			name,
			args,
			target: TypeId::UNSET,
			object,
		}));
		let ty = self.intern(TypeKind::Rec(rec));
		self.rec_by_args.insert((key, args), ty);
		// an instance of a generic declaration already built: its target now
		if let Some((generic, params)) = self.rec_params(key)
			&& generic != ty
			&& let Some(target) = self.rec_target(generic)
		{
			let bindings = Bindings {
				params,
				types: self.list_items(args).to_vec(),
				..Default::default()
			};
			let target = self.substitute(target, &bindings);
			self.set_rec_target(ty, target);
		}
		ty
	}

	/// The parameters of a generic recursive declaration.
	fn rec_params(&self, key: DeclKey) -> Option<(TypeId, Vec<ParamId>)> {
		if let Some(found) = self.rec_params.get(&key) {
			return Some(found.clone());
		}
		if is_local(key.0) {
			return None;
		}
		self.global_late(&key).rec_params.get(&key).cloned()
	}

	/// Records the parameters of a generic recursive declaration (its own
	/// reference `generic`, whose arguments are the parameters).
	pub fn set_rec_params(&mut self, key: DeclKey, generic: TypeId, params: Vec<ParamId>) {
		self.rec_params.insert(key, (generic, params));
	}

	/// Sets the type a recursive reference stands for, once its declaration
	/// is built; its flags become those of the target. A global reference
	/// has its target already.
	pub fn set_rec_target(&mut self, ty: TypeId, target: TypeId) {
		let TypeKind::Rec(rec) = self.kind(ty) else {
			return;
		};
		let Some(info) = self.recs.local_mut(rec.0) else {
			return;
		};
		info.target = target;
		let target_flags = self.flags(target);
		if let Some(entry) = self.types.local_mut(ty.0) {
			let kept = entry.flags & (TypeFlags::HAS_PARAM | TypeFlags::HAS_UNKNOWN);
			entry.flags = target_flags | kept | TypeFlags::RECURSIVE;
		}
	}

	/// A recursive reference.
	pub fn rec_info(&self, rec: RecId) -> RecInfo {
		*self.recs.get(rec.0)
	}

	/// The type a recursive reference stands for (`None` when `ty` is not
	/// one, or while its declaration is being built).
	pub fn rec_target(&self, ty: TypeId) -> Option<TypeId> {
		match self.kind(ty) {
			TypeKind::Rec(rec) => {
				let target = self.recs.get(rec.0).target;
				(!target.is_unset()).then_some(target)
			}
			_ => None,
		}
	}

	/// `ty`, with a recursive reference replaced by the type it stands for.
	pub fn unfold(&self, ty: TypeId) -> TypeId {
		self.rec_target(ty).unwrap_or(ty)
	}

	/// Sets the constraint and the default of a type parameter, once they are
	/// resolved (they may name earlier parameters). A global parameter has
	/// its bounds already.
	pub fn set_param_bounds(&mut self, param: ParamId, constraint: TypeId, default: TypeId) {
		let Some(info) = self.params.local_mut(param.0) else {
			return;
		};
		info.constraint = constraint;
		info.default = default;
		self.pending_bounds.remove(&param);
	}

	/// The data of a member (`UNSET`: none, or not known yet).
	pub fn payload(&self, nominal: NominalId, index: u16) -> TypeId {
		if let Some(payloads) = self.payloads.get(&nominal) {
			return payloads
				.get(index as usize)
				.copied()
				.unwrap_or(TypeId::UNSET);
		}
		if is_local(nominal.0) {
			return TypeId::UNSET;
		}
		self.global_late(&nominal)
			.payloads
			.get(&nominal)
			.and_then(|payloads| payloads.get(index as usize))
			.copied()
			.unwrap_or(TypeId::UNSET)
	}

	/// The index of the member named `name` of a nominal type.
	pub fn member_index(&self, nominal: NominalId, name: &str) -> Option<u16> {
		let members = self.sym_list(self.nominal(nominal).members);
		members
			.iter()
			.position(|&member| self.syms.resolve(member) == name)
			.map(|index| index as u16)
	}

	/// The name of a member of a nominal type.
	pub fn member_name(&self, nominal: NominalId, index: u16) -> &str {
		let members = self.sym_list(self.nominal(nominal).members);
		members
			.get(index as usize)
			.map_or("", |&member| self.syms.resolve(member))
	}

	/// Interns a module path.
	pub fn module(&mut self, path: &str) -> ModuleId {
		let sym = self.syms.intern(path);
		ModuleId(self.modules.intern(sym))
	}

	/// The id of a module path, if it is interned already.
	pub fn existing_module(&self, path: &str) -> Option<ModuleId> {
		let sym = self.syms.get(path)?;
		self.modules.get(&sym).map(ModuleId)
	}

	/// The path of a module.
	pub fn module_path(&self, module: ModuleId) -> &str {
		self.syms.resolve(*self.modules.resolve(module.0))
	}

	/// Interns a declaration key: the module, the names of the enclosing
	/// declarations then the declaration's own name, a disambiguator.
	pub fn decl_key(&mut self, module: ModuleId, names: &[&str], disambiguator: u16) -> DeclKey {
		let syms: smallvec::SmallVec<[Sym; 4]> =
			names.iter().map(|name| self.syms.intern(name)).collect();
		let names = self.intern_sym_list(&syms);
		DeclKey(self.decl_keys.intern(DeclKeyData {
			module,
			names,
			disambiguator,
		}))
	}

	/// Interns a declaration key from its interned names.
	pub fn decl_key_of(
		&mut self,
		module: ModuleId,
		names: crate::ids::SymListId,
		disambiguator: u16,
	) -> DeclKey {
		DeclKey(self.decl_keys.intern(DeclKeyData {
			module,
			names,
			disambiguator,
		}))
	}

	/// The parts of a declaration key.
	pub fn decl_key_data(&self, key: DeclKey) -> DeclKeyData {
		*self.decl_keys.resolve(key.0)
	}

	/// The name of a declaration (the last of its names).
	pub fn decl_name(&self, key: DeclKey) -> &str {
		let names = self.sym_list(self.decl_key_data(key).names);
		names.last().map_or("", |&sym| self.syms.resolve(sym))
	}

	/// Declares a type parameter.
	pub fn add_param(&mut self, info: ParamInfo) -> ParamId {
		ParamId(self.params.push(info))
	}

	/// Declares a type parameter whose bounds are set later
	/// ([`TypeStore::set_param_bounds`]): until then, a relation reading
	/// them is not cached (P9).
	pub fn add_param_pending(&mut self, info: ParamInfo) -> ParamId {
		let param = self.add_param(info);
		self.pending_bounds.insert(param);
		param
	}

	/// Notes the read of an incomplete type (P9): the relation being
	/// computed is not cached.
	pub(crate) fn note_incomplete(&self) {
		self.incomplete_reads
			.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
	}

	/// The reads of incomplete types so far.
	pub(crate) fn incomplete_count(&self) -> u32 {
		self.incomplete_reads
			.load(std::sync::atomic::Ordering::Relaxed)
	}

	/// The constraint of a type parameter, noting a read of bounds not set
	/// yet (P9).
	pub(crate) fn param_constraint(&self, param: ParamId) -> TypeId {
		if self.pending_bounds.contains(&param) {
			self.note_incomplete();
		}
		self.param(param).constraint
	}

	/// Whether the data of a nominal's members are set, noting the read of
	/// data not set yet (P9).
	pub(crate) fn note_payloads(&self, nominal: NominalId) {
		let set = self.payloads.contains_key(&nominal)
			|| (!is_local(nominal.0) && self.global_late(&nominal).payloads.contains_key(&nominal));
		if !set {
			self.note_incomplete();
		}
	}

	/// A type parameter.
	pub fn param(&self, param: ParamId) -> ParamInfo {
		*self.params.get(param.0)
	}

	/// Interns a spelling.
	pub fn spelling(&mut self, spelling: Spelling) -> SpellingId {
		SpellingId(self.spellings.intern(spelling))
	}

	/// A spelling.
	pub fn spelling_data(&self, spelling: SpellingId) -> Spelling {
		*self.spellings.resolve(spelling.0)
	}

	// ── literals ─────────────────────────────────────────────────────────

	/// Whether a type is a literal type (integer, float, big integer, string, boolean).
	pub fn is_literal(&self, ty: TypeId) -> bool {
		match self.kind(ty) {
			TypeKind::Int(id) => self.int(id).literal().is_some(),
			TypeKind::BoolLit(_)
			| TypeKind::FloatLit(..)
			| TypeKind::BigIntLit(_)
			| TypeKind::StringLit(_) => true,
			_ => false,
		}
	}

	/// The widened type of a literal (values brief §2.1): `42` → `Integer`,
	/// `"a"` → `String`, `true` → `Boolean`, `1.5` → `Float`; an integer
	/// literal of a sized storage widens to that storage (`Int8`). Other types,
	/// ranged integers included (`Integer(min = 0)`), are returned unchanged:
	/// a non-literal initializer keeps its type (values brief §2.1).
	pub fn widen(&self, ty: TypeId) -> TypeId {
		match self.kind(ty) {
			TypeKind::Int(id) if self.int(id).literal().is_some() => {
				TypeId::of_storage(self.int(id).storage)
			}
			TypeKind::BoolLit(_) => TypeId::BOOLEAN,
			TypeKind::FloatLit(FloatBits::F64, _) => TypeId::FLOAT,
			TypeKind::FloatLit(FloatBits::F32, _) => TypeId::FLOAT32,
			TypeKind::BigIntLit(_) => TypeId::BIG_INT,
			TypeKind::StringLit(_) => TypeId::STRING,
			_ => ty,
		}
	}

	// ── flags and stable hashes ──────────────────────────────────────────

	fn compute_flags(&self, kind: TypeKind) -> TypeFlags {
		use TypeFlags as F;
		let scalar = F::FLAT | F::TRIVIALLY_COPYABLE | F::COMPARABLE;
		let ordered_scalar = scalar | F::ORDERED;
		match kind {
			TypeKind::Never | TypeKind::Void => F::FLAT | F::TRIVIALLY_COPYABLE,
			TypeKind::Unknown => F::HAS_UNKNOWN | scalar,
			TypeKind::None => scalar.with_runtime_kind(RuntimeKind::None),
			TypeKind::ErrorTop => (F::COMPARABLE).with_runtime_kind(RuntimeKind::Object),
			TypeKind::Boolean | TypeKind::BoolLit(_) => {
				scalar.with_runtime_kind(RuntimeKind::Boolean)
			}
			TypeKind::Int(id) => {
				let storage = self.int(id).storage;
				let (js, kind) = if storage.is_js_bigint() {
					(F::JS_BIGINT, RuntimeKind::BigInt)
				} else {
					(F::empty(), RuntimeKind::Number)
				};
				(ordered_scalar | js).with_runtime_kind(kind)
			}
			TypeKind::SymInt(_) => {
				(ordered_scalar | F::HAS_PARAM).with_runtime_kind(RuntimeKind::Number)
			}
			TypeKind::Float(_) | TypeKind::FloatLit(..) | TypeKind::Percentage => {
				ordered_scalar.with_runtime_kind(RuntimeKind::Number)
			}
			TypeKind::BigInt | TypeKind::BigIntLit(_) => {
				(F::FLAT | F::COMPARABLE | F::ORDERED | F::JS_BIGINT)
					.with_runtime_kind(RuntimeKind::BigInt)
			}
			TypeKind::String | TypeKind::StringLit(_) | TypeKind::Character => {
				(F::FLAT | F::COMPARABLE | F::ORDERED).with_runtime_kind(RuntimeKind::String)
			}
			TypeKind::Range => scalar.with_runtime_kind(RuntimeKind::Range),
			TypeKind::Subscription => F::MOVE_ONLY.with_runtime_kind(RuntimeKind::Object),
			TypeKind::Sync(kind, item) => {
				let inherited = self.flags(item) & (F::HAS_PARAM | F::HAS_UNKNOWN);
				match kind {
					crate::types::SyncKind::Ordering => {
						scalar.with_runtime_kind(RuntimeKind::Number)
					}
					crate::types::SyncKind::Guard
					| crate::types::SyncKind::Worker
					| crate::types::SyncKind::TcpListener
					| crate::types::SyncKind::TcpStream
					| crate::types::SyncKind::WorkerPool => {
						(inherited | F::MOVE_ONLY).with_runtime_kind(RuntimeKind::Object)
					}
					_ => inherited.with_runtime_kind(RuntimeKind::Object),
				}
			}
			TypeKind::Enum(..) | TypeKind::EnumMember(..) => {
				scalar.with_runtime_kind(RuntimeKind::Number)
			}
			TypeKind::NamedUnion(..) | TypeKind::Member(..) => {
				F::COMPARABLE.with_runtime_kind(RuntimeKind::Object)
			}
			// a JavaScript number up to 32 flags, a `bigint` above (D14)
			TypeKind::Fields(_) if self.fields_width_of_kind(kind) > 32 => {
				(scalar | F::JS_BIGINT).with_runtime_kind(RuntimeKind::BigInt)
			}
			TypeKind::Fields(_) => scalar.with_runtime_kind(RuntimeKind::Number),
			TypeKind::CStruct(_) => {
				(F::TRIVIALLY_COPYABLE | F::COMPARABLE).with_runtime_kind(RuntimeKind::Object)
			}
			TypeKind::Pointer(target) => {
				(scalar | F::HAS_FOREIGN | (self.flags(target) & (F::HAS_PARAM | F::HAS_UNKNOWN)))
					.with_runtime_kind(RuntimeKind::Other)
			}
			TypeKind::CString => (scalar | F::HAS_FOREIGN).with_runtime_kind(RuntimeKind::Other),
			TypeKind::CArray(item, _) => self
				.aggregate(
					std::iter::once(item),
					F::FLAT | F::TRIVIALLY_COPYABLE | F::COMPARABLE,
				)
				.with_runtime_kind(RuntimeKind::Array),
			TypeKind::Object(shape) => {
				let fields = self.shape_fields(shape).iter().map(|field| field.ty);
				let resource = self
					.shape(shape)
					.nominal
					.is_some_and(|nominal| self.nominal(nominal).kind == NominalKind::Resource);
				let mut flags = self
					.aggregate(fields, F::FLAT | F::TRIVIALLY_COPYABLE | F::COMPARABLE)
					.with_runtime_kind(RuntimeKind::Object);
				if let Some(nominal) = self.shape(shape).nominal {
					// Phantom parameters affect substitution, but don't own a value.
					flags |= self.aggregate(
						self.list_items(self.nominal(nominal).args).iter().copied(),
						F::empty(),
					) & (F::HAS_PARAM | F::HAS_UNKNOWN);
				}
				// a pointer in a private field stays inside its foreign file (D29)
				if flags.contains(F::HAS_FOREIGN)
					&& !self.shape_fields(shape).iter().any(|field| {
						!field.flags.contains(FieldFlags::PRIVATE)
							&& self.flags(field.ty).contains(F::HAS_FOREIGN)
					}) {
					flags -= F::HAS_FOREIGN;
				}
				if resource {
					(flags | F::MOVE_ONLY) - F::TRIVIALLY_COPYABLE - F::COMPARABLE
				} else {
					flags
				}
			}
			// tuples have no order (`values/tuples.md`)
			TypeKind::Tuple(list) => {
				let items = self.list_items(list).iter().copied();
				self.aggregate(items, F::FLAT | F::TRIVIALLY_COPYABLE | F::COMPARABLE)
					.with_runtime_kind(RuntimeKind::Array)
			}
			TypeKind::Union(list) => {
				let members = self.list_items(list).iter().copied();
				let flags = self.aggregate(
					members,
					F::FLAT | F::TRIVIALLY_COPYABLE | F::COMPARABLE | F::ORDERED,
				);
				// a union's own runtime kind is that of its members when they agree
				let mut kinds = self
					.list_items(list)
					.iter()
					.map(|&member| self.runtime_kind(member));
				let first = kinds.next().unwrap_or(RuntimeKind::Other);
				let kind = if kinds.all(|kind| kind == first) {
					first
				} else {
					RuntimeKind::Other
				};
				(flags - F::HAS_VOID).with_runtime_kind(kind)
			}
			TypeKind::Collection(collection, item, key) => {
				let inherited = self.aggregate([item, key].into_iter(), F::COMPARABLE);
				(inherited - F::FLAT - F::TRIVIALLY_COPYABLE)
					.with_runtime_kind(collection.runtime_kind())
			}
			TypeKind::Function(sig) => {
				let sig = self.sig(sig);
				let mut flags = F::FLAT.with_runtime_kind(RuntimeKind::Function);
				let types = self
					.params(sig.params)
					.iter()
					.map(|param| param.ty)
					.chain([sig.ret, sig.throws]);
				for ty in types {
					flags |= self.flags(ty) & (F::HAS_PARAM | F::HAS_UNKNOWN | F::HAS_FOREIGN);
				}
				flags
			}
			TypeKind::Iterator(item, _) => self.flags(item) & (F::HAS_PARAM | F::HAS_UNKNOWN),
			TypeKind::Ref(target, _) => {
				(self.flags(target)
					& (F::HAS_PARAM | F::HAS_UNKNOWN | F::COMPARABLE | F::ORDERED | F::KIND))
					| F::TRIVIALLY_COPYABLE
			}
			TypeKind::Task(result, throws) => {
				let inherited =
					(self.flags(result) | self.flags(throws)) & (F::HAS_PARAM | F::HAS_UNKNOWN);
				(inherited | F::MOVE_ONLY).with_runtime_kind(RuntimeKind::Task)
			}
			TypeKind::Param(_) | TypeKind::ImplicitParam(..) | TypeKind::Const(_) => F::HAS_PARAM,
			TypeKind::Rec(rec) => {
				let info = *self.recs.get(rec.0);
				let mut flags = F::RECURSIVE | F::COMPARABLE;
				for &arg in self.list_items(info.args) {
					flags |= self.flags(arg) & (F::HAS_PARAM | F::HAS_UNKNOWN);
				}
				if info.object {
					flags = flags.with_runtime_kind(RuntimeKind::Object);
				}
				flags
			}
		}
	}

	/// Flags of an aggregate: `all` flags kept only when every child has them,
	/// containment flags (`HAS_*`, `MOVE_ONLY`, `RECURSIVE`, `JS_BIGINT`… no) inherited.
	fn aggregate(&self, children: impl Iterator<Item = TypeId>, all: TypeFlags) -> TypeFlags {
		use TypeFlags as F;
		let mut flags = all;
		for child in children {
			let child_flags = self.flags(child);
			flags &= child_flags | !all;
			flags |= child_flags
				& (F::HAS_PARAM
					| F::HAS_UNKNOWN
					| F::MOVE_ONLY | F::HAS_VOID
					| F::RECURSIVE | F::HAS_FOREIGN);
			if child == TypeId::VOID {
				flags |= F::HAS_VOID;
			}
		}
		flags
	}

	fn compute_stable_hash(&self, kind: TypeKind) -> u128 {
		let mut hasher = StableHasher::new();
		match kind {
			TypeKind::Never => hasher.tag(0),
			TypeKind::Void => hasher.tag(1),
			TypeKind::None => hasher.tag(2),
			TypeKind::ErrorTop => hasher.tag(3),
			TypeKind::Unknown => hasher.tag(4),
			TypeKind::Boolean => hasher.tag(5),
			TypeKind::BoolLit(value) => hasher.tag(6).byte(value as u8),
			TypeKind::Int(id) => {
				let int = self.int(id);
				hasher
					.tag(7)
					.bytes(&int.min.to_le_bytes())
					.bytes(&int.max.to_le_bytes())
					.byte(int.storage as u8)
			}
			TypeKind::SymInt(id) => hasher.tag(8).u32(id.0),
			TypeKind::Float(bits) => hasher.tag(9).byte(bits as u8),
			TypeKind::FloatLit(bits, value) => {
				hasher.tag(10).byte(bits as u8).hash(self.const_hash(value))
			}
			TypeKind::BigInt => hasher.tag(11),
			TypeKind::BigIntLit(value) => hasher.tag(12).hash(self.const_hash(value)),
			TypeKind::Percentage => hasher.tag(13),
			TypeKind::String => hasher.tag(14),
			TypeKind::StringLit(sym) => hasher.tag(15).str(self.syms.resolve(sym)),
			TypeKind::Character => hasher.tag(16),
			TypeKind::Object(shape) => {
				let data = self.shape(shape);
				hasher.tag(17);
				for field in self.shape_fields(shape) {
					hasher
						.str(self.syms.resolve(field.name))
						.hash(self.stable_hash(field.ty))
						.byte(field.flags.bits());
				}
				if let Some(nominal) = data.nominal {
					hasher.hash(self.nominal_hash(nominal));
				}
				&mut hasher
			}
			TypeKind::Tuple(list) => hasher.tag(18).hash(self.list_hash(list)),
			TypeKind::Union(list) => {
				// members are sorted by id, which is session-dependent: combine order-independently
				let mut hashes: smallvec::SmallVec<[u128; 8]> = self
					.list_items(list)
					.iter()
					.map(|&member| self.stable_hash(member))
					.collect();
				hashes.sort_unstable();
				hasher.tag(19);
				for hash in hashes {
					hasher.hash(hash);
				}
				&mut hasher
			}
			TypeKind::NamedUnion(nominal, mask) | TypeKind::Enum(nominal, mask) => {
				hasher.tag(20).hash(self.nominal_hash(nominal));
				for &word in self.mask_bits(mask) {
					hasher.u64(word);
				}
				&mut hasher
			}
			TypeKind::Member(nominal, index) | TypeKind::EnumMember(nominal, index) => hasher
				.tag(21)
				.hash(self.nominal_hash(nominal))
				.u32(index as u32),
			TypeKind::Fields(list) => hasher.tag(22).hash(self.list_hash(list)),
			TypeKind::CStruct(nominal) => hasher.tag(23).hash(self.nominal_hash(nominal)),
			TypeKind::Function(sig) => {
				let sig = self.sig(sig);
				hasher.tag(24);
				for param in self.params(sig.params) {
					hasher
						.hash(self.stable_hash(param.ty))
						.byte(param.variadic as u8);
				}
				hasher
					.hash(self.stable_hash(sig.ret))
					.hash(self.stable_hash(sig.throws));
				for requirement in self.requirements(sig.requires) {
					hasher
						.str(self.syms.resolve(requirement.name))
						.hash(self.stable_hash(requirement.ty));
				}
				hasher
					.u32(sig.stored)
					.byte(sig.is_async as u8)
					.hash(self.effects_hash(sig.effects))
			}
			TypeKind::Collection(collection, item, key) => hasher
				.tag(25)
				.u32(collection.stable_index())
				.hash(self.stable_hash(item))
				.hash(self.stable_hash(key)),
			TypeKind::Iterator(item, path) => hasher
				.tag(26)
				.hash(self.stable_hash(item))
				.hash(self.path_hash(path)),
			TypeKind::Ref(target, path) => hasher
				.tag(27)
				.hash(self.stable_hash(target))
				.hash(self.path_hash(path)),
			TypeKind::Range => hasher.tag(28),
			TypeKind::Subscription => hasher.tag(34),
			TypeKind::Pointer(target) => hasher.tag(35).hash(self.stable_hash(target)),
			TypeKind::CString => hasher.tag(36),
			TypeKind::CArray(item, count) => hasher.tag(37).hash(self.stable_hash(item)).u32(count),
			TypeKind::Sync(kind, item) => hasher
				.tag(38)
				.u32(kind.stable_index())
				.hash(self.stable_hash(item)),
			TypeKind::Task(result, throws) => hasher
				.tag(29)
				.hash(self.stable_hash(result))
				.hash(self.stable_hash(throws)),
			TypeKind::Param(param) => {
				let info = self.param(param);
				hasher
					.tag(30)
					.hash(self.decl_hash(info.owner))
					.u32(info.index as u32)
			}
			TypeKind::ImplicitParam(owner, index) => {
				hasher.tag(31).hash(self.decl_hash(owner)).u32(index as u32)
			}
			TypeKind::Const(value) => hasher.tag(32).hash(self.const_hash(value)),
			TypeKind::Rec(rec) => {
				let info = *self.recs.get(rec.0);
				hasher
					.tag(33)
					.hash(self.decl_hash(info.key))
					.hash(self.list_hash(info.args))
			}
		};
		hasher.finish()
	}

	fn list_hash(&self, list: ListId) -> u128 {
		let mut hasher = StableHasher::new();
		for &item in self.list_items(list) {
			hasher.hash(self.stable_hash(item));
		}
		hasher.finish()
	}

	/// The stable hash of a declaration key (from its names, never from ids).
	pub fn decl_hash(&self, key: DeclKey) -> u128 {
		let data = self.decl_key_data(key);
		let mut hasher = StableHasher::new();
		hasher.str(self.module_path(data.module));
		for &name in self.sym_list(data.names) {
			hasher.str(self.syms.resolve(name));
		}
		hasher.u32(data.disambiguator as u32);
		hasher.finish()
	}

	fn nominal_hash(&self, nominal: NominalId) -> u128 {
		let info = self.nominal(nominal);
		let mut hasher = StableHasher::new();
		hasher
			.hash(self.decl_hash(info.key))
			.hash(self.list_hash(info.args));
		hasher.finish()
	}

	/// The stable hash of an effect path (from names, never from ids).
	fn path_hash(&self, path: PathId) -> u128 {
		let (data, steps) = self.path_data(path);
		let mut hasher = StableHasher::new();
		match data.root {
			crate::types::PathRoot::Param(index) => hasher.tag(0).u32(index as u32),
			crate::types::PathRoot::SelfParam => hasher.tag(1),
			crate::types::PathRoot::ModuleValue(key) => hasher.tag(2).hash(self.decl_hash(key)),
			crate::types::PathRoot::Requirement(index) => hasher.tag(3).u32(index as u32),
		};
		for &step in steps {
			match step {
				PathStep::Field(name) => hasher.tag(10).str(self.syms.resolve(name)),
				PathStep::Private(name) => hasher.tag(11).str(self.syms.resolve(name)),
				PathStep::IndexParam(index) => hasher.tag(12).u32(index as u32),
				PathStep::IndexConst(value) => hasher.tag(13).hash(self.const_hash(value)),
				PathStep::IndexFrom(index) => hasher.tag(14).u32(index as u32),
				PathStep::Last => hasher.tag(15),
				PathStep::All => hasher.tag(16),
				PathStep::Length => hasher.tag(17),
				PathStep::Size => hasher.tag(18),
				PathStep::Capacity => hasher.tag(19),
			};
		}
		hasher.finish()
	}

	/// The stable hash of an effect set.
	fn effects_hash(&self, set: EffectSetId) -> u128 {
		let (mutates, invalidates) = self.effects(set);
		let mut hasher = StableHasher::new();
		hasher.tag(0);
		for &path in mutates {
			hasher.hash(self.path_hash(path));
		}
		hasher.tag(1);
		for &path in invalidates {
			hasher.hash(self.path_hash(path));
		}
		hasher.finish()
	}

	fn const_hash(&self, value: ConstId) -> u128 {
		let mut hasher = StableHasher::new();
		match self.const_value(value) {
			ConstValue::Bool(value) => hasher.tag(0).byte(value as u8),
			ConstValue::Int(value) => hasher.tag(1).bytes(&value.to_le_bytes()),
			ConstValue::Float(bits) => hasher.tag(2).u64(bits),
			ConstValue::String(sym) => hasher.tag(3).str(self.syms.resolve(sym)),
			ConstValue::Character(sym) => hasher.tag(4).str(self.syms.resolve(sym)),
			ConstValue::BigInt(sym) => hasher.tag(5).str(self.syms.resolve(sym)),
			ConstValue::Binding(index) => hasher.tag(7).u32(index),
			ConstValue::EnumMember(nominal, index) => hasher
				.tag(6)
				.hash(self.nominal_hash(nominal))
				.u32(index as u32),
		};
		hasher.finish()
	}
}

/// A 128-bit FNV-1a hasher: deterministic across runs, platforms and versions.
pub(crate) struct StableHasher(u128);

impl StableHasher {
	const OFFSET: u128 = 0x6c62272e07bb014262b821756295c58d;
	const PRIME: u128 = 0x0000000001000000000000000000013B;

	pub(crate) fn new() -> Self {
		Self(Self::OFFSET)
	}

	fn byte(&mut self, byte: u8) -> &mut Self {
		self.0 = (self.0 ^ byte as u128).wrapping_mul(Self::PRIME);
		self
	}

	fn bytes(&mut self, bytes: &[u8]) -> &mut Self {
		// 16 bytes at a time: one multiplication per word (an `I256` is two)
		let (words, rest) = bytes.as_chunks::<16>();
		for word in words {
			self.0 = (self.0 ^ u128::from_le_bytes(*word)).wrapping_mul(Self::PRIME);
		}
		for &byte in rest {
			self.byte(byte);
		}
		self
	}

	fn tag(&mut self, tag: u8) -> &mut Self {
		self.byte(tag)
	}

	fn u32(&mut self, value: u32) -> &mut Self {
		self.bytes(&value.to_le_bytes())
	}

	fn u64(&mut self, value: u64) -> &mut Self {
		self.bytes(&value.to_le_bytes())
	}

	pub(crate) fn hash(&mut self, value: u128) -> &mut Self {
		self.bytes(&value.to_le_bytes())
	}

	fn str(&mut self, text: &str) -> &mut Self {
		self.u32(text.len() as u32).bytes(text.as_bytes())
	}

	pub(crate) fn finish(&self) -> u128 {
		self.0
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::types::NominalKind;

	fn field(store: &mut TypeStore, name: &str, ty: TypeId) -> FieldEntry {
		FieldEntry {
			name: store.syms.intern(name),
			ty,
			flags: FieldFlags::empty(),
		}
	}

	#[test]
	fn predefined_ids_are_fixed() {
		let store = TypeStore::new();
		assert_eq!(store.kind(TypeId::NEVER), TypeKind::Never);
		assert_eq!(store.kind(TypeId::STRING), TypeKind::String);
		assert_eq!(store.kind(TypeId::RANGE), TypeKind::Range);
		assert_eq!(store.as_int(TypeId::INT8).unwrap().storage, IntStorage::I8);
		assert_eq!(store.as_int(TypeId::BYTE).unwrap().storage, IntStorage::U8);
		assert_eq!(
			store.interval_of(TypeId::INTEGER),
			store.interval_of(TypeId::INT64)
		);
		assert_ne!(TypeId::INTEGER, TypeId::INT64);
		assert_eq!(store.len(), TypeId::RANGE.index() + 1);
	}

	#[test]
	fn structurally_equal_types_have_the_same_id() {
		let mut store = TypeStore::new();
		let a = store.int_range(I256::ZERO, I256::from(10), IntStorage::Default);
		let b = store.int_range(I256::ZERO, I256::from(10), IntStorage::Default);
		let c = store.int_range(I256::ZERO, I256::from(10), IntStorage::I8);
		assert_eq!(a, b);
		assert_ne!(a, c);
		assert_eq!(
			store.int_type(IntType::full(IntStorage::U16)),
			TypeId::UINT16
		);
		let t1 = store.tuple(&[TypeId::INTEGER, TypeId::STRING]);
		let t2 = store.tuple(&[TypeId::INTEGER, TypeId::STRING]);
		let t3 = store.tuple(&[TypeId::STRING, TypeId::INTEGER]);
		assert_eq!(t1, t2);
		assert_ne!(t1, t3);
		assert_eq!(store.string_literal("hi"), store.string_literal("hi"));
		assert_eq!(
			store.float_literal(FloatBits::F64, 1.5),
			store.float_literal(FloatBits::F64, 1.5)
		);
		assert_ne!(
			store.float_literal(FloatBits::F64, 0.1),
			store.float_literal(FloatBits::F32, 0.1)
		);
	}

	#[test]
	fn objects_are_interned_regardless_of_field_order() {
		let mut store = TypeStore::new();
		let x = field(&mut store, "x", TypeId::INTEGER);
		let y = field(&mut store, "y", TypeId::STRING);
		let a = store.object(&[x, y], None);
		let b = store.object(&[y, x], None);
		assert_eq!(a, b);
		let names: Vec<_> = store
			.fields_of(a)
			.iter()
			.map(|f| store.syms.resolve(f.name).to_string())
			.collect();
		assert_eq!(names, ["x", "y"]);
	}

	#[test]
	fn nominal_objects_substitute_and_infer_phantom_arguments() {
		let mut store = TypeStore::new();
		let module = store.module("main");
		for kind in [
			NominalKind::Private,
			NominalKind::Resource,
			NominalKind::CStruct,
		] {
			let key = store.decl_key(module, &[&format!("{kind:?}")], 0);
			let name = store.syms.intern("Item");
			let param = store.add_param(ParamInfo {
				owner: key,
				index: 0,
				name,
				constraint: TypeId::UNKNOWN,
				default: TypeId::UNSET,
				value: false,
			});
			let parameter = store.intern(TypeKind::Param(param));
			let args = store.list(&[parameter]);
			let members = store.intern_sym_list(&[]);
			let nominal = store.intern_nominal(NominalInfo {
				key,
				args,
				kind,
				members,
			});
			let z = field(&mut store, "z", TypeId::INT32);
			let a = field(&mut store, "a", TypeId::INT32);
			let object = if kind == NominalKind::CStruct {
				store.object_in_order(&[z, a], nominal)
			} else {
				store.object(&[z, a], Some(nominal))
			};
			let array = store.array(object);
			assert!(store.flags(array).contains(TypeFlags::HAS_PARAM));
			let task = store.intern(TypeKind::Task(TypeId::VOID, TypeId::NEVER));
			let bindings = crate::types::Bindings {
				params: vec![param],
				types: vec![task],
				..Default::default()
			};
			let concrete = store.substitute(object, &bindings);
			assert_ne!(concrete, object);
			assert!(!store.flags(concrete).contains(TypeFlags::HAS_PARAM));
			assert_eq!(
				store.flags(concrete).contains(TypeFlags::MOVE_ONLY),
				kind == NominalKind::Resource
			);
			let TypeKind::Object(shape) = store.kind(concrete) else {
				panic!("an object instance");
			};
			let nominal = store.shape(shape).nominal.unwrap();
			assert_eq!(store.list_items(store.nominal(nominal).args), &[task]);
			if kind == NominalKind::CStruct {
				assert_eq!(
					store
						.shape_fields(shape)
						.iter()
						.map(|field| field.name)
						.collect::<Vec<_>>(),
					[z.name, a.name]
				);
			}
			let mut candidates = crate::types::Candidates::default();
			store.unify(
				object,
				concrete,
				&crate::types::Bindings::new(&[param]),
				&mut candidates,
			);
			assert_eq!(candidates.found, [vec![task]]);
		}
	}

	#[test]
	fn functions_and_collections_intern() {
		let mut store = TypeStore::new();
		let sig = store.simple_sig(&[TypeId::INTEGER], TypeId::STRING, TypeId::NEVER);
		let f = store.function(sig);
		let sig2 = store.simple_sig(&[TypeId::INTEGER], TypeId::STRING, TypeId::NEVER);
		assert_eq!(store.function(sig2), f);
		let g = {
			let sig = store.simple_sig(&[TypeId::INTEGER], TypeId::BOOLEAN, TypeId::NEVER);
			store.function(sig)
		};
		assert_ne!(f, g);
		let array = store.array(TypeId::INTEGER);
		assert_eq!(
			store.collection(CollKind::Array, TypeId::INTEGER, TypeId::NONE),
			array
		);
		let map = store.collection(CollKind::Map, TypeId::STRING, TypeId::INTEGER);
		assert_ne!(map, array);
		assert!(!store.flags(f).contains(TypeFlags::COMPARABLE));
		assert_eq!(store.runtime_kind(f), RuntimeKind::Function);
		assert_eq!(store.runtime_kind(map), RuntimeKind::Map);
	}

	#[test]
	fn flags_are_computed_at_interning() {
		let mut store = TypeStore::new();
		let int8 = store.flags(TypeId::INT8);
		assert!(int8.contains(
			TypeFlags::FLAT
				| TypeFlags::TRIVIALLY_COPYABLE
				| TypeFlags::COMPARABLE
				| TypeFlags::ORDERED
		));
		assert!(!int8.contains(TypeFlags::JS_BIGINT));
		assert!(store.flags(TypeId::INT64).contains(TypeFlags::JS_BIGINT));
		assert!(!store.flags(TypeId::INTEGER).contains(TypeFlags::JS_BIGINT));
		assert_eq!(store.runtime_kind(TypeId::INT64), RuntimeKind::BigInt);
		assert_eq!(store.runtime_kind(TypeId::INTEGER), RuntimeKind::Number);
		assert_eq!(store.runtime_kind(TypeId::CHARACTER), RuntimeKind::String);
		assert!(
			!store
				.flags(TypeId::STRING)
				.contains(TypeFlags::TRIVIALLY_COPYABLE)
		);
		assert!(
			store
				.flags(TypeId::UNKNOWN)
				.contains(TypeFlags::HAS_UNKNOWN)
		);
		assert!(!store.flags(TypeId::BOOLEAN).contains(TypeFlags::ORDERED));

		let array = store.array(TypeId::INTEGER);
		assert!(!store.flags(array).contains(TypeFlags::FLAT));
		assert!(store.flags(array).contains(TypeFlags::COMPARABLE));
		let unknown_array = store.array(TypeId::UNKNOWN);
		assert!(store.flags(unknown_array).contains(TypeFlags::HAS_UNKNOWN));

		let name = field(&mut store, "name", TypeId::STRING);
		let score = field(&mut store, "score", TypeId::INTEGER);
		let player = store.object(&[name, score], None);
		assert!(
			store
				.flags(player)
				.contains(TypeFlags::FLAT | TypeFlags::COMPARABLE)
		);
		assert!(!store.flags(player).contains(TypeFlags::TRIVIALLY_COPYABLE));
		let point = {
			let x = field(&mut store, "x", TypeId::INTEGER);
			store.object(&[x], None)
		};
		assert!(store.flags(point).contains(TypeFlags::TRIVIALLY_COPYABLE));

		let sig = store.simple_sig(&[], TypeId::VOID, TypeId::NEVER);
		let callback = store.function(sig);
		let with_function = {
			let f = field(&mut store, "onClick", callback);
			store.object(&[f], None)
		};
		assert!(!store.flags(with_function).contains(TypeFlags::COMPARABLE));

		let void_field = {
			let f = field(&mut store, "nothing", TypeId::VOID);
			store.object(&[f], None)
		};
		assert!(store.flags(void_field).contains(TypeFlags::HAS_VOID));
		assert!(!store.flags(callback).contains(TypeFlags::HAS_VOID));

		let task = store.intern(TypeKind::Task(TypeId::INTEGER, TypeId::NEVER));
		let holder = store.array(task);
		assert!(store.flags(holder).contains(TypeFlags::MOVE_ONLY));
	}

	#[test]
	fn masks_and_nominals() {
		let mut store = TypeStore::new();
		let module = store.module("main");
		let key = store.decl_key(module, &["Shape"], 0);
		let members: Vec<Sym> = ["Circle", "Square", "Point"]
			.iter()
			.map(|name| store.syms.intern(name))
			.collect();
		let members = store.intern_sym_list(&members);
		let shape = store.intern_nominal(NominalInfo {
			key,
			args: ListId::EMPTY,
			kind: NominalKind::Union,
			members,
		});
		let all = store.nominal_type(shape);
		assert!(matches!(store.kind(all), TypeKind::NamedUnion(..)));
		let one = store.single_mask(1);
		assert_eq!(store.named_union(shape, one), store.member(shape, 1));
		let none = store.mask(&[0]);
		assert_eq!(store.named_union(shape, none), TypeId::NEVER);
		let full = store.full_mask(3);
		assert!(store.mask_subset(one, full));
		assert!(!store.mask_subset(full, one));
		assert!(store.mask_has(full, 2));
		assert!(!store.mask_has(one, 0));
		assert_eq!(store.decl_name(key), "Shape");
		let wide = store.single_mask(70);
		assert!(store.mask_has(wide, 70));
		assert_eq!(store.mask_bits(wide).len(), 2);
	}

	#[test]
	fn constants_reserve_none() {
		let mut store = TypeStore::new();
		let f = store.constant(ConstValue::Bool(false));
		assert_ne!(f, ConstId::NONE);
		assert_eq!(store.const_value(f), ConstValue::Bool(false));
		assert_eq!(store.constant(ConstValue::Bool(false)), f);
		let t = store.constant(ConstValue::Bool(true));
		assert_ne!(t, f);
		let n = store.constant(ConstValue::Int(I256::from(3)));
		assert_eq!(store.constant(ConstValue::Int(I256::from(3))), n);
	}

	#[test]
	fn stable_hashes_ignore_ids() {
		let mut a = TypeStore::new();
		let mut b = TypeStore::new();
		// intern in different orders: ids differ, stable hashes don't
		let a_tuple = a.tuple(&[TypeId::INTEGER, TypeId::STRING]);
		b.string_literal("padding");
		b.array(TypeId::BOOLEAN);
		let b_tuple = b.tuple(&[TypeId::INTEGER, TypeId::STRING]);
		assert_eq!(a.stable_hash(a_tuple), b.stable_hash(b_tuple));
		let a_lit = a.string_literal("x");
		let b_lit = b.string_literal("x");
		assert_ne!(a_lit, b_lit);
		assert_eq!(a.stable_hash(a_lit), b.stable_hash(b_lit));
		assert_ne!(a.stable_hash(TypeId::INTEGER), a.stable_hash(TypeId::INT64));
	}

	#[test]
	fn widening_literals() {
		let mut store = TypeStore::new();
		let five = store.int_literal(I256::from(5));
		assert_eq!(store.widen(five), TypeId::INTEGER);
		let small = store.int_range(I256::ZERO, I256::from(5), IntStorage::I8);
		assert_eq!(store.widen(small), small, "a ranged type is not a literal");
		let seven = store.int_range(I256::from(7), I256::from(7), IntStorage::I8);
		assert_eq!(store.widen(seven), TypeId::INT8);
		let text = store.string_literal("ready");
		assert_eq!(store.widen(text), TypeId::STRING);
		assert_eq!(store.widen(TypeId::TRUE), TypeId::BOOLEAN);
		let ratio = store.float_literal(FloatBits::F64, 0.5);
		assert_eq!(store.widen(ratio), TypeId::FLOAT);
		assert!(
			store.is_literal(five) && store.is_literal(text) && store.is_literal(TypeId::FALSE)
		);
		assert!(!store.is_literal(small) && !store.is_literal(TypeId::INTEGER));
	}
}
