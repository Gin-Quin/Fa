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

Strings are always valid UTF-8: `\x80` to `\xFF` are errors (use `\u{...}`), and so is a `\u{...}` that is not a Unicode scalar value (a surrogate, from `D800` to `DFFF`, or a value above `10FFFF`). Any other character after a backslash is an error.

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

## Characters

A `Character` is a single Unicode grapheme cluster: what a reader perceives as one character, which may be made of several code points.

```fa
let snow: Character = "☃"
let flag: Character = "🇫🇷"
```

A `Character` is written exactly like a `String`; the expected type is what distinguishes them. A string literal adapts to `Character` when the expected type is `Character`, and is a `String` otherwise. Only a literal holding exactly one grapheme cluster, without interpolation, adapts: anything else is a compile error when a `Character` is expected:

```fa
let letters: Character = "ab" -- error: a `Character` holds exactly one grapheme cluster
let nothing: Character = ""   -- error: same
let letter = "a"              -- a String: there is no expected type
```

A `Character` is accepted where a `String` is expected, not the reverse:

```fa
let first: Character = "F"
let text: String = first        -- valid
let label = "Letter: " + first  -- valid: "Letter: F", a String
let character: Character = text -- error: a `String` may hold several characters
```

Since `name[0]` is a `Character`, `name[0] == "F"` compares two characters: the literal adapts to the type of the other operand. Adding two characters gives a `String`.

## Length

The `length` property counts **grapheme clusters**, not bytes and not code points. This is a deliberate difference with JavaScript, whose `length` counts UTF-16 code units (`"🇫🇷".length` is 4 in JavaScript):

```fa
let text = "héllo"
assert text.length == 5

let flag = "🇫🇷"
assert flag.length == 1
```

## Indexing

Index a string with `[]` to get a `Character`. Indexing counts grapheme clusters, like `length`.

```fa
let name = "Fa"
let first = name[0] -- "F"
```

You can also use ranges and negative indices. Slicing with a range copies the selected characters into a new `String`:

```fa
let someString = "hello"
let middle = someString[1..3]      -- "el"  (end-exclusive)
let inclusive = someString[1..=3]  -- "ell" (end-inclusive)
let fromSecond = someString[1..]   -- "ello"
let last = someString[-1]          -- "o"
```

Ranges follow the same rules everywhere in Fa: `..` excludes the upper bound, `..=` includes it. See [ranges](/values/ranges).

### Slices

