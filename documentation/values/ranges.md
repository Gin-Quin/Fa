# Ranges

Ranges are a set of consecutive numbers. They can be mutable or immutable.

Like in Rust, there are two kinds of ranges:

- `start..end` is **exclusive**: it contains the numbers from `start` to `end`, `end` excluded.
- `start..=end` is **inclusive**: it contains the numbers from `start` to `end`, `end` included.

```fa
let exclusive = 1..3 -- 1, 2
let inclusive = 1..=3 -- 1, 2, 3
```

```fa
let myRange = 1..3
let myRange: Range = 1..3
let myRange = Range(start = 1, end = 3)
let myRange = Range(1, 3)

mutable myMutableRange = 1..3

myMutableRange.start = 2
myMutableRange.end = 4
```

You can create ranges without lower or upper bounds:

```fa
let myRange = 1.. -- 1 to infinity
let myRange = ..3 -- -infinity to 3 (excluded)
let myRange = ..=3 -- -infinity to 3 (included)
let myRange = .. -- everything
```

## Iterating over a range

```fa
for i in 1..10 {
  console.log(i) -- 1 to 9
}

for i in 1..=10 {
  console.log(i) -- 1 to 10
}
```

## Slicing over a range

```fa
let subarrayOfNumbers = arrayOfNumbers[6..9] -- copies the elements at indices 6, 7 and 8 into a new array
```
