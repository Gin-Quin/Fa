# Arrays

An array holds an ordered list of values of the same type. Arrays can be mutable or immutable.

A list literal `[...]` is an array, unless the context expects another collection (see [creating collections](#creating-collections)).

```fa
let myArray = [1, 2, 3] -- immutable array: you cannot modify, add or remove elements
let myTypedArray: Array(Integer) = [1, 2, 3]

mutable myMutableArray = [1, 2, 3] -- mutable array: you can modify, add and remove elements
mutable myTypedMutableArray: Array(Integer) = [1, 2, 3]
```

An empty array needs its type, because there is nothing to infer from:

```fa
mutable empty: Array(Integer) = []
```

Reserve capacity up front when you know the size, with the `capacity` option of `from`:

```fa
mutable buffer = Array(Integer).from([], capacity = 1000)
```

Arrays own their storage on the heap. See [memory management](/advanced/memory-management).

## Creating collections

All collections (`Array`, `Set`, `Map`, `Bag`, `Columnar`, ...) are created from a list literal `[...]`. Braces `{ ... }` are only used for objects.

When the type of the collection is known from the context (a declaration, a parameter, a return type, a field), the literal takes this type:

```fa
let numbers: Set(Integer) = [1, 2, 3]
let scores: Map(String, Integer) = [("ana", 12), ("bao", 17)]
mutable humans: Bag(Human) = []

function total = (values: Set(Integer)): Integer => values.sum()
total([1, 2, 3]) -- the literal is a `Set(Integer)`
```

When there is no context, use the `from` function of the collection type. The type of the elements is inferred from the list, or can be given explicitly:

```fa
let distinctCount = Set.from([1, 2, 2, 3]).length -- 3
let floats = Set(Float).from([1, 2, 3])
```

`from` also accepts the options of the collection:

```fa
let users = Set.from(userList, compare = (a, b) => a.id == b.id)
mutable buffer = Array(Integer).from([], capacity = 1000)
```

There is one form per situation: using `from` where the context already gives the type (and no option is passed) is a warning, and the formatter rewrites it into a plain literal.

## Adding and removing items

```fa
mutable users = ["Ana", "Bao"]

users.push("Cleo")     -- ["Ana", "Bao", "Cleo"]
users.insert(1, "Dan") -- ["Ana", "Dan", "Bao", "Cleo"]
users.delete(0)        -- ["Dan", "Bao", "Cleo"]
users.pop()            -- String?: removes and returns the last item, or `none` if the array is empty
users.clear()          -- []
```

Adding or removing items requires a `mutable` array: these methods declare the `invalidates self` effect, and calling a function that mutates a value is only allowed on a `mutable` value.

`push`, `insert`, `delete`, `pop` and `clear` may move the array's storage or shift its items, which invalidates every reference into it. The compiler tracks this through the `invalidates self` effect: a reference used after one of these calls is a compile error, until it is declared again with `use`. See [owned references](/advanced/owned-references#arrays).

```fa
mutable users = ["Ana", "Bao"]
use first = users[0]

users.push("Cleo")

console.log(first) -- error: `first` may have been invalidated by `users.push`
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

Indexing is checked at compile time when the compiler can prove the index is valid. Otherwise, indexing `throws IndexError`, which must be propagated with `try` or handled with `catch` like any other [error](/values/errors). When a missing item is a normal situation, use `get`, which returns an [optional](/values/optionals) instead:

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

## Common methods

```fa
let numbers = [3, 1, 2]

numbers.length                           -- 3
numbers.isEmpty()                        -- false
numbers.includes(2)                      -- true
numbers.indexOf(2)                       -- Integer?
numbers.find((n) => n > 2)               -- Integer?
numbers.filter((n) => n > 1)             -- Array(Integer)
numbers.map((n) => n * 2)                -- Array(Integer)
numbers.reduce((total, n) => total + n, 0)
numbers.every((n) => n > 0)
numbers.some((n) => n > 2)
numbers.reverse()                        -- a reversed copy: [2, 1, 3]
numbers.sort()                           -- a sorted copy: [1, 2, 3]
numbers.sort((a, b) => b - a)            -- a copy sorted with a custom order: [3, 2, 1]
```

`length` is a property, not a method. `sort()` and `reverse()` never modify the array: they return a new one.

On a mutable array, these methods work in place:

```fa
mutable numbers = [3, 1, 2]

numbers.sortInPlace()             -- [1, 2, 3]
numbers.reverseInPlace()          -- [3, 2, 1]
numbers.deleteWhere((n) => n < 2) -- deletes the matching items: [3, 2]
```

`deleteWhere` is the way to remove items while examining them: removing items of an array during a loop over it is a compile error.

Chained calls are fused into a single pass. See [iterating with methods](/control-flow/loops#iterating-with-methods).

### Effects of the methods

The methods that modify an array declare it in their signature with an [effect](/functions/defining-functions):

| Method | Effect |
| --- | --- |
| `push`, `insert`, `delete`, `pop`, `clear` | `invalidates self` |
| `sortInPlace`, `reverseInPlace`, `deleteWhere` | `invalidates self` |
| `array[index] = value` | `mutates self[index]` |
| `length`, `get`, `includes`, `indexOf`, `find`, `filter`, `map`, `reduce`, `every`, `some`, `sort`, `reverse` | none |

`invalidates self` implies `mutates self`. Writing to one item (`mutates self[index]`) does not move the other items, so it does not invalidate references.

## Smart bound checks

Fa's compiler tracks how large a collection is and what range an index can take, so most bounds checks disappear at compile time.

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
console.log(@typeToString(Type(first))) -- "Integer"

-- rejected at compile time
let fourth = array[3]
```

When the index comes from a [ranged integer](/values/numbers#minimum-and-maximum-values), the same reasoning applies:

```fa
let index: Integer(min = 0, max = 2) = readIndex()
let value = array[index] -- no bounds check
```

When the compiler cannot prove the index is valid, indexing `throws IndexError`. Propagate the error with `try`, handle it with `catch`, or use `get` to receive an optional:

```fa
function readValue = (array: Array(Integer), index: Integer): Integer throws IndexError => {
	let value = try array[index] -- propagates `IndexError` to the caller
	value
}

let caught = array[index] catch error => 0
let fallback = array.get(index) ?? 0
```

This is why Fa needs no `get_unchecked`: there is a single indexing syntax, checked at compile time when possible, and its signature tells you whether it can fail. `get` is only for the cases where a missing item is expected.
