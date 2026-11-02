//! The members of the collections: `Array` (types brief §5.1), `Set` and
//! `GrowingSet` (§5.4, paths `Set#…`), `Map` and `GrowingMap` (§5.3, paths
//! `Map#…`), and their `from` functions (`Set.from`). The types depend on the
//! collection's item and key types: the walk builds them (`check/arrays.rs`,
//! `check/collections.rs`); these entries give them their identity
//! (`Resolution::Builtin`). [`effects`] is the normative table of their
//! effects (types brief §5.0.9), which the method types carry
//! (`FnSig::effects`). Also the members every enumeration has (types brief
//! §3.8): `members`, `toString`.

use crate::builtins::BuiltinKind::{Function, Property};
use crate::builtins::{Builtin, Param, Signature, Ty};
use crate::ids::{EffectSetId, PathId, TypeId};
use crate::types::{CollKind, PathRoot, PathStep, TypeStore};

/// `push(item)`: the item type is the array's.
static PUSH: Signature = Signature {
	params: &[Param {
		name: "item",
		ty: Ty::Is(TypeId::UNKNOWN),
		default: false,
	}],
	variadic: false,
	ret: Ty::Is(TypeId::VOID),
	throws: None,
};

/// `toString()`: the name of the member.
static TO_STRING: Signature = Signature {
	params: &[],
	variadic: false,
	ret: Ty::Is(TypeId::STRING),
	throws: None,
};

/// The members of arrays and enumerations.
pub static BUILTINS: &[Builtin] = &[
	Builtin {
		path: "Enum#toString",
		kind: Function(&TO_STRING),
	},
	// `Color.members`: an `Array(Color)` in declaration order (typed by the walk)
	Builtin {
		path: "Enum.members",
		kind: Property(Ty::Array(&Ty::Is(TypeId::UNKNOWN))),
	},
	// `Array.from(source, capacity = n)`, typed by the walk (`check/from.rs`)
	Builtin {
		path: "Array.from",
		kind: Function(&FROM),
	},
	Builtin {
		path: "Array#length",
		kind: Property(Ty::Natural),
	},
	Builtin {
		path: "Array#push",
		kind: Function(&PUSH),
	},
	method("Array#get", &ONE),
	method("Array#insert", &TWO),
	method("Array#delete", &ONE),
	method("Array#take", &ONE),
	method("Array#pop", &NONE),
	method("Array#clear", &NONE),
	method("Array#reverse", &NONE),
	method("Array#reverseInPlace", &NONE),
	method("Array#isEmpty", &NONE),
	method("Array#includes", &ONE),
	method("Array#indexOf", &ONE),
	method("Array#find", &ONE),
	method("Array#filter", &ONE),
	method("Array#every", &ONE),
	method("Array#some", &ONE),
	method("Array#deleteWhere", &ONE),
	method("Array#map", &ONE),
	method("Array#reduce", &TWO),
	method("Array#sort", &COMPARE),
	method("Array#sortInPlace", &COMPARE),
	method("Array#sum", &NONE),
	// `Set.from(source, key = …, capacity = 0)`, `Map.from(source, capacity = 0)`
	// (D59): typed by the walk (`check/collections.rs`)
	method("Set.from", &SET_FROM),
	method("Map.from", &FROM),
	// sets (§5.4); `GrowingSet` values share these paths
	Builtin {
		path: "Set#size",
		kind: Property(Ty::Natural),
	},
	method("Set#isEmpty", &NONE),
	method("Set#includes", &ONE),
	method("Set#add", &ONE),
	method("Set#delete", &ONE),
	method("Set#take", &ONE),
	method("Set#deleteWhere", &ONE),
	method("Set#clear", &NONE),
	method("Set#find", &ONE),
	method("Set#filter", &ONE),
	method("Set#map", &ONE),
	method("Set#reduce", &TWO),
	method("Set#every", &ONE),
	method("Set#some", &ONE),
	method("Set#sum", &NONE),
	method("Set#union", &ONE),
	method("Set#intersection", &ONE),
	method("Set#difference", &ONE),
	method("Set#isSubsetOf", &ONE),
	// maps (§5.3); `GrowingMap` values share these paths
	Builtin {
		path: "Map#size",
		kind: Property(Ty::Natural),
	},
	method("Map#isEmpty", &NONE),
	method("Map#includes", &ONE),
	method("Map#get", &ONE),
	method("Map#getOrInsert", &TWO),
	method("Map#delete", &ONE),
	method("Map#take", &ONE),
	method("Map#deleteWhere", &ONE),
	method("Map#clear", &NONE),
	method("Map#find", &ONE),
	method("Map#filter", &ONE),
	method("Map#map", &ONE),
	method("Map#reduce", &TWO),
	method("Map#every", &ONE),
	method("Map#some", &ONE),
];

/// An array method: its parameters (names and defaults) only; the walk types it.
const fn method(path: &'static str, signature: &'static Signature) -> Builtin {
	Builtin {
		path,
		kind: Function(signature),
	}
}

const ANY: Ty = Ty::Is(TypeId::UNKNOWN);

const fn param(name: &'static str, default: bool) -> Param {
	Param {
		name,
		ty: ANY,
		default,
	}
}

const fn signature(params: &'static [Param]) -> Signature {
	Signature {
		params,
		variadic: false,
		ret: ANY,
		throws: None,
	}
}

