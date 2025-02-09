In Fa, a `property` is a value with a getter and a setter.

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
