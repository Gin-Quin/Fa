use ethnum::I256;

use super::*;
use crate::ids::{ListId, NominalId, ParamId};
use crate::types::store::GlobalStore;
use crate::types::{
	Bindings, CollKind, ConstValue, FieldEntry, FloatBits, NominalInfo, NominalKind, ParamInfo,
	PathRoot,
};

fn field(types: &mut TypeStore, name: &str, ty: TypeId) -> FieldEntry {
	FieldEntry {
		name: types.syms.intern(name),
		ty,
		flags: FieldFlags::empty(),
	}
}

fn nominal(types: &mut TypeStore, name: &str, kind: NominalKind, names: &[&str]) -> NominalId {
	let module = types.module("ownership");
	let key = types.decl_key(module, &[name], 0);
	let names: Vec<_> = names.iter().map(|name| types.syms.intern(name)).collect();
	let members = types.intern_sym_list(&names);
	types.intern_nominal(NominalInfo {
		key,
		args: ListId::EMPTY,
		kind,
		members,
	})
}

fn parameter(types: &mut TypeStore, value: bool) -> (ParamId, TypeId) {
	let module = types.module("ownership");
	let owner = types.decl_key(module, &["Generic"], 0);
	let name = types.syms.intern("Item");
	let param = types.add_param(ParamInfo {
		owner,
		index: 0,
		name,
		constraint: TypeId::UNKNOWN,
		default: TypeId::UNSET,
		value,
	});
	(param, types.intern(TypeKind::Param(param)))
}

fn task(types: &mut TypeStore) -> TypeId {
	types.intern(TypeKind::Task(TypeId::INTEGER, TypeId::NEVER))
}

#[test]
fn ordinary_scalars_literals_and_type_level_values() {
	let mut types = TypeStore::new();
	let text = types.string_literal("ordinary");
	let integer = types.int_literal(I256::from(7));
	let float = types.float_literal(FloatBits::F64, 1.25);
	let digits = types.syms.intern("123456789012345678901234567890");
	let big = types.constant(ConstValue::BigInt(digits));
	let big_literal = types.intern(TypeKind::BigIntLit(big));
	let constant = types.constant(ConstValue::Int(I256::from(7)));
	let constant = types.intern(TypeKind::Const(constant));
	let fields = types.fields_type(&[("field".into(), TypeId::STRING)]);
	let enumeration = nominal(&mut types, "Choice", NominalKind::Enum, &["One", "Two"]);
	let enum_type = types.nominal_type(enumeration);
	let enum_member = types.intern(TypeKind::EnumMember(enumeration, 1));
	for ty in [
		TypeId::NEVER,
		TypeId::VOID,
		TypeId::NONE,
		TypeId::BOOLEAN,
		TypeId::TRUE,
		TypeId::FALSE,
		TypeId::INTEGER,
		TypeId::INT128,
		TypeId::UINT128,
		TypeId::FLOAT,
		TypeId::FLOAT32,
		TypeId::BIG_INT,
		TypeId::PERCENTAGE,
		TypeId::STRING,
		TypeId::CHARACTER,
		TypeId::RANGE,
		text,
		integer,
		float,
		big_literal,
		constant,
		fields,
		enum_type,
		enum_member,
	] {
		assert_eq!(
			types.ownership(ty),
			Ownership::Ordinary,
			"{}",
			types.display(ty)
		);
		assert!(types.is_ordinary(ty));
		assert!(!types.is_structure(ty) && !types.is_unique(ty) && !types.is_shared_handle(ty));
	}
}

#[test]
fn every_collection_owns_its_items_and_map_keys() {
	let mut types = TypeStore::new();
	let task = task(&mut types);
	for kind in [
		CollKind::Array,
		CollKind::GrowingArray,
		CollKind::Set,
		CollKind::GrowingSet,
		CollKind::Map,
		CollKind::GrowingMap,
		CollKind::Pool,
		CollKind::GrowingPool,
		CollKind::Columnar,
		CollKind::GrowingColumnar,
		CollKind::OrderedColumnar,
		CollKind::RelationPool(0),
	] {
		let ordinary = types.collection(kind, TypeId::STRING, TypeId::NONE);
		assert!(types.is_structure(ordinary), "{kind:?}");
		let unique = types.collection(kind, task, TypeId::NONE);
		assert!(types.is_unique(unique), "{kind:?}");
	}
	let bytes = types.array(TypeId::BYTE);
	assert!(
		types.is_structure(bytes),
		"current byte arrays, unlike String, own a collection"
	);
	let unique_key = types.collection(CollKind::Map, TypeId::INTEGER, task);
	assert!(types.is_unique(unique_key));
}

