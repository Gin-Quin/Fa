# Growing collections

Like every collection, this type moves at a proven last use; keeping both owners requires `copy value`. Storing its elements follows their own ownership category, and borrowing an element never copies it. See [structures](/advanced/memory-management#structures).

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
| all of them | `reserve(additional)` | `mutates self.capacity` |

Since their elements never move, `reserve` only allocates: it invalidates nothing.

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

A growing collection is destroyed like any other value: at the end of its scope, or when the value that owns it is destroyed. Storage is freed page by page. Elements owning strings, collections, resources, handles or relation bookkeeping still need their own cleanup; ordinary flat data and link fields need no element destruction walk.

```fa
function parse = (source: String): SyntaxTree => {
	mutable tokens: GrowingArray<Token> = []
	-- ... fill `tokens`, hold references into it freely ...
	return buildTree(tokens)
} -- token storage is released here, with owned payload cleanup when needed
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

## Links and index limits

`GrowingPool<Item, Index = UInt32>` and `GrowingArray<Item, Index = UInt32>` accept `UInt8`, `UInt16`, `UInt32` or `UInt64` as Index. This parameter belongs to the target collection and is part of its type; every stored incoming [link](/advanced/relations#links-without-a-reverse-side) uses that width. Abstract over Index with a generic parameter; collections of different widths are not interchangeable mutable views. `reserve` changes available storage, never the width or the limit.

For example, this chain chooses a 16-bit index for every link into its pool:

```fa
-- Acceptance: Index and stored links are implemented in subsequent link slices.
type Chain<Item, Index = UInt16> = {
	nodes: GrowingPool<Node, Index> = []
	link head: (Node in nodes)? = none
	type Node = {
		value: Item
		link next: (Node in nodes)? = none
	}
}
mutable chain: Chain<Integer> = {}
use first = chain.nodes.add({ value = 7 })
chain.head = first
let independent = copy chain
first.value = 9
console.log(independent.head?.value) -- 7: its head points into its own pool
```

A link denotes an initialized **whole element**, including a whole union element after narrowing. Its target cannot be a subfield, slice, union payload, GrowingMap value or GrowingSet key. GrowingColumnar is outside this first link contract. Source holders may live in an owning Array: only the target needs stable places.

The maximum count is 255 for UInt8, 65,535 for UInt16, 4,294,967,295 for UInt32, and 18,446,744,073,709,551,615 for UInt64, additionally limited by the target's usable address space. The limit counts elements, not bytes. A statically impossible size is a compile error. Otherwise `add`, `push`, `reserve` and `from` stop the program with a message naming Index and suggesting a wider type **before publishing** an unrepresentable element. This resource stop remains enabled when arithmetic overflow checks are disabled; it adds no `throws` effect and never widens silently. Test the limit yourself and throw an application error when rejection must be recoverable.

`GrowingPool` iteration is guaranteed to follow **insertion order**. A whole-element replacement preserves its place and that order; all incoming links observe the new value. Replacement writes the new outgoing link fields, including defaults. A union tag change still invalidates borrows into its old payload.

A scalar link and its optional use the same width: absence requires no extra bit. Local references still have the usual representation and checks. Ordered lists of links can use [InlineArray](/collections/inline-arrays), which preserves duplicates and does not spill, or Array for an unbounded list.

A stored link adds a containment condition to replacement: resetting a pool is allowed when all its incoming holders die with it. If a root field or another pool may hold links into it, reset the complete owner instead. Clearing one optional root does not prove every possible holder empty. See [link containment](/advanced/reference-checking#link-containment).

:::implementation
## Implementation details

### Native

- **Returned collections.** In synchronous compilation mode, collection functions receive an optional bounded caller buffer and return the final header pointer. Fresh literals, supported transforms and proven moved immutable locals forward it through calls and return branches. A mutable or escaping owner supplies no frame buffer, so construction starts on the heap. Constant returned locals can share read-only storage with a proven immutable caller. Arrays and growing arrays construct aggregate-returning literal items directly in their final slots; completed items still drop on an error. Object-columnar chunks retain their larger heap layout.

- **Reviewed storage limits.** The collection byte budget is 2 KiB; the total is 4 KiB per non-recursive frame and 512 bytes in a resolved direct-call cycle (including methods and generic instances). Literals reserve their exact aligned size. Non-recursive calls receive at most `min(2 KiB, remaining frame budget)`. In a direct-call cycle, all call-result offers share 128 bytes per frame inside the 512-byte total: each receives `min(128 − offer bytes reserved, 512 − collection bytes reserved)`, with no offer below 64 bytes. Exact literals count against the total in either lowering order; forwarded offers reserve no frame bytes. The inclusive 2× per-frame gate covers call-result offers (one and four live splits): unchecked depths must satisfy `baseline_depth ≤ 2 × final_depth + 8`, with eight frames for fixed setup overhead and native measurements under a 256 KiB stack limit. Leak-checked depths are informational; exact literals are excluded. Indirect recursion is not detected. Larger objects/tuples use caller output slots; a layout with at most two scalar leaves and 16 bytes on 64-bit targets, or exactly one scalar leaf on wasm32, returns by value. Small throwing results keep `{ value, error }`; register results store directly into their final owner. An output marked `noalias` is unreachable from arguments, captures and globals during the call. Native x86-64/aarch64 functions probe crossed stack pages.

- **Immutable literals (current LLVM backend).** Constant `let` values use read-only data for the supported element/key shapes. Runtime local literals of `GrowingArray`, `GrowingMap`, `GrowingSet` and `GrowingPool` can use a fixed frame buffer within the 2 KiB collection and 4 KiB non-recursive frame budgets. Segments/chunks and indices are included; items keep their addresses. Runtime object `GrowingColumnar` chunks exceed this budget and keep heap storage. Captured values and asynchronous programs keep the heap path. Static drops do nothing; frame drops destroy owned items without freeing the buffer. An explicit copy of static data constructs independent mutable storage; a movable runtime owner chooses its destination storage before construction. An owning or uncertain use of a runtime literal chooses the heap before construction; later growth uses ordinary stable segments and never moves the old items. Input-sized constructors are not destination-buffer lowered yet.

- **`GrowingArray`** is a **segmented list** (Zig's `SegmentedList`) on every native target, including wasm32 and the freestanding one; it never reserves address space (see "GrowingArray native layout" in `design/decisions.md`). With a first segment of `2^k` items, segment `s` holds `2^k` items for `s = 0` and `2^(k + s - 1)` items after, so every segment after the first doubles the capacity. The lookup has no branch on the index: with `t = i >> k`, `s = 63 - clz((t << 1) | 1)` and `K = 1 << k`, its start is `((K << s) >> 1) & -K`. This works for `k = 0` and for segment zero, whose start is zero; the `clz` operand is always nonzero. The directory stores the integer address `items - start * item_size`, so lookup only adds `i * item_size`. The subtraction and addition wrap at pointer width; only the final in-bounds address becomes a pointer. Freeing or walking a segment restores its actual base address. Segment zero has a zero bias, including in frame and static headers.
	- **`k` belongs to the array**, not to its item type: it is stored in the header, stays fixed while the array is non-empty, and is read once before a loop. By default the first segment fills about 64 bytes. With a capacity (`from(items, capacity = n)`, or `reserve` on an empty array), `k` is the smallest value with `2^k >= n`: the array is one contiguous block until it outgrows its capacity. `reserve` on a non-empty array allocates the next segments ahead. On an empty array it may replace retained segments with one larger first segment, keeping at least the old total capacity.
	- **The header** holds the length, `k`, the cursor and the end of the current segment (`push` is one compare, one store and one increment), and the segment pointers. It is kept small: three pointers inline and an always-valid directory pointer: it points at the inline slots until a heap directory is needed. That directory mirrors the inline pointers, grows once, and may move; the header and segments never do.
	- **Iteration, copies and the read-only methods** run segment by segment: a plain loop or one block copy per segment. No method copies the array into a flat one, except those that produce an `Array`.
	- **Native loop facts.** An ordinary `for` scan has an outer loop over live segments and a counted inner loop over each segment's live prefix. The iteration rule keeps the header stable, while item mutations, callbacks, `break`, `continue` and suspension keep their normal behavior. Byte searches retain their specialized segment search. A synchronous counted loop can also snapshot a private owner's length, first-segment exponent, directory and contiguous mode when the owner's complete use set exposes no header to another function, capture or file and the loop only indexes or measures it. For parameters, the stricter no-reference proof also rejects every call, since a push through an alias can change length without invalidating items. In a loop that also pushes, only cursor, end and length are cached; every push writes through to the real header and a segment advance refreshes cursor/end. All lowerings of an admitted push obey this same rule: after a bulk fill or copy updates the real header, `growing_loops::refresh_growing_after_append` reloads length, cursor and end together before any observation or following push. The proof walks nested loops and the owner’s complete use set; a call or capture that can reach the header rejects it. Element writes preserve these facts. Replacement, reservation, unknown receiver uses, shared/reactive owners and uncertain aliases retain the ordinary loads. No capacity is inferred and no segment moves.
	- **`x = []`** on a `GrowingArray` held by a `mutable` variable or field drops the items and keeps the segments: the length goes back to 0 and the cursor to the first segment. The other growing collections keep their chunks, pages and index the same way.
- **`GrowingPool`** is a list of chunks with a bump pointer in the last one: no bitmap, no free list, no per-slot state. Iteration walks every slot of every chunk; only the last chunk is partially filled. Destruction frees the chunks, and visits the elements only if they own memory.
- **`GrowingColumnar`** is the columnar layout with a bump index instead of the occupancy bitmap and the free lists.
- **`GrowingMap`** is the compact dict of `Map` without the occupancy bitmap and the free lists: entries are appended to the last chunk, and the index has no tombstones.
- **`GrowingSet`** is a `GrowingMap` without values: a dense, chunked arena of `{ hash, value }` plus a Swiss-table index of `u32` slot numbers. The index rehashes freely, the values never move.
- **References** are pointers to the item, slot or entry, except for `GrowingColumnar`, where they are slot indices.

### JavaScript

JavaScript objects never move, so the growing collections only differ from their standard counterparts by the methods they lack:

- `GrowingArray` is a JavaScript array.
- `GrowingPool` uses the same JavaScript object/value representations as `Pool`, with no removal operations. Object references are the objects themselves. Primitive and mixed-union borrows retain the direct slot array and its index; local reads and writes are array member accesses, and user calls pass allocation-free container/key parts. No accessor cell or reference cache is allocated.
- `GrowingColumnar`, `GrowingMap` and `GrowingSet` use the representation of `Columnar`, `Map` and `Set`.

### Compact stored links: every target

Stored links encode ordinal + 1; zero is none, including for optional links. UInt64 is subject to address-space limits. Validate count representability before publication, reservation and bulk construction, including in the interpreter and JavaScript. Proven count bounds may remove the guard. Index is invariant collection metadata in interfaces and split compilation; local aliases need not be compact.

Native GrowingPool uses stable dense pages with a power-of-two slot count and a directory: for nonzero x, i = x - 1, address = pages[i >> slotShift] + (i & slotMask) * stride(Item). A single page can use its base. GrowingArray uses its stable segment decoder. Directory growth moves no element; following a link has no bounds, generation or liveness test and stores no owner pointer. Scans use page/segment outer loops and a plain stride inner loop, with no per-item occupancy branch. Union slots contain a tag and their largest payload at fixed stride, with no per-node box. Packed variable-size nodes require a separate measured layout decision. The existing root-region inference is not this owner/ordinal contract.

On JavaScript an object target is the object itself, none is undefined, and a list is a dense array of those objects. Primitive targets use indices into their statically named collection. Stores retain targets directly, without owning copiers or per-read wrappers/maps. Replacing an element mutates its existing object in place, including its union tag, preserving identity. The engine may later visit graphs during GC; native bulk destruction timing is not promised on JavaScript.

The interpreter keeps owner identities and ordinals, exporting complete immutable graph constants symbolically rather than embedding host addresses. Partial construction/error cleanup drops initialized owned data exactly once and never follows edges.

:::
