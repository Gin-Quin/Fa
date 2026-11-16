# String operations

Fa strings expose the JavaScript `String` API, with the same meaning: positions, lengths and searches count UTF-16 code units, on every target. See [strings](/values/strings).

## Concatenation

Use `+` to join two strings:

```fa
let greeting = "Hello" + " " + "world"
```

Both operands of `+` must be strings: Fa never converts a value to a string implicitly.

```fa
let score = 42

let label = "Score: " + score                    -- error: expected a `String`, found an `Integer`
let explicitLabel = "Score: " + score.toString() -- valid: explicit conversion
let interpolatedLabel = "Score: {score}"         -- valid: interpolation
```

Interpolation is usually clearer. Interpolated values are converted with their `toString` method. See [template strings](/values/strings#template-strings).

## Common operations

```fa
let name = "Fa"

let upper = name.toUpperCase()
let lower = name.toLowerCase()
let length = name.length
let trimmed = "  padded  ".trim()
let repeated = "ab".repeat(3)
```

`length` is a property, and it counts **UTF-16 code units**, as in JavaScript, matching how strings are indexed:

```fa
assert "héllo".length == 5
assert "😀".length == 2
```

## Slicing and searching

```fa
let text = "hello world"

let part = text[0..5]                -- "hello"
let rest = text[6..]                 -- "world"
let hasWorld = text.includes("world")
let starts = text.startsWith("hello")
let ends = text.endsWith("world")
let index = text.indexOf("world")    -- 6, an `Integer?`
```

Slicing with a [range](/values/ranges) copies the code units into a new string. Ranges follow the usual rule: `..` excludes the upper bound, `..=` includes it.

`indexOf` returns an [optional](/values/optionals): `none` when there is no match, rather than `-1`, so a missing match can't be mistaken for a position. The position counts code units, like indexing.

```fa
let planetIndex = text.indexOf("planet") -- none

if planetIndex is not none {
	console.log("found at {planetIndex}")
}

let position = text.indexOf("planet") ?? 0
```

An empty search is included in every string, including `""`: `includes("")`, `startsWith("")` and `endsWith("")` are `true`. `indexOf("")` is `0`; `lastIndexOf("")` is the string's length. A search longer than the text gives `false` or `none`, respectively.

## Replace and split

```fa
let clean = "hello-world".replace("-", " ")
let all = "a-b-c".replaceAll("-", " ")
let parts = "a,b,c".split(",")        -- ["a", "b", "c"]
let joined = parts.join(", ")         -- "a, b, c"
```

`replace` replaces the first match and `replaceAll` replaces every match. The replacement is inserted as written: `$` has no special meaning, including `$$`, `$&`, `` $` `` and `$'`.

```fa
assert "a-b".replace("-", "$&") == "a$&b"
```

Matches are consumed from left to right without overlapping. For example, `"aaa".replaceAll("aa", "x")` is `"xa"`. A missing match (including a search longer than the text) leaves the text unchanged.

An empty search in `replace` inserts the replacement once at the start. In `replaceAll`, it inserts at every code unit boundary, including both ends; an empty text gets one insertion. A boundary inside a surrogate pair may create lone surrogates: `"😀".replaceAll("", "-")` has five code units and is not well-formed.

`split` retains every empty field, including leading, trailing and repeated separators: `",a,,".split(",")` gives `["", "a", "", ""]`. With no match, it gives `[text]`. `"".split("")` gives `[]`; `"".split(",")` gives `[""]`.

## Padding, repetition, case and trimming

`padStart` and `padEnd` return the text unchanged when the target is no larger than its current length, including zero or a negative target, or when the filler is empty. Otherwise they repeat and cut the filler to the exact number of code units needed. The cut may split a pair: `"x".padEnd(2, "😀")` ends with a lone high surrogate.

`repeat(0)` and repeating an empty string give `""`. Impossible repetition or padding sizes use the existing allocation and overflow stops: they never wrap and introduce no declared error.

Case conversion preserves lone surrogates and follows Unicode's default full, context-sensitive mapping, without a locale: `"ΟΣ".toLowerCase()` is `"ος"` (final sigma), and `"ß".toUpperCase()` is `"SS"`. Casing and `characters()` use the JavaScript engine's Unicode tables on JavaScript targets, and the compiler's tables natively and at compile time; results can depend on that Unicode version.

`trim`, `trimStart` and `trimEnd` remove exactly the ECMAScript white space and line terminators: U+0009–U+000D, U+0020, U+00A0, U+1680, U+2000–U+200A, U+2028, U+2029, U+202F, U+205F, U+3000 and U+FEFF. U+0085 and U+200B are retained. No normalization is performed.

## Methods

No method changes the string it is called on, except `reserve`: a `mutable` string is only changed by assignment and `+=`.

| Method | Returns |
| --- | --- |
| `length` (a property) | `Integer<min = 0>`, the number of UTF-16 code units |
| `utf8Length()` | `Integer<min = 0>`, the number of bytes of `toUtf8()`, without building it |
| `capacity` (a property) | `Integer<min = 0>`, the bytes the string can hold without allocating (see [capacity and reservation](/values/strings#capacity-and-reservation)) |
| `reserve(additional: Integer<min = 0>)` | nothing: makes room for `additional` more bytes; a `mutable` string only (`mutates self`) |
| `isEmpty()` | `Boolean` |
| `get(index: Integer)` | `String?`, the code unit at `index` as a string of length 1 |
| `codeUnitAt(index: Integer)` | `UInt16`, the code unit at `index` (JavaScript's `charCodeAt`); it throws `IndexError` like `text[index]` |
| `isWellFormed()` | `Boolean`: `false` when the string holds a [lone surrogate](/values/strings#surrogate-pairs) |
| `includes(search: String)`, `startsWith(search: String)`, `endsWith(search: String)` | `Boolean` |
| `indexOf(search: String)`, `lastIndexOf(search: String)` | `Integer?` |
| `toUpperCase()`, `toLowerCase()` | `String`, with the Unicode rules, independent of the locale (`"ß".toUpperCase()` is `"SS"`) |
| `trim()`, `trimStart()`, `trimEnd()` | `String`, without the leading or trailing ECMAScript white space and line terminators |
| `padStart(length: Integer, filler: String = " ")`, `padEnd(...)` | `String`; `length` is in code units |
| `repeat(count: Integer<min = 0>)` | `String` |
| `replace(search: String, replacement: String)` | `String`, with the first match replaced |
| `replaceAll(search: String, replacement: String)` | `String`, with every match replaced |
| `split(separator: String)` | `Array<String>`; an empty separator splits into code units |
| `characters()` | `Array<String>`, the [grapheme clusters](/values/strings#grapheme-clusters) |
| `codePoints()` | `Array<UInt32>`; `text.codePoints().length` counts the code points without building the array |
| `toUtf8()` | [`Bytes`](/values/bytes), the string encoded in UTF-8; iterating it without keeping it (`for byte in text.toUtf8()`) doesn't build it |
| `toString()` | the string itself |
| `String.compareLocale(a: String, b: String, locale: String? = none)` | `Integer`: negative, zero or positive; runtime only, never evaluated at compile time |
| `String.fromUtf8(bytes: Bytes)` | `String`, decoded from UTF-8; `throws Utf8Error` when the bytes are not valid UTF-8 |
| `String.fromUtf8Lossy(bytes: Bytes)` | `String`, where each invalid sequence becomes `U+FFFD` |
| `String.from(text: String, capacity = n)` | `String`: a copy of `text` with room for at least `n` bytes |

On an array of strings, `join(separator: String)` returns a `String`.

There are no regular expressions in this version.

## Iterating

Iterating a string yields its code points, each one as a `String` of one or two code units, like `for…of` in JavaScript: it never cuts a surrogate pair. The index is the position of the code point in code units, so it advances by 2 after a surrogate pair. See [iterating over a string](/values/strings#iterating-over-a-string).

```fa
for character in text {
	console.log(character)
}

for character, index in text {
	console.log("{index}: {character}")
}
```

To visit what a reader sees as characters, iterate over `text.characters()`.

## Building strings

Repeated concatenation of immutable strings creates a new string each time. Use a mutable string, which grows in place, and reserve its capacity when you know its size (see [capacity and reservation](/values/strings#capacity-and-reservation)):

```fa
mutable output = String.from("", capacity = items.length * 8)

for item in items {
	output += item.toString()
	output += "\n"
}
```

## Comparing

```fa
assert "abc" == "abc"
assert "abc" < "abd" -- lexicographic, by code unit
assert "Abc".toLowerCase() == "abc"
```

Comparison is by UTF-16 code unit, as in JavaScript, not by locale. It is the order of the code points, except that the characters outside the Basic Multilingual Plane (most emoji) come before the characters from `U+E000` to `U+FFFF`. For locale-aware ordering, use `String.compareLocale(a, b)`, which returns a negative integer when `a` comes first, zero when they are equivalent, and a positive integer otherwise.

`String.compareLocale` cannot be evaluated at compile time: its result depends on the locale data of the machine running the program.

:::implementation
## Implementation details

`replace` and `replaceAll` insert the replacement literally on every target, including compile-time evaluation. On JavaScript, emit a replacement literal without `$` as it is; double each `$` in a replacement literal that contains it. For a dynamic replacement, evaluate it once and pass a replacement function returning that value, so JavaScript never interprets replacement patterns. Preserve the evaluation order of the receiver, search and replacement.
:::
