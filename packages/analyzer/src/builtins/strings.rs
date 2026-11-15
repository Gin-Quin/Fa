//! String members (`values/strings.md`, `operations/string.md`). Positions
//! and lengths count UTF-16 code units. Only `reserve` mutates its receiver.

use crate::builtins::BuiltinKind::{Function, Property};
use crate::builtins::{Builtin, Param, Signature, Ty};
use crate::ids::TypeId;
use crate::types::{TypeKind, TypeStore};

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
	Ty::Optional(&Ty::Is(TypeId::STRING)),
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
static CHARACTERS: Signature = method(&[], Ty::Array(&Ty::Is(TypeId::STRING)));
static CODE_POINTS: Signature = method(&[], Ty::Array(&Ty::Is(TypeId::UINT32)));
/// `String.from(text, capacity = n)`: a copy with room for `n` bytes.
static FROM: Signature = method(
	&[
		param("text", STRING),
		Param {
			name: "capacity",
			ty: Ty::Natural,
			default: true,
		},
	],
	STRING,
);
/// `reserve(additional)`: room for more bytes (`mutates self`).
static RESERVE: Signature = method(&[param("additional", Ty::Natural)], Ty::Is(TypeId::VOID));
static TO_UTF8: Signature = method(&[], Ty::Is(TypeId::BYTES));
static UTF8_LENGTH: Signature = method(&[], Ty::Natural);
static CODE_UNIT_AT: Signature = Signature {
	params: &[param("index", INTEGER)],
	variadic: false,
	ret: Ty::Is(TypeId::UINT16),
	throws: Some(Ty::Error("IndexError")),
};
static FROM_UTF8: Signature = Signature {
	params: &[param("bytes", Ty::Is(TypeId::BYTES))],
	variadic: false,
	ret: STRING,
	throws: Some(Ty::Error("Utf8Error")),
};
static FROM_UTF8_LOSSY: Signature = method(&[param("bytes", Ty::Is(TypeId::BYTES))], STRING);
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
	// The `String` type name is registered in scalars.rs.
	"String.compareLocale" => Function(&COMPARE_LOCALE),
	"String.from" => Function(&FROM),
	"String.fromUtf8" => Function(&FROM_UTF8),
	"String.fromUtf8Lossy" => Function(&FROM_UTF8_LOSSY),
	"String#length" => Property(Ty::Natural),
	// the room of the string, in bytes; `reserve` is its one mutating method
	"String#capacity" => Property(Ty::Natural),
	"String#reserve" => Function(&RESERVE),
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
	"String#codeUnitAt" => Function(&CODE_UNIT_AT),
	"String#isWellFormed" => Function(&IS_EMPTY),
	"String#utf8Length" => Function(&UTF8_LENGTH),
	"String#toUtf8" => Function(&TO_UTF8),
	"String#toString" => Function(&NO_ARGUMENT_STRING),
}

/// Whether a type is a string type: `String` or a string literal type.
pub fn is_string_like(types: &TypeStore, ty: TypeId) -> bool {
	matches!(types.kind(ty), TypeKind::String | TypeKind::StringLit(_))
}

/// The number of UTF-16 code units in a well-formed literal.
pub fn utf16_length(text: &str) -> usize {
	text.encode_utf16().count()
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn utf16_units_are_counted() {
		for (text, count) in [
			("", 0),
			("a", 1),
			("héllo", 5),
			("🇫🇷", 4),
			("e\u{301}", 2),
			("☃", 1),
			("a😀", 3),
			("\r\n", 2),
			("👨‍👩‍👧", 8),
			("First line\nSecond line", 22),
		] {
			assert_eq!(utf16_length(text), count, "{text:?}");
		}
	}
}
