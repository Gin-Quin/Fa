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

Arrays own their storage on the heap. See [memory management](/advanced/memory-management).

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
let users = Set.from(userList, key = (user) => user.id)
mutable buffer = Array<Integer>.from([], capacity = 1000)
```

The options are `capacity`, for every collection (see [capacity](#capacity)), and `key`, for `Set` and `GrowingSet` (see [custom key](/collections/sets#custom-key)). Any other option is an error.

There is one form per situation: using `from` where the context already gives the type (and no option is passed) is a warning, and the formatter rewrites it into a plain literal.

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

## Capacity

A collection has a **capacity**: the number of elements it can hold before it needs more memory. It grows on its own, but when you know how many elements are coming, say so, and the collection allocates once instead of growing several times:

```fa
mutable buffer = Array(Integer).from([], capacity = 1000) -- room for 1000 items

for index in 0..1000 {
	buffer.push(index) -- never reallocates
}

buffer.reserve(500) -- room for 500 more items
console.log(buffer.capacity) -- at least 1500 natively
```

- `from(…, capacity = n)` creates the collection with room for at least `n` elements.
- `capacity` is a read-only property: the number of elements the collection can hold without allocating, never less than `length`.
- `reserve(additional)` makes room for at least `additional` elements more than the current `length`. It requires a `mutable` array and, like `push`, declares `mutates self`: it may move the items.

Every collection has them, and so does [`String`](/values/strings#capacity), counted in bytes. On JavaScript, the engine manages the memory: `reserve` does nothing and `capacity` is the `length` (the `size` of the collections without positions), so a program must never depend on its exact value.

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

## Slicing and copying

Indexing with a [range](/values/ranges) copies the selected items into a new array. `..` excludes the upper bound, `..=` includes it.

```fa
let data = [1, 2, 3, 4, 5]

let middle = data[1..4] -- copies the items at indices 1, 2 and 3: [2, 3, 4]
let same = data[1..=3]  -- [2, 3, 4]
let tail = data[2..]    -- [3, 4, 5]
let copy = data[..]     -- a full copy
```

Because a slice is a copy, modifying it never modifies the original array.

A bound of a slice can be negative, counting from the end, like an index: `data[-2..]` is `[4, 5]`. A slice whose start is after its end is empty: `data[3..1]` is `[]`. The bounds themselves are checked like an index, from `-length` to `length`: `data[2..10]` doesn't stop at the end of the array, it `throws IndexError` unless the compiler proves the bounds valid.

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

Reserve the capacity with `from` or `reserve` when you know it, to skip the intermediate reallocations (see [capacity](#capacity)). When references must survive `push`, use a [`GrowingArray`](/collections/growing-collections), which never moves its items.

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

### Native

- **Layout.** An array value is a pointer to a heap header `{ pointer, length, capacity }`. Small item buffers share the header's allocation; larger buffers use a separate block from `fa_alloc`. An empty array allocates its header only.
- **Recursive items.** A collection supplies storage for its object items, including recursive objects: each object is stored directly in its slot, without an additional box. Direct recursive fields inside that object, including optionals, retain an owning box when their inline expansion would cycle. The same rule applies to map entries, set elements, growing arrays, pools and columnars. Copies still recursively copy owned payloads; moving the array's items follows the same reference invalidation rules.
- **Growth.** The capacity doubles, starting at 4 items. Growth allocates a larger buffer, relocates the items bytewise, and frees the old buffer. Before a counted loop that always pushes one item to an empty array field, the compiler can reserve the loop's item count. It keeps the normal capacity checks: calls through aliases may add further items. A nonempty field is not reserved early, so references to its existing items remain valid until the actual push.
- **Capacity.** `from(…, capacity = n)` allocates `n` items up front; `reserve(additional)` grows the buffer to at least `length + additional` items (doubling when that is more), so a `reserve` in a loop stays constant on average. `capacity` reads the header.
- **`push`** has an inline fast path (`length < capacity`: store, then increment) and a cold call to the growth function. An object literal is stored field by field straight into the new slot, never built in a temporary and copied; a value with more fields than the item type is copied without them.
- **`insert` and `delete(index)`** are a `memmove` of the tail. `deleteWhere` compacts in place in one pass, with a read index and a write index, and drops the deleted items.
- **`filter` of a temporary array** (`text.split(" ").filter(…)`, with a callback that can't throw) compacts that array in place, like `deleteWhere`, and returns it: no second array. On any other array, `filter` builds a new array with room for every item.
- **Indexing.** A proven index is a plain load. Otherwise the bound check is one compare and a cold branch that returns `IndexError`. A negative index is `length + index`, checked the same way.
- **Slicing** allocates a new array and copies the range: a `memcpy` for flat items, the generated `copy<T>` otherwise.
- **References.** A reference to an item is a pointer to it.
- **Iteration.** The items pointer and length are loaded before the loop. The iteration rule forbids structural changes to the array, including through aliases and calls; writes to the items remain visible.

### JavaScript

- An `Array` is a JavaScript array. The `capacity` option of `from` is ignored and `reserve` compiles to nothing (its argument is still evaluated): preallocating with `new Array(n)` would create a *holey* array, which engines optimize less. `capacity` is the `length`.
- Negative indices compile to `array[array.length + index]` after the bound check. A proven index needs no check.
- A reference to an object item is the object itself. A reference to a primitive item is its index: the type of the reference names the array, so the index is enough, and `score *= 10` in a loop compiles to `scores[i] *= 10`.
- Since a reference to an object item is the object, writing a whole object into a place that a live reference may designate (`users[0] = other`, `mutates self[index]`) assigns it field by field into the existing object (a generated `assign` function), so that the reference sees the new value. Elsewhere, the value is simply replaced.
- Slicing is `slice()`, followed by a copy of each item when the items own memory (objects are values in Fa, not shared pointers).
- `sortInPlace` and `reverseInPlace` are `sort` and `reverse`; `sort()` and `reverse()` work on a copy (`toSorted`, `toReversed`).
:::
