# Types

In Fa, types can be manipulated like any other value.

A type is declared with the `type` keyword.

```ts
type MyType = {
  foo: String
}

MyType = type {
  foo: String
}
```

Unlike a value, a type is **immutable** and cannot have the `mutable` keyword.

```ts
type main = {
  foo: String
}
```
