# Arrays

An array holds an ordered list of values of the same type. Arrays can be mutable or immutable.

A list literal `[...]` is an array, unless the context expects another collection (see [creating collections](#creating-collections)).

```fa
let myArray = [1, 2, 3] -- immutable array: you cannot modify, add or remove elements
let myTypedArray: Array<Integer> = [1, 2, 3]

mutable myMutableArray = [1, 2, 3] -- mutable array: you can modify, add and remove elements
mutable myTypedMutableArray: Array<Integer> = [1, 2, 3]
```

Without an expected type, the elements of a literal get their base types, joined like the branches of an `if`: `[1, "a"]` is an `Array<Integer | String>`, and mixing integer and float literals, like `[1, 2.5]`, is an error (write `2.0`, or annotate).

An empty array needs its type, because there is nothing to infer from:

```fa
mutable empty: Array<Integer> = []
```

Reserve capacity up front when you know the size, with the `capacity` option of `from`:

```fa
mutable buffer = Array<Integer>.from([], capacity = 1000)
```

Arrays own their storage. The native compiler can keep immutable values in read-only data or bounded stack storage. See [memory management](/advanced/memory-management).

For a small list with a fixed maximum length, use an [InlineArray](/collections/inline-arrays). Its items stay inside its owner and never move on `push`. For exactly a fixed count of initialized items, use [CArray](/advanced/foreign-code).

## Creating collections

All collections (`Array`, `Set`, `Map`, `Pool`, `Columnar`, ...) are created from a list literal `[...]`. Braces `{ ... }` are only used for objects.

When the type of the collection is known from the context (a declaration, a parameter, a return type, a field), the literal takes this type:

```fa
let numbers: Set<Integer> = [1, 2, 3]
let scores: Map<String, Integer> = [("ana", 12), ("bao", 17)]
mutable humans: Pool<Human> = []

function total = (values: Set<Integer>): Integer => values.sum()
total([1, 2, 3]) -- the literal is a `Set<Integer>`
```

When there is no context, use the `from` function of the collection type. The type of the elements is inferred from the list, or can be given explicitly:

```fa
let distinctCount = Set.from([1, 2, 2, 3]).size -- 3
let floats = Set<Float>.from([1, 2, 3])
```

`from` also accepts the options of the collection:

```fa
mutable buffer = Array<Integer>.from([], capacity = 1000)
```

The option is `capacity`, a storage hint for every collection, the growing variants and `OrderedColumnar` included (see [capacity and reservation](#capacity-and-reservation)). `Set` and `GrowingSet` use [standard hashing and equality](/collections/sets#standard-hashing-and-equality); a separate key belongs in a `Map`. Any other option is an error (`FN023`).

There is one form per situation: using `from` where the context already gives the type (and no option is passed) is a warning, and the formatter rewrites it into a plain literal.

## Capacity and reservation

A collection has a **capacity**: the number of elements it can hold before it needs more memory. It grows on its own, but when you know how many elements are coming, say so, and the collection allocates once instead of growing several times:

```fa
mutable buffer = Array<Integer>.from([], capacity = 1000) -- room for 1000 items

for index in 0..1000 {
	buffer.push(index) -- never reallocates
}

buffer.reserve(500) -- room for 500 more items
console.log(buffer.capacity) -- at least 1500 natively
```

Every resizable collection exposes a read-only `capacity: Integer<min = 0>` property and `reserve(additional: Integer<min = 0>)`, returning `Void`. These apply to `Array`, `Set`, `Map`, `Pool`, `Columnar`, `OrderedColumnar` and all their growing variants. `from(source, capacity = n)` requests room for at least `n` items in total, never truncating the source.

Natively, `capacity` is the number of items the collection can hold without allocating or reallocating storage, including its live items. `reserve(n)` takes a number of **additional** items, while the `capacity` option of `from` is a total: it makes room for at least `length + n` items (`size + n` for an unordered collection); it never changes the contents or their order and never shrinks storage. The allocator may provide more room than requested. On a map or set, the capacity accounts for the hash index's load limit as well as entry storage. Deletion may reduce capacity when it releases pages.

`capacity` takes constant time. It is a storage observation, not a portable limit: on JavaScript and during compile-time evaluation, storage hints are ignored, `reserve` does nothing, and `capacity` equals `length` (or `size`). This avoids tracking fictitious storage that the JavaScript engine does not expose. Always use `length` or `size` for program logic. A [`String`](/values/strings#capacity-and-reservation) has the same property, option and method, counted in bytes natively.

The only relation that holds on every target is `capacity >= length` (or `size`). The value itself is unspecified: it differs between targets, between versions of the allocator, and between a call evaluated at compile time and the same call at run time. An assertion such as `assert buffer.capacity >= 1000` therefore only passes natively. To bound a buffer, compare its `length` with a limit of your own.

`reserve` requires a `mutable` receiver on every target. On `Array`, `Set` and `OrderedColumnar` it declares `mutates self`, like `push` or `add`: reallocation may move every item, invalidating references even when a particular reservation fits. On `Map`, `Pool`, `Columnar` and all growing collections it declares `mutates self.capacity`: existing items never move during reservation, references into them and into `size` or `length` stay valid. An effect on `length` or `size` implies the same effect on `capacity`, since adding or deleting may allocate or release storage: a function that declares `mutates items.length` may call `items.reserve`, while `mutates items.capacity` alone allows neither `push` nor `add`. A copy preserves values, not spare capacity; reserve the destination when needed.

## Adding and removing items

```fa
mutable users = ["Ana", "Bao"]

users.push("Cleo")     -- ["Ana", "Bao", "Cleo"]
users.insert(1, "Dan") -- ["Ana", "Dan", "Bao", "Cleo"]
users.delete(0)        -- ["Dan", "Bao", "Cleo"]
users.take(1)          -- "Bao": removes the item and returns it
users.pop()            -- String?: removes and returns the last item, or `none` if the array is empty
users.clear()          -- []
```

`insert`, `delete` and `take` take an index like `users[index]` does: an index the compiler can't prove valid `throws IndexError` (see [smart bound checks](#smart-bound-checks)). `insert` accepts any index from `0` to `length`, to insert at the end. In the example above, the compiler knows the length of `users` at each line, so no `try` is needed.

Adding or removing items requires a `mutable` array: these methods declare the `mutates` or `invalidates` effect, and calling a function that mutates a value is only allowed on a `mutable` value.

Each method invalidates only the references it may break. The compiler tracks this through its effects: a reference used after it was invalidated is a compile error, until it is declared again with `use`. See [owned references](/advanced/owned-references#after-invalidation).

- `push` and `insert` may reallocate the array's storage, which moves every item: they declare `mutates self`, which invalidates every reference into the array.
- `delete(index)` shifts the items after `index`: it declares `invalidates self[index..]`, and the items before stay valid.
- `pop` removes the last item only: it declares `invalidates self[self.length - 1]`, and the other references stay valid.

```fa
mutable users = ["Ana", "Bao", "Cleo"]
use first = users[0]
use last = users[2]

users.pop() -- invalidates `last` only
console.log(first) -- "Ana"

users.push("Dan")

console.log(first) -- error: `first` was invalidated by `users.push`: declare it again with `use first = users[...]`
```

## Access and update

```fa
let numbers = [10, 20, 30]
let first = numbers[0]

mutable scores = [100, 90, 80]
scores[1] = 95
```

Negative indices count from the end:

```fa
let last = numbers[-1] -- 30
```

Indexing is checked at compile time when the compiler can prove the index is valid. Otherwise, indexing `throws IndexError`, which must be propagated with `try` or handled with `catch` like any other [error](/values/errors). When a missing item is a normal situation, use `get`, which returns an [optional](/values/optionals) instead. `get` returns an optional reference to the item: `let` copies it, `use` keeps the reference (`(Integer in numbers)?`).

```fa
let maybeValue = numbers.get(index) -- Integer?
let value = numbers.get(index) ?? 0
```

See [smart bound checks](#smart-bound-checks) below.

## Iteration

```fa
let items = ["a", "b", "c"]

for item in items {
	console.log(item)
}

for item, index in items {
	console.log("{index}: {item}")
}
```

Iterating yields **references**. When the array is `mutable`, the loop variable is a mutable reference, so writing through it writes into the array:

```fa
mutable scores = [1, 2, 3]

for score in scores {
	score *= 10
}

console.log(scores) -- [10, 20, 30]
```

Adding or removing items of an array while iterating over it is a compile error. See [loops](/control-flow/loops#loop-variables-are-references).

## Changing the item type

An array's item type is invariant for implicit conversions. Changing it builds a new array and requires `copy` in the expected type (`MEM004` otherwise), even at last use or for a borrowed argument:

```fa
let small: Array<Int8> = [1, 2]
let invalid: Array<Integer> = small -- error: MEM004: write copy small
let wide: Array<Integer> = copy small
function read = (items: Array<Integer>) => console.log(items)
read(copy small)
```

The only exception is literal and range widening within one primitive type, at any depth of the item type. Tuples and objects with the same fields may differ only by those widenings: `[("é", 0), ("日", 1)]` can be stored as `Array<(String, Integer)>` without `copy`. These types have the primitive components' storage:

```fa
let exact: Array<1 | 2> = [1, 2]
function read = (items: Array<Integer>) => console.log(items)
read(exact) -- borrows
let wide: Array<Integer> = exact -- moves
```

The same exception covers `Array<Integer<min = 0, max = 10>>` to `Array<Integer>`, `Array<"a" | "b">` to `Array<String>`, and boolean literals to `Boolean`. Another numeric type, adding a union alternative or optionality, or projecting an item's fields requires `copy`: for example `Array<Int8>` to `Array<Int8 | String>`, `Array<Integer>` to `Array<Integer?>`, and `Array<Person>` to `Array<{ name: String }>`.

A generic signature `<Item: { name: String }>(items: Array<Item>)` preserves the actual item type and borrows without a conversion. The rule is identical on every target. See [type conversion](/types/type-conversion#converting-the-inside-of-a-structure).

## Slicing and copying

The ownership and conversion restrictions apply to stores. Equality and ordering, `is`, membership, printing and hashing for a lookup only inspect their operands: they retain their comparability rules and never copy or convert either collection.

Assigning an existing array moves it at last use. To keep both arrays, write `let duplicate = copy original`. This is the general structure rule, including recursive values and elements owning structures. `get`, indexing and iteration borrow elements; storing a borrowed structure element requires `copy`. See [memory management](/advanced/memory-management).

Indexing with a [range](/values/ranges) copies the selected items into a new array. `..` excludes the upper bound, `..=` includes it.

```fa
let data = [1, 2, 3, 4, 5]

let middle = data[1..4] -- copies the items at indices 1, 2 and 3: [2, 3, 4]
let same = data[1..=3]  -- [2, 3, 4]
let tail = data[2..]    -- [3, 4, 5]
let duplicate = copy data -- a full copy
```

Because a slice constructs an independent array, modifying it never modifies the original array. Construction copies ordinary items as values. Every stored structure item is a store like any other: it moves from a last-use owner or requires `copy` on the source (`MEM004`); unique items cannot be duplicated (`MEM003`). A callback always borrows its parameters, including structure items: `nested.some((item) => item.length > 0)` copies nothing. The word `copy` alone authorizes deep copying, never a method name.

### Construction with structure items

The same rule covers range slices, `filter`, `map`, concatenation with spreads, `Array.from` and other collection conversions, iterator `toArray`, array/object spreads and rest patterns. An in-place operation or a reference avoids creating another owner. The explicit copy of a source supplies independent items that can be transferred into the new construction:

```fa
let matrix = [[1], [2]]
let slice = (copy matrix)[0..1]
let filtered = (copy matrix).filter((row) => row.length > 0)
let mapped = matrix.map((row) => copy row)
let concatenated = [...copy matrix, ...copy matrix]
let converted = Array.from(copy matrix)
let collected = (copy matrix).values().toArray()
let spread = [...copy matrix]
let [first, ...rest] = copy matrix
console.log(matrix, slice, filtered, mapped, concatenated, converted, collected, spread, first, rest)
```

Without those copies, retaining `matrix` while storing its nested arrays is `MEM004`. `matrix.some((row) => row.length > 0)` still borrows and needs no copy. The same item rule applies to set algebra (`union`, `intersection`, `difference`, `symmetricDifference`), non-in-place `sort`/`reverse`, map keys and values, and constructors building other collection kinds.

`use` keeps a view of the range instead. For example, `mutable use part = items[0..2]` refers to those items without copying them. You can assign its items through a mutable view, or reseat the reference with `reuse`. You cannot assign the view as a whole: `part = [7, 7]` is an error even when the lengths match. Assign the items in a loop to copy a range, or use `reuse part = items[2..4]` to change what the reference designates. An ordinary `mutable` copy of a slice remains assignable.

Negative endpoints are normalized from the end: `data[-2..]` is `[4, 5]`. A start equal to `length` is allowed and gives an empty slice; reversed normalized bounds are empty, and `[..]` on an empty array succeeds. Bounds must remain within `-length` and `length`, with one exception: an inclusive end must designate an existing item. `data[0..=data.length]` is rejected; use `data[0..data.length]` to include every item. Certain invalid bounds are compile errors; unproven bounds throw `IndexError` rather than being clamped. These rules also apply to strings and `Bytes`.

A slice view has no structural operation: `part.push(7)`, `part.clear()` and `part.reserve(8)` are errors. The same applies to inserting, deleting, taking or popping items and `deleteWhere`. Change the owner, then `reuse` the view. Item assignments through a mutable view remain valid.

## Common methods

```fa
let numbers = [3, 1, 2]

numbers.length                           -- 3
numbers.isEmpty()                        -- false
numbers.includes(2)                      -- true
numbers.indexOf(2)                       -- Integer?
numbers.find((n) => n > 2)               -- (Integer in numbers)?: `let` copies it into an `Integer?`
numbers.filter((n) => n > 1)             -- Array<Integer>
numbers.map((n) => n * 2)                -- Array<Integer>
numbers.reduce((total, n) => total + n, 0)
numbers.sum()                            -- 6
numbers.every((n) => n > 0)
numbers.some((n) => n > 2)
numbers.reverse()                        -- a reversed copy: [2, 1, 3]
numbers.sort()                           -- a sorted copy: [1, 2, 3]
numbers.sort((a, b) => b - a)            -- a copy sorted with a custom order: [3, 2, 1]
```

`length` is a property, not a method. `sort()` and `reverse()` never modify the array: they return a new one.

- Like `get`, `find` returns an optional reference to the item it finds, on every collection: `let` copies it, `use` keeps the reference.
- The callbacks of a collection whose elements have positions (`Array`, `GrowingArray`, `OrderedColumnar`) receive the item and its index, `(item, index)`; the other collections pass the item only, and maps pass `(key, value)`. A callback can declare fewer parameters.
- `filter` returns a collection of the same kind (a `Set` for a `Set`; a growing collection gives its standard counterpart). `map` returns an `Array`, except on a map, where it maps the values and returns a `Map` with the same keys: mapping a set to a set is `Set.from(set.map(transform))`, since equal results merge.
- `sum()` works on a collection of numbers, returns their base type, and is `0` when the collection is empty. An overflow stops the program, like any [arithmetic](/operations/arithmetic) overflow.
- `sort()` uses the natural order of the items, which must be ordered (numbers are never sorted as strings). A custom order is a function whose result's sign decides: negative when `a` comes first, positive when `b` does, `0` (or `NaN`) when they are equivalent. Sorting is stable on every target: equivalent items keep their order.

The collections whose elements have positions (`Array`, `GrowingArray`, `OrderedColumnar`, and `String`) have a `length`: every index from `0` to `length - 1` is valid. The other collections (`Set`, `Map`, `Pool`, `Columnar`) have a `size`, the number of their elements, which bounds no index.

On a mutable array, these methods work in place:

```fa
mutable numbers = [3, 1, 2]

numbers.sortInPlace()             -- [1, 2, 3]
numbers.reverseInPlace()          -- [3, 2, 1]
numbers.deleteWhere((n) => n < 2) -- deletes the matching items: [3, 2]
```

`deleteWhere` is the way to remove items while examining them: removing items of an array during a loop over it is a compile error.

If its predicate throws, the items already matched stay deleted; the throwing item and the unvisited tail stay after the retained items. Mutations performed by the callback remain visible. A failing `sortInPlace` comparator keeps the original order, with its item mutations still visible. Only a comparator whose type can throw needs a staged permutation; a non-throwing comparator sorts in place without a staging buffer. `reverseInPlace` calls no callback and cannot throw, so it reverses in place without staging. See [effects before an error](/values/errors#effects-before-an-error).

Chained calls whose callbacks are [pure](/functions/defining-functions#pure-functions) are fused into a single pass. See [iterating with methods](/control-flow/loops#iterating-with-methods).

### Effects of the methods

The methods that modify an array declare it in their signature with an [effect](/functions/defining-functions):

| Method | Effect |
| --- | --- |
| `push`, `insert`, `clear`, `reserve` | `mutates self` |
| `sortInPlace`, `reverseInPlace`, `deleteWhere` | `mutates self` |
| `delete(index)`, `take(index)` | `invalidates self[index..]` |
| `pop` | `invalidates self[self.length - 1]` |
| `array[index] = value` | `mutates self[index]` |
| `length`, `capacity`, `get`, `includes`, `indexOf`, `find`, `filter`, `map`, `reduce`, `every`, `some`, `sort`, `reverse` | none |

`invalidates` implies `mutates`. Writing to one item (`mutates self[index]`) does not move the other items: the references to them stay valid, and so do the references to the written item. Only the references into the memory the old item owned are invalidated, like its string slices or the elements of its arrays; there are none for a flat item, like an `Integer`. See [mutations](/advanced/owned-references#mutations). The paths of an effect are evaluated with the values before the call: `self.length - 1` is the index of the last item before `pop` removes it, and `pop` on an empty array invalidates nothing.

An item of an array can't be passed to a method that mutates the same array: `users.push(users[0])` is an error, because `push` may move `users[0]` while reading it. Copy it first: `let first = users[0]`, then `users.push(first)`. See [arguments that alias](/advanced/reference-checking#arguments-that-alias).

## How it works

An array stores its items **contiguously**, in a single block of memory, with its `length` and a **capacity**: the number of items the block can hold before it is full.

- Reading `array[index]` is one memory access, and iterating reads the block from start to end: this is the fastest layout for a loop.
- `push` writes after the last item when the capacity allows it. Otherwise, it allocates a block twice as large and moves every item there: this is why `push` declares `mutates self`. Doubling keeps `push` constant on average.
- `insert` and `delete(index)` move the items after `index` by one place: their cost grows with the number of items after `index`.
- `pop` only shrinks the length.

| Operation | Cost |
| --- | --- |
| `array[index]`, `get`, `pop` | constant |
| `push` | constant on average |
| `insert`, `delete(index)` | linear in the number of items after `index` |
| `includes`, `indexOf`, `find` | linear |

Reserve the capacity with `from` or `reserve` when you know it, to skip the intermediate reallocations (see [capacity and reservation](#capacity-and-reservation)). When references must survive `push`, use a [`GrowingArray`](/collections/growing-collections), which never moves its items.

## Smart bound checks

Fa's compiler tracks how large a collection is and what range an index can take, so most bounds checks disappear at compile time. The rules it follows, and so the indices it proves, are fixed by the [range analysis](/values/numbers#range-analysis): every compiler accepts the same programs.

Consider this Rust code:

```rs
let mut array = vec![1, 2, 3];

// returns an Option<i32>
let first = array.get(0);

// unsafe
let first = unsafe { array.get_unchecked(0) };

// bounds-checked at runtime
let first = array[0];
```

A human can see that the first element is always there, but the compiler cannot.

Fa knows the length of the array at that point, so it can read the first element with no check at all:

```fa
mutable array = [1, 2, 3]

-- no bounds check: the compiler knows the array has three items
let first = array[0]
console.log(@typeToString(Type<first>)) -- "Integer"

-- rejected at compile time
let fourth = array[3]
```

When the index comes from a [ranged integer](/values/numbers#minimum-and-maximum-values), the same reasoning applies:

```fa
let index: Integer<min = 0, max = 2> = readIndex()
let value = array[index] -- no bounds check
```

When the compiler cannot prove the index is valid, indexing `throws IndexError`. Propagate the error with `try`, handle it with `catch`, or use `get` to receive an optional:

```fa
function readValue = (array: Array<Integer>, index: Integer): Integer throws IndexError => {
	let value = try array[index] -- propagates `IndexError` to the caller
	value
}

let caught = array[index] catch error => 0
let fallback = array.get(index) ?? 0
```

Destructuring an array whose length is not proven can fail the same way, so it is preceded by `try`:

```fa
function firstTwo = (values: Array<Integer>): Integer throws IndexError => {
	try let [first, second] = values -- `IndexError` if `values` has fewer than two items
	first + second
}
```

This is why Fa needs no `get_unchecked`: there is a single indexing syntax, checked at compile time when possible, and its signature tells you whether it can fail. `get` is only for the cases where a missing item is expected.

:::implementation
## Implementation details

The shared slice proof checks normalized endpoints, with an inclusive end strictly below length; unproven bounds keep their runtime check. The analyzer rejects structural operations and whole-slice assignment, including through reference aliases and after `reuse`. Item writes remain writes through the view; `reuse` only reseats the reference. A contiguous copy loop over flat items can be lowered to one block copy.

### Native

Literal/range widening at nested tuple and object components preserves item storage. Equality reads each array with its own item representation and calls a cached comparator for the pair; it never converts an operand array into the other item type.

- **Returned collections.** In synchronous compilation mode, collection functions receive an optional bounded caller buffer and return the final header pointer. Fresh literals, supported transforms and proven moved immutable locals forward it through calls and return branches. A mutable or escaping owner supplies no frame buffer, so construction starts on the heap. Constant returned locals can share read-only storage with a proven immutable caller. Arrays and growing arrays construct aggregate-returning literal items directly in their final slots; completed items still drop on an error. Object-columnar chunks retain their larger heap layout.

- **Reviewed storage limits.** The collection byte budget is 2 KiB; the total is 4 KiB per non-recursive frame and 512 bytes in a resolved direct-call cycle (including methods and generic instances). Literals reserve their exact aligned size. Non-recursive calls receive at most `min(2 KiB, remaining frame budget)`. In a direct-call cycle, all call-result offers share 128 bytes per frame inside the 512-byte total: each receives `min(128 − offer bytes reserved, 512 − collection bytes reserved)`, with no offer below 64 bytes. Exact literals count against the total in either lowering order; forwarded offers reserve no frame bytes. The inclusive 2× per-frame gate covers call-result offers (one and four live splits): unchecked depths must satisfy `baseline_depth ≤ 2 × final_depth + 8`, with eight frames for fixed setup overhead and native measurements under a 256 KiB stack limit. Leak-checked depths are informational; exact literals are excluded. Indirect recursion is not detected. Larger objects/tuples use caller output slots; a layout with at most two scalar leaves and 16 bytes on 64-bit targets, or exactly one scalar leaf on wasm32, returns by value. Small throwing results keep `{ value, error }`; register results store directly into their final owner. An output marked `noalias` is unreachable from arguments, captures and globals during the call. Native x86-64/aarch64 functions probe crossed stack pages.

- **Immutable literals.** Literal `let` arrays whose supported items are constant use read-only globals, including nested arrays and objects of constants. Other local `let` literals use a fixed entry-block buffer when their header and items fit 2 KiB and the non-recursive function stays within its 4 KiB collection budget. Captured values, asynchronous programs and larger constructions keep the heap path. The word following the header marks static or frame storage; it is independent of the negative capacity used for compact string arrays. Static drops do nothing; frame drops destroy owned items without freeing the buffer. An explicit copy of static storage constructs independent storage for its new owner. Runtime literals with owning or unknown uses start on the heap and retain ordinary moves; a frame address is never copied out to rescue an escape. Nonempty-separator `split` can construct directly in a proven non-escaping local’s fixed caller buffer, with a heap choice before construction when it does not fit; other input-sized constructors and value slices keep their existing paths.

- **Caller destinations.** `map` and `filter` can build a proven immutable local directly in its fixed caller buffer, using the input length as a capacity bound and selecting the heap before construction if it does not fit. Callback errors drop the completed prefix. `Array.from` of a live borrowed array without an explicit capacity hint constructs its required value copy directly in the same bounded storage; already owned or movable arrays keep their storage. Explicit capacity hints retain the existing snapshot-before-hint path.
- **Layout.** An array value is a pointer to a header `{ pointer, length, capacity }`. Small item buffers share the header's allocation; larger buffers use a separate block from `fa_alloc`. An empty array allocates its header only.
- **Recursive items.** A collection supplies storage for its object items, including recursive objects: each object is stored directly in its slot, without an additional box. Direct recursive fields inside that object, including optionals, retain an owning box when their inline expansion would cycle. The same rule applies to map entries, set elements, growing arrays, pools and columnars. Copies still recursively copy owned payloads; moving the array's items follows the same reference invalidation rules.
- **Growth.** The capacity doubles, starting at 4 items. Growth allocates a larger buffer, relocates the items bytewise, and frees the old buffer. Before a counted loop that always pushes one item to an empty array field, the compiler can reserve the loop's item count. A closed analyzer proof that no module observes any array capacity permits reserving that count without geometric rounding. Missing module contributions, generic or deferred bodies, key-loop snapshots and foreign code withhold the permission; a capacity read through any type or alias removes it. Per-unit builds log the fact for object-cache invalidation. The array keeps its header address and the normal push checks: calls through aliases may add further items. A nonempty field is not reserved early, so references to its existing items remain valid until the actual push. Local arrays keep geometric reservation to amortize repeated append loops.
- **Capacity.** `from(…, capacity = n)` allocates `n` items up front; `reserve(additional)` grows the buffer to at least `length + additional` items (doubling when that is more), so a `reserve` in a loop stays constant on average. `capacity` reads the header.
- **`push`** has an inline fast path (`length < capacity`: store, then increment) and a cold call to the growth function. An object literal is stored field by field straight into the new slot, never built in a temporary and copied; a value with more fields than the item type is copied without them.
- **Constant fills.** A finite counted loop whose only statement pushes the same constant byte or Boolean to an `Array` or `GrowingArray` makes room once and fills the appended region (one block per growing segment). Boolean items occupy one byte, `false` as 0 and `true` as 1. The receiver must be a local or parameter followed only by plain fields; reactive reads/writes and coroutine loops keep their rounds. Dynamic values, other item representations and additional effects also keep the ordinary loop. Bounds are evaluated in their original order, empty ranges append nothing, and existing items remain intact.
- **`insert` and `delete(index)`** are a `memmove` of the tail. `deleteWhere` compacts in place in one pass, with a read index and a write index, and drops the deleted items.
- **`filter` of a temporary array** (`text.split(" ").filter(…)`, with a callback that can't throw) compacts that array in place, like `deleteWhere`, and returns it: no second array. On any other array, `filter` builds a new array with room for every item.
- **Sorting and errors.** A comparator whose checked type cannot throw goes straight to the stable runtime sort, without a rollback permutation. The runtime's stable merge-sort scratch is algorithm workspace, not a saved original order. A comparator that may throw sorts indices into the original array, borrowing its actual item slots. Success applies that permutation with ownership transfers through cycles; failure frees the indices and leaves the order unchanged, retaining callback writes. Item payloads are not copied for staging.
- **Indexing.** A proven index is a plain load. Otherwise the bound check is one compare and a cold branch that returns `IndexError`. A negative index is `length + index`, checked the same way.
- **Exact extents.** A private array of objects and the closed family of parameters receiving it can retain a literal's exact length through field mutations and `reserve`. The whole family must never grow, be replaced or escape to unknown code. Construction must be a direct passive literal without spreads, calls, implicit defaults, conversions or reactive observations. Opaque inputs and generic or unknown types withhold the fact. Native lowering substitutes a length only for a recognized slot whose every binding has the same proven extent, and continues loading the actual items pointer. Package claims and their absence are cache dependencies. Existing scoped distinct-index facts remain local to each outer iteration; floating-point flags and operation order stay unchanged.
- **Package proofs.** An array private to one file, never shrunk and only passed to private functions of that file can use an invariant of its index fields or collections when no other file supplies those values. Every unit publishes its stores and type uses before dependent JavaScript or native code is generated, including wider objects, generic stores and imported defaults. Arrays shared between files keep their checks. Whole-program store proofs stop conservatively at bodies analyzed separately for each key of a key-set `@for`; ordinary local flow proofs still apply there.
- **Slicing** allocates a new array and copies the range: a `memcpy` for flat items, the generated `copy<T>` otherwise.
- **References.** A reference to an item is a pointer to it.
- **Iteration.** The items pointer and length are loaded before the loop. The iteration rule forbids structural changes to the array, including through aliases and calls; writes to the items remain visible.

- **Packed arrays.** Named-union arrays built only by `push` from an empty literal, read by `for` and measured by `length` store each member byte and its data without padding or a box per item.
- **Large blocks.** On Linux, arrays and strings of at least 1 MiB have mappings that grow by moving pages rather than copying bytes. A small array moved from a local into a literal field is compacted with its header at its exact size.

### JavaScript

- An `Array` is a JavaScript array. The `capacity` option of `from` is ignored and `reserve` compiles to nothing (its argument is still evaluated): preallocating with `new Array(n)` would create a *holey* array, which engines optimize less. `capacity` is the `length`.
- Negative indices compile to `array[array.length + index]` after the bound check. A proven index needs no check.
- A reference to an object item is the object itself. A reference to a primitive item is its index: the type of the reference names the array, so the index is enough, and `score *= 10` in a loop compiles to `scores[i] *= 10`.
- Since a reference to an object item is the object, writing a whole object into a place that a live reference may designate (`users[0] = other`, `mutates self[index]`) assigns it field by field into the existing object (a generated `assign` function), so that the reference sees the new value. Elsewhere, the value is simply replaced.
- Owning slices use `slice()`, followed by a copy of each item when the items own memory (objects are values in Fa, not shared pointers). A `use` range is a live Array proxy over its owner and normalized bounds: indexed reads/writes, iteration and ordinary array methods use the selected items without copying them. `reuse` replaces that view; structural operations and whole-view assignment are rejected before lowering.
- `reverseInPlace` is `reverse`. A non-throwing `sortInPlace` comparator calls the JavaScript array's stable `sort` directly; a potentially throwing comparator uses staged item handles and commits the order only after all comparisons succeed, so its object-field writes remain visible on failure. `sort()` and `reverse()` construct a new array.
### Compile-time interpreter

A borrowed range keeps the stable owner slot and normalized bounds. Nested indices, iteration and callback items resolve back to that owner; they do not copy the collection or its items. `reuse` changes the saved designation at runtime. Array sorting and reversal operate on the designated items. An owning read or explicit copy materializes an independent value under the interpreter's memory budget.

:::
