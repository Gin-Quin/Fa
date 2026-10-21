# Declaring values

## Basic declaration

In Fa, you declare values using one of the following declaration keywords:

- `let` — an immutable value
- `mutable` — a mutable value
- `use` — a reference to an existing value, without copying it
- `static` — a value that exists only at compile time
- `function` — an immutable, hoisted function
- `reactive` — a mutable value that can be observed for changes
- `derived` — a read-only value computed from other reactive values

All of them share the same syntax:

```
<keyword> <identifier>[: <type>] = <value>
```

The type annotation is optional whenever the type can be inferred from the value.

```fa
-- an immutable integer, with a type annotation
let myNumber: Int = 12

-- the type can be inferred, so the annotation is optional
let myInferredNumber = 42

-- an immutable string
let myString: String = "hello"

-- an immutable array
let myArray = [1, 2, 3]
let myArray: Array(Int) = [1, 2, 3]
let myArray = Array(Int) { 1, 2, 3 }

-- an immutable object
let myObject = {
  foo = 12
  bar = "hello"
  nested = {
    baz = 42
  }
}

-- fields inside an object literal can carry type annotations
let myObject = {
  foo: Int = 12
  bar: String = "hello"
  nested: { baz: Int } = {
    baz = 42
  }
}

-- an optional value, declared with `?` or with `Optional`
-- an optional value may hold `none`
let myOptionalNumber: Int? = 12
let myOptionalNumber: Int? = none
let myOptionalNumber: Optional(Int) = 24
```

There is no way to introduce a name without a keyword. A bare `name = value` assigns to an existing mutable value; it never declares one.

## Immutable values

Values declared with `let` are constants with **deep immutability**:

- you cannot reassign the value,
- you cannot change any field of the object,
- you cannot add or remove items in a collection.

```fa
let myValue = 12

myValue += 1 -- error: cannot reassign an immutable value

let myObject = { foo = 12 }

myObject.foo += 1 -- error: cannot mutate a field of an immutable object
```

## Mutable values

To create a mutable value, declare it with `mutable`:

```fa
mutable myValue = 12

myValue += 1 -- works

mutable myObject = { foo = 12 }

myObject.foo += 1 -- works

myObject = { foo = 14 } -- works as well
```

## References

`use` declares a **reference** to an existing value instead of copying it. The reference and the original are the same value: writing through one is writing through the other.

```fa
mutable people = [Person { name = "Ana" }, Person { name = "Bao" }]

let copied = people[1]     -- a copy: changing it leaves the array untouched
use referenced = people[1] -- a reference into the array

referenced.name = "Cleo"
console.log(people[1].name) -- "Cleo"
```

A reference is mutable if and only if the value it points into is mutable, and it cannot outlive that value. The compiler enforces both rules. See [owned references](../advanced/owned-references).

## Compile-time values

`static` declares a value that exists only during compilation. It is not present at runtime.

```fa
static version = @readFile("../version.txt")

console.log(version) -- compiled to `console.log("1.4.2")`
```

A `static` value must be initialised with an expression the compiler can evaluate. See [metaprogramming](../advanced/metaprogramming).

## Functions

Functions are ordinary values, so they are declared with the same syntax as everything else:

```fa
let add = (a: Int, b: Int): Int => a + b
```

The dedicated `function` keyword declares a function that is both immutable and **hoisted**, meaning it can be used before the line that declares it:

```fa
-- `add` can be used even though it is declared below
let sum = add(6, 7)

function add = (a: Int, b: Int): Int => a + b
```

Learn more about [functions](../functions/defining-functions).

## Reactive values

A **reactive value** is a mutable value that may trigger side effects when it changes. Declare it with `reactive`, and observe it with `derived` or `effect`.

`derived` declares a read-only value computed from other reactive values, and recomputed whenever they change.

```fa
reactive x = 12

-- derived from a reactive value
derived doubled = x * 2

-- derived from another derived value
derived quadrupled = doubled * 2

x = 4
assert(doubled == 8)
assert(quadrupled == 16)
```

Learn more about [reactivity](../advanced/reactivity).

## Shadowing

Declaring a name that already exists in an **enclosing** scope shadows it for the rest of the current scope:

```fa
let value = 1

do {
  let value = 2      -- shadows the outer `value`
  console.log(value) -- 2
}

console.log(value) -- 1
```

Declaring the same name twice in the **same** scope is a compile error.
