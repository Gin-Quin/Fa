
### Type difference

❓ It can be done via key lookup, but it's somehow complicated to get the result.

```ts
type TypeA = { foo: String, bar: Integer }
type TypeB = { foo: Integer, baz: String }

type MyType = TypeA - TypeB
// MyType is { bar: Integer }
```

Can be done with the `Keys` type function:

```ts
type MyType = Pick(TypeA, Exclude(Keys(TypeA), Keys(TypeB)))
// MyType is { bar: Integer }
```

### Type addition

❌ No need because it's already handled by type extension `{ ...TypeA, ...TypeB }`

```ts
type MyType = { foo: String, bar: Integer } + { foo: Integer, baz: String }
// MyType is { foo: Integer, bar: Integer, baz: String }
```
