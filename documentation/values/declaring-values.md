# Declaring values

## Basic Declaration

In Fa, you declare values using a declarative statement:

- `let` to declare an immutable value
- `mutable` to declare a mutable value
- `function` to declare an immutable hoisted function
- `reactive` to declare a mutable value that can be observed for changes
- `derived` to declare a value that is computed from other reactive values

All these statements have the same syntax:

`<declaration> <identifier>[: <type>] = <value>`

The type annotation is optional if it can be inferred from the value.

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
-- an optional can take the value `none`
let myOptionalNumber: Number? = 12
let myOptionalNumber: Number? = none
let myOptionalNumber: Optional(Number) = 24
```

## Mutable Values

By default, all values declared with `let` are constants with "deep immutability":

- you cannot re-assign the value
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

## Function Parameters

Function parameters are **immutable references** by default.

```fa
function increaseFoo = (input: { foo: Number, bar: Number }) => {
	-- compiler error: cannot mutate immutable reference
	input.foo += input.bar
}
```

Again, you have to use the **mutable** keyword to declare it as **mutable reference**:

```fa
function increaseFoo = (mutable input: { foo: Number, bar: Number }) => {
	input.foo += input.bar -- this works
}
```

You cannot pass an immutable value to a mutable function parameter:

```fa
let myValue = { foo = 12, bar = 2 }

-- compiler error: cannot pass immutable value to mutable parameter
increaseFoo(myValue)

mutable myMutableValue = { foo = 12, bar = 2 }

-- this works
increaseFoo(myMutableValue)
```

For mutable function parameters, only **interior mutability** is allowed. This means you can update the fields of an object or the elements of a container, but you cannot replace the object or container itself:

```fa
function increaseFoo = (mutable input: { foo: Number, bar: Number }) => {
	-- compiler error: cannot reassign function parameter
	-- even if it's mutable
	input = {
		foo = input.foo + input.bar
		bar = input.bar
	}
	
	-- this works
	input.foo += input.bar
}
```
