What syntax for functions?

There are two different types of functions:

1. Pure functions (`fn` equivalent in Rust).
2. Functions that are declared with the `=>` syntax (closures in Rust).

The `=>` syntax is clear, the question is about the pure functions.

Propositions:

```ts
// Closure syntax
let add = (a: Number, b: Number): Number => {
  return a + b
}
let add = (a: Number, b: Number) => a + b // short syntax

// 1. Using 'let' with a fat arrow (same syntax as closures, but without the '=')
let add(a: Number, b: Number): Number => {
  return a + b
}
let add(a: Number, b: Number) => a + b // short syntax

// 2. Using 'let' without a fat arrow
let add(a: Number, b: Number): Number {
  return a + b
}
let add(a: Number, b: Number) => a + b // short syntax

// 3. Using 'function' without a fat arrow
function add(a: Number, b: Number): Number {
  return a + b
}
function add(a: Number, b: Number) => a + b // short syntax?
```

The three propositions are great:

1. The first one is the closest to the closure syntax. It's very easy to switch from closure to pure function.
2. The second one is a nicer version of the first one because no arrow is necessary.
3. The third one is the closest to the Typescript and Rust syntaxes and is more explicit.

I'll go with the third one.