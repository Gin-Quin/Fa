# Arrays

An array holds an ordered list of values of the same type. Arrays can be mutable or immutable.

```fa
let myArray = [1, 2, 3]
let myArray: Array(Int) = [1, 2, 3]
let myArray = Array(Int) { 1, 2, 3 }

mutable myMutableArray = [1, 2, 3]
mutable myMutableArray: Array(Int) = [1, 2, 3]
mutable myMutableArray = Array(Int) { 1, 2, 3 }
```

An empty array needs its type, because there is nothing to infer from:

```fa
mutable empty = Array(Int) {}
```

Reserve capacity up front when you know the size:

```fa
mutable buffer = Array(Int, capacity = 1000) {}
```

Arrays own their storage on the heap. See [memory management](../advanced/memory-management).

## Adding and removing items

```fa
mutable users = ["Ana", "Bao"]

users.push("Cleo")     -- ["Ana", "Bao", "Cleo"]
users.insert(1, "Dan") -- ["Ana", "Dan", "Bao", "Cleo"]
users.delete(0)        -- ["Dan", "Bao", "Cleo"]
users.pop()            -- String?, removes the last item
users.clear()
```

`push`, `insert` and `delete` may move the array's storage, which invalidates every reference into it. The compiler tracks this — see [owned references](../advanced/owned-references).

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

## Iteration

```fa
let items = ["a", "b", "c"]

for items >> item {
  console.log(item)
}

for items >> item, index {
  console.log("{index}: {item}")
}
```

Iterating yields **references**, so writing through the binding writes into the array:

```fa
mutable scores = [1, 2, 3]

for scores >> score {
  score *= 10
}

console.log(scores) -- [10, 20, 30]
```

## Slicing and copying

Indexing with a range copies the selected items into a new array. `..` excludes the upper bound, `..=` includes it.

```fa
let data = [1, 2, 3, 4, 5]

let middle = data[1..4]  -- [2, 3, 4]
let same = data[1..=3]   -- [2, 3, 4]
let tail = data[2..]     -- [3, 4, 5]
let copy = data[..]      -- a full copy
```

## Common methods

```fa
let numbers = [3, 1, 2]

numbers.length
numbers.isEmpty()
numbers.includes(2)
numbers.indexOf(2)                              -- Int | NotFound
numbers.find((n: Int) => n > 2)                 -- Int?
numbers.filter((n: Int) => n > 1)               -- Array(Int)
numbers.map((n: Int) => n * 2)                  -- Array(Int)
numbers.reduce((acc: Int, n: Int) => acc + n, 0)
numbers.every((n: Int) => n > 0)
numbers.some((n: Int) => n > 2)
numbers.reverse()
numbers.sort()
numbers.sort((a: Int, b: Int) => a - b)
```

On a mutable array, these also apply in place:

```fa
mutable numbers = [3, 1, 2]

numbers.sortInPlace()
numbers.reverseInPlace()
numbers.retain((n: Int) => n > 1) -- keeps matching items, drops the rest
```

`retain` is the way to remove items while examining them: mutating an array during iteration is a compile error.

Chained calls are fused into a single pass — see [iterating with methods](../control-flow/loops#iterating-with-methods).

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
console.log(@typeToString(TypeOf(first))) -- "Int"

-- rejected at compile time
let fourth = array[3]
```

When the index comes from a [ranged integer](../values/numbers#minimum-and-maximum-values), the same reasoning applies:

```fa
let index: Int(min = 0, max = 2) = readIndex()
let value = array[index] -- no bounds check
```

When the compiler cannot prove the index is valid, indexing returns an error union that you must handle:

```fa
let index = readAnyNumber()

let value = array[index]       -- Int | IndexError
let value = array[index] ?? 0
```

This is why Fa needs neither `get` nor `get_unchecked`: there is a single indexing syntax, and its result type tells you whether a check was needed.
