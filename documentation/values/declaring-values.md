# Declaring values

## Basic Declaration

In Fa, you declare values using one of the following declaration statements:

- `let` to declare an immutable value
- `mutable` to declare a mutable value
- `function` to declare an immutable hoisted function
- `reactive` to declare a mutable value that can be observed for changes
- `derived` to declare a value that is computed from other reactive values

All these statements have the same syntax:

`<declaration> <identifier>[: <type>] = <value>`

The type annotation is optional when it can be inferred from the value.

Basic examples:

```fa
-- declaring an immutable number
-- with a type annotation
let myNumber: Integer = 12

-- type can be inferred, so annotations are optional
let myInferredNumber = 42

-- declaring an immutable string
let myString: String = "hello"

-- declaring an immutable array
let myArray = [1, 2, 3]
let myArray: Array(Integer) = [1, 2, 3]
let myArray = Array(Integer)[1, 2, 3]

-- declaring an immutable object
let myObject = {
  foo = 12
  bar = "hello"
  nested = {
    baz = 42
  }
}

-- you can add type information to fields within an object
let myObject = {
  foo: Number = 12
  bar: String = "hello"
  nested: { baz: Number } = {
    baz = 42
  }
}

-- you can declare an optional value with `?` or `Optional`
-- an optional value can take the value `none`
let myOptionalNumber: Number? = 12
let myOptionalNumber: Number? = none
let myOptionalNumber: Optional(Number) = 24
```

## Mutable Values

By default, all values declared with `let` are constants with "deep immutability":

- you cannot reassign the value
- you cannot change any fields in the object
- you cannot add or remove items in a collection

```fa
-- by default, "let" declares a constant value
let myValue = 12

myValue += 1 -- error: cannot reassign constant value

let myObject = { foo = 12 }

myObject.foo += 1 -- cannot change the field of an immutable object
```

To create a mutable value, declare it with the **mutable** keyword:

```fa
mutable myValue = 12

myValue += 1 -- this works

mutable myObject = { foo = 12 }

myObject.foo += 1 -- this works too

myObject = { foo = 14 } -- and this works as well
```

## Functions

Functions can be declared using the special `function` keyword:

```fa
function add = (a: Number, b: Number) => {
	a + b
}
```

Using the `function` keyword enables "hoisting," which means the function can be used before it is declared.

```fa
-- can use `add` even if it's not defined yet
let sum = add(6, 7)

-- here, we define the `add` function
function add = (a: Number, b: Number) => {
	a + b
}
```

Learn more about [functions](../functions/defining-functions).

## Reactive Values

A **reactive value** is a mutable value that may trigger side effects when it changes. It's defined with `reactive` or `derived`, and can be observed using `derived` or `effect`.

The `reactive` keyword lets you define a mutable reactive value.

The `derived` keyword is used to define a read-only reactive value that is derived from other reactive values.

```fa
reactive x = 12

-- deriving a value from a reactive value
derived doubled = x * 2

-- deriving a value from another derived value
derived quadrupled = doubled * 2

x = 4
assert(doubled == 8)
assert(quadrupled == 16)
```

Learn more about [reactivity in Fa](../advanced/reactivity).
