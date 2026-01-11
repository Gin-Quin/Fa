# Ranges

Ranges are a set of numbers. They can be mutable or immutable.

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
let myRange = ..3 -- -infinity to 3
let myRange = .. -- everything
```

## Iterating over a range

```fa
for 1..10 >> i {
  console.log(i)
}
```

## Slicing over a range

```fa
let subarrayOfNumbers = arrayOfNumbers[6..9] -- will copy the elements into a new array
```
