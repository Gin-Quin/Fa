# Strings

Strings represent text. They are written with double quotes only.

```fa
let greeting = "Hello"
```

Strings can span several lines, preserving the line breaks:

```fa
let message = "First line
Second line"
```

## Mutability

```fa
let immutableValue = "You can't change me"
mutable mutableValue = "But you can change me"
```

:::tip
When compiling to Rust, mutable strings become `std::String` and immutable string literals become `&'static str`.
:::

## Escapes

Use a backslash to escape a character.

```fa
let escaped = "Quote: \" Backslash: \\ Newline:\n Tab:\t Null:\0"
let hex     = "Byte: \x41"
let unicode = "Unicode: \u{1F600}"
```

Supported escape sequences:

- `\\` backslash
- `\"` double quote
- `\n` newline, `\r` carriage return, `\t` tab, `\0` null byte
- `\xNN` two-digit hexadecimal byte
- `\u{...}` Unicode code point, 1 to 6 hexadecimal digits
- `\{` and `\}` literal braces

## Template strings

Any string can interpolate expressions with `{}`:

```fa
let name = "Fa"
let welcome = "Hello {name}"

let a = 2
let b = 3
console.log("{a} + {b} = {a + b}") -- "2 + 3 = 5"
```

There is no separate template-string syntax and no backticks: every string is a template string.

To write a literal brace, double it or escape it:

```fa
let literal = "Braces: {{ and }}"  -- "Braces: { and }"
let escaped = "Braces: \{ and \}"  -- "Braces: { and }"
```

Interpolated values are converted with their `toString` method.

## Characters

A `Character` is a single Unicode grapheme cluster — what a reader perceives as one character, which may be several code points.

```fa
let snow: Character = "☃"
let flag: Character = "🇫🇷"
```

A `Character` is written exactly like a `String`; the type annotation is what distinguishes them. A string literal holding more than one grapheme cluster is a compile error when annotated as a `Character`.

## Indexing

Index a string with `[]` to get a `Character`:

```fa
let name = "Fa"
let first = name[0]
```

Indexing counts **grapheme clusters**, not bytes and not code points.

```fa
let text = "héllo"
assert(text.length == 5)
```

You can also use ranges and negative indices:

```fa
let someString = "hello"
let middle     = someString[1..3]  -- "el"  (end-exclusive)
let inclusive  = someString[1..=3] -- "ell" (end-inclusive)
let fromSecond = someString[1..]   -- "ello"
let last       = someString[-1]    -- "o"
```

Ranges follow the same rules everywhere in Fa: `..` excludes the upper bound, `..=` includes it. See [ranges](./ranges).

:::tip
Indexing by grapheme cluster is `O(n)`. When you are scanning a whole string, iterate it instead — iteration is `O(1)` per character.

```fa
for someString >> character {
  console.log(character)
}
```
:::

## String API

The string API mirrors the JavaScript `String` API. See [string operations](../operations/string).
