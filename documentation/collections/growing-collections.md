# Growing collections

A growing collection only ever accepts new elements. Nothing is removed until the whole collection is destroyed.

Giving up removal buys a strong guarantee: **any reference to an item stays valid for the entire lifetime of the collection**. A reference never has to be declared again, even after the collection has grown. Only the references *into* an item, like the slices of its strings, are invalidated when the item itself is replaced (see [mutations](/advanced/owned-references#mutations)).

Giving up removal also makes them **faster**: there is no free slot to track and none to skip, so adding an element is a single write after the last one, and iterating reads dense memory, with no occupancy test. See [how it works](#how-it-works).

Storage can be planned with `from(source, capacity = n)`, the read-only `capacity` property and `reserve(additional)`. See [capacity and reservation](/collections/arrays#capacity-and-reservation) for the shared rules and effects.

## When to use one

Growing collections suit data that accumulates until it is all discarded at once:

- a chat application storing messages,
- a parser accumulating tokens from an input stream,
- a logger accumulating log lines,
- a compiler's arena of syntax nodes.

## Available variants

Every standard collection has a growing counterpart, except `OrderedColumnar`:

- `Array` → `GrowingArray`
- `Set` → `GrowingSet`
- `Map` → `GrowingMap`
- `Pool` → `GrowingPool`
- `Columnar` → `GrowingColumnar`

They are created like any other collection, from a list literal typed by the context, or with `from` (see [creating collections](/collections/arrays#creating-collections)).

The methods are identical, minus the ones that remove or move elements: no `delete`, no `take`, no `deleteWhere`, no `pop`, no `clear`, and for `GrowingArray` no `insert`, `sortInPlace` or `reverseInPlace`. None of the remaining methods declares the `invalidates` effect, and adding an element only declares `mutates self.length` (`GrowingArray`) or `mutates self.size` (the others), which invalidates nothing.

| Collection | Operation | Effect |
| --- | --- | --- |
| `GrowingArray` | `push` | `mutates self.length` |
| | `array[index] = value` | `mutates self[index]` |
| `GrowingMap` | `map[key] = value` | `mutates self[key], self.size`: it may add the key |
| | `getOrInsert` | `mutates self.size` |
| `GrowingSet` | `add` | `mutates self.size` |
| `GrowingPool`, `GrowingColumnar` | `add` | `mutates self.size`: returns a reference, distinct from every existing one |

Writing an element or a value replaces it: the references to it stay valid, and only the references into the memory the old value owned are invalidated. As in a `Set` and a `Map`, the values of a `GrowingSet` and the keys of a `GrowingMap` are read-only.

## Example

```fa
mutable humans: GrowingPool<Human> = []

use john = humans.add({ name = "John", age = 30 })
use jane = humans.add({ name = "Jane", age = 28 })

-- adding more elements never invalidates existing references
humans.add({ name = "Alice", age = 25 })
humans.add({ name = "Bob", age = 32 })

john.age += 1
console.log(john.age)  -- 31
console.log(jane.name) -- "Jane"
```

Note the `use`: `add` returns a reference into the collection, and `let` would copy it, leaving the stored value untouched.

## Comparison with the standard collections

```fa
mutable array: Array<Human> = [{ name = "Ana", age = 30 }]
use first = array[0]
array.push({ name = "Bao", age = 25 })
console.log(first.name) -- error: `first` was invalidated by `array.push`: declare it again with `use first = array[...]`

mutable growing: GrowingArray<Human> = [{ name = "Ana", age = 30 }]
use firstGrowing = growing[0]
growing.push({ name = "Bao", age = 25 })
console.log(firstGrowing.name) -- "Ana", guaranteed valid
```

`Array.push` declares `mutates self`, which invalidates every reference into the array; `GrowingArray.push` only declares `mutates self.length`. The compiler enforces this difference: it is part of the type, not a convention.

Because their elements never move, a `GrowingPool` and a `GrowingColumnar` are among the collections that can hold the objects linked by [relations](/advanced/relations), with `Pool` and `Columnar`.

## How it works

Each growing collection is its standard counterpart without the machinery that removal needs:

| Collection | Storage | Difference with the standard collection |
| --- | --- | --- |
| `GrowingArray` | **segments** whose size doubles, for example 8 items, then 8, 16, 32… | `push` never moves the items: a full segment is kept, and a new one is added. Reading `array[index]` finds the segment with a few bit operations. |
| `GrowingPool` | pages, filled one after the other | No occupancy bitmap and no free slots: `add` writes after the last element, and iteration reads every slot. |
| `GrowingColumnar` | columns split into pages, filled one after the other | Same as `GrowingPool`, column by column. |
| `GrowingMap` | entries in pages, plus an index | No free slots among the entries, which are dense. |
| `GrowingSet` | values in pages, plus an index | Unlike a `Set`, whose values are stored in the hash table and move when it grows, the values live in stable pages, and the index holds their slot numbers: references survive insertions. `includes` reads one more place in memory than in a `Set`. |

A `GrowingArray` is a little slower to index than an `Array`, since it has to find the segment first, but iterating over it is as fast. A `GrowingArray` created with a `capacity` (`GrowingArray<Integer>.from([], capacity = 1000)`) holds that many items in its first segment: as long as it stays within its capacity, it is a single block, indexed as directly as an `Array`, and it keeps growing by segments past it. The other growing collections are faster than their standard counterparts, except `GrowingSet`, which trades some `includes` speed for stable references.

## Freeing the memory

A growing collection is destroyed like any other value: at the end of its scope, or when the value that owns it is destroyed. Everything it holds is released in one step, which is what makes it fast.

```fa
function parse = (source: String): SyntaxTree => {
	mutable tokens: GrowingArray<Token> = []
	-- ... fill `tokens`, hold references into it freely ...
	return buildTree(tokens)
} -- every token is released here, in one deallocation
```

To keep the data beyond the scope, return the collection itself rather than references into it.

## Emptying a growing collection

There is no `clear`: no method of a growing collection invalidates a reference. To empty one, assign an empty collection to it:

```fa
mutable tokens: GrowingArray<Token> = []

for line in lines {
	tokens = [] -- empties `tokens`: its memory is kept for the next line
	for word in line.split(" ") {
		tokens.push(Token { text = word })
	}
	console.log(tokens.length)
}
```

The assignment replaces the whole collection, so, like any assignment, it invalidates the references into the old one. The memory of the collection is not released: the next elements reuse it, which makes a growing collection a good buffer to fill again and again. It is released when the collection is destroyed.

:::implementation
## Implementation details

### Native

- **Returned collections.** In synchronous compilation mode, collection functions receive an optional bounded caller buffer and return the final header pointer. Fresh literals, supported transforms and proven moved immutable locals forward it through calls and return branches. A mutable or escaping owner supplies no frame buffer, so construction starts on the heap. Constant returned locals can share read-only storage with a proven immutable caller. Arrays and growing arrays construct aggregate-returning literal items directly in their final slots; completed items still drop on an error. Object-columnar chunks retain their larger heap layout.

- **Reviewed storage limits.** The collection byte budget is 2 KiB; the total is 4 KiB per non-recursive frame and 512 bytes in a resolved direct-call cycle (including methods and generic instances). Literals reserve their exact aligned size. Non-recursive calls receive at most `min(2 KiB, remaining frame budget)`. In a direct-call cycle, all call-result offers share 128 bytes per frame inside the 512-byte total: each receives `min(128 − offer bytes reserved, 512 − collection bytes reserved)`, with no offer below 64 bytes. Exact literals count against the total in either lowering order; forwarded offers reserve no frame bytes. The factor-two recursion-depth gate covers call-result offers (one and four live splits), with native measurements under a 256 KiB stack limit; it excludes exact literals. Indirect recursion is not detected. Larger objects/tuples use caller output slots; a layout with at most two scalar leaves and 16 bytes on 64-bit targets, or exactly one scalar leaf on wasm32, returns by value. Small throwing results keep `{ value, error }`; register results store directly into their final owner. An output marked `noalias` is unreachable from arguments, captures and globals during the call. Native x86-64/aarch64 functions probe crossed stack pages.

- **Immutable literals (current LLVM backend).** Constant `let` values use read-only data for the supported element/key shapes. Runtime local literals of `GrowingArray`, `GrowingMap`, `GrowingSet` and `GrowingPool` can use a fixed frame buffer within the 2 KiB collection and 4 KiB non-recursive frame budgets. Segments/chunks and indices are included; items keep their addresses. Runtime object `GrowingColumnar` chunks exceed this budget and keep heap storage. Captured values and asynchronous programs keep the heap path. Static drops do nothing; frame drops destroy owned items without freeing the buffer. Static data copies for a mutable owner. An owning or uncertain use of a runtime literal chooses the heap before construction; later growth uses ordinary stable segments and never moves the old items. Input-sized constructors are not destination-buffer lowered yet.

- **`GrowingArray`** is a **segmented list** (Zig's `SegmentedList`) on every native target, including wasm32 and the freestanding one; it never reserves address space (see "GrowingArray native layout" in `design/decisions.md`). With a first segment of `2^k` items, segment `s` holds `2^k` items for `s = 0` and `2^(k + s - 1)` items after, so every segment after the first doubles the capacity. Item `i` lives in segment `s = 0` when `i < 2^k`, and otherwise in segment `s = floor(log2(i >> k)) + 1` (one `clz`), at offset `i - 2^(k + s - 1)`.
	- **`k` belongs to the array**, not to its item type: it is stored in the header, stays fixed while the array is non-empty, and is read once before a loop. By default the first segment fills about 64 bytes. With a capacity (`from(items, capacity = n)`, or `reserve` on an empty array), `k` is the smallest value with `2^k >= n`: the array is one contiguous block until it outgrows its capacity. `reserve` on a non-empty array allocates the next segments ahead. On an empty array it may replace retained segments with one larger first segment, keeping at least the old total capacity.
	- **The header** holds the length, `k`, the cursor and the end of the current segment (`push` is one compare, one store and one increment), and the segment pointers. It is kept small: a few pointers inline, the others in a table allocated when they are needed (the table may move, the segments never do).
	- **Iteration, copies and the read-only methods** run segment by segment: a plain loop or one block copy per segment. No method copies the array into a flat one, except those that produce an `Array`.
	- **`x = []`** on a `GrowingArray` held by a `mutable` variable or field drops the items and keeps the segments: the length goes back to 0 and the cursor to the first segment. The other growing collections keep their chunks, pages and index the same way.
- **`GrowingPool`** is a list of chunks with a bump pointer in the last one: no bitmap, no free list, no per-slot state. Iteration walks every slot of every chunk; only the last chunk is partially filled. Destruction frees the chunks, and visits the elements only if they own memory.
- **`GrowingColumnar`** is the columnar layout with a bump index instead of the occupancy bitmap and the free lists.
- **`GrowingMap`** is the compact dict of `Map` without the occupancy bitmap and the free lists: entries are appended to the last chunk, and the index has no tombstones.
- **`GrowingSet`** is a `GrowingMap` without values: a dense, chunked arena of `{ hash, value }` plus a Swiss-table index of `u32` slot numbers. The index rehashes freely, the values never move.
- **References** are pointers to the item, slot or entry, except for `GrowingColumnar`, where they are slot indices.

### JavaScript

JavaScript objects never move, so the growing collections only differ from their standard counterparts by the methods they lack:

- `GrowingArray` is a JavaScript array.
- `GrowingPool` is a JavaScript array of its elements: there is no deletion, so the identity-based `Set` used by `Pool` is not needed. A reference is the object, or the index for primitive elements.
- `GrowingColumnar`, `GrowingMap` and `GrowingSet` use the representation of `Columnar`, `Map` and `Set`.
:::