static NONE: Signature = signature(&[]);
static ONE: Signature = signature(&[param("value", false)]);
static TWO: Signature = signature(&[param("first", false), param("second", false)]);
/// `Array.from(source, capacity = hint)`
static FROM: Signature = signature(&[param("source", false), param("capacity", true)]);
/// `sort(compare = natural)`
static COMPARE: Signature = signature(&[param("compare", true)]);
/// `Set.from(source, key = …, capacity = 0)`
static SET_FROM: Signature = signature(&[
	param("source", false),
	param("key", true),
	param("capacity", true),
]);

/// The place an operation affects, below the collection (`self`).
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum EffectStep {
	/// The collection itself: `self`.
	Whole,
	/// `self[key]` / `self[index]`: the element of the first argument (the
	/// written index or key for `[]=`).
	Element,
	/// `self[index..]`: the elements from the first argument on.
	ElementsFrom,
	/// `self[self.length - 1]`: the last element.
	Last,
	/// `self.size`.
	Size,
	/// `self.length`.
	Length,
}

/// One effect of a collection operation (types brief §5.0.9).
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct CollectionEffect {
	/// `invalidates` (references into the place die) rather than `mutates`.
	pub invalidates: bool,
	/// The affected place.
	pub step: EffectStep,
}

const fn mutates(step: EffectStep) -> CollectionEffect {
	CollectionEffect {
		invalidates: false,
		step,
	}
}

const fn invalidates(step: EffectStep) -> CollectionEffect {
	CollectionEffect {
		invalidates: true,
		step,
	}
}

/// The effects of an operation of a collection (types brief §5.0.9, advanced
/// brief §7): a method name, or `"[]="` for an element write
/// (`map[key] = value`). Reads have none. The method types built by the walk
/// carry them in `FnSig::effects` ([`effect_set`]).
pub fn effects(kind: CollKind, operation: &str) -> &'static [CollectionEffect] {
	use EffectStep::{Element, ElementsFrom, Last, Length, Size, Whole};
	const MUTATES_SELF: &[CollectionEffect] = &[mutates(Whole)];
	const MUTATES_SIZE: &[CollectionEffect] = &[mutates(Size)];
	const MUTATES_LENGTH: &[CollectionEffect] = &[mutates(Length)];
	const MUTATES_ELEMENT: &[CollectionEffect] = &[mutates(Element)];
	const MUTATES_ELEMENT_AND_SIZE: &[CollectionEffect] = &[mutates(Element), mutates(Size)];
	const INVALIDATES_ELEMENT: &[CollectionEffect] = &[invalidates(Element)];
	const INVALIDATES_FROM: &[CollectionEffect] = &[invalidates(ElementsFrom)];
	const INVALIDATES_LAST: &[CollectionEffect] = &[invalidates(Last)];
	match (kind, operation) {
		(CollKind::Array | CollKind::OrderedColumnar | CollKind::GrowingArray, "[]=") => {
			MUTATES_ELEMENT
		}
		(CollKind::Array | CollKind::OrderedColumnar, "delete" | "take") => INVALIDATES_FROM,
		(CollKind::Array | CollKind::OrderedColumnar, "pop") => INVALIDATES_LAST,
		(
			CollKind::Array,
			"push" | "insert" | "sortInPlace" | "reverseInPlace" | "deleteWhere" | "clear",
		) => MUTATES_SELF,
		(CollKind::GrowingArray, "push") => MUTATES_LENGTH,
		(CollKind::Map | CollKind::GrowingMap, "[]=") => MUTATES_ELEMENT_AND_SIZE,
		(CollKind::Map | CollKind::GrowingMap, "getOrInsert") => MUTATES_SIZE,
		(CollKind::Map, "delete" | "take") => INVALIDATES_ELEMENT,
		(CollKind::Map, "deleteWhere" | "clear") => MUTATES_SELF,
		(CollKind::Set, "add" | "delete" | "take" | "deleteWhere" | "clear") => MUTATES_SELF,
		(CollKind::GrowingSet, "add") => MUTATES_SIZE,
		(CollKind::OrderedColumnar, "add" | "deleteWhere" | "clear") => MUTATES_SELF,
		_ => &[],
	}
}

/// The effect set of an operation (see [`effects`]): rooted at
/// `PathRoot::SelfParam`; the first argument is `PathStep::IndexParam(0)`.
pub fn effect_set(types: &mut TypeStore, kind: CollKind, operation: &str) -> EffectSetId {
	let mut mutated = smallvec::SmallVec::<[PathId; 2]>::new();
	let mut invalidated = smallvec::SmallVec::<[PathId; 2]>::new();
	for effect in effects(kind, operation) {
		let steps: &[PathStep] = match effect.step {
			EffectStep::Whole => &[],
			EffectStep::Element => &[PathStep::IndexParam(0)],
			EffectStep::ElementsFrom => &[PathStep::IndexFrom(0)],
			EffectStep::Last => &[PathStep::Last],
			EffectStep::Size => &[PathStep::Size],
			EffectStep::Length => &[PathStep::Length],
		};
		let path = types.path(PathRoot::SelfParam, steps);
		if effect.invalidates {
			invalidated.push(path);
		} else {
			mutated.push(path);
		}
	}
	types.effect_set(&mutated, &invalidated)
}

/// Whether an operation changes its collection: it needs a mutable receiver
/// (`EFF007`).
pub fn changes_collection(kind: CollKind, operation: &str) -> bool {
	!effects(kind, operation).is_empty()
}

/// The removal methods a growing collection lacks (`CL040`).
pub fn is_removal(method: &str) -> bool {
	matches!(
		method,
		"delete" | "take" | "deleteWhere" | "clear" | "pop" | "insert"
	)
}