#[test]
fn ordinary_objects_and_transitive_structure_owners() {
	let mut types = TypeStore::new();
	let name = field(&mut types, "name", TypeId::STRING);
	let ordinary = types.object(&[name], None);
	assert!(types.is_ordinary(ordinary));
	let array = types.array(TypeId::INTEGER);
	let mut nested = array;
	for _ in 0..30 {
		let owned = field(&mut types, "owned", nested);
		nested = types.object(&[owned], None);
	}
	assert!(types.is_structure(nested));
	let pair = types.tuple(&[ordinary, nested]);
	let optional = types.optional(pair);
	let union = types.union(&[TypeId::STRING, optional]);
	assert!(types.is_structure(pair) && types.is_structure(optional) && types.is_structure(union));
	for flags in [
		FieldFlags::PRIVATE,
		FieldFlags::READONLY,
		FieldFlags::REACTIVE,
	] {
		let mut owned = field(&mut types, "owned", array);
		owned.flags = flags;
		let object = types.object(&[owned], None);
		assert!(types.is_structure(object));
	}
}

#[test]
fn only_synchronization_handles_share_and_their_payloads_are_boundaries() {
	let mut types = TypeStore::new();
	let task = task(&mut types);
	let array = types.array(task);
	for kind in [
		SyncKind::Mutex,
		SyncKind::Channel,
		SyncKind::AtomicInteger,
		SyncKind::AtomicBoolean,
	] {
		let handle = types.intern(TypeKind::Sync(kind, array));
		assert!(types.is_shared_handle(handle));
		assert!(!types.is_unique(handle) && !types.is_structure(handle));
		let stored = field(&mut types, "handle", handle);
		let object = types.object(&[stored], None);
		assert!(
			types.is_ordinary(object),
			"the object owns a bounded handle, not its payload"
		);
	}
	let ordering = types.intern(TypeKind::Sync(SyncKind::Ordering, TypeId::NONE));
	assert!(types.is_ordinary(ordering));
}

#[test]
fn tasks_resources_and_all_builtin_lifetime_owners_are_unique() {
	let mut types = TypeStore::new();
	let task = task(&mut types);
	let subscription = types.intern(TypeKind::Subscription);
	let resource_nominal = nominal(&mut types, "Resource", NominalKind::Resource, &[]);
	let scalar = field(&mut types, "id", TypeId::INTEGER);
	let resource = types.object(&[scalar], Some(resource_nominal));
	let mut unique = vec![task, subscription, resource];
	for kind in [
		SyncKind::Guard,
		SyncKind::Worker,
		SyncKind::WorkerPool,
		SyncKind::TcpListener,
		SyncKind::TcpStream,
	] {
		unique.push(types.intern(TypeKind::Sync(kind, TypeId::STRING)));
	}
	for ty in unique {
		assert!(types.is_unique(ty), "{}", types.display(ty));
		assert!(!types.is_ordinary(ty) && !types.is_structure(ty) && !types.is_shared_handle(ty));
		let owned = field(&mut types, "owned", ty);
		let object = types.object(&[owned], None);
		let tuple = types.tuple(&[TypeId::STRING, object]);
		let optional = types.optional(tuple);
		let array = types.array(optional);
		assert!(types.is_unique(array));
	}
}

#[test]
fn nonowning_edges_and_callable_signatures_do_not_own_their_targets() {
	let mut types = TypeStore::new();
	let task = task(&mut types);
	let path = types.path(PathRoot::SelfParam, &[]);
	let reference = types.intern(TypeKind::Ref(task, path));
	let pointer = types.intern(TypeKind::Pointer(task));
	let iterator = types.intern(TypeKind::Iterator(task, path));
	let signature = types.simple_sig(&[task], task, task);
	let function = types.function(signature);
	for ty in [
		reference,
		pointer,
		iterator,
		function,
		types.intern(TypeKind::CString),
	] {
		assert!(types.is_ordinary(ty));
	}
	for flags in [FieldFlags::RELATION, FieldFlags::METHOD] {
		let mut borrowed = field(&mut types, "borrowed", task);
		borrowed.flags = flags;
		let object = types.object(&[borrowed], None);
		assert!(types.is_ordinary(object));
	}
}

#[test]
fn fixed_c_arrays_propagate_their_item_category() {
	let mut types = TypeStore::new();
	let array = types.array(TypeId::STRING);
	let task = task(&mut types);
	for (item, expected) in [
		(TypeId::INTEGER, Ownership::Ordinary),
		(array, Ownership::Structure),
		(task, Ownership::Unique),
	] {
		let inline = types.intern(TypeKind::CArray(item, 3));
		assert_eq!(types.ownership(inline), expected);
		let nominal = nominal(&mut types, &format!("C{item:?}"), NominalKind::CStruct, &[]);
		let data = field(&mut types, "data", inline);
		let object = types.object_in_order(&[data], nominal);
		assert_eq!(types.ownership(object), expected);
	}
	let opaque = nominal(&mut types, "Opaque", NominalKind::CStruct, &[]);
	let opaque = types.nominal_type(opaque);
	assert!(types.is_ordinary(opaque));
}

