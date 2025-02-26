# Declaring values

In Fa, we talk of "values" instead of "variables".

## Basic Declaration

Unlike TypeScript or JavaScript, Fa doesn't use keywords like `const`, `let`, or `var` for declarations. You simply write the name followed by the value:

```ts
// declaring a value
myValue = 12

// declaring with a type annotation (optional)
myString: String = "hello"

// type can be inferred, so annotations are optional
myInferredNumber = 42
```

## Immutability and Shadowing

By default, all values are constants with "interior mutability". You cannot reassign a value directly, but you can:

1. Change its inner content (if it's a mutable data structure)
2. Shadow it using the `set` keyword

The behavior of shadowing differs between primitive and non-primitive values:

- For primitive values (numbers, strings, booleans), using `set` effectively reassigns the value
- For non-primitive values (objects, arrays), the original value continues to live until the end of the scope

```ts
// With primitive values
myValue = 12
set myValue = 13 // this reassigns the value
console.log(myValue) // 13

// With objects
myObject = { count: 0 }
set myObject = { count: 1 } // this shadows the value with a new object

// the initial value of myObject still exists in memory until the end of the scope
```

## Mutating Operators

You can use mutating operators with the `set` keyword for concise modifications:

```ts
counter = 10

// These are equivalent:
set counter = counter + 2
set counter += 2

// Other mutating operators work too
set counter *= 3
set counter **= 3
set counter /= 2
set counter %= 5
```

## Type Inference and Annotations

Fa uses the same type annotation syntax as TypeScript, but type annotations are optional as types can be inferred:

```ts
// With type annotation
user: {
  name: String
  age: Number
} = {
  name: "Alice"
  age: 30
}

// Without type annotation (inferred)
user = { name: "Alice", age: 30 }
```
