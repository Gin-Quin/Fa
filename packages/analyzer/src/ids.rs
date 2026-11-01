//! The `u32` handles of the analyzer.
//!
//! Every id is a `Copy` newtype over a `u32` index into a table of the
//! session, of the `TypeStore` or of a per-file store. Ids are only stable
//! within a session; stable identities across sessions go through
//! [`DeclKey`]s and the stable hashes of the `TypeStore`.

/// Declares `u32` newtype ids.
macro_rules! Ids {
	($($(#[$meta:meta])* $name:ident),* $(,)?) => {$(
		$(#[$meta])*
		#[derive(Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, Debug, Default)]
		pub struct $name(pub u32);

		impl $name {
			/// Builds the id of the item at `index` of its table.
			///
			/// Panics when the table outgrows `u32`, an invariant of the analyzer.
			#[inline]
			pub fn new(index: usize) -> Self {
				Self(u32::try_from(index).expect(concat!(stringify!($name), " overflow")))
			}

			/// The index of the item in its table.
			#[inline]
			pub fn index(self) -> usize {
				self.0 as usize
			}
		}
	)*};
}

Ids! {
	/// A node of a `TypedSyntaxTree` (its index in `nodes`).
	NodeId,
	/// One parse of one file: a re-parse gets a new id.
	FileId,
	/// An interned module path, stable for the session.
	ModuleId,
	/// A symbol of a file's symbol table.
	SymbolId,
	/// A scope of a file's scope arena.
	ScopeId,
	/// An interned identifier or string (see [`crate::intern::Interner`]).
	Sym,
	/// An interned type of the `TypeStore`. Index 0 is [`TypeId::UNSET`].
	TypeId,
	/// An interned list of types of the `TypeStore` (tuple members, union members, type arguments).
	ListId,
	/// An interned list of symbols (the enclosing names of a [`DeclKey`]).
	SymListId,
	/// Nominal identity: an interned (module, enclosing names, disambiguator).
	DeclKey,
	/// An interned object shape.
	ShapeId,
	/// An interned `Fields` flag tree (M2).
	FieldsShapeId,
	/// An interned function signature.
	SigId,
	/// An interned list of parameter signatures.
	ParamListId,
	/// An interned list of requirements `(name, type)` of a signature.
	ReqListId,
	/// An interned effect path.
	PathId,
	/// An interned list of effect path steps.
	StepListId,
	/// An interned list of paths.
	PathListId,
	/// An interned, normalized effect set. Index 0 is the empty set.
	EffectSetId,
	/// An interned nominal type (named union, `Errors`, enumeration, C struct, private object).
	NominalId,
	/// An interned integer type (`IntType`).
	IntId,
	/// An integer type whose bounds are affine in value parameters (M2).
	SymIntId,
	/// An interned member bitset of a named union or an enumeration.
	MaskId,
	/// A type parameter of a generic declaration.
	ParamId,
	/// An interned compile-time value. Index 0 is [`ConstId::NONE`].
	ConstId,
	/// An interned list of compile-time values. Index 0 is the empty list.
	ConstListId,
	/// A node of a canonical recursive type graph (M2).
	RecId,
	/// An interned spelling: a type name and its arguments as written.
	SpellingId,
	/// A function summary.
	SummaryId,
	/// A place of a file's flow store (a trie of paths).
	PlaceId,
	/// A hash-consed flow fact.
	FactId,
	/// A hash-consed integer interval. Index 0 is [`IntervalId::NONE`].
	IntervalId,
	/// A hash-consed integer interval with excluded intervals. Index 0 is [`IntervalSetId::NONE`].
	IntervalSetId,
	/// A reference validity state (M2).
	RefStateId,
	/// A built-in of the prelude.
	BuiltinId,
}

impl TypeId {
	/// No type yet, or not an expression.
	pub const UNSET: TypeId = TypeId(0);

	/// Whether the id is [`TypeId::UNSET`].
	#[inline]
	pub fn is_unset(self) -> bool {
		self == Self::UNSET
	}
}

impl ConstId {
	/// No compile-time value.
	pub const NONE: ConstId = ConstId(0);
}

impl ConstListId {
	/// The empty list.
	pub const EMPTY: ConstListId = ConstListId(0);
}

impl IntervalId {
	/// No interval fact: the range of the type.
	pub const NONE: IntervalId = IntervalId(0);
}

impl IntervalSetId {
	/// No interval fact: the range of the type.
	pub const NONE: IntervalSetId = IntervalSetId(0);
}

impl EffectSetId {
	/// No effect.
	pub const EMPTY: EffectSetId = EffectSetId(0);
}

impl ListId {
	/// The empty list of types.
	pub const EMPTY: ListId = ListId(0);
}

impl SymListId {
	/// The empty list of symbols.
	pub const EMPTY: SymListId = SymListId(0);
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn ids_round_trip_their_index() {
		assert_eq!(NodeId::new(42).index(), 42);
		assert_eq!(TypeId::new(7), TypeId(7));
		assert!(TypeId::UNSET.is_unset());
		assert!(!TypeId(1).is_unset());
	}

	#[test]
	fn ids_are_four_bytes() {
		assert_eq!(std::mem::size_of::<TypeId>(), 4);
		assert_eq!(std::mem::size_of::<Option<NodeId>>(), 8);
	}
}