#[test]
fn named_union_masks_and_individual_members_use_only_their_payloads() {
	let mut types = TypeStore::new();
	let names: Vec<_> = (0..72).map(|index| format!("Member{index}")).collect();
	let names: Vec<_> = names.iter().map(String::as_str).collect();
	let nominal = nominal(&mut types, "Wide", NominalKind::Union, &names);
	let array = types.array(TypeId::INTEGER);
	let task = task(&mut types);
	let mut payloads = vec![TypeId::UNSET; 72];
	payloads[1] = array;
	payloads[70] = task;
	types.set_payloads(nominal, payloads);
	let all = types.nominal_type(nominal);
	let ordinary = types.member(nominal, 0);
	let structured = types.member(nominal, 1);
	let unique = types.member(nominal, 70);
	assert!(types.is_unique(all));
	assert!(types.is_ordinary(ordinary));
	assert!(types.is_structure(structured));
	assert!(types.is_unique(unique));
	let mask = types.mask(&[3, 0]);
	let narrowed = types.named_union(nominal, mask);
	assert!(
		types.is_structure(narrowed),
		"member 70 is absent from this narrowed union"
	);
}

#[test]
fn recursive_data_is_a_structure_even_with_only_scalar_leaves() {
	let mut types = TypeStore::new();
	let module = types.module("ownership");
	let key = types.decl_key(module, &["List"], 0);
	let rec = types.recursive_reference(key, "List", &[], true);
	assert_eq!(types.ownership(rec), Ownership::Unresolved);
	let next = types.optional(rec);
	let value = field(&mut types, "value", TypeId::INTEGER);
	let next = field(&mut types, "next", next);
	let list = types.object(&[value, next], None);
	types.set_rec_target(rec, list);
	assert!(types.is_structure(rec) && types.is_structure(list));
	let tuple = types.tuple(&[rec, TypeId::STRING]);
	assert!(types.is_structure(tuple));
}

#[test]
fn recursion_through_borrowed_or_shared_storage_does_not_own_a_cycle() {
	let mut types = TypeStore::new();
	let module = types.module("ownership");
	let key = types.decl_key(module, &["Link"], 0);
	let rec = types.recursive_reference(key, "Link", &[], true);
	let path = types.path(PathRoot::SelfParam, &[]);
	let reference = types.intern(TypeKind::Ref(rec, path));
	let next = field(&mut types, "next", reference);
	let link = types.object(&[next], None);
	types.set_rec_target(rec, link);
	assert!(types.is_ordinary(rec) && types.is_ordinary(link));
	let key = types.decl_key(module, &["Cell"], 0);
	let rec = types.recursive_reference(key, "Cell", &[], false);
	let mutex = types.intern(TypeKind::Sync(SyncKind::Mutex, rec));
	types.set_rec_target(rec, mutex);
	assert!(types.is_shared_handle(rec));
}

#[test]
fn shared_children_are_not_mistaken_for_a_recursive_value() {
	let mut types = TypeStore::new();
	let scalar = field(&mut types, "value", TypeId::INTEGER);
	let ordinary = types.object(&[scalar], None);
	let pair = types.tuple(&[ordinary, ordinary]);
	assert!(types.is_ordinary(pair));
}

#[test]
fn generic_instances_depend_on_data_fields_not_phantom_arguments() {
	let mut types = TypeStore::new();
	let (param, parameter_ty) = parameter(&mut types, false);
	let owned = field(&mut types, "item", parameter_ty);
	let holder = types.object(&[owned], None);
	assert_eq!(types.ownership(holder), Ownership::Dependent);
	let array = types.array(TypeId::INTEGER);
	let task = task(&mut types);
	for (argument, expected) in [
		(TypeId::STRING, Ownership::Ordinary),
		(array, Ownership::Structure),
		(task, Ownership::Unique),
	] {
		let concrete = types.substitute(
			holder,
			&Bindings {
				params: vec![param],
				types: vec![argument],
				..Bindings::default()
			},
		);
		assert_eq!(types.ownership(concrete), expected);
	}
	let module = types.module("ownership");
	let key = types.decl_key(module, &["Phantom"], 0);
	let args = types.list(&[task]);
	let nominal = types.intern_nominal(NominalInfo {
		key,
		args,
		kind: NominalKind::Private,
		members: crate::ids::SymListId::EMPTY,
	});
	let phantom = types.object(&[], Some(nominal));
	assert!(types.is_ordinary(phantom));
	let mutex = types.intern(TypeKind::Sync(SyncKind::Mutex, parameter_ty));
	let task = types.intern(TypeKind::Task(parameter_ty, parameter_ty));
	assert!(types.is_shared_handle(mutex) && types.is_unique(task));
	let (_, value_parameter) = parameter(&mut types, true);
	assert!(types.is_ordinary(value_parameter));
}

