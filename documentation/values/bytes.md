# Bytes

`Bytes` is a sequence of bytes of a fixed length: the content of a file, a network message, an image, a hash. It is the raw data that input and output work with, and it is not text: a [`String`](/values/strings) is made from bytes by decoding them.

```fa
let greeting = Bytes.from([104, 105])
let empty = Bytes.filled(16) -- 16 bytes, all zero

assert greeting.length == 2
assert greeting[0] == 104
```

`Bytes` is a collection: keeping both an original and a second owner requires `copy`; a last use moves it. Objects, tuples, optionals and unions owning `Bytes` follow the same rule. This differs from `String`, which is an ordinary value.

```fa
mutable original = Bytes.from([1, 2])
mutable duplicate = copy original
duplicate[0] = 3
console.log(original[0], duplicate[0]) -- 1 3
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

Slices share the [range rules](/values/ranges#slicing-with-a-range) of arrays and strings: normalize negative endpoints, allow `start == length`, return an empty slice for reversed normalized bounds, and allow `[..]` on an empty value. An inclusive end must name an existing byte; length is a valid end only for an exclusive range. For example, `Bytes.filled(2)[0..=2]` is a compile error.

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

Decoding follows the WHATWG UTF-8 algorithm on every target and at compile time. Lossy decoding writes one U+FFFD per maximal invalid subpart; for example, the truncated bytes `[240, 159]` decode to one replacement character, while `[237, 160, 128]` decode to three. `Utf8Error.position: Integer` is the index of the first byte of the first invalid subpart. A byte order mark is retained.

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

The current `TcpStream` buffers still use `Array<Byte>`; their migration to `Bytes` is deferred to the standard library pass.

`console.log` writes text. `@readFile` gives a `String` at compile time: the file must be valid UTF-8.

:::implementation
## Implementation details

The shared slice proof requires an inclusive end to name an existing byte and checks unproven bounds at runtime. The analyzer rejects whole-slice assignment before lowering, including through aliases and after `reuse`; assigning items and reseating references are separate operations. A contiguous copy loop over flat items can be lowered to one block copy. No replacement of a view's header may silently leave its owner unchanged.

### Native

- A `Bytes` has the layout of an `Array<Byte>` (see [arrays](/collections/arrays)), without spare capacity. A slice is a pointer and a length.
- The native lowering keeps `Bytes` distinct from arrays, including in constants, optionals and unions. An owning value points to its block header; a borrowed value tags the address of a two-word descriptor (payload pointer and pointer-sized byte length) with its low bit. The descriptor is 16 bytes on 64-bit targets and 8 on wasm32. Borrowed calls preserve this tag; `use` and `reuse` snapshot the descriptor, while whole-place aliases retain the owner's place. Owning copies detach borrowed or read-only storage. Byte comparison and hashing always read raw bytes, even when they resemble a string's cut-surrogate edge form. Array traversal mechanics use a temporary stack header that borrows the same payload.
- `Bytes.from(array)` and `toArray()` copy, or move the block when the source is not used afterwards.
- `toUtf8()` and `String.fromUtf8` copy, or move the block of a heap string or of the `Bytes` when the source is not used afterwards. `String.fromUtf8` validates the bytes in one vectorized pass, which also computes the ASCII flag of the string. See [strings](/values/strings).
- Large byte payloads reserve the existing String metadata prefix (32 bytes on 64-bit targets, 20 on wasm32); it is unread while the payload is Bytes. A dead heap String transfers this block to Bytes after releasing breadcrumbs and replacing any lone surrogate in place. Decoding a dead prefixed block validates first, then initializes the Unicode metadata before exposing the String. Small combined blocks keep their array layout and an even allocation extent; this padding never extends the logical byte range. They can become a String prefix in place (on wasm32, at most 96 bytes shift within the same allocation). Live inputs and incompatible array storage are copied. Conversion to an array moves an ordinary compatible block, otherwise builds an ordinary array allocation. Source evaluation and cleanup remain ordered, and an invalid consuming decode releases its input before raising `Utf8Error`.
- Reading text (`File.readText`) reads the bytes into the block that becomes the string, then validates it: there is no second copy and no transcoding.

### JavaScript

- A `Bytes` is a `Uint8Array`. A copy is `slice()`, a slice declared with `use` is `subarray()` (a view of the same buffer).
- `Bytes.from(array)` is `Uint8Array.from(array)`, `Bytes.filled(length)` is `new Uint8Array(length)`, followed by `fill(value)` when the value is not zero.
- `==` is a helper that compares the lengths, then the bytes.
- `toString()` is the helper that prints arrays, not `Uint8Array.prototype.toString` (which gives `104,105`).
- `toUtf8()`, `String.fromUtf8` and `String.fromUtf8Lossy` use a shared `TextEncoder` and two shared `TextDecoder`s (see [strings](/values/strings)).
### Compile-time interpreter

A borrowed range keeps the stable owner slot and normalized bounds. Nested indices, iteration and callback items resolve back to that owner; they do not copy the collection or its items. `reuse` changes the saved designation at runtime. Array sorting and reversal operate on the designated items. An owning read or explicit copy materializes an independent value under the interpreter's memory budget.

:::
