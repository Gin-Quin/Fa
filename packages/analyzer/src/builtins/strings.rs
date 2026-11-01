//! The members of `String` and `Character` values (`operations/string.md`,
//! values brief §7). No method mutates its receiver. Positions, lengths and
//! searches count grapheme clusters; the analyzer only types them.
//!
//! Paths: `String#name` for the members of the values (a `Character` has the
//! members of a `String`), `String.compareLocale` for the associated function.

use crate::builtins::BuiltinKind::{Function, Property};
use crate::builtins::{Builtin, Param, Signature, Ty};
use crate::ids::TypeId;

const STRING: Ty = Ty::Is(TypeId::STRING);
const INTEGER: Ty = Ty::Is(TypeId::INTEGER);

const fn param(name: &'static str, ty: Ty) -> Param {
	Param {
		name,
		ty,
		default: false,
	}
}

const fn method(params: &'static [Param], ret: Ty) -> Signature {
	Signature {
		params,
		variadic: false,
		ret,
		throws: None,
	}
}

static NO_ARGUMENT_STRING: Signature = method(&[], STRING);
static IS_EMPTY: Signature = method(&[], Ty::Is(TypeId::BOOLEAN));
static GET: Signature = method(
	&[param("index", INTEGER)],
	Ty::Optional(&Ty::Is(TypeId::CHARACTER)),
);
static SEARCH: Signature = method(&[param("search", STRING)], Ty::Is(TypeId::BOOLEAN));
static INDEX_OF: Signature = method(&[param("search", STRING)], Ty::Optional(&INTEGER));
static PAD: Signature = method(
	&[
		param("length", INTEGER),
		Param {
			name: "filler",
			ty: STRING,
			default: true,
		},
	],
	STRING,
);
static REPEAT: Signature = method(&[param("count", Ty::Natural)], STRING);
static REPLACE: Signature = method(
	&[param("search", STRING), param("replacement", STRING)],
	STRING,
);
static SPLIT: Signature = method(&[param("separator", STRING)], Ty::Array(&STRING));
static CHARACTERS: Signature = method(&[], Ty::Array(&Ty::Is(TypeId::CHARACTER)));
static CODE_POINTS: Signature = method(&[], Ty::Array(&Ty::Is(TypeId::UINT32)));
static COMPARE_LOCALE: Signature = method(
	&[
		param("a", STRING),
		param("b", STRING),
		Param {
			name: "locale",
			ty: Ty::Optional(&STRING),
			default: true,
		},
	],
	INTEGER,
);

macro_rules! members {
	($($path:literal => $kind:expr),* $(,)?) => {
		/// `String` with its associated function, and the members of its values.
		pub static BUILTINS: &[Builtin] = &[$(Builtin { path: $path, kind: $kind }),*];
	};
}

members! {
	// `String` and `Character` are type names (scalars.rs) with members here
	"String.compareLocale" => Function(&COMPARE_LOCALE),
	"String#length" => Property(Ty::Natural),
	"String#isEmpty" => Function(&IS_EMPTY),
	"String#get" => Function(&GET),
	"String#includes" => Function(&SEARCH),
	"String#startsWith" => Function(&SEARCH),
	"String#endsWith" => Function(&SEARCH),
	"String#indexOf" => Function(&INDEX_OF),
	"String#lastIndexOf" => Function(&INDEX_OF),
	"String#toUpperCase" => Function(&NO_ARGUMENT_STRING),
	"String#toLowerCase" => Function(&NO_ARGUMENT_STRING),
	"String#trim" => Function(&NO_ARGUMENT_STRING),
	"String#trimStart" => Function(&NO_ARGUMENT_STRING),
	"String#trimEnd" => Function(&NO_ARGUMENT_STRING),
	"String#padStart" => Function(&PAD),
	"String#padEnd" => Function(&PAD),
	"String#repeat" => Function(&REPEAT),
	"String#replace" => Function(&REPLACE),
	"String#replaceAll" => Function(&REPLACE),
	"String#split" => Function(&SPLIT),
	"String#characters" => Function(&CHARACTERS),
	"String#codePoints" => Function(&CODE_POINTS),
	"String#toString" => Function(&NO_ARGUMENT_STRING),
}

