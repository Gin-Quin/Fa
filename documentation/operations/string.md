# String operations

Fa strings expose the same API as JavaScript strings.

## Concatenation

Use `+` to append to a string. The right operand is converted with its `toString` method.

```fa
let greeting = "Hello" + " " + "world"
let label = "Score: " + 42
```

Interpolation is usually clearer:

```fa
let label = "Score: {score}"
```

See [template strings](../values/strings#template-strings).

## Common operations

```fa
let name = "Fa"

let upper = name.toUpperCase()
let lower = name.toLowerCase()
let length = name.length
let trimmed = "  padded  ".trim()
let repeated = "ab".repeat(3)
```

`length` counts grapheme clusters, matching how strings are indexed.

## Slicing and searching

```fa
let text = "hello world"

let part = text.slice(0, 5)         -- "hello"
let same = text[0..5]               -- "hello"
let hasWorld = text.includes("world")
let index = text.indexOf("world")   -- Int | NotFound
let starts = text.startsWith("hello")
let ends = text.endsWith("world")
```

`indexOf` returns an error rather than `-1`, so a missing match cannot be mistaken for a position:

```fa
let index = text.indexOf("world") ?? 0
```

## Replace and split

```fa
let clean = "hello-world".replace("-", " ")
let all = "a-b-c".replaceAll("-", " ")
let parts = "a,b,c".split(",")        -- ["a", "b", "c"]
let joined = parts.join(", ")         -- "a, b, c"
```

## Iterating

Iterating a string yields its characters, and is `O(1)` per character — unlike indexing, which has to count from the start.

```fa
for text >> character {
  console.log(character)
}

for text >> character, index {
  console.log("{index}: {character}")
}
```

## Building strings

Repeated concatenation in a loop reallocates each time. Use a mutable string, which grows in place:

```fa
mutable output = ""

for items >> item {
  output += item.toString()
  output += "\n"
}
```

## Comparing

```fa
assert("abc" == "abc")
assert("abc" < "abd")     -- lexicographic, by code point
assert("Abc".toLowerCase() == "abc")
```

Comparison is by Unicode code point, not by locale. For locale-aware ordering, use `String.compareLocale(a, b)`.
