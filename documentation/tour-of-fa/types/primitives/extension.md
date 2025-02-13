# Type extension

```ts
type TypeA = { foo: String }
type TypeB = { ...TypeA, bar: Integer }

// TypeB is { foo: String, bar: Integer }
```
