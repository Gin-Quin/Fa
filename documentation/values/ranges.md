# Ranges

A range describes a span of consecutive integers. Ranges are values of the type `Range`, and can be mutable or immutable.

```fa
let myRange = 1..3
let myTypedRange: Range = 1..3

mutable myMutableRange = 1..3

myMutableRange.start = 2
myMutableRange.end = 4
```

## Exclusive and inclusive bounds

Like in Rust, `..` **excludes** the upper bound, and `..=` **includes** it.

```fa
0..3  -- 0, 1, 2
0..=3 -- 0, 1, 2, 3
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
let fromOne = 1..         -- 1 and upwards
let belowTen = ..10       -- everything below 10
let upToTen = ..=10       -- everything up to and including 10
let everything = ..       -- everything
```

Unbounded ranges are useful for slicing, where a missing bound is taken from the collection:

```fa
let data = [10, 20, 30, 40, 50]

data[2..]  -- [30, 40, 50]
data[..2]  -- [10, 20]
data[..]   -- a copy of the whole array
```

## Iterating over a range

```fa
for i in 0..10 {
	console.log(i) -- 0 to 9
}

for i in 1..=10 {
	console.log(i) -- 1 to 10
}
```

When you do not need the current number, leave the binding out:

```fa
for 1..=3 {
	console.log("hello") -- printed three times
}
```

A range can only be iterated if it has a lower bound. A range without an upper bound is an infinite loop, which you leave with `break`:

```fa
for i in 1.. {
	if i > 100 {
		break
	}
}

for i in ..10 { -- error: this range has no start
}
```

## Slicing with a range

Indexing a collection or a string with a range copies the selected items into a new value:

```fa
let subarray = numbers[6..9] -- copies the elements at indices 6, 7 and 8 into a new array
let substring = text[0..5]   -- copies the first five characters into a new string
```

When the compiler cannot prove that the range is in bounds, the slice `throws IndexError`, which you must propagate with `try` or handle with `catch`:

```fa
let data = [1, 2, 3]

let a = data[0..2]                 -- provably in bounds: no error possible
let b = try data[0..count]         -- `count` is unknown: throws IndexError
let c = data[0..count] catch error => []
```

See [smart bound checks](/collections/arrays#smart-bound-checks) and [errors](/values/errors).

## Matching a range

Ranges are patterns: they can be used with `is` and in the branches of a `when`. A value matches a range when the range contains it.

```fa
if age is 18.. {
	console.log("Adult")
}

if score is not 0..=100 {
	console.log("Invalid score")
}
```

```fa
let label = when score is {
	0..60 => "fail"
	60..80 => "pass"
	80..=100 => "excellent"
	else => "out of range"
}
```

Like every `is` check, matching a range narrows the type of the value: inside the `0..60` branch, `score` is an `Integer(min = 0, max = 59)`. See [numbers](/values/numbers#minimum-and-maximum-values) and [conditionals](/control-flow/conditionals).

## Ranges as values

A range is an ordinary value: it can be stored, passed to a function and returned.

```fa
function middleThird = (length: Integer): Range => (length // 3)..(2 * length // 3)

let data = [1, 2, 3, 4, 5, 6, 7, 8, 9]
let middle = data[middleThird(data.length)] catch error => [] -- [4, 5, 6]
```
