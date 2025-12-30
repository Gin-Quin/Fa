# Loops

## For

Syntax: `for <expression> { <code> }`

`<expression>` should be either an iterable, or an iterable with an extracted value.

Examples:

```fa
for 1..10 {
  console.log("Hello, world!")
}

for 1..10 >> count {
  console.log("Count:", count)
}

let fruits = ["apple", "banana", "cherry"]

for fruits >> fruit {
  console.log("Fruit:", fruit)
}
```

### Compile-time for

You can use `@for` to iterate over a range of values at compile time.

The iterable expression should be a compile-time value.

For example, writing this:

```fa
let fruits = @["apple", "banana", "cherry"]

@for fruits >> fruit {
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

### Extract a value in the while loop

You can use `while` with the extract operator `>>>` to iterate over a range of values at runtime.

```fa
while stack.pop() >> value {
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
  for values >> value {
      if value == target {
          break Some(v)
      }
  }
  break None
}
```

:::tip
Only `loop` can be used as an expression, not `for` nor `while`.
:::
