# Declaring values

## Basic declaration

In Fa, you declare values using a declaration keyword:

- `let` to declare an immutable value
- `mutable` to declare a mutable value
- `use` to give another name to an existing value, without copying it
- `static` to declare a value computed at compile time
- `function` to declare a function (see [defining functions](/functions/defining-functions))
- `reactive` to declare a mutable value that can be observed for changes
- `derived` to declare a value that is computed from other reactive values

All these statements have the same syntax:

`<declaration> <identifier>[: <type>] = <value>`

The type annotation is optional if it can be inferred from the value. When it is present, it is a constraint: the value must satisfy the type.

Basic examples:

```fa
-- declaring an immutable integer
-- with a type annotation
let myNumber: Integer = 12

-- the type can be inferred, so annotations are optional
let myInferredNumber = 42

-- declaring an immutable string
let myString: String = "hello"

-- declaring an immutable array
let myArray = [1, 2, 3]
let myTypedArray: Array(Integer) = [1, 2, 3]

-- declaring an immutable object
let myObject = {
	foo = 12
	bar = "hello"
	nested = {
		baz = 42
	}
}

-- you can add type information to fields within an object
let myTypedObject = {
	foo: Integer = 12
	bar: String = "hello"
	nested: { baz: Integer } = {
		baz = 42
	}
}

-- you can declare an optional value with `?` or `Optional`
-- an optional can take the value `none`
let someNumber: Integer? = 12
let noNumber: Integer? = none
let otherNumber: Optional(Integer) = 24
```

## Assignment

There is no way to introduce a name without a keyword. A bare `name = value` assigns a new value to an existing **mutable** value; it never declares one.

```fa
mutable count = 0
count = 1 -- assignment

total = 12 -- error: `total` is not declared
```

## Immutable values

By default, all values declared with `let` are constants with "deep immutability":

- you cannot re-assign the value
- you cannot change any fields in the object
- you cannot add or remove items in a collection

```fa
-- by default, "let" declares a constant value
let myValue = 12

myValue += 1 -- error: cannot reassign an immutable value

let myObject = { foo = 12 }

myObject.foo += 1 -- error: cannot change the field of an immutable object
```

## Mutable values

To create a mutable value, declare it with the `mutable` keyword:

```fa
mutable myValue = 12

myValue += 1 -- this works

mutable myObject = { foo = 12 }

myObject.foo += 1 -- this works too

myObject = { foo = 14 } -- and this works as well
```

## References with `use`

`let` and `mutable` copy the value they are given. `use` does not copy: it gives another name to an existing value. Reading or writing through the alias is reading or writing the original value.

```fa
mutable people: Array(Person) = [Person { name = "Ana" }, Person { name = "Bao" }]

let copied = people[1]     -- a copy: changing it leaves the array untouched
use referenced = people[1] -- an alias of the element inside the array

referenced.name = "Cleo"
console.log(people[1].name) -- "Cleo"
```

An alias can be written through only if the original value is mutable, and it cannot outlive the value it refers to. If the original value is changed by an operation that can move its elements (for example `people.pop()`), the alias is invalidated and using it afterwards is a compile error. See [owned references](/advanced/owned-references).

`use` also works with destructuring and imports:

```fa
use { name, age } = person
use { Button as UIButton, Card } = ui.components
```

## Compile-time values

`static` declares a value that is computed by the compiler. It is used plainly, like any other value: every use is replaced by its value in the compiled output.

```fa
static maximumUsers = 1000
static version = @readFile("version.txt")

console.log(version) -- compiled to `console.log("1.4.2")`
```

A `static` value must be initialized with an expression the compiler can evaluate. Compile-time evaluation is deterministic: it cannot read the clock, the network or random numbers. See [metaprogramming](/advanced/metaprogramming).

## Functions

Functions are declared with the `function` keyword:

```fa
function add = (a: Integer, b: Integer): Integer => a + b
```

Lambdas such as `(x) => x + 1` are only written where a value is expected, for example as an argument. Declaring a function with `let` is an error:

```fa
let add = (a: Integer, b: Integer) => a + b -- error: use `function` to declare a function
```

At the top level of a file and inside `type` declarations, functions are hoisted: they can be used before the line that declares them. Inside a block, a function is visible after its declaration.

```fa
-- `multiply` can be used even though it is declared below
let product = multiply(6, 7)

function multiply = (a: Integer, b: Integer): Integer => a * b
```

Learn more about [functions](/functions/defining-functions).

## Reactive values

A **reactive value** is a mutable value that can trigger side effects when it changes. Declare it with `reactive`.

`derived` declares a read-only value computed from other reactive values, and recomputed whenever they change.

```fa
reactive x = 12

-- derived from a reactive value
derived doubled = x * 2

-- derived from another derived value
derived quadrupled = doubled * 2

x = 4
assert doubled == 8
assert quadrupled == 16
```

Learn more about [reactivity](/advanced/reactivity).

## Function Parameters

Function parameters are **immutable references**: the function reads the value given by the caller without copying it, and cannot change it.

```fa
function increaseFoo = (input: { foo: Integer, bar: Integer }) => {
	-- compiler error: cannot mutate an immutable parameter
	input.foo += input.bar
}
```

To change a parameter, a function declares the mutation as an **effect** in its signature, with `mutates`:

```fa
function increaseFoo = (input: { foo: Integer, bar: Integer }) mutates input => {
	input.foo += input.bar -- this works
}
```

The effect can be as precise as a single field (`mutates input.foo`) or a single index (`mutates items[index]`). See [defining functions](/functions/defining-functions).

Calling a mutating function requires a mutable value:

```fa
let myValue = { foo = 12, bar = 2 }

-- compiler error: `increaseFoo` mutates `input`, but `myValue` is immutable
increaseFoo(myValue)

mutable myMutableValue = { foo = 12, bar = 2 }

-- this works
increaseFoo(myMutableValue)
```

A function can only change the inside of a parameter: it can update the fields of an object or the elements of a collection, but it cannot replace the parameter itself:

```fa
function increaseFoo = (input: { foo: Integer, bar: Integer }) mutates input => {
	-- compiler error: cannot reassign a function parameter
	input = {
		foo = input.foo + input.bar
		bar = input.bar
	}

	-- this works
	input.foo += input.bar
}
```

For the same reason, a parameter of a primitive type (numbers, booleans, strings, ...) cannot be mutated:

```fa
function increment = (value: Integer) mutates value => { -- error: a primitive parameter cannot be mutated
	value += 1
}

-- return the new value instead
function incremented = (value: Integer) => value + 1
```

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

Declaring the same name twice in the **same** scope is a compile error:

```fa
let value = 1
let value = 2 -- error: `value` is already declared in this scope
```

## End of scope

Every value is destroyed at the end of the scope that declares it: there is no garbage collector. A `use` alias does not own anything, so it simply stops existing at the end of its scope; the value it refers to is destroyed with its own scope. See [memory management](/advanced/memory-management).

```fa
do {
	let numbers = [1, 2, 3]
	console.log(numbers.length)
} -- `numbers` is destroyed here
```
