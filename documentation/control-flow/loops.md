# Loops

## For

Syntax: `for [<pattern> in] <iterable> { <code> }`

The pattern is a name, a name and an index (`item, index`), or a [destructuring](/operations/destructuring) pattern (`{ x, y }`). It can be omitted if the elements are not used.

Examples:

```fa
for 1..10 {
  console.log("Hello, world!")
}

for count in 1..10 {
  console.log("Count:", count)
}

let fruits = ["apple", "banana", "cherry"]

for fruit in fruits {
  console.log("Fruit:", fruit)
}
```

### Loop variables are references

The values extracted by a `for` loop are references to the elements of the collection: elements are never copied.

- If the collection is immutable, the references are immutable.
- If the collection is `mutable`, the references are mutable: you can modify the elements from inside the loop.

```fa
mutable players = [Player("John", 10), Player("Jane", 12)]

for player in players {
	player.score += 1 -- OK: modifies the element in place
}
```

However, the structure of a collection cannot be modified while iterating over it: adding or removing elements inside a loop over the same collection is a compile error, since it would invalidate the references of the loop.

```fa
for player in players {
	if player.score == 0 {
		players.delete(player) -- error: cannot remove elements from `players` while iterating over it
	}
}

-- instead, use a dedicated method
players.deleteWhere((player) => player.score == 0)
```

### Compile-time for

You can use `@for` to iterate over a range of values at compile time.

The iterable expression should be a compile-time value, declared with `let @name = value` (see [metaprogramming](/advanced/metaprogramming)).

For example, writing this:

```fa
let @fruits = ["apple", "banana", "cherry"]

@for fruit in @fruits {
  console.log("Fruit:", fruit)
}
```

Produces exactly the same code as writing this:

```fa
console.log("Fruit:", "apple")
console.log("Fruit:", "banana")
console.log("Fruit:", "cherry")
```

:::tip
`for` is the only loop statement that can be used at compile time.
:::

## While

Syntax: `while <expression> { <code> }`

The code runs until the expression evaluates to `false`.

### Loop over an optional value

Use `while let` to run a loop as long as an optional expression is not `none`. The value is available in the loop, narrowed to its non-optional type.

```fa
while let value = stack.pop() {
	console.log("Popped:", value)
	console.log("Stack length:", stack.length)
}
```

:::tip
This is similar to `while let` in Rust.
:::

## Loop

Syntax: `loop { <code> }`

`loop` is similar to `while true`. It must have a `break` or `return` statement somewhere in the loop body.

### Loop as expression

You can use `loop` as an expression as well:

```fa
let found = loop {
  for value in values {
      if value == target {
          break value
      }
  }
  break none
}
```

:::tip
Only `loop` can be used as an expression, not `for` nor `while`.
:::
