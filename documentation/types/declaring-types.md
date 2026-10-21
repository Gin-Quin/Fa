# Types

In Fa, types can be manipulated like any other value.

A type is declared with the `type` keyword. Type names are capitalized, and fields are declared with `:`.

```fa
type MyType = {
  foo: String
}
```

Unlike a value, a type is **immutable** and cannot be declared with the `mutable` keyword.

```fa
mutable type MyType = { -- error: a type cannot be mutable
  foo: String
}
```