`let` copies the selected characters. To refer to them without copying, declare a **slice** with `use`: a reference into the string, like an [alias](/values/declaring-values#references-with-use). A slice of `text` has the type `String in text`.

```fa
mutable text = "hello world"

use word = text[0..5] -- a slice: no copy
console.log(word) -- "hello"

text += "!" -- mutates `text`: it may reallocate its characters
console.log(word) -- error: `word` was invalidated: declare it again with `use word = text[...]`
```

A string owns its characters, so any mutation of the string invalidates its slices: appending to it, or replacing it, as in `person.name = "Bao"`. See [mutations](/advanced/owned-references#mutations).

As with collections, an index or a range that the compiler cannot prove to be in bounds `throws IndexError`. Use `get` to receive an optional instead:

```fa
let character = try text[index]      -- throws IndexError if `index` is out of bounds
let maybeCharacter = text.get(index) -- Character?
```

See [errors](/values/errors) and [optionals](/values/optionals).

## Iterating over a string

A `for` loop over a string visits each `Character`:

```fa
for character in text {
	console.log(character)
}

for character, index in text {
	console.log("{index}: {character}")
}
```

:::tip
Indexing is `O(1)` for ASCII text. For other text, grapheme clusters have different sizes: the first indexing after a change scans the string, and the next ones scan at most 64 characters. When you are scanning a whole string, iterate over it instead: iteration is `O(1)` per character.
:::

## Bytes

A string stores its text as **UTF-8 bytes**, like in Rust and Go. Characters, indexing and `length` are views over these bytes: they never store the text a second time.

- `bytes()` gives the UTF-8 bytes, as an `Array<Byte>`. It is the way to work on raw text, like a parser does: comparing bytes is faster than comparing characters, and every ASCII character is one byte.
- `characters()` gives the characters, as an `Array<Character>`. It is a view of the string: indexing it, iterating over it or reading its `length` reads the string in place. The characters are copied out only when the array is kept and changed.

```fa
let text = "naïve"

for byte in text.bytes() {
	if byte == 32 { -- a space
		console.log("found a space")
	}
}

console.log(text.bytes().length)      -- 6: "ï" takes two bytes
console.log(text.characters().length) -- 5
```

Reading `text.bytes()` or `text.characters()` without keeping the array (`text.bytes()[i]`, `for byte in text.bytes()`) builds no array.

## Capacity

A `mutable` string that grows, with `+=`, keeps room for more bytes: its **capacity**, like the capacity of a [collection](/collections/arrays#capacity). When you know how long a string will get, reserve the room up front:

```fa
mutable output = String.from("", capacity = 4096) -- room for 4096 bytes

for line in lines {
	output += line
	output += "\n"
}

output.reserve(1024) -- room for 1024 more bytes
```

- `String.from(text, capacity = n)` copies `text` into a new string with room for at least `n` bytes.
- `capacity` is a read-only property, in bytes: what the string can hold without allocating.
- `reserve(additional)` makes room for at least `additional` more bytes. It requires a `mutable` string and declares `mutates self`: like `+=`, it may move the bytes, which invalidates the [slices](#slices) of the string.

On JavaScript, the engine manages the memory of strings: `reserve` does nothing and `capacity` is the `length`, so a program must never depend on its exact value.

## Grapheme clusters

Every operation that counts or splits characters works on grapheme clusters: `length`, indexing, slicing, iteration, and the [string operations](/operations/string) that take or return a position (`indexOf`) or that search and split (`includes`, `split`, `replace`). A search never matches half of a grapheme cluster: `"e\u{301}"` (an `e` with a combining accent, one character) doesn't include `"e"`.

`==` compares the code points exactly, with no Unicode normalization, as in JavaScript: `"é"` written as one code point is not equal to `"e\u{301}"`.

The segmentation into grapheme clusters follows the Unicode rules (UAX #29) of the Unicode version supported by the compiler. On JavaScript targets, it is done by `Intl.Segmenter`, so it follows the Unicode version of the JavaScript engine.

## String API

The string API is close to the JavaScript `String` API, with grapheme clusters instead of UTF-16 code units. See [string operations](/operations/string) for the list of methods.

:::implementation
## Implementation details

### Native

- A `String` is UTF-8, 24 bytes: a pointer, a length in bytes and a capacity with flags (static, inline, ASCII). A string of at most 23 bytes is stored inline, without allocation. A literal is stored in read-only data, with its grapheme count computed at compile time, and copied only when a mutable string is created from it.
- **Small strings never touch the heap.** Besides inline strings, a local string that doesn't escape its function (it is not returned, thrown, stored in a field, a collection or a module-level value, nor captured by an escaping closure or a task; the analyzer lists them, `Analysis::non_escaping`) lives in a stack buffer of the function, up to 64 bytes. A string goes to the heap only when it is longer, or escapes.
- **Capacity.** `String.from(text, capacity = n)` and `reserve(additional)` allocate the heap block at once (an inline string moves to the heap when it reserves more than fits inline); `capacity` reads the capacity word without its flags (23 for an inline string, 0 for a literal).
- **Characters are views.** A `Character` is the bytes of one grapheme cluster: inline when they are short, otherwise a pointer and a length into the string's bytes. `characters()` is a view of the string, like `bytes()`: indexing it, reading its `length` or iterating over it reads the string in place. Only an `Array<Character>` that the program keeps and changes is built, of compact characters.
- `length`, indexing and slicing scan the grapheme boundaries with the compiler's UAX #29 tables; a string known to be ASCII skips the scan (one byte per character). A heap string caches its grapheme count, and builds breadcrumbs (the byte offset of every 64th grapheme) at the second indexing after a write; a write clears both.
- A slice (`String in text`) is a pointer and a byte length.
- **The parts of `split` are views** of the text when the compiler proves that the text outlives them unchanged: the call, possibly followed by `filter`, initializes a `let`; the source is a local or module-level value that no reference designates, and no code writes it while the parts live. A view is a pointer and a byte length; keeping a part in another value copies its bytes. Parts of an inline string view a copy in the function frame, so a module initializer uses copies instead. A loop over a split by a nonempty literal reads views without testing their representation. A one-byte separator is counted first to allocate the array once.
- A character read from a literal (`letters[i]`) is a view of the literal's bytes, which live as long as the program.
- `<` compares the UTF-8 bytes, which orders by code point.
- Equality compares two inline values word by word. Otherwise it checks byte lengths before reading bytes; empty and one-byte strings compare inline, and longer strings use the shared byte comparison. Reading a compact ASCII character array produces an inline one-byte value, without borrowing the source's storage.

- **Destination passing.** A function that builds a mutable string and returns it, possibly with a final suffix, is expanded into an append destination when its result is appended there. The function has no effects, errors, early returns, captures or calls of program functions, and its arguments cannot read the destination.

### JavaScript

- A `String` is a JavaScript string. Grapheme operations go through a shared `Intl.Segmenter` (`granularity: "grapheme"`), with an ASCII fast path (`/^[\x00-\x7F]*$/`) where `length` and indexing are the JavaScript ones.
- `<` compares by code point, not by UTF-16 code unit: it compiles to a helper that falls back to a code point comparison only when the strings contain surrogates.
- A slice is a copy (`substring`), since JavaScript strings are immutable: invalidation is a compile-time rule only.
- `characters()` is a JavaScript array of strings (one per grapheme cluster): the semantics of the view are the same, since nothing can change the string while the array is read. `String.from(text, capacity = n)` is `text`, `reserve` compiles to nothing (its argument is still evaluated), and `capacity` is the `length`.
:::
