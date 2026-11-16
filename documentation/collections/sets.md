# Sets

A set stores unordered, unique values. Sets can be mutable or immutable.

Uniqueness is decided by [equality](/operations/logical#equality) of the values: `==` compares objects and collections deeply, by their content.

A set is meant for **primitives and small values** compared by content, like tags, ids or points. Every object in Fa is a value, so adding one to a set compares its content with the content of the values already there, not an address as in JavaScript. To store objects that you reference and modify, use a [`Pool`](/collections/pools).

Like every collection, a set is created from a list literal, typed by the context, or with `Set.from` when there is no context (see [creating collections](/collections/arrays#creating-collections)).

```fa
let setOfIntegers: Set<Integer> = [1, 2, 2, 3]

console.log(setOfIntegers.size) -- 3, because 2 is only stored once

let distinct = Set.from([1, 2, 2, 3]) -- no context: `Set.from` infers a `Set<Integer>`
```

```fa
mutable setOfHumans: Set<Human> = [
	{ name = "John", age = 20 }
	{ name = "Jane", age = 21 }
	{ name = "John", age = 22 }
]

let anotherJohn = Human { name = "John", age = 20 }
setOfHumans.add(anotherJohn) -- not added: an equal Human is already present
```

Note that the third human *is* stored: `{ name = "John", age = 22 }` is not equal to `{ name = "John", age = 20 }`.

Storage can be planned with `from(source, capacity = n)`, the read-only `capacity` property and `reserve(additional)`. See [capacity and reservation](/collections/arrays#capacity-and-reservation) for the shared rules and effects.

## Standard hashing and equality

A set always hashes and compares its complete item using the item's standard hashing and equality. There is no `key` option: `Set.from(source, key = ...)` and `GrowingSet.from(source, key = ...)` are errors (`FN023`). Use a [map](/collections/maps) when the key is separate from the value:

```fa
type User = { id: Integer, name: String }
mutable users: Map<Integer, User> = []
users.set(1, User { id = 1, name = "Ana" })
users.set(1, User { id = 1, name = "Bao" }) -- replaces the value at key 1
```

Storing a collection-valued item follows the [copy and move rules](/advanced/memory-management#structures). The hash/equality rule never authorizes copying unique data.

## Adding and removing

```fa
mutable tags: Set<String> = ["fa", "docs"]

tags.add("parser")
tags.delete("docs")
tags.clear()

tags.includes("fa") -- false: the set has been cleared
tags.size           -- 0
tags.isEmpty()      -- true
```

`add` returns whether the value was inserted, and `delete` whether it was present:

```fa
if tags.add("fa") {
	console.log("this tag is new")
}
```

`add`, `delete`, `take`, `deleteWhere` and `clear` require a `mutable` set, and declare the `mutates self` effect (see [stable references](#stable-references)). `take(value)` removes a value and returns it, or `none` when it is absent.

| Operation | Effect |
| --- | --- |
| `add`, `delete`, `take`, `deleteWhere`, `clear` | `mutates self` |
| `reserve(additional)` | `mutates self`: it may rebuild the table |
| `includes`, `size`, `capacity`, `find`, `union`, `intersection`, `difference`, `isSubsetOf` | none |

The values of a set can't be modified in place: their content decides where they are stored. A reference to a value of a set is read-only, even in a `mutable` set, and so is the loop variable. To change a value, delete it and add the new one. For the same reason, the element type of a set is exact: a `Set<Hero>` is not a `Set<Human>`, even where a [structural view](/advanced/structural-typing) would accept a `Hero` for a `Human`.

## Iteration

Order is not guaranteed and may change as the set is modified. The loop variable is a read-only reference.

```fa
for tag in tags {
	console.log(tag)
}
```

Adding or removing values of a set while iterating over it is a compile error.

## Set operations

```fa
let a: Set<Integer> = [1, 2, 3]
let b: Set<Integer> = [2, 3, 4]

a.union(b)        -- [1, 2, 3, 4]
a.intersection(b) -- [2, 3]
a.difference(b)   -- [1]
a.isSubsetOf(b)   -- false
```

These operations return a new set and leave `a` and `b` unchanged.

Two sets are equal when they contain the same values, in any order:

```fa
let c: Set<Integer> = [3, 2, 1]
assert a == c
```

## Stable references

Adding to or removing from a set may rehash it, which moves its values and invalidates every reference into it. This is why `add`, `delete`, `take`, `deleteWhere` and `clear` declare `mutates self`, and the compiler rejects any use of a reference obtained before one of these calls until it is declared again (see [owned references](/advanced/owned-references#after-invalidation)):

```fa
mutable people: Set<Person> = [{ name = "Ana" }]
use ana = people.find((person) => person.name == "Ana") -- type: `(Person in people)?`

people.add({ name = "Bao" })

console.log(ana?.name) -- error: `ana` was invalidated by `people.add`: declare it again with `use ana = ...`

use ana = people.find((person) => person.name == "Ana") -- declared again: it finds Ana at her new place
console.log(ana?.name) -- "Ana"
```

When you need references that survive modification, use a [`Pool`](/collections/pools) or a [growing collection](/collections/growing-collections) instead.

:::tip
This is why the objects linked by [relations](/advanced/relations) live in a `Pool`, a `Columnar`, a `GrowingPool` or a `GrowingColumnar`, never in a `Set`.
:::

## How it works

A set is a hash table that stores its values **inline**, in the table itself:

- `includes` hashes the value and compares it with the few candidates found at that place in the table: one memory access in the common case, and none beyond the table when the value is absent.
- `add` does the same, then writes the value in a free place. When the table is too full, it is rebuilt at twice the size, which moves every value: this is why `add` declares `mutates self`.
- `delete` marks the place as deleted, and the next rebuild reclaims it.
- Iteration walks the table and skips the empty places.

| Operation | Cost |
| --- | --- |
| `includes`, `add`, `delete` | constant on average |
| iteration | linear in the capacity of the table |

`set.size` is the number of values. A set has a `size` rather than a `length`: its values have no positions. `set.capacity` is the number of values it can hold before it is rebuilt.

A set gives up stable references to keep `includes` as fast as possible. When references must survive insertions, use a [`GrowingSet`](/collections/growing-collections), whose values live in stable slots.

## Key sets

A set can contain a fixed number of values known at compile time:

```fa
let myKeySet: Set<"hello" | "world" | "!"> = ["hello", "!"]
```

The keys of an object type form a **key set**, written with braces: `{ foo, nested { bar } }`. Besides [blocks](/getting-started/syntax-basics#braces), it is the only use of braces that is not an object. Key sets are what the key-taking [type functions](/types/type-functions) expect:

```fa
type MyType = {
	foo: String
	bar: Integer
	nested: {
		baz: Integer
		qux: Integer
	}
}

type Picked = Pick<MyType, { foo, nested { baz } }> -- { foo: String, nested: { baz: Integer } }
type Omitted = Omit<MyType, { bar }>                -- { foo: String, nested: { baz: Integer, qux: Integer } }
```

Because a key set is a compile-time value, a function can take one as a compile-time parameter and use it to generate code:

```fa
function logProperties = <Item, keys: DeepKeys<Item>>(object: Item) => {
	@for key in keys {
		console.log("{key} = {object[key]}")
	}
}

let myObject = {
	foo = 12
	bar = 13
	baz = 14
	nested = {
		foo = 15
		bar = 16
	}
}

logProperties<Type<myObject>, { foo, baz, nested { foo } }>(myObject)
```

The call above compiles to three `console.log` lines with no runtime lookup at all. See [fields](/types/fields#key-sets) and [generics](/types/generics).

## Choosing a set

- Need uniqueness and fast membership tests on primitives or small values? `Set`.
- Need to store many objects, with stable references? [`Pool`](/collections/pools).
- Need a fixed set of names known at compile time? A [key set](/types/fields#key-sets), written `{ ... }`.

:::implementation
## Implementation details

### Native

- **Returned collections.** In synchronous compilation mode, collection functions receive an optional bounded caller buffer and return the final header pointer. Fresh literals, supported transforms and proven moved immutable locals forward it through calls and return branches. A mutable or escaping owner supplies no frame buffer, so construction starts on the heap. Constant returned locals can share read-only storage with a proven immutable caller. Arrays and growing arrays construct aggregate-returning literal items directly in their final slots; completed items still drop on an error. Object-columnar chunks retain their larger heap layout.

- **Reviewed storage limits.** The collection byte budget is 2 KiB; the total is 4 KiB per non-recursive frame and 512 bytes in a resolved direct-call cycle (including methods and generic instances). Literals reserve their exact aligned size. Non-recursive calls receive at most `min(2 KiB, remaining frame budget)`. In a direct-call cycle, all call-result offers share 128 bytes per frame inside the 512-byte total: each receives `min(128 − offer bytes reserved, 512 − collection bytes reserved)`, with no offer below 64 bytes. Exact literals count against the total in either lowering order; forwarded offers reserve no frame bytes. The inclusive 2× per-frame gate covers call-result offers (one and four live splits): unchecked depths must satisfy `baseline_depth ≤ 2 × final_depth + 8`, with eight frames for fixed setup overhead and native measurements under a 256 KiB stack limit. Leak-checked depths are informational; exact literals are excluded. Indirect recursion is not detected. Larger objects/tuples use caller output slots; a layout with at most two scalar leaves and 16 bytes on 64-bit targets, or exactly one scalar leaf on wasm32, returns by value. Small throwing results keep `{ value, error }`; register results store directly into their final owner. An output marked `noalias` is unreachable from arguments, captures and globals during the call. Native x86-64/aarch64 functions probe crossed stack pages.

- **Immutable literals (current LLVM backend).** Constant `let` literals with supported scalar keys use read-only entries and index. Runtime local literals use a fixed frame buffer when the header, entries, index and any stable chunk directory fit the 2 KiB collection and 4 KiB non-recursive frame budgets. Captured or asynchronous values and larger constructions keep heap storage. Static drops do nothing; frame drops destroy owned elements without freeing their storage. An explicit copy of static values constructs independent mutable storage. Proven moves choose suitable destination storage before construction. Runtime owning or uncertain uses choose the heap before construction, without frame-to-heap copy-out. Duplicate constant elements retain set semantics.

The set is a **Swiss table** (Abseil's `flat_hash_set`, Rust's `hashbrown`, Go's maps since 1.24), with the values stored inline:

- **Layout.** One allocation holding an array of control bytes followed by an array of values. A control byte is empty, deleted, or 7 bits of the value's hash. The control array is padded with a copy of its first group, so that probing never wraps in the middle of a group.
- **Probing.** The hash gives a starting group; a group is 16 control bytes compared at once with SIMD (SSE2, NEON; a portable 8-byte word otherwise). Each control byte equal to the hash's 7 bits is a candidate, compared with the value's `==`. A group with an empty byte ends the search. Groups are probed in triangular order.
- **Growth.** The maximum load factor is 7/8, counting tombstones. Beyond it, the table is rebuilt: at twice the capacity when the live values exceed half the capacity, at the same capacity otherwise (to purge tombstones).
- **Specialization.** `hash<Shape>` and `==` are derived structurally per element type, and **inlined into the probe loop** by monomorphization. A set of small flat values (integers, points) compares them with a few instructions.
- **Hashing.** Fast and non-cryptographic, with a random seed per set to resist hash flooding. Tests can fix the seed.
- **References.** A reference into a set is a pointer to the value in the table, invalidated by any `add`, `delete` or `clear`, as declared.

The current LLVM backend stores a set like a [map](/collections/maps) without values: its values in stable chunks, and an index of 32-bit hash tags and value numbers.

### JavaScript

- A set of primitives (`Integer`, `Float`, `String`, `Boolean`, a union of literals) is a JavaScript `Set`: the engine's `includes` (`has`) is as fast as anything written by hand.
- A set of objects or tuples can't use a JavaScript `Set` directly, which compares objects by identity. It is a JavaScript `Map` from the structural hash of the value (a 32-bit number, computed by a function generated per element type) to the value, or to an array of values when several values share a hash. `includes` hashes the value, then compares it with the generated `==`. `size` is a counter kept next to the `Map`.
- A reference to an object value is the object itself. Hashing and equality always use the item type; the table header contains no custom key callback. Removing that legacy callback from both backends belongs to the Set implementation slices.
:::
