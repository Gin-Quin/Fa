# Iteration

Should I implement an `iterate().collect()` like in Rust? I like this approach as it makes iteration with functions much faster.

```rust
let a = vec![0, 1, 2, 3, 4, 5].iter().map(|x| x + 1).filter(|x| x % 2 === 0).collect();
```

```ts
const a = [0, 1, 2, 3, 4, 5].map((x) => x + 1).filter((x) => x % 2 === 0);
```

The advantage of Rust is that it only does one total iteration over the array.

This would be the JS equivalent:

```ts
const a = []

for (const value of [0, 1, 2, 3, 4, 5]) {
  const x = value + 1
  if (x % 2 === 0) {
    a.push(x)
  }
}
```

With Fa, two possibilities:

1. `[0, 1, 2, 3, 4, 5].map((x) => x + 1).filter((x) => x % 2 === 0)` - We implicitly transform multiple iterations into one.
2. `[0, 1, 2, 3, 4, 5].iterate().map((x) => x + 1).filter((x) => x % 2 === 0)` - We demand explicitness with iterate() and collect().

