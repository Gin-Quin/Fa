# String operations

Fa strings expose an API close to JavaScript's `String` API, with one difference: positions, lengths and searches count [grapheme clusters](/values/strings#grapheme-clusters), not UTF-16 code units. See [strings](/values/strings).

## Concatenation

Use `+` to join two strings:

```fa
let greeting = "Hello" + " " + "world"
```

Both operands of `+` must be strings (a [`Character`](/values/strings#characters) is accepted as a string): Fa never converts a value to a string implicitly.

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

`length` is a property, and it counts **grapheme clusters**, matching how strings are indexed:

```fa
assert "héllo".length == 5
assert "🇫🇷".length == 1
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

Slicing with a [range](/values/ranges) copies the characters into a new string. Ranges follow the usual rule: `..` excludes the upper bound, `..=` includes it.

`indexOf` returns an [optional](/values/optionals): `none` when there is no match, rather than `-1`, so a missing match can't be mistaken for a position. The position counts grapheme clusters, like indexing.

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
| `length` (a property) | `Integer(min = 0)`, the number of grapheme clusters |
| `isEmpty()` | `Boolean` |
| `get(index: Integer)` | `Character?` |
| `includes(search: String)`, `startsWith(search: String)`, `endsWith(search: String)` | `Boolean` |
| `indexOf(search: String)`, `lastIndexOf(search: String)` | `Integer?` |
| `toUpperCase()`, `toLowerCase()` | `String`, with the Unicode rules, independent of the locale (`"ß".toUpperCase()` is `"SS"`) |
| `trim()`, `trimStart()`, `trimEnd()` | `String`, without the leading or trailing Unicode white space |
| `padStart(length: Integer, filler: String = " ")`, `padEnd(...)` | `String` |
| `repeat(count: Integer(min = 0))` | `String` |
| `replace(search: String, replacement: String)` | `String`, with the first match replaced |
| `replaceAll(search: String, replacement: String)` | `String`, with every match replaced |
| `split(separator: String)` | `Array(String)`; an empty separator splits into characters |
| `characters()` | `Array(Character)` |
| `codePoints()` | `Array(UInt32)` |
| `toString()` | the string itself |
| `String.compareLocale(a: String, b: String, locale: String? = none)` | `Integer`: negative, zero or positive |

On an array of strings, `join(separator: String)` returns a `String`.

There are no regular expressions in this version.

## Iterating

Iterating a string yields its characters, as [`Character`](/values/strings#characters) values. Iteration is `O(1)` per character, unlike indexing, which has to count grapheme clusters from the start.

```fa
for character in text {
	console.log(character)
}

for character, index in text {
	console.log("{index}: {character}")
}
```

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
assert "abc" < "abd" -- lexicographic, by code point
assert "Abc".toLowerCase() == "abc"
```

Comparison is by Unicode code point, not by locale and not by UTF-16 code unit (JavaScript's `<` compares code units, which orders some characters differently). For locale-aware ordering, use `String.compareLocale(a, b)`, which returns a negative integer when `a` comes first, zero when they are equivalent, and a positive integer otherwise.
