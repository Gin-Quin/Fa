# Ranges

A range describes a span of integers. Ranges can be mutable or immutable.

```fa
let myRange = 1..3
let myRange: Range = 1..3
let myRange = Range(start = 1, end = 3)

mutable myMutableRange = 1..3

myMutableRange.start = 2
myMutableRange.end = 4
```

## Exclusive and inclusive bounds

`..` **excludes** the upper bound, `..=` **includes** it.

```fa
0..3   -- 0, 1, 2
0..=3  -- 0, 1, 2, 3
```

This rule holds everywhere a range appears: iteration, slicing, and pattern matching.

```fa
let data = [10, 20, 30, 40, 50]

data[1..3]  -- [20, 30]
data[1..=3] -- [20, 30, 40]
```

:::tip
`..` is the form to reach for by default. `array[0..array.length]` covers the whole array, and `a..b` always contains `b - a` items.
:::

## Unbounded ranges

You can leave either bound out:

```fa
let fromOne  = 1..   -- 1 and upwards
let upToTen  = ..10  -- everything below 10
let upToTenI = ..=10 -- everything up to and including 10
let all      = ..    -- everything
```

Unbounded ranges are useful for slicing, where the missing bound is taken from the collection:

```fa
let data = [10, 20, 30, 40, 50]

data[2..]  -- [30, 40, 50]
data[..2]  -- [10, 20]
data[..]   -- a copy of the whole array
```

An unbounded range cannot be iterated on its own unless it has a lower bound:

```fa
for 1.. >> i {   -- an infinite loop; use `break` to leave it
  if i > 100 { break }
}

for ..10 >> i {  -- error: this range has no start
}
```

## Iterating a range

```fa
for 0..10 >> i {
  console.log(i) -- 0 to 9
}
```

## Slicing with a range

Indexing a collection or a string with a range copies the selected items into a new value:

```fa
let subarray = numbers[6..9] -- copies three elements into a new array
let substring = text[0..5]
```

Slicing with a range that the compiler cannot prove to be in bounds returns an error:

```fa
let data = [1, 2, 3]

let a = data[0..2]     -- Array(Int): provably in bounds
let b = data[0..n]     -- Array(Int) | IndexError: `n` is unknown
```

See [smart bound checks](../collections/arrays#smart-bound-checks).

## Matching a range

Ranges can be used as patterns:

```fa
let label = when score is {
  0..60   => "fail"
  60..80  => "pass"
  80..=100 => "excellent"
  else    => "out of range"
}
```

## Ranges as values

A range is an ordinary value: it can be stored, passed to a function and returned.

```fa
function middleThird = (length: Int): Range => (length // 3)..(2 * length // 3)

let data = [1, 2, 3, 4, 5, 6, 7, 8, 9]
let middle = data[middleThird(data.length)] -- [4, 5, 6]
```
