//! The effects of the built-in collection methods (advanced brief §7, the
//! reference table of the language reference): what a call does to its
//! receiver, applied by the walk like the declared effects of a user
//! function (`check/effects.rs`). Keyed by the builtin's path; a method not
//! listed changes nothing.

/// One effect of a built-in method on its receiver (`self`).
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum BuiltinEffect {
	/// `mutates self`: any change (`push`, `insert`, `sortInPlace`, `clear`…).
	MutatesSelf,
	/// `mutates self.size` (or `.length`): adds an element, moves none.
	MutatesSize,
	/// `invalidates self[self.length - 1]`: `pop`.
	InvalidatesLast,
	/// `invalidates self[index..]` with `index` the argument at this position
	/// (`delete(index)`, `take(index)` of an array).
	InvalidatesFrom(u8),
	/// `invalidates self[key]` with `key` the argument at this position, and
	/// `mutates self.size` (`delete(key)` of a map).
	InvalidatesKey(u8),
	/// `mutates self[key], self.size` with `key` the argument at this
	/// position (`set(key, value)` of a map).
	MutatesKey(u8),
	/// `invalidates element` with `element` the argument at this position,
	/// a reference into the receiver, and `mutates self.size` (`delete` and
	/// `take` of a pool or a columnar).
	InvalidatesArgument(u8),
}

use BuiltinEffect::*;

/// The collection kind a built-in path's owner names (`Array#push`): the
/// default kind of that family when the receiver's type isn't known.
fn kind_of_owner(owner: &str) -> Option<crate::types::CollKind> {
	use crate::types::CollKind;
	Some(match owner {
		"Array" => CollKind::Array,
		"GrowingArray" => CollKind::GrowingArray,
		"Set" => CollKind::Set,
		"GrowingSet" => CollKind::GrowingSet,
		"Map" => CollKind::Map,
		"GrowingMap" => CollKind::GrowingMap,
		"Pool" => CollKind::Pool,
		"GrowingPool" => CollKind::GrowingPool,
		"Columnar" => CollKind::Columnar,
		"GrowingColumnar" => CollKind::GrowingColumnar,
		"OrderedColumnar" => CollKind::OrderedColumnar,
		_ => return None,
	})
}

/// The effects of the built-in method at `path` (`Array#push`): the
/// reference table of the collections (`builtins::array::effects`). The
/// members of a growing collection share the paths of their standard
/// counterpart (`GrowingArray#push` is `Array#push`): use
/// [`effects_of_kind`] with the receiver's kind.
pub fn effects_of(path: &str) -> Vec<BuiltinEffect> {
	let Some((owner, method)) = path.split_once('#') else {
		return Vec::new();
	};
	match kind_of_owner(owner) {
		Some(kind) => effects_of_kind(kind, method),
		None => Vec::new(),
	}
}

/// The effects of the method `method` of a collection of kind `kind`.
pub fn effects_of_kind(kind: crate::types::CollKind, method: &str) -> Vec<BuiltinEffect> {
	use crate::builtins::array::EffectStep;
	crate::builtins::array::effects(kind, method)
		.iter()
		.map(|effect| match (effect.step, effect.invalidates) {
			(EffectStep::Whole, _) => MutatesSelf,
			(EffectStep::Size | EffectStep::Length, _) => MutatesSize,
			(EffectStep::Element, false) => MutatesKey(0),
			(EffectStep::Element, true) => InvalidatesKey(0),
			(EffectStep::ElementsFrom, _) => InvalidatesFrom(0),
			(EffectStep::Last, _) => InvalidatesLast,
			(EffectStep::Argument, _) => InvalidatesArgument(0),
		})
		.collect()
}

/// Whether the built-in method at `path` keeps its arguments (a closure
/// given to it escapes: it may run after the call).
pub fn stores_arguments(path: &str) -> bool {
	let method = path.rsplit('#').next().unwrap_or(path);
	let owner = path.split('#').next().unwrap_or(path);
	kind_of_owner(owner).is_some()
		&& matches!(method, "push" | "insert" | "set" | "getOrInsert" | "add")
}

/// Whether the built-in method at `path` changes its receiver.
pub fn changes_receiver(path: &str) -> bool {
	!effects_of(path).is_empty()
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn methods_have_the_effects_of_the_reference_table() {
		assert_eq!(effects_of("Array#push"), [MutatesSelf]);
		assert_eq!(effects_of("Array#pop"), [InvalidatesLast]);
		assert_eq!(effects_of("Array#delete"), [InvalidatesFrom(0)]);
		assert_eq!(effects_of("Map#delete"), [InvalidatesKey(0)]);
		assert!(effects_of("Array#map").is_empty());
		assert!(changes_receiver("Array#clear"));
		assert!(!changes_receiver("Array#get"));
	}
}
