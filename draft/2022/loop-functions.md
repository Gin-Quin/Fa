There are two ways of making a loop:

- via a **control statement** (for, while, ...)
- or via a loop function (items.forEach(), ...)

In Fa, they would look like this:

```coffee
const items = [1, 2, 3]

# 1. control statement
for item in items
  print item

# 2. loop function
items.each: item ->
  print item
```

Solution #1 pros:
- developpers are used to this syntax
- dense

Solution #2 pros:
- you get autocompletion
- creating his own iterator just means creating a new function
- you can use the `?.` syntax:

```coffee
items?.each: item -> print item
```

## Conclusion

- I should implement both.
- `for ... in <array>` should allow `<array>` to be null.
