# String operations

Fa strings expose an API close to JavaScript's `String` API. See [strings](/values/strings).

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

`indexOf` returns an [optional](/values/optionals): `none` when there is no match, rather than `-1`, so a missing match can't be mistaken for a position.

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

Comparison is by Unicode code point, not by locale. For locale-aware ordering, use `String.compareLocale(a, b)`.
