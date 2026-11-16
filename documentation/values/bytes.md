# Bytes

`Bytes` is a sequence of bytes of a fixed length: the content of a file, a network message, an image, a hash. It is the raw data that input and output work with, and it is not text: a [`String`](/values/strings) is made from bytes by decoding them.

```fa
let greeting = Bytes.from([104, 105])
let empty = Bytes.filled(16) -- 16 bytes, all zero

assert greeting.length == 2
assert greeting[0] == 104
```

`Bytes` is a `Uint8Array` on JavaScript targets, and a block of memory natively.

## Reading and writing bytes

`Bytes` reads like an [`Array<Byte>`](/collections/arrays): `length`, indexing, negative indices, ranges, iteration, `==` (same length and same bytes). An index that the compiler cannot prove in bounds `throws IndexError`, and `get` returns an optional.

```fa
let data = Bytes.from([1, 2, 3, 4])

let first = data[0]     -- a Byte
let last = data[-1]
let middle = data[1..3] -- a copy: Bytes [2, 3]

for byte in data {
	console.log(byte)
}
```

The bytes of a `mutable` value can be replaced, and `use` declares a slice without copying, as for an array:

```fa
mutable buffer = Bytes.filled(4)
buffer[0] = 255

use header = buffer[0..2] -- a slice: no copy
```

A slice reference is a view of a range of items. You cannot assign it as a whole, even with the same length: `part = Bytes.filled(2, 7)` is an error when `part` refers to a slice. Assign its items through a mutable view, using a loop to copy a range, or declare the reference with `mutable use` and reseat it with `reuse part = buffer[1..3]`. A slice copied into an ordinary `mutable` value remains assignable.

The length never changes: there is no `push`, `reserve`, `capacity` property or capacity option in `from` or `filled`. Spare storage cannot be used by a fixed-length value; `filled(n)` already allocates exactly the requested length. To build a sequence of bytes of unknown length, fill an `Array<Byte>` and convert it with `Bytes.from(array)`.

| Operation | Returns |
| --- | --- |
| `Bytes.from(items: Array<Byte>)` | `Bytes`, a copy of the items |
| `Bytes.filled(length: Integer<min = 0>, value: Byte = 0)` | `Bytes` of `length` times `value` |
| `length` (a property) | `Integer<min = 0>` |
| `isEmpty()` | `Boolean` |
| `get(index: Integer)` | `Byte?` |
| `toArray()` | `Array<Byte>` |
| `toString()` | the bytes in Fa notation, like an array: `[104, 105]`. It never decodes them |

## Text and bytes

Text becomes bytes by **encoding** it, and bytes become text by **decoding** them. Both are explicit, and the encoding is always UTF-8:

```fa
let bytes = "héllo".toUtf8()         -- Bytes, 6 bytes
let text = try String.fromUtf8(bytes) -- "héllo"
```

| Operation | Returns |
| --- | --- |
| `text.toUtf8()` | `Bytes`. A [lone surrogate](/values/strings#surrogate-pairs) has no encoding: it is written as `U+FFFD` (`�`) |
| `text.utf8Length()` | the length of `text.toUtf8()`, without building it |
| `String.fromUtf8(bytes)` | `String`; `throws Utf8Error` when the bytes are not valid UTF-8. A byte order mark is kept, as the character `U+FEFF` |
| `String.fromUtf8Lossy(bytes)` | `String`; each invalid sequence becomes `U+FFFD` instead of an error |

`Utf8Error` has one field, `position: Integer`: the index of the first byte that is not valid UTF-8.

The `length` of a string is not the number of its bytes: it counts UTF-16 code units, as in JavaScript. A limit expressed in bytes is checked with `utf8Length()`:

```fa
let name = "Zoé"

assert name.length == 3
assert name.utf8Length() == 4
```

`toString()` on bytes prints them, like on an array. It doesn't decode them: use `String.fromUtf8`.

## Input and output

Input and output are **bytes first**: a function that reads gives `Bytes`, a function that writes takes `Bytes`, and nothing is decoded unless the program asks for text. Copying a file, serving an image or forwarding a request never looks at the content.

Each one has a text variant, named with `Text`, which decodes or encodes UTF-8:

| Bytes | Text |
| --- | --- |
| `File.read(path)`: `Bytes` | `File.readText(path)`: `String`; a file that is not valid UTF-8 is a `FileError` |
| `File.write(path, bytes)` | `File.writeText(path, text)` |
| `request.body`, `response.body`: `Bytes` | `request.text()`, `response.text()`: `String`, decoded like `String.fromUtf8Lossy`, as `fetch` does |

Parsers of text formats take either: `Json.parse` accepts `Bytes` as well as a `String`, and reads the bytes directly, without building a string first.

`console.log` writes text. `@readFile` gives a `String` at compile time: the file must be valid UTF-8.

:::implementation
## Implementation details

The analyzer rejects whole-slice assignment before lowering, including through aliases and after `reuse`; assigning items and reseating references are separate operations. A contiguous copy loop over flat items can be lowered to one block copy. No replacement of a view's header may silently leave its owner unchanged.

### Native

- A `Bytes` has the layout of an `Array<Byte>` (see [arrays](/collections/arrays)), without spare capacity. A slice is a pointer and a length.
- `Bytes.from(array)` and `toArray()` copy, or move the block when the source is not used afterwards.
- `toUtf8()` and `String.fromUtf8` copy, or move the block of a heap string or of the `Bytes` when the source is not used afterwards. `String.fromUtf8` validates the bytes in one vectorized pass, which also computes the ASCII flag of the string. See [strings](/values/strings).
- Reading text (`File.readText`) reads the bytes into the block that becomes the string, then validates it: there is no second copy and no transcoding.

### JavaScript

- A `Bytes` is a `Uint8Array`. A copy is `slice()`, a slice declared with `use` is `subarray()` (a view of the same buffer).
- `Bytes.from(array)` is `Uint8Array.from(array)`, `Bytes.filled(length)` is `new Uint8Array(length)`, followed by `fill(value)` when the value is not zero.
- `==` is a helper that compares the lengths, then the bytes.
- `toString()` is the helper that prints arrays, not `Uint8Array.prototype.toString` (which gives `104,105`).
- `toUtf8()`, `String.fromUtf8` and `String.fromUtf8Lossy` use a shared `TextEncoder` and two shared `TextDecoder`s (see [strings](/values/strings)).
:::
