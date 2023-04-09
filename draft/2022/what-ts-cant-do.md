1. Meta-programming
2. Type functions
3. POJO constructors
4. Spread an object as function parameters

Other minor advantages:
5. better question operator (different than the optional access operator `?.`)

```ts
  if (x) return x
```

In Fa:
```coffee
  return x?
```

It's a small change but awesome for recursive functions for example, where you have to manually create and assign a new variable before checking its value.
