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

Everything between the quotes is kept as written, including the indentation of the following lines. A line break in the source is always `\n` in the string, even in a file saved with Windows line endings (`\r\n`).

## Mutability

```fa
let immutableValue = "You can't change me"
mutable mutableValue = "But you can change me"
```

## Escapes

Use a backslash to escape a character.

```fa
let escaped = "Quote: \" Backslash: \\ Newline:\n Tab:\t Null:\0"
let hex = "Byte: \x41"
let unicode = "Unicode: \u{1F600}"
```

Supported escape sequences:

- `\\` backslash
- `\"` double quote
- `\n` newline, `\r` carriage return, `\t` tab, `\0` null byte
- `\xNN` an ASCII character, from `\x00` to `\x7F`
- `\u{...}` Unicode code point, 1 to 6 hexadecimal digits

A literal is always well-formed Unicode: `\x80` to `\xFF` are errors (use `\u{...}`), and so is a `\u{...}` that is not a Unicode scalar value (a surrogate, from `D800` to `DFFF`, or a value above `10FFFF`). Any other character after a backslash is an error.

A string literal **type** also always holds a well-formed string. A compile-time string value containing a lone surrogate has the type `String`; its exact literal type is not kept. Trying to use that value as a literal type, directly or through `Schema.StringLiteral` and `TypeFromSchema`, is an error: use `String`, or make the value well-formed first.

## Template strings

Any string can interpolate expressions with `{}`:

```fa
let name = "Fa"
let welcome = "Hello {name}"

let a = 2
let b = 3
console.log("{a} + {b} = {a + b}") -- "2 + 3 = 5"
```

There is no separate template string syntax and no backticks: every string is a template string.

To write a literal brace, double it. A lone `}` is an error, like a lone `{`:

```fa
let literal = "Braces: {{ and }}" -- "Braces: { and }"
```

The braces hold any expression, including strings and object literals: the string continues after the `}` that matches the opening `{`, and quotes inside the braces start a nested string.

```fa
let label = "Name: {user.name ?? "anonymous"}"
let origin = "Origin: {Point { x = 0, y = 0 }}"
```

