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

/// The effects of the built-in method at `path` (`Array#push`).
pub fn effects_of(path: &str) -> &'static [BuiltinEffect] {
	match path {
		"Array#push"
		| "Array#insert"
		| "Array#sortInPlace"
		| "Array#reverseInPlace"
		| "Array#deleteWhere"
		| "Array#clear" => &[MutatesSelf],
		"Array#pop" => &[InvalidatesLast],
		"Array#delete" | "Array#take" => &[InvalidatesFrom(0)],
		"GrowingArray#push" => &[MutatesSize],
		"Map#set" | "GrowingMap#set" => &[MutatesKey(0)],
		"Map#getOrInsert" | "GrowingMap#getOrInsert" => &[MutatesSize],
		"Map#delete" | "Map#take" => &[InvalidatesKey(0)],
		"Map#deleteWhere" | "Map#clear" => &[MutatesSelf],
		"Set#add" | "Set#delete" | "Set#take" | "Set#deleteWhere" | "Set#clear" => &[MutatesSelf],
		"GrowingSet#add" => &[MutatesSize],
		_ => &[],
	}
}

/// Whether the built-in method at `path` changes its receiver.
pub fn changes_receiver(path: &str) -> bool {
	!effects_of(path).is_empty()
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn array_methods_have_their_effects() {
		assert_eq!(effects_of("Array#push"), &[MutatesSelf]);
		assert_eq!(effects_of("Array#pop"), &[InvalidatesLast]);
		assert_eq!(effects_of("Array#delete"), &[InvalidatesFrom(0)]);
		assert!(effects_of("Array#map").is_empty());
		assert!(changes_receiver("Array#clear"));
		assert!(!changes_receiver("Array#get"));
	}
}
