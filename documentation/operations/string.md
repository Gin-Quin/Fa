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

## Replace and split

```fa
let clean = "hello-world".replace("-", " ")
let all = "a-b-c".replaceAll("-", " ")
let parts = "a,b,c".split(",")        -- ["a", "b", "c"]
let joined = parts.join(", ")         -- "a, b, c"
```

## Methods

No method changes the string it is called on: a `mutable` string is only changed by assignment and `+=`.

| Method | Returns |
| --- | --- |
| `length` (a property) | `Integer<min = 0>`, the number of UTF-16 code units |
| `utf8Length()` | `Integer<min = 0>`, the number of bytes of `toUtf8()`, without building it |
| `isEmpty()` | `Boolean` |
| `get(index: Integer)` | `String?`, the code unit at `index` as a string of length 1 |
| `codeUnitAt(index: Integer)` | `UInt16`, the code unit at `index` (JavaScript's `charCodeAt`); it throws `IndexError` like `text[index]` |
| `isWellFormed()` | `Boolean`: `false` when the string holds a [lone surrogate](/values/strings#surrogate-pairs) |
| `includes(search: String)`, `startsWith(search: String)`, `endsWith(search: String)` | `Boolean` |
| `indexOf(search: String)`, `lastIndexOf(search: String)` | `Integer?` |
| `toUpperCase()`, `toLowerCase()` | `String`, with the Unicode rules, independent of the locale (`"ß".toUpperCase()` is `"SS"`) |
| `trim()`, `trimStart()`, `trimEnd()` | `String`, without the leading or trailing Unicode white space |
| `padStart(length: Integer, filler: String = " ")`, `padEnd(...)` | `String`; `length` is in code units |
| `repeat(count: Integer<min = 0>)` | `String` |
| `replace(search: String, replacement: String)` | `String`, with the first match replaced |
| `replaceAll(search: String, replacement: String)` | `String`, with every match replaced |
| `split(separator: String)` | `Array<String>`; an empty separator splits into code units |
| `characters()` | `Array<String>`, the [grapheme clusters](/values/strings#grapheme-clusters) |
| `codePoints()` | `Array<UInt32>`; `text.codePoints().length` counts the code points without building the array |
| `toUtf8()` | [`Bytes`](/values/bytes), the string encoded in UTF-8; iterating it without keeping it (`for byte in text.toUtf8()`) doesn't build it |
| `toString()` | the string itself |
| `String.compareLocale(a: String, b: String, locale: String? = none)` | `Integer`: negative, zero or positive |
| `String.fromUtf8(bytes: Bytes)` | `String`, decoded from UTF-8; `throws Utf8Error` when the bytes are not valid UTF-8 |
| `String.fromUtf8Lossy(bytes: Bytes)` | `String`, where each invalid sequence becomes `U+FFFD` |

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

Repeated concatenation of immutable strings creates a new string each time. Use a mutable string, which grows in place:

```fa
mutable output = ""

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
