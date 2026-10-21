# Do

`do` creates a block of code. Used as an expression, its value is the value of its last expression.

```fa
let helloWorld = do {
  mutable hello = "Hello" -- scoped to the block
  hello += " World"
  hello += "!"
  hello
}
```

Values declared inside a `do` block are destroyed when the block ends, so they do not clutter the surrounding scope.

```fa
let array = [1, 2, 3, 4, 5]

let sum = do {
  mutable total = 0
  for array >> value {
    total += value
  }
  total
}

-- `total` no longer exists here
```

## Use cases

1. Keep temporary values out of the surrounding scope.
2. Replace an **IIFE** (*immediately invoked function expression*). Unlike a function, a `do` block has no call, no closure, and can `return` from the enclosing function.
3. Combined with a `for` loop, serve as an alternative to `reduce` when the accumulation is too involved to read as a single expression.
4. Scope a [context](../advanced/contexts#scoped-contexts) to part of a function.

## `do` and control flow

A `do` block is part of the function that contains it, so `return`, `break` and `continue` act on that function or loop:

```fa
function findFirstEven = (numbers: Array(Int)): Int? => {
  let result = do {
    for numbers >> number {
      if number %% 2 == 0 {
        return number -- returns from findFirstEven, not from the block
      }
    }
    none
  }

  return result
}
```

## `do` as a statement

Without being assigned, `do` is simply a scope:

```fa
do {
  use connection = pool.acquire()
  connection.send(message)
}
-- `connection` is released here
```
