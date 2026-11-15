# Pools

A `Pool` stores unordered values with no uniqueness constraint. It is the collection to reach for when you need many objects to live somewhere and stay referenceable.

> Other languages call this an *object pool*, a *slab*, a *hive* (C++26 `std::hive`) or an *arena*.

Insertion is fast, and stored objects keep a **stable address** even as the pool grows (see [how it works](#how-it-works)).

```fa
mutable pool: Pool<Human> = []

use john = pool.add({ name = "John", age = 20 })
use jane = pool.add({ name = "Jane", age = 21 })
pool.add({ name = "John", age = 22 })

for human in pool {
	console.log(human.name) -- "John", "Jane", "John", in no guaranteed order
}
```

`add` returns a **reference** to the stored value. Bind it with `use` to reach the object later: `let` would copy the value, and modifying the copy would leave the pool untouched.

```fa
john.age += 1 -- modifies the Human stored in the pool
```

An object literal given to `add` is built **directly in the pool**: no temporary object is created and then copied. Keep the reference with `use`:

```fa
type Point = { x: Integer, y: Integer }

mutable points: Pool<Point> = []

use point = points.add({ x = 3, y = 4 }) -- good: a reference to the point stored in the pool
point.x += 1 -- modifies the stored point
```

With `let`, the point is built in the pool, then copied out of it:

```fa
let point = points.add({ x = 3, y = 4 }) -- bad practice: `point` is a copy, not the stored point
```

A value with more fields than the element type is copied into the pool, and its extra fields are dropped: see [collections of supersets](/advanced/structural-typing#collections-of-supersets).

Storage can be planned with `from(source, capacity = n)`, the read-only `capacity` property and `reserve(additional)`. See [capacity and reservation](/collections/arrays#capacity-and-reservation) for the shared rules and effects.

## Removing

Remove a value with `delete`, passing a reference to it:

```fa
pool.delete(john)

console.log(john.name) -- error: `john` was invalidated by `pool.delete`: declare it again with `use john = ...`
console.log(jane.name) -- fine: `jane` is another element
```

`delete` returns nothing; `take(element)` removes the element the same way and returns it.

Deleting a value doesn't move the others: `delete(element)` declares `invalidates element`, which invalidates the references that may designate the deleted value, and only them. The compiler knows that `john` and `jane` are different elements: a reference returned by `add` is distinct from every reference that existed before that `add` (see [may designate](/advanced/reference-checking#may-designate)).

When the compiler can't prove that a reference designates another element, it is invalidated too, and must be declared again before it is used:

```fa
use someone = pool.find((human) => human.age > 30) -- type: `(Human in pool)?`

if someone is not none {
	pool.delete(someone)
	console.log(jane.name) -- error: `jane` was invalidated by `pool.delete`: `someone` may be `jane`
}

use jane = pool.find((human) => human.name == "Jane") -- declared again
```

References stored in fields are not affected: they are [relations](/advanced/relations), cleaned up when their target is deleted. This cleanup writes the objects that pointed at the deleted one, and counts as an effect of `delete` (see [reference checking](/advanced/reference-checking#relations)).

Removing values during a loop over the pool is a compile error, because it would invalidate the iteration. Use `deleteWhere` instead:

```fa
pool.deleteWhere((human) => human.name == "John")
```

## Stable references

This is what distinguishes a pool from a set or an array:

```fa
mutable pool: Pool<Human> = []

use john = pool.add({ name = "John", age = 20 })

-- adding more elements does not move the existing ones
pool.add({ name = "Alice", age = 25 })
pool.add({ name = "Bob", age = 32 })

john.age += 1
console.log(john.age) -- 21: `john` is still valid
```

Compare with an array, where `push` may reallocate the storage and invalidate every reference into it.

| Operation | Effect | Invalidates |
| --- | --- | --- |
| `add` | `mutates self.size` | nothing |
| `delete(element)`, `take(element)` | `invalidates element`, which implies `mutates self.size` | the references that may designate `element` |
| `deleteWhere`, `clear` | `mutates self` | every local reference into the pool |
| `size`, `find`, `includes`, iteration | none | nothing |

`pool.size` is the number of elements. A pool has a `size` rather than a `length`: its elements have no positions, so there is no index to bound.

Two pools are equal when they hold equal elements the same number of times, in any order. Relation fields are not compared, as for every [equality](/operations/logical#equality) of linked objects. The same holds for columnars.

When you never delete, use a [`GrowingPool`](/collections/growing-collections): its type guarantees that references stay valid for the whole lifetime of the collection, and it iterates faster, since it has no free slots to skip.

## How it works

A pool stores its elements in **pages** of fixed size. A page is allocated when the previous ones are full, and is never moved: this is why an element never moves either, and why `add` invalidates nothing.

Each page keeps an **occupancy bitmap**, one bit per slot, which tells the live slots from the free ones:

- `add` fills a free slot when there is one, taking it from the first page that has one, and otherwise appends the element after the last one. Filling the first pages first keeps the elements packed at the start of the pool.
- `delete` marks the slot as free, without moving anything. A page that becomes empty is released.
- `for` walks the pages in order and reads each bitmap 64 slots at a time: a run of free slots costs almost nothing, and a page is read contiguously.

The order of iteration is therefore the order of the slots, not the order of insertion: after a deletion, the next `add` can fill the hole.

| Operation | Cost |
| --- | --- |
| `add` | constant |
| `delete(element)` | constant |
| iteration | linear in the number of slots, close to an array when the pool is dense |
| `find` | linear |

A pool with many deletions and few insertions keeps holes, and iterates a little slower than a dense one. A [`GrowingPool`](/collections/growing-collections) never has holes.

## When to use a pool

Use a `Pool` when you want:

- fast insertion,
- stable references to stored objects,
- no uniqueness checks,
- no meaningful order.

Pools are the natural home for the objects of a graph, a scene, or an entity system: anything where objects refer to each other. Because its elements never move, a pool is also a collection that can hold the objects linked by [relations](/advanced/relations).

## Choosing a collection

| Need | Collection |
| --- | --- |
| Ordered list, indexed access | [`Array`](/collections/arrays) |
| Unique values, fast membership | [`Set`](/collections/sets) |
| Key to value | [`Map`](/collections/maps) |
| Many objects, stable references | `Pool` |
| Iterate one field over many objects | [`Columnar`](/collections/columnars) |
| Any of the above, never removing | [growing collections](/collections/growing-collections) |

:::implementation
## Implementation details

### Native

- **Returned collections.** In synchronous compilation mode, collection functions receive an optional bounded caller buffer and return the final header pointer. Fresh literals, supported transforms and proven moved immutable locals forward it through calls and return branches. A mutable or escaping owner supplies no frame buffer, so construction starts on the heap. Constant returned locals can share read-only storage with a proven immutable caller. Arrays and growing arrays construct aggregate-returning literal items directly in their final slots; completed items still drop on an error. Object-columnar chunks retain their larger heap layout.

- **Reviewed storage limits.** The collection byte budget is 2 KiB; the total is 4 KiB per non-recursive frame and 512 bytes in a resolved direct-call cycle (including methods and generic instances). Literals reserve their exact aligned size. Calls receive at most `min(2 KiB, remaining frame budget)`, with no offer below 64 bytes. Indirect recursion is not detected. Larger objects/tuples use caller output slots; a layout with at most two scalar leaves and 16 bytes on 64-bit targets, or exactly one scalar leaf on wasm32, returns by value. Small throwing results keep `{ value, error }`; register results store directly into their final owner. An output marked `noalias` is unreachable from arguments, captures and globals during the call. Native x86-64/aarch64 functions probe crossed stack pages.

- **Immutable literals (current LLVM backend).** Constant `let` literals without relation sides use read-only headers, chunks, occupancy words and slots. Runtime local literals can use a compact read-only chunk in a fixed frame buffer (2 KiB per collection, 4 KiB per non-recursive function); a read never needs to recover its chunk by masking its address. A mutable owner of static data requires an independent heap copy with the ordinary slab layout; runtime literals with owning or unknown uses choose the heap from creation. Captured or asynchronous values, relation-bearing elements and constructions above the budgets retain the heap path. Static drops do nothing; frame drops still destroy owned elements. References and iteration borrow the original slots.

The pool is a **slab** (also called a hive or a colony): `plf::colony`, C++26 `std::hive` and the Rust `slab` crate are the references.

- **Chunks.** A `Pool<T>` allocates chunks of one compile-time size, aligned to that size: 64 KiB, or more for a large `T` so that a chunk holds at least 64 slots. Large chunks are allocated with an aligned `mmap`. The header at the start of a chunk holds the occupancy bitmap (one `u64` word per 64 slots), the number of live slots, the head of the chunk's free list, and the links to the previous and next chunks.
- **Finding the chunk of a slot** is `pointer & ~(chunkSize - 1)`: no search, no stored chunk index.
- **Free slots.** Each chunk has its own intrusive free list: the index of the next free slot is stored in the dead slot, so a slot is at least 4 bytes. The pool keeps a bitset of the chunks that have a free slot; `add` takes the lowest set bit (`tzcnt`), pops that chunk's free list, sets the occupancy bit and returns the slot pointer. When no chunk has a free slot, `add` bumps the last chunk, or allocates a new one (cold path).
- **Deletion.** `delete(ref)` runs `drop<T>` on the slot, clears its occupancy bit, pushes it on its chunk's free list, and sets the chunk's bit in the bitset. A chunk whose count drops to zero is unlinked and freed, except one spare chunk, kept to avoid allocating and freeing in a loop that adds and deletes one element.
- **Iteration.** For each chunk, for each occupancy word: `while word != 0 { slot = tzcnt(word); word &= word - 1; ... }`. A full word is iterated without branches on the bitmap. Iteration order is the slot order; the documentation only promises "no guaranteed order".
- **No compaction.** `deleteWhere` and `clear` declare `mutates self` and could move elements, but relation links are raw pointers into the slots: compacting would mean rewriting them. The first version never compacts. `clear` frees every chunk but the spare one.
- **Destruction.** A pool of flat elements is destroyed by freeing its chunks, without visiting the slots. Otherwise the occupancy bitmaps drive the `drop<T>` calls.
- **Construction in place.** `add({ … })` never builds a temporary element to copy into the slot. The field expressions that may throw or call a function are evaluated first, so that an error leaves the pool untouched and a nested `add` can't take the same slot; then `add` takes the slot and stores every field straight into it (nested literals included), sets the occupancy bit and returns the slot pointer. An existing value is moved with a `memcpy` when it has exactly the element's shape, and otherwise copied field by field into the slot, without its extra fields.
- **References.** A reference into a pool is a pointer to the slot. `size` is a field of the pool header, updated by `add` and `delete`.
- A reverse side of a relation, like `bestFriendOf: Pool<people->bestFriend>`, is not stored as a pool: it is an intrusive list through the elements. See `design/compilation-with-llvm.md`.

### JavaScript

- A `Pool` of objects is a JavaScript `Set` of these objects. `add` adds an object to the set and returns it: the reference **is** the object. A literal is created once, as `set.add({ x: 3, y: 4 })`, and never copied. An existing value is moved (the same object is added) only when it is not used afterwards and has exactly the element's fields; otherwise it is copied by the generated copy function of the element type, which reads only the element's fields, never by a spread (`{ ...value }`) that would keep the extra ones. `delete(ref)` is `set.delete(ref)`, `size` is `set.size`, and iteration is `for (const element of set)`. A `Set` compares objects by identity, which is right here: two equal humans added to a pool are two distinct elements.
- A `Pool` of primitives (`Pool<Integer>`, `Pool<String>`, …) can't use a `Set`, which would merge the duplicates. It is a slot array `Array<T>`, an occupancy array and a free list of slot indices (a plain `number[]` used as a stack). A reference is the slot index; the type of the reference names the pool, so it does not need to carry it.
- Relations are plain object fields in JavaScript: the direct side holds the target object, and the reverse side holds a `Set` of the objects that point at it.
:::