#[test]
fn late_payloads_are_reclassified_and_global_views_read_the_same_category() {
	let global = GlobalStore::new();
	let mut types = TypeStore::view(&global);
	let nominal = nominal(&mut types, "Late", NominalKind::Union, &["Data"]);
	let member = types.member(nominal, 0);
	assert_eq!(types.ownership(member), Ownership::Unresolved);
	let task = task(&mut types);
	types.set_payloads(nominal, vec![task]);
	assert!(types.is_unique(member));
	let published = types.merger(None).ty(member);
	let reader = TypeStore::view(&global);
	assert!(reader.is_unique(published));
}

#[test]
fn open_errors_are_structures_and_concrete_errors_follow_their_payload() {
	let mut types = TypeStore::new();
	assert!(types.is_structure(TypeId::ERROR));
	assert!(!types.is_unique(TypeId::ERROR));
	let nominal = nominal(
		&mut types,
		"Failure",
		NominalKind::Errors,
		&["Code", "Data", "Shared"],
	);
	let array = types.array(TypeId::INTEGER);
	let task = task(&mut types);
	let mutex = types.intern(TypeKind::Sync(SyncKind::Mutex, task));
	types.set_payloads(nominal, vec![TypeId::INTEGER, array, mutex]);
	let code = types.member(nominal, 0);
	let data = types.member(nominal, 1);
	let shared = types.member(nominal, 2);
	assert!(types.is_ordinary(code));
	assert!(types.is_structure(data));
	assert!(types.is_shared_handle(shared));
}

#[test]
fn deeply_nested_types_have_no_recursion_or_depth_limit() {
	let mut types = TypeStore::new();
	let mut ty = types.array(TypeId::INTEGER);
	for _ in 0..20_000 {
		let item = field(&mut types, "item", ty);
		ty = types.object(&[item], None);
	}
	assert!(types.is_structure(ty));
}

#[test]
fn unresolved_data_defers_checks_but_known_uniqueness_wins() {
	let mut types = TypeStore::new();
	assert_eq!(types.ownership(TypeId::UNSET), Ownership::Unresolved);
	assert_eq!(types.ownership(TypeId::UNKNOWN), Ownership::Unresolved);
	let array = types.array(TypeId::UNKNOWN);
	assert_eq!(types.ownership(array), Ownership::Unresolved);
	let (_, parameter) = parameter(&mut types, false);
	let task = task(&mut types);
	let generic = field(&mut types, "generic", parameter);
	let unique = field(&mut types, "unique", task);
	let object = types.object(&[generic, unique], None);
	assert!(types.is_unique(object));
}

#[test]
fn recursive_generic_instances_classify_substituted_payloads() {
	let mut types = TypeStore::new();
	let (param, parameter) = parameter(&mut types, false);
	let key = types.param(param).owner;
	let rec = types.recursive_reference(key, "Generic", &[parameter], true);
	let tail = types.optional(rec);
	let head = field(&mut types, "head", parameter);
	let tail = field(&mut types, "tail", tail);
	let target = types.object(&[head, tail], None);
	types.set_rec_params(key, rec, vec![param]);
	types.set_rec_target(rec, target);
	assert_eq!(types.ownership(rec), Ownership::Dependent);
	let ordinary = types.recursive_reference(key, "Generic", &[TypeId::STRING], true);
	assert!(types.is_structure(ordinary));
	let task = task(&mut types);
	let unique = types.recursive_reference(key, "Generic", &[task], true);
	assert!(types.is_unique(unique));
	let published = types.merger(None).ty(unique);
	let reader = TypeStore::view(&types.global);
	assert!(reader.is_unique(published));
}

#[test]
fn implicit_type_parameters_defer_ownership() {
	let mut types = TypeStore::new();
	let (param, _) = parameter(&mut types, false);
	let key = types.param(param).owner;
	let implicit = types.intern(TypeKind::ImplicitParam(key, 0));
	assert_eq!(types.ownership(implicit), Ownership::Dependent);
	let array = types.array(implicit);
	assert_eq!(types.ownership(array), Ownership::Dependent);
}
