# Functions

## Defining Functions

In Fa, you define a function with the fat arrow `=>` operator.

The syntax is very similar to TypeScript's arrow functions, except that you use the `function` keyword:

```
// in TypeScript
const myFunction = (): ReturnType => {}

// in Fa
function myFunction = (): ReturnType => {}
```

When you declare a function with the `function` keyword:

- the function value is immutable,
- the function is hoisted, which means you can use it before it's declared.

It's also possible to define a function using a `let` or a `mutable` keyword:

```fa
let myImmutableFunction = (): ReturnType => {}

mutable myMutableFunction = (): ReturnType => {}
```

In that case, the function is not hoisted and follows the same rules as any value declared with the `let` or `mutable` keyword.

The return type of a function can be opted out if the function's body is a single expression. Otherwise, it's mandatory.

```fa
-- valid: type of a single expression is inferred
function addOne = (x: Number) => x + 1

 -- invalid: missing explicit return type
function addOne = (x: Number) => {
	return x + 1
}

-- valid: explicit return type
function addOne = (x: Number): Number => {
	return x + 1
}
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

### Generic Functions

Generic functions work similarly to generic functions in TypeScript. Generic parameters are declared between angle brackets and are available in the function's parameters, return type, and body.

Type parameters must be capitalized and included in the function's parameters:

```fa
function foo = <Type>(a: Type, b: Type) => {
	-- ...
}
```

A type parameter can be constrained to another type using the `:` syntax:

```fa
-- Type must be a structure with a Number field named bar
function foo = <Type: { bar: Number }>(a: Type, b: Type) => {
	-- ...
}
```

Generic parameters can also be static values. Static value parameters use camel case, with a lowercase first letter:

```fa
function foo = <staticNumber: Number>(dynamicNumber: Number) => {
	return staticNumber + dynamicNumber
}
```

### Methods and function pointers

Inside objects, you can define different types of functions:

- Methods
- Function pointers (using arrow syntax)

```fa
let myObject = {
    -- Associated functions (no self parameter)
    someAssociatedFunction(): Result => {
        -- ...
        return someValue
    }

    -- Methods (with self parameter)
    someMethod(self): Result => {
        -- ...
        return someValue
    }

    -- Mutable methods (with mutable self parameter)
    someMutation(mutable self): Result => {
        -- ...
        return someValue
    }

    -- Function pointers (using arrow syntax)
    someFunctionPointer = () => expression
    someFunctionPointer = (): Result => {
        -- ...
        return someValue
    }

    -- Function pointer methods
    someFunctionPointer = (self) => expression
    someFunctionPointer = (self): Result => {
        -- ...
        return someValue
    }

    -- Function pointer mutations
    someFunctionPointer = (mutable self): Result => {
        -- ...
    }
}
```

### Function aliases

You can also define function aliases:

```fa
function myFunctionClone = myFunction
```

It's especially useful when composing functions together:

```fa
function addAndMultiply = add ||> multiply
```

You can also use the `with` keyword to bind parameters:

```fa
function xyz = (x: Number, y: Number, z: Number) => x + y + z

function xz = xyz with (y = 0)
```

### Function factories

A factory is a function that returns another function. You can write it very naturally:

```fa
function makeAdd = (y: Number) => {
	return (x: Number) => x + y
}

// or, in one line:
function makeAdd = (y: Number) => (x: Number) => x + y

let add3 = makeAdd(3)
let add5 = makeAdd(5)
```

When making a new function from a factory, you will most of the time use `let`, unless the parameters of the function factory are known at build time (otherwise it will not be possible to hoist the function).

### Destructuring function parameters

You can use destructuring to simplify function parameters:

```fa
type SomeObject = {
	foo: String
	bar: Number
	baz: Boolean
}

function someFunction1 = (SomeObject >> { foo, bar }) => {
	console.log(foo, bar)
}

someFunction1({ foo: "Hello", bar: 42 })

function someFunction2 = (
	SomeObject >> { foo, bar }
	SomeObject >> { baz }
) => {
	console.log(foo, bar, baz)
}

someFunction2({ foo: "Hello", bar: 42 }, { baz: true })
```

### Anonymous structure parameter

Another syntax is to use anonymous structures as parameters:

```fa
function someFunction = ({ foo: String, bar: Number = 12 }) => {
	console.log(foo, bar)
}

someFunction3({ foo: "Hello", bar: 42 })
```
