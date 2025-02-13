In Fa, a `property` is a single value with a getter and a setter.

It should be used sparingly as it produces **side-effects**. Prefer functions or explicit state management when possible.

```ts
value = Property(Number) {
  state = 12 /* initial state */
  get = () => state
  set = (value) => state = value
}

// then you can use value like a number
log(value) // log 12
value = 42
```
