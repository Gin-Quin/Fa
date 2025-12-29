# Declaring values

In Fa, we talk of "values" instead of "variables".

## Basic Declaration

Unlike TypeScript or JavaScript, Fa doesn't use keywords like `const`, `let`, or `var` for declarations. You simply write the name followed by the value:

```fa
-- declaring a value
let myValue: Number = 12

-- declaring with a type annotation (optional)
let myString: String = "hello"

-- type can be inferred, so annotations are optional
let myInferredNumber = 42
```

## Mutable Values

By default, all values are constants with "interior mutability". You cannot reassign a value directly, but you can change its inner content (if it's a mutable data structure).

```fa
-- by default, "let" declares a constant value
let myValue = 12

myValue += 1 -- error: cannot reassign constant value

-- to create a mutable value, declare it with the **mutable** keyword
mutable myValue = 12

myValue += 1 -- this works

-- objects and containers have "inner mutability", which means you can update them
let myObject = { foo = 12 }

myObject.foo = 13 -- this is allowed

-- however, only the current scope is allowed to update them
-- for example, a function cannot modify the value of a variable declared outside its scope

function updateFoo = (object: { foo: Number }) {
  object.foo += 1 -- this is not allowed because `object` is a **constant reference**
}

-- instead, the function can return a new object with the updated value
type Foo = { foo: Number }

function incrementFoo = (object: Foo): Partial(Foo) {
  return { foo = object.foo + 1 }
}

myObject << incrementFoo(myObject)
```

The only functions that are able to mutate values are **methods**.

## Type Inference and Annotations

Fa uses the same type annotation syntax as TypeScript, but type annotations are optional as types can be inferred:

```fa
-- With type annotation
let user: {
  name: String
  age: Number
} = {
  name = "Alice"
  age = 30
}

-- Without type annotation (inferred)
let user = { name = "Alice", age = 30 }
```
