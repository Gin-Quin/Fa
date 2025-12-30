# Ranges

Ranges are a set of numbers. They can be mutable or immutable.

```ts
myRange = 1..3
myRange: Range = 1..3
myRange = Range { start = 1, end = 3, step = 1 }
myRange = Range(1, 3)

myMutableRange = mutable 1..3

myMutableRange.start = 2
myMutableRange.end = 4
```

You can create ranges without lower or upper bounds:

```ts
myRange = 1.. // 1 to infinity
myRange = ..3 // -infinity to 3
myRange = ..-1..3 // +infinity to 3 (reverse order)
myRange = .. // everything
```

You can also create ranges with a step:

```ts
myRange = 1..+3..10
myRange = 1..-4..-16
```


## Iterating over a range

```ts
for 1..10 >> i {
  console.log(i)
}
```

```ts
arrayOfNumbers = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]

// in a `for` loop, the range will internally call `arrayOfNumbers.iterateOverRange(6, 9)`
for arrayOfNumbers[6..9] >> i {
  console.log(i)
}
```

## Slicing over a range

```ts
subarrayOfNumbers = arrayOfNumbers[6..9] // will copy the elements into a new array

use sliceOfNumbers = arrayOfNumbers[6..9] // will alias the array on the given range
```
