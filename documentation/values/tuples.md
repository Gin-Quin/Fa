# Tuples

A **tuple** groups a fixed number of values of possibly different types, addressed by position rather than by name.

The type of a tuple is written `Tuple(...)`, with the type of each element, and a tuple value is written with parentheses:

```fa
let pair = (1, "one")
let typedPair: Tuple(Integer, String) = (1, "one")
```

Like objects, tuples are stored inline and are copied on assignment.

## When to use a tuple

Use a tuple for a short-lived grouping where naming the fields would add noise: most often to return two values from a function, or to hold a key/value pair while iterating over a map.

Use an [object](/values/objects) for anything that is passed around, stored, or has more than three components. Named fields are easier to read than positions.

```fa
-- fine as a tuple
function divide = (a: Integer, b: Integer(min = 1)): Tuple(Integer, Integer) => (a // b, a %% b)

-- better as an object
type Measurement = { value: Float, unit: String }
```

## Accessing elements

Tuple elements are accessed by index, and the index must be a literal, so that the compiler knows the type of the element:

```fa
let pair = (1, "one")

let number = pair[0] -- Integer
let text = pair[1]   -- String

let index = readIndex()
let unknown = pair[index] -- error: a tuple index must be a literal
```

## Destructuring a tuple

Destructuring unpacks a tuple into named values:

```fa
let pair = (1, "one")

let (number, text) = pair

console.log(number) -- 1
console.log(text)   -- "one"
```

Destructuring works with every declaration keyword:

```fa
mutable (count, label) = pair
use (first, second) = pair
```

It is also the natural way to receive several values from a function:

```fa
let (quotient, remainder) = divide(17, 5) -- 3, 2
```

See [destructuring](/operations/destructuring).

## Tuples in `for` loops

Iterating over a collection of tuples destructures them directly. This is how maps are iterated:

```fa
let usersById: Map(Integer, String) = [(1, "Ana"), (2, "Bao")]

for (id, name) in usersById {
	console.log("{id} -> {name}")
}
```

## Tuples in `when`

A `when` branch can bind the elements of a tuple with `as`:

```fa
when entry is {
	Tuple(Integer, String) as (id, name) => console.log("{id} -> {name}")
	else => console.log("not an entry")
}
```

## Tuples and objects

Tuples and objects are **distinct** types, and there is no implicit conversion between them:

```fa
type Point = { x: Integer, y: Integer }

let tuple: Tuple(Integer, Integer) = (3, 4)
let point: Point = tuple -- error: a tuple is not an object
```

Convert explicitly:

```fa
let (x, y) = tuple
let point: Point = { x = x, y = y }
```

## Comparing tuples

`==` and `!=` compare tuples element by element:

```fa
assert (1, "a") == (1, "a")
assert (1, "a") != (2, "a")
```

Two tuples of different lengths, or with different element types, are different types and cannot be compared.

## No unit tuple

An empty tuple `()` is not a value in Fa: there is no "unit" type.

A function that returns nothing has no return type. In a function type, the absence of a return value is written `Void`:

```fa
-- returns nothing: no return type
function log = (message: String) => {
	console.log(message)
}

-- `report` is a function pointer to a function that returns nothing
type Logger = {
	report: (message: String) => Void
}

let logger: Logger = { report = log }
```

`Void` is not `None`: `None` is the type of the `none` value of [optionals](/values/optionals), while `Void` means that there is no value at all.
