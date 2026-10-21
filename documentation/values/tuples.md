# Tuples

A **tuple** groups a fixed number of values of possibly different types, addressed by position rather than by name.

```fa
let pair = (1, "hello")
let pair: Tuple(Int, String) = (1, "hello")
```

Like objects, tuples live on the stack and are copied on assignment.

## When to use a tuple

Use a tuple for a short-lived grouping where naming the fields would add noise — most often to return two values from a function, or to hold a key/value pair while iterating a map.

Use an [object](./objects) for anything that is passed around, stored, or has more than three components. Named fields are cheaper to read than positions.

```fa
-- fine as a tuple
function divide = (a: Int, b: Int(min = 1)): Tuple(Int, Int) => (a // b, a %% b)

-- better as an object
type Measurement = { value: Decimal, unit: String }
```

## Accessing elements

Tuple elements are accessed by index, and the index must be a literal so the compiler knows the element's type:

```fa
let pair = (1, "hello")

let number = pair[0] -- Int
let text   = pair[1] -- String

let index = readIndex()
let oops  = pair[index] -- error: a tuple index must be known at compile time
```

## Extracting from a tuple

The extract operator `>>` unpacks a tuple into named values:

```fa
let pair = (1, "hello")

pair >> use (number, text)

console.log(number) -- 1
console.log(text)   -- "hello"
```

You can extract straight into existing mutable values, or provide defaults:

```fa
mutable a = 0
mutable b = 0

pair >> (a, b)

triple >> use (x, y, z = 0)
```

See [extract and insert](../operations/extract-insert).

## Tuples in `for` loops

Iterating a collection of tuples extracts them directly. This is how maps are iterated:

```fa
let usersById = Map(Int, String) {
  (1, "Ana")
  (2, "Bao")
}

for usersById >> (id, name) {
  console.log("{id} -> {name}")
}
```

## Tuples and objects

Tuples and objects are **distinct** types, and there is no implicit conversion between them:

```fa
type Point = { x: Int, y: Int }

let tuple: Tuple(Int, Int) = (3, 4)
let point: Point = tuple -- error: a tuple is not an object
```

Convert explicitly:

```fa
let point = Point(3, 4)        -- constructor syntax
let point = Point { x = 3, y = 4 }
```

## Comparing tuples

`==` compares tuples element by element:

```fa
assert((1, "a") == (1, "a"))
```

Two tuples of different lengths, or with different element types, are different types and cannot be compared.

## The unit tuple

An empty tuple `()` is not a value in Fa. A function that returns nothing returns `None`. See [optionals](./optionals).