Interpolated values are converted with their `toString` method, which every value has except functions (see [type conversion](/types/type-conversion)). Numbers print as in JavaScript (see [printing numbers](/values/numbers#printing-numbers)), and composite values in Fa notation, with the strings they contain quoted: `{ name = "Ana", scores = [1, 2] }`, `(1, "one")`, `Shape.Circle { radius = 1 }`, `none`.

`console.log` prints its arguments the same way, separated by a space, followed by a newline: `console.log("Total:", 3)` prints `Total: 3`.

## Strings are JavaScript strings

A `String` behaves exactly like a JavaScript string, on every target: it is a sequence of **UTF-16 code units**. `length`, indexing, slicing, the positions of `indexOf` and the order of `<` all count and compare code units, as in JavaScript. The same code gives the same result in the browser, on a server and at compile time.

A character of the Basic Multilingual Plane (Latin, Greek, Cyrillic, Chinese, Japanese, Korean…) is one code unit. Any other character, like most emoji, is two: a **surrogate pair**.

```fa
assert "héllo".length == 5
assert "日本語".length == 3
assert "😀".length == 2  -- one code point, a surrogate pair
assert "🇫🇷".length == 4 -- two code points, one flag on screen
```

Three other ways to see a string are explicit:

| View | How | Use it for |
| --- | --- | --- |
| Code points | `for character in text`, `codePoints()` | walking the text without ever cutting a surrogate pair |
| Grapheme clusters, what a reader sees as one character | [`characters()`](#grapheme-clusters) | cursors, truncating a text for display |
| UTF-8 bytes | [`toUtf8()`, `utf8Length()`](/values/bytes) | files, the network, limits expressed in bytes |

There is no character type: one character of a string is a `String`.

## Length

The `length` property is the number of UTF-16 code units. It is `O(1)`.

```fa
let text = "héllo"
assert text.length == 5
```

`utf8Length()` is the number of bytes of the string once encoded in UTF-8, without encoding it: the size that a file, a database column or a protocol counts.

```fa
assert "héllo".utf8Length() == 6
assert "😀".utf8Length() == 4
```

## Indexing

Index a string with `[]` to get the code unit at that position, as a `String` of length 1:

```fa
let name = "Fa"
let first = name[0] -- "F"
assert name[0] == "F"
```

You can also use ranges and negative indices. Slicing with a range copies the selected code units into a new `String`:

```fa
let someString = "hello"
let middle = someString[1..3]      -- "el"  (end-exclusive)
let inclusive = someString[1..=3]  -- "ell" (end-inclusive)
let fromSecond = someString[1..]   -- "ello"
let last = someString[-1]          -- "o"
```

Ranges follow the same rules everywhere in Fa: `..` excludes the upper bound, `..=` includes it. See [ranges](/values/ranges).

As with collections, an index or a range that the compiler cannot prove to be in bounds `throws IndexError` (JavaScript gives `undefined` instead). Use `get` to receive an optional:

```fa
let unit = try text[index]      -- throws IndexError if `index` is out of bounds
let maybeUnit = text.get(index) -- String?
```

See [errors](/values/errors) and [optionals](/values/optionals).

`codeUnitAt(index)` gives the code unit as a number (JavaScript's `charCodeAt`), which is what a lexer compares:

```fa
let digit = "7"
assert digit.codeUnitAt(0) == 55
```

### Slices

`let` copies the selected code units. To refer to them without copying, declare a **slice** with `use`: a reference into the string, like an [alias](/values/declaring-values#references-with-use). A slice of `text` has the type `String in text`.

```fa
mutable text = "hello world"

use word = text[0..5] -- a slice: no copy
console.log(word) -- "hello"

text += "!" -- mutates `text`: it may reallocate its storage
console.log(word) -- error: `word` was invalidated: declare it again with `use word = text[...]`
```

A string owns its storage, so any mutation of the string invalidates its slices: appending to it, or replacing it, as in `person.name = "Bao"`. See [mutations](/advanced/owned-references#mutations).

A slice is a view of a range of code units, not an independently replaceable string. Assigning the whole view, with `word = "other"` or `word += "!"`, is an error, even for a `mutable use`. Replace the owning string instead, or reseat a `mutable use` reference with `reuse`.

### Surrogate pairs

An index or a range may fall between the two code units of a surrogate pair. As in JavaScript, this is not an error: the result holds a **lone surrogate**, half of a character.

```fa
let face = "😀"
let half = face[0]  -- a lone surrogate

assert half.length == 1
assert not half.isWellFormed()
assert face[0] + face[1] == face -- the two halves make the character again
```

A string with a lone surrogate is not valid Unicode text. It can be stored, compared, sliced and concatenated like any string, but it has no UTF-8 encoding: [`toUtf8()`](/values/bytes#text-and-bytes) and every output (`console.log`, files, the network) write the replacement character `U+FFFD` (`�`) in place of each lone surrogate. `isWellFormed()` tells whether a string has none.

Lone surrogates only come from cutting a pair, or from JavaScript code that produces one: a literal can't contain one, and [decoding bytes](/values/bytes#text-and-bytes) never creates one. To walk a text that may hold characters outside the Basic Multilingual Plane, iterate over it rather than indexing it.

## Iterating over a string

A `for` loop over a string visits its **code points**, each one as a `String` of one or two code units. It never cuts a surrogate pair, like `for…of` in JavaScript.

```fa
for character in text {
	console.log(character)
}
```

The optional index is the position of the code point in the string, in code units, so it is a valid index for `text[index]` and for slicing. It advances by 2 after a surrogate pair:

```fa
for character, index in "a😀b" {
	console.log("{index}: {character}") -- "0: a", "1: 😀", "3: b"
}
```

A lone surrogate is visited as it is, a `String` of one code unit.

## Grapheme clusters

What a reader perceives as one character may be made of several code points: an `e` followed by a combining accent, an emoji with a skin tone, a flag. These are **grapheme clusters**. `characters()` splits a string into them, each one as a `String`:

```fa
let flags = "🇫🇷🇩🇪"

assert flags.length == 8
assert flags.characters().length == 2

let accented = "e\u{301}" -- an `e` and a combining accent
assert accented.length == 2
assert accented.characters().length == 1
```

It is the only string operation that depends on the Unicode tables: the segmentation follows the Unicode rules (UAX #29) of the Unicode version supported by the compiler, and, on JavaScript targets, of the JavaScript engine (`Intl.Segmenter`). Every other operation works on code units: a search can match a part of a grapheme cluster (`"e\u{301}".includes("e")` is `true`), as in JavaScript.

`==` compares the code units exactly, with no Unicode normalization, as in JavaScript: `"é"` written as one code point is not equal to `"e\u{301}"`.

## String API

The string API is the JavaScript `String` API, with a few differences that don't change what a position means: an unproven index throws instead of giving `undefined`, `indexOf` returns an optional instead of `-1`, and no value is converted to a string implicitly. See [string operations](/operations/string) for the list of methods.

`String.compareLocale` is available only at runtime: it depends on the locale data of the machine running the program and cannot be evaluated at compile time.

## Capacity and reservation

A `mutable` string that grows, with `+=`, keeps room for more bytes: its **capacity**, like the capacity of a [collection](/collections/arrays#capacity-and-reservation). When you know how long a string will get, reserve the room up front.

`String.from(text, capacity = n)` copies a string and requests room for at least `n` bytes natively. The source must be a `String`; this is not an implicit conversion. A string exposes a read-only `capacity: Integer<min = 0>` property and `reserve(additional: Integer<min = 0>)`, returning `Void`:

```fa
mutable output = String.from("", capacity = 4096) -- room for 4096 bytes

for line in lines {
	output += line
	output += "\n"
}

output.reserve(1024) -- room for 1024 more bytes than its current byte length
```

Natively, `capacity` counts storage bytes, not UTF-16 code units: it is at least the current byte length, and an inline string has room for 23 bytes (11 on wasm32). A static string reports its byte length until a reservation or append gives it owned storage. `reserve(n)` takes a number of additional bytes, so no length has to be converted from code units: it makes room for at least the current byte length plus `n`, never shrinks and never changes the text. It requires a `mutable` receiver and declares `mutates self`, like an append: references into the string must be declared again afterwards.

On JavaScript and during compile-time evaluation, the hints are ignored, `reserve` does nothing and `capacity` equals `length`. This keeps the property constant-time without measuring an encoding or inventing storage the engine does not expose. Use `utf8Length()` to measure an encoding, and `length` for code-unit positions. Capacity is a storage hint, not a portable limit: only `capacity >= length` holds on every target (see [capacity and reservation](/collections/arrays#capacity-and-reservation)). Copies need not preserve spare capacity.

:::implementation
## Implementation details

The semantics are UTF-16 code units; the native storage is UTF-8. Text arrives and leaves as UTF-8 (files, the network, C), so nothing is transcoded at these boundaries, and ASCII text, the common case, pays nothing: one byte is one code unit.

String literal types store well-formed UTF-8. An exact compile-time value containing a lone surrogate widens to `String`; schema reconstruction or using that value as a literal type must diagnose it rather than replace or encode away the surrogate.

### Native

Interpolation constructors and appended interpolations check each addition of byte lengths against the String representation limit before allocating or reserving. The check applies at both native and wasm pointer widths; an LLVM no-wrap annotation alone does not establish the bound.

- **Layout.** A `String` is 24 bytes: a pointer, a length in bytes and a capacity with flags (static, inline, ASCII, surrogates). A string of at most 23 bytes is stored inline, without allocation. A literal is stored in read-only data, with its UTF-16 length computed at compile time, and copied only when a mutable string is created from it. There are no ropes: a `mutable` string that is appended to grows in place.
- **Encoding.** The bytes are UTF-8, extended to lone surrogates (WTF-8): a lone surrogate is stored as the three bytes that the UTF-8 scheme gives to its value (`ED A0 80` to `ED BF BF`), and a surrogate pair is always stored as the four bytes of its code point. Equal strings therefore have equal bytes.
- **The ASCII flag** says that every byte is below `0x80`. It is computed for literals at compile time, kept by copies and slices, kept by an append when the suffix has it, and computed by `String.fromUtf8` while it validates. It may be missing from an ASCII string (only slower), never set on another. With it, a code unit is a byte: `length` is the byte length, indexing is a load, a position is a byte offset.
- **The surrogates flag** says that the string may hold a lone surrogate. Only an index or a slice that cuts a surrogate pair sets it (and operations on such strings keep it). Without it, the bytes are valid UTF-8. A concatenation whose left operand ends with a lone high surrogate and whose right operand starts with a lone low one writes the four bytes of the code point.
- **`length`** of a string without the ASCII flag is the number of bytes that are not continuation bytes, plus the number of four-byte sequences. An inline string counts its bytes; a heap string caches the count in its block header; a write clears the cache.
- **Indexing and slicing** a string without the ASCII flag convert code unit positions to byte offsets by scanning. A heap string builds breadcrumbs (the byte offset of every 64th code unit) at the second conversion after a write, so that a conversion scans at most 64 code units; a write clears them. A position inside a four-byte sequence yields the lone surrogate, as three bytes, and sets the surrogates flag of the result. `text[i]` returns an inline string; `codeUnitAt` returns the unit without building a string.
- **A slice** (`String in text`) is a pointer and a byte length, with the ASCII flag of its string. Whole-slice assignment is rejected by the analyzer; a reference can be reseated with `reuse`.
- **Comparing a one-unit string with a literal** (`text[i] == "a"`, `when text[i] is { … }`) compares code units: no string is built.
- **Iteration** decodes one code point per step from the bytes, `O(1)`, and advances the index by 1 or 2.
- **Equality** compares two inline values word by word. Otherwise it checks byte lengths before reading bytes; empty and one-byte strings compare inline, and longer strings use the shared byte comparison.
- **`<`** compares the bytes, then corrects the result at the first byte that differs: the UTF-8 order is the code point order, which differs from the UTF-16 order only when one of the two bytes starts a four-byte sequence (`F0` to `F4`) and the other is `EE` or `EF` (`U+E000` to `U+FFFF`); the result is then inverted. When a string has the surrogates flag, the two sequences at the first difference are compared as code units.
- **Searches** (`includes`, `indexOf`, `split`, `replace`…) search the bytes. A byte offset is converted to a position by counting the code units before it (the identity with the ASCII flag). A searched string that starts with a lone low surrogate or ends with a lone high one can match half of a pair: it is searched code unit by code unit.
- **`utf8Length()`** is the byte length: a lone surrogate is three bytes, like the `U+FFFD` that replaces it.
- **`toUtf8()`** copies the bytes, or gives the string's own block to the `Bytes` when the string is not used afterwards; with the surrogates flag, each lone surrogate is written as `EF BF BD`. `String.fromUtf8` validates the bytes (which computes the ASCII flag) and copies them, or adopts the block of a `Bytes` that is not used afterwards.
- **`characters()`** uses the compiler's UAX #29 tables, which are linked into the program only when it calls `characters()`.
- **Small strings never touch the heap.** Besides inline strings, a local string that doesn't escape its function (it is not returned, thrown, stored in a field, a collection or a module-level value, nor captured by an escaping closure or a task; the analyzer lists them, `Analysis::non_escaping`) lives in a stack buffer of the function, up to 64 bytes. A string goes to the heap only when it is longer, or escapes.
- **Capacity.** `String.from(text, capacity = n)` and `reserve(additional)` allocate the heap block at once (an inline string moves to the heap when it reserves more than fits inline); `capacity` reads the capacity word without its flags (the inline room for an inline string, the byte length of a literal).
- **The parts of `split` are views** of the text when the compiler proves that the text outlives them unchanged: the call, possibly followed by `filter`, initializes a `let`; the source is a local or module-level value that no reference designates, and no code writes it while the parts live. A view is a pointer and a byte length; keeping a part in another value copies its bytes. Parts of an inline string view a copy in the function frame, so a module initializer uses copies instead. A loop over a split by a nonempty literal reads views without testing their representation. A one-byte separator is counted first to allocate the array once.

- **Coroutine decimal appends.** A decimal string owned by a coroutine is dropped immediately after its append, inside the branch that creates it, before joining the inline and one-digit paths. It never leaves a branch-local value in the statement cleanup after that join.

- **Counted append loops.** A loop appending characters or digits can keep a dynamic short result inline. It reserves the prefix plus the number of rounds before the loop; a longer suffix uses the ordinary append path and reserves the remaining rounds again. An invariant mode lets LLVM select the inline or buffer loop once. The payload byte and the final length/tag byte are stored separately, preserving the bytes in the last word.

- **Destination passing.** A function that builds a mutable string and returns it, possibly with a final suffix, is expanded into an append destination when its result is appended there. The function has no effects, errors, early returns, captures or calls of program functions, and its arguments cannot read the destination.

### JavaScript

- A `String` is a JavaScript string, and every operation is the JavaScript one: `length` is `.length`, `text[i]` is `text[i]` after the bound check, a slice is `slice` (a copy, since JavaScript strings are immutable: invalidation is a compile-time rule only), `<` is `<`, `codeUnitAt` is `charCodeAt`, `isWellFormed` is `isWellFormed`. There is no helper and no cache.
- `indexOf` and `lastIndexOf` map `-1` to `undefined` (`none`).
- Iteration without an index is `for…of`. With an index, it is a loop that reads `codePointAt` and advances by the length of the code point.
- `characters()` uses a shared `Intl.Segmenter` (`granularity: "grapheme"`).
- `utf8Length()` is `Buffer.byteLength(text)` on Bun and Node, and a loop over `charCodeAt` in browsers. It is `O(n)`: this is why it is a method and not a property.
- `toUtf8()` is the `encode` of a shared `TextEncoder`. `String.fromUtf8` is the `decode` of a shared `TextDecoder("utf-8", { fatal: true, ignoreBOM: true })`, and `String.fromUtf8Lossy` the same without `fatal`.
- `String.from(text, capacity = n)` is `text`, `reserve` compiles to nothing (its argument is still evaluated), and `capacity` is the `length`.
### Compile-time interpreter

- Ordinary text stays in shared UTF-8 storage with a cached UTF-16 length. Only a string holding lone surrogates needs raw UTF-16 storage. ASCII copies and concatenations retain their known lengths without rescanning their new buffers.
- Indexing, slices, searches, padding and ordering use code units; iteration visits code points with code-unit indices. Concatenating cut halves joins a pair again. Formatting into another string preserves raw units; only an output or UTF-8 encoding replaces lone surrogates.
- `Bytes` has distinct compact byte storage and a distinct type test. Encoding, strict/lossy decoding and parsing JSON bytes use the same rules as runtime evaluation. `@readFile` remains strict UTF-8 text.
- Direct `codePoints().length` counts without building an array; direct iteration over `toUtf8()` encodes through a four-byte cursor without building `Bytes`. Allocation preflights include temporary conversions, borrowed slices and byte copies on write, under the interpreter's existing step, depth and heap limits.
- `String.compareLocale` runs only at runtime: its result depends on the locale data of the machine running the program, so it cannot be evaluated at compile time.

:::
