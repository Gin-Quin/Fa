# Maps

Like every collection, this type moves at a proven last use; keeping both owners requires `copy value`. Storing its elements follows their own ownership category, and borrowing an element never copies it. See [structures](/advanced/memory-management#structures).

A map associates keys with values. Maps can be mutable or immutable.

A map is written as a list of `(key, value)` [tuples](/values/tuples). Like every collection, it is typed by the context, or created with `Map.from` when there is no context (see [creating collections](/collections/arrays#creating-collections)).

```fa
let myMap: Map<String, Integer> = [
	("foo", 12)
	("bar", 13)
]

let sameMap = Map.from([("foo", 12), ("bar", 13)]) -- no context: a `Map<String, Integer>`
```

A key written twice in a literal is an error when both are literals, like `[("foo", 1), ("foo", 2)]`: a literal has one meaning.

An empty map needs its type:

```fa
mutable inventory: Map<String, Integer> = []
```

Storage can be planned with `from(source, capacity = n)`, the read-only `capacity` property and `reserve(additional)`. See [capacity and reservation](/collections/arrays#capacity-and-reservation) for the shared rules and effects.

## Access and update

Reading a key with `map[key]` is checked at compile time when the compiler can prove the key is present. Otherwise, it `throws KeyNotFound`, so a missing key can never be mistaken for a value. Propagate the error with `try`, or handle it with `catch` (see [errors](/values/errors)):

```fa
let scores: Map<String, Integer> = [
	("ana", 12)
	("bao", 17)
]

function scoreOf = (scores: Map<String, Integer>, name: String): Integer throws KeyNotFound => {
	let score = try scores[name] -- propagates `KeyNotFound` to the caller
	score
}

let cleoScore = scores["cleo"] catch error => 0
```

The compiler proves that a key is present in three cases only, the same for every compiler:

- the map is a literal with literal keys, not modified since, and the key is one of them: `scores["ana"]` above needs no `try`,
- inside `if scores.includes(name) { ... }`, with an immutable `name`, until the map is modified,
- the key of a reference returned by `getOrInsert`.

Use `get` when absence is expected rather than exceptional. It returns an [optional](/values/optionals) reference to the value: `let` copies it, and `use` keeps the reference (see [stable references](#stable-references)):

```fa
let maybeScore = scores.get("cleo") -- Integer?
let score = scores.get("cleo") ?? 0

if maybeScore is not none {
	console.log("Cleo scored {maybeScore}") -- `maybeScore` is narrowed to `Integer`
}
```

Writing a key replaces its value, or adds the key when it is absent:

```fa
mutable counters: Map<String, Integer> = [
	("visits", 1)
]

counters["visits"] = 2
```

## Adding and removing

```fa
mutable inventory: Map<String, Integer> = []

inventory["apples"] = 3
inventory["oranges"] = 2
inventory.delete("apples")             -- true: it returns whether the key was present
inventory.take("oranges")              -- 2: removes the key and returns its value, or `none` when absent
inventory["oranges"] = 2

inventory.includes("oranges") -- true: `includes` checks the keys
inventory.size                -- 1
inventory.isEmpty()           -- false

inventory.clear()
```

## Iteration

Iterating a map yields `(key, value)` tuples, which the loop destructures:

```fa
let usersById: Map<Integer, String> = [
	(1, "Ana")
	(2, "Bao")
]

for (id, name) in usersById {
	console.log("{id} -> {name}")
}
```

Iteration order is not guaranteed. On a `mutable` map, the value is a mutable reference, and writing to it writes into the map; the key is always read-only, since changing it would change its place in the index. Keys and values can be iterated separately:

```fa
for id in usersById.keys() {
	console.log(id)
}

for name in usersById.values() {
	console.log(name)
}
```

Adding or removing keys of a map while iterating over it is a compile error.

The callbacks of the methods of a map, like `find`, `filter` or `every`, receive the key and the value, `(key, value)`. `find` returns a copy of the `(key, value)` tuple it finds, or `none`; `map` keeps the keys and maps the values into a new `Map`:

```fa
let labels = usersById.map((id, name) => "{id}: {name}") -- Map<Integer, String>
let second = usersById.find((id) => id == 2)              -- Tuple<Integer, String>?: `(2, "Bao")`
```

## Default values

`getOrInsert` returns a reference to the value of a key, inserting a default value first when the key is absent:

```fa
mutable counts: Map<String, Integer> = []

for word in words {
	use count = counts.getOrInsert(word, 0)
	count += 1
}
```

Since `getOrInsert` returns a reference, it is bound with `use`: `let` would copy the value, and the map would not be updated. The compiler knows the key of this reference: it is the key passed to `getOrInsert`.

## Keys

Any type with a well-defined equality can be a key: numbers, strings, booleans, enumerations, tuples and objects. Keys are compared with `==`, which is a deep structural equality. A type that contains a function, a task, an iterator, a reference or a [thread-safe](/advanced/concurrency#thread-safety-and-synchronization) handle has no `==`, so it can't be a key, nor a value of a [set](/collections/sets).

A key can't be modified while it is in the map: a reference to a key is read-only, even in a `mutable` map. For the same reason, the key type of a map is exact: a `Map<Hero, Integer>` is not a `Map<Human, Integer>`, even where a [structural view](/advanced/structural-typing) would accept a `Hero` for a `Human`.

```fa
let byPoint: Map<Tuple<Integer, Integer>, String> = [
	((0, 0), "origin")
]
```

## Stable references

A map stores its values in stable slots, like a [`Pool`](/collections/pools): adding a key never moves the other values. Writing a key (`map[key] = value`) declares `mutates self[key], self.size`, since it may add the key: it only invalidates the references into the memory the previous value of that key owned, and none when the values are flat, like integers. Because it may change the size, writing a key is not allowed while iterating over the map: write through the value the loop gives instead (`for (name, score) in scores { score += 1 }`). `getOrInsert` only adds a key, and declares `mutates self.size`: it invalidates nothing.

```fa
mutable scores: Map<String, Integer> = [("ana", 10)]

use score = scores.get("ana") -- type: `(Integer in scores)?`

scores["bao"] = 7 -- fine: adding a key moves no value

if score is not none {
	console.log(score) -- 10
}
```

Removing a key invalidates the references to its value only: `delete(key)` and `take(key)` declare `invalidates self[key]`. The compiler keeps the exact key of each reference when it is known, so deleting another key leaves it valid. When it can't prove that the keys are different, the reference is invalidated, and must be declared again before it is used (see [owned references](/advanced/owned-references#after-invalidation)):

```fa
mutable scores: Map<String, Integer> = [("ana", 10), ("bao", 7)]

use score = scores.get("ana")

scores.delete("bao") -- fine: "bao" is not "ana"
console.log(score)

let loser = readName()
scores.delete(loser) -- `loser` may be "ana"

console.log(score) -- error: `score` was invalidated by `scores.delete`: declare it again with `use score = scores.get(...)`

use score = scores.get("ana") -- declared again
```

`deleteWhere` and `clear` declare `mutates self`: they invalidate every reference into the map.

| Operation | Effect |
| --- | --- |
| `map[key] = value` | `mutates self[key], self.size` |
| `getOrInsert(key, value)` | `mutates self.size` |
| `delete(key)`, `take(key)` | `invalidates self[key]` |
| `deleteWhere`, `clear` | `mutates self` |
| `reserve(additional)` | `mutates self.capacity`: it moves no value |
| `map[key]`, `get`, `includes`, `size`, `capacity`, `keys`, `values` | none |

A map created with `Map<String, Integer>.from([], capacity = 1000)` has room for 1000 keys, and `reserve(additional)` makes room for `additional` more. Since the values live in stable slots, `reserve` invalidates nothing.

:::note
Stable slots cost one indirection per lookup, compared to a hash table storing its values inline. In exchange, references into a map survive insertions.
:::

## How it works

A map has two parts:

- the **entries**, the `(key, value)` pairs, stored in pages like the elements of a [pool](/collections/pools#how-it-works): they never move, and a deleted entry leaves a free slot that a later insertion reuses,
- the **index**, a hash table that holds the slot number of each entry rather than the entry itself.

Looking a key up hashes it, finds the candidate slots in the index, and compares the key of the entry. Adding a key fills a free slot, then records it in the index. When the index is too full, it is rebuilt at twice the size: only slot numbers move, never the entries, which is why references into a map survive insertions.

Iteration walks the pages of entries, not the index: it is as fast as iterating a pool, and skips the free slots with the occupancy bitmap of each page.

| Operation | Cost |
| --- | --- |
| `map[key]`, `get`, `includes` | constant on average: one probe in the index, one read of the entry |
| `map[key] = value`, `getOrInsert` | constant on average |
| `delete(key)` | constant on average |
| iteration | linear in the number of slots |

`map.size` is the number of keys. Like a set, a map has a `size` rather than a `length`: its entries have no positions.

## Maps versus objects

An object has a fixed set of fields known at compile time; a map has keys discovered at runtime.

```fa
type Settings = { host: String, port: Integer } -- known fields
let headers: Map<String, String> = []           -- arbitrary keys
```

If you find yourself writing `Map<String, Item>` with a fixed set of keys, use an object or a [`Record`](/types/type-functions#recordkeys-item) instead: it is faster and type-checked.

:::implementation
## Implementation details

### Native

- **Returned collections.** In synchronous compilation mode, collection functions receive an optional bounded caller buffer and return the final header pointer. Fresh literals, supported transforms and proven moved immutable locals forward it through calls and return branches. A mutable or escaping owner supplies no frame buffer, so construction starts on the heap. Constant returned locals can share read-only storage with a proven immutable caller. Arrays and growing arrays construct aggregate-returning literal items directly in their final slots; completed items still drop on an error. Object-columnar chunks retain their larger heap layout.

- **Reviewed storage limits.** The collection byte budget is 2 KiB; the total is 4 KiB per non-recursive frame and 512 bytes in a resolved direct-call cycle (including methods and generic instances). In per-unit builds, a function calling into another unit also uses the smaller recursive budget until recursion facts are shared across units. Literals reserve their exact aligned size. Non-recursive calls receive at most `min(2 KiB, remaining frame budget)`. In a direct-call cycle, all call-result offers share 128 bytes per frame inside the 512-byte total: each receives `min(128 − offer bytes reserved, 512 − collection bytes reserved)`, with no offer below 64 bytes. Exact literals count against the total in either lowering order; forwarded offers reserve no frame bytes. The inclusive 2× per-frame gate covers call-result offers (one and four live splits): unchecked depths must satisfy `baseline_depth ≤ 2 × final_depth + 8`, with eight frames for fixed setup overhead and native measurements under a 256 KiB stack limit. Leak-checked depths are informational; exact literals are excluded. Indirect recursion is not detected. Larger objects/tuples use caller output slots; a layout with at most two scalar leaves and 16 bytes on 64-bit targets, or exactly one scalar leaf on wasm32, returns by value. Small throwing results keep `{ value, error }`; register results store directly into their final owner. An output marked `noalias` is unreachable from arguments, captures and globals during the call. Native x86-64/aarch64 functions probe crossed stack pages.

- **Immutable literals (current LLVM backend).** A constant `let` literal with supported scalar keys uses read-only entries, index and chunk directory. A runtime local `let` literal can put all those blocks in a fixed frame buffer (2 KiB per collection, 4 KiB per non-recursive function, or the smaller recursive budget described above); the ordinary stable chunks are preserved. Larger, captured or asynchronous constructions keep heap storage. Static drops do nothing; frame drops destroy keys and values, and free no blocks. Explicit copies of static values construct independent storage. Runtime literals with owning or unknown uses start on the heap, avoiding frame-to-heap copy-out. The three layouts of the entries (chunks of 32, one dense array, the probe array itself for small integer keys) have the same read-only and frame forms; the compiler hashes the constant keys (integers, booleans, floats, strings) as the program does, and other keys keep the heap.

The map is a **compact dict**, the design of Python's `dict` since 3.6 and of Zig's `ArrayHashMap`, with a Swiss-table index.

- **Entries.** A chunked arena of `{ hash: u32, key: K, value: V }`, organized exactly like a [pool](/collections/pools): size-aligned chunks with an occupancy bitmap, per-chunk free lists, the bitset of chunks with a free slot (lowest chunk first). Keeping the hash in the entry lets the index be rebuilt without hashing the keys again, and lets a lookup compare the hash before calling the key's `==`.
- **Index.** An open-addressing table of `{ control: u8, slot: u32 }`: the control byte holds 7 bits of the hash (or the empty or deleted marker), probed by groups of 16 control bytes with SIMD (SSE2, NEON; a portable 8-byte word otherwise), like Abseil's `flat_hash_map` and Rust's `hashbrown`. The maximum load factor is 7/8; beyond it, the index is rebuilt at twice the capacity from the hashes stored in the entries. A deleted key leaves a tombstone in the index, cleared by the next rebuild.
- **Lookup.** Hash the key, probe the index groups for matching control bytes, and for each candidate compare the stored hash, then the key. A miss usually never reads an entry.
- **References.** A reference to a value is a pointer to the entry's value. `getOrInsert` returns it.
- **Hashing.** `hash<Shape>` is derived structurally per key type, like `==`. The hash function is fast and non-cryptographic, with a random seed per map to resist hash flooding. Tests can fix the seed.
- **Iteration** walks the entry chunks with `tzcnt` over the occupancy words, like a pool.

The current LLVM backend implements stable entries `{ key, value }` (no hash,
no presence word) in fixed chunks of 32 slots, with a movable directory holding
each chunk's pointer, its 32-bit occupancy word and its link in the list of
chunks with a free slot. An insertion takes the lowest free slot of the first
chunk of that list, so that a map without deletions keeps its insertion order;
sets use the same storage. Its index is probed linearly, at most half
full: a slot is one 64-bit word holding the low 32 bits of the key's hash (its
home slot, and a filter compared before an entry is read) and the entry number,
or the full hash and entry number (16 bytes) beyond 2^31 slots. Probing is still
scalar, rather than the grouped Swiss probing above. A deletion shifts the
following slots of its run back instead of leaving a tombstone, so a lookup
stops at the first empty slot, and an unsuccessful lookup gives the slot where
an insertion goes; `delete` and `take` take the key out of the index in the walk
that finds it. The index grows from its own hashes: the keys are hashed again
only when it becomes wide. Integer keys are hashed with one multiplication
between two folds (bijective) and compared bitwise in the probe loop; the other
keys use their generated `==`. Iteration currently tests the occupancy bit of
each slot rather than using `tzcnt`; the randomized hash seed remains planned.

- **Compact integer slots.** An inline map with 32-bit integer keys and values of at most 32 bits stores 8 bytes per slot. Zero has one extra slot and a separate presence bit, so every key and value bit pattern remains available. Copies, growth, reserve and shifting deletion preserve the zero entry. Other integer widths retain their existing layouts.

- **Dense entries.** When the program keeps no reference into the values of maps of a type, entries live in one array that can move on growth: a lookup needs no chunk-directory read. A short reference ending before any map write or program call does not prevent this layout. Each compilation unit publishes its reference uses so every unit agrees. Before lowering, the driver substitutes the complete type-argument tuples of generic calls into retained map-use templates, including forwarded calls and imports. Whole-program and split builds consult the same canonical type keys; unrelated map types keep their optimizations. Imported signatures and copy, drop and hash helpers retain the exact type bindings that select the storage policy. `GrowingMap` retains stable chunks.

### JavaScript

- A map whose keys are primitives (`Integer`, `Float`, `String`, `Boolean`, a union of literals) is a JavaScript `Map`. Its iteration order is the insertion order, which the documentation allows since it promises no order.
- A map whose keys are objects or tuples can't use a JavaScript `Map` directly, which compares objects by identity. It is a JavaScript `Map` from the structural hash of the key (a 32-bit number, computed by a function generated per key type) to the entry, or to an array of entries when several keys share a hash. Each entry is `{ key, value }`, and the key is compared with the generated `==`.
- A reference to an object value is the object itself, including function parameters and results. A borrowed primitive or mixed-union slot uses a cached accessor cell retaining the map and key: reads and writes call the map's `get` and `set`. Aliases, defaults and user calls forward that cell; repeated borrowing of the same slot reuses it. The weak container cache bounds stale-key retention; see the [reference ABI](/advanced/owned-references).
:::
