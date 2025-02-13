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
myRange = 1.. // 1 to infinite
myRange = ..3 // -infinite to 3
myRange = ..-1..3 // +infinite to 3 (reverse order)
myRange = .. // everything
```

You can also create ranges with a step:

```ts
myRange = 1..+3..10
myRange = 1..-4..-16
```
