# Strings

Strings represent text values. They are declared with double quotes only.

```fa
let greeting = "Hello"
```

Strings can be multiline and preserve line breaks:

```fa
let message = "First line
Second line"
```

## Characters

Strings are sequences of characters. You can write characters directly or with escape sequences.

```fa
let snow = "Snowman: ☃️"
```

## Mutability

Immutable strings use `let`, while mutable strings use `mutable`.


```fa
let immutableValue = "You can't change me"
mutable mutableValue = "But you can change me"
```

:::tip
Mutable strings have additional methods and are internally coded as `std::String` in Rust.
Constant strings are treated as `&'static str`.
:::

## Escapes

Use backslashes to escape characters inside a string.

```fa
let escaped = "Quote: \" Backslash: \\ Newline:\n Tab:\t Null:\0"
let hex = "Byte: \x41"
let unicode = "Unicode: \u{1F600}"
```

Supported escape sequences:
- `\\` backslash
- `\"` double quote
- `\n` newline, `\r` carriage return, `\t` tab, `\0` null
- `\xNN` two-digit hex byte
- `\u{...}` Unicode code point with 1-6 hex digits
- `{{` and `}}` to insert braces

## Template strings

Strings can interpolate expressions with `{}`.

```fa
let name = "Fa"
let welcome = "Hello {name}"
```

Use `{{` or `\{` to insert a literal `{`, and `}}` or `\}` to insert a literal `}`.

```fa
let literal = "Braces: {{ and }}"
```

## Indexing

Strings can be indexed with `[]` to access a character by position.

```fa
let name = "Fa"
let first = name[0]
```

You can also use ranges and negative indices.

```fa
let someString = "hello"
let middle = someString[1..2]
let fromSecond = someString[1..]
let last = someString[-1]
```

Indexing counts characters, not bytes.

## String API

The string API matches the JavaScript `String` API.
