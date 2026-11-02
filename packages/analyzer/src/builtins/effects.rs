//! The effects of the built-in collection methods (advanced brief §7, the
//! reference table of the language reference): what a call does to its
//! receiver, applied by the walk like the declared effects of a user
//! function (`check/effects.rs`). Keyed by the builtin's path; a method not
//! listed changes nothing.
//!
//! Maps and sets (E1b) add their rows here.

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
}

use BuiltinEffect::*;

/// The effects of the built-in method at `path` (`Array#push`): the
/// reference table of the collections (`builtins::array::effects`).
pub fn effects_of(path: &str) -> Vec<BuiltinEffect> {
	use crate::builtins::array::EffectStep;
	use crate::types::CollKind;
	let Some((owner, method)) = path.split_once('#') else {
		return Vec::new();
	};
	let kind = match owner {
		"Array" => CollKind::Array,
		"GrowingArray" => CollKind::GrowingArray,
		"Set" => CollKind::Set,
		"GrowingSet" => CollKind::GrowingSet,
		"Map" => CollKind::Map,
		"GrowingMap" => CollKind::GrowingMap,
		"OrderedColumnar" => CollKind::OrderedColumnar,
		_ => return Vec::new(),
	};
	crate::builtins::array::effects(kind, method)
		.iter()
		.map(|effect| match (effect.step, effect.invalidates) {
			(EffectStep::Whole, _) => MutatesSelf,
			(EffectStep::Size | EffectStep::Length, _) => MutatesSize,
			(EffectStep::Element, false) => MutatesKey(0),
			(EffectStep::Element, true) => InvalidatesKey(0),
			(EffectStep::ElementsFrom, _) => InvalidatesFrom(0),
			(EffectStep::Last, _) => InvalidatesLast,
		})
		.collect()
}

/// Whether the built-in method at `path` keeps its arguments (a closure
/// given to it escapes: it may run after the call).
pub fn stores_arguments(path: &str) -> bool {
	matches!(
		path,
		"Array#push"
			| "Array#insert"
			| "GrowingArray#push"
			| "Map#set"
			| "GrowingMap#set"
			| "Map#getOrInsert"
			| "GrowingMap#getOrInsert"
			| "Set#add"
			| "GrowingSet#add"
	)
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
