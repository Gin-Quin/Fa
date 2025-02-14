# Iteration

Should I implement an `iterate().collect()` like in Rust? I like this approach as it makes iteration with functions much faster.

```rust
let a = vec![0, 1, 2, 3, 4, 5].iter().map(|x| x + 1).collect();
```

```ts
const a = [0, 1, 2, 3, 4, 5].iterate().map((x) => x + 1).collect();
```
