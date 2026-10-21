# Loops

Fa has three loop statements: `for`, `while` and `loop`.

## `for`

```
for <iterable> [>> <bindings>] { <code> }
```

The expression must be an iterable: a range, a collection, a string, or a generator.

```fa
for 0..10 {
  console.log("Hello, world!")
}

for 0..10 >> count {
  console.log("Count: {count}")
}

let fruits = ["apple", "banana", "cherry"]

for fruits >> fruit {
  console.log("Fruit: {fruit}")
}
```

Ranges follow the usual rule: `0..10` runs ten times, with `count` going from 0 to 9. Use `0..=10` for eleven.

### Bindings

The extract operator `>>` binds what the loop produces. A second binding receives the index:

```fa
for fruits >> fruit, index {
  console.log("{index}: {fruit}")
}
```

Iterating a collection of tuples — a map, for instance — destructures them:

```fa
for usersById >> (id, name) {
  console.log("{id} -> {name}")
}
```

Iterating objects can destructure their fields, which is what makes [columnars](../collections/columnars) fast:

```fa
for people >> { name, age } {
  console.log("{name} is {age}")
}
```

### Iteration yields references

Iterating a collection produces **references** to its items, not copies. Writing through one writes into the collection:

```fa
mutable scores = [1, 2, 3]

for scores >> score {
  score += 1
}

console.log(scores) -- [2, 3, 4]
```

Mutating the collection's *structure* while iterating it is a compile error, because it would invalidate those references:

```fa
for scores >> score {
  scores.push(0) -- error: `scores` is being iterated
}
```

To remove items while iterating, collect them first, or use `retain`:

```fa
scores.retain((score: Int) => score > 1)
```

### Compile-time `for`

`@for` unrolls a loop at compile time. Its iterable must be a compile-time value.

```fa
static fruits = @["apple", "banana", "cherry"]

@for fruits >> fruit {
  console.log("Fruit: {fruit}")
}
```

produces exactly the same program as:

```fa
console.log("Fruit: apple")
console.log("Fruit: banana")
console.log("Fruit: cherry")
```

:::tip
`for` is the only loop that can run at compile time.
:::

### Parallel `for`

`parallel` distributes the iterations across CPU cores. See [concurrency](../advanced/concurrency#parallel-iteration).

```fa
for parallel numbers >> number {
  results[index] = expensiveComputation(number)
}
```

## `while`

```
while <expression> { <code> }
```

The body runs until the expression becomes falsy.

```fa
mutable remaining = 10

while remaining > 0 {
  remaining -= 1
}
```

### Extracting in a `while`

`>>` binds the value the condition produced, and the loop stops when it is `none`:

```fa
while stack.pop() >> value {
  console.log("Popped: {value}")
  console.log("Stack length: {stack.length}")
}
```

:::tip
This is Fa's equivalent of Rust's `while let`.
:::

## `loop`

```
loop { <code> }
```

`loop` repeats forever. It must contain a `break` or a `return`.

```fa
loop {
  let line = readLine()
  if line is none { break }
  console.log(line)
}
```

### `loop` as an expression

`break` can carry a value, which becomes the value of the loop:

```fa
let found = loop {
  for values >> value {
    if value == target {
      break value
    }
  }
  break none
}
```

:::tip
Only `loop` can be used as an expression. `for` and `while` cannot, because they may run zero times and would have no value.
:::

## `break` and `continue`

`break` leaves the innermost loop; `continue` skips to its next iteration.

```fa
for numbers >> number {
  if number < 0 { continue }
  if number > 100 { break }
  console.log(number)
}
```

### Labels

Label a loop to break out of an outer one. A label is an identifier followed by `:`.

```fa
outer: for rows >> row {
  for row >> cell {
    if cell is none {
      break outer
    }
  }
}
```

`continue outer` works the same way.

## Iterating with methods

Collection methods are often clearer than an explicit loop:

```fa
let evens = numbers.filter((n: Int) => n %% 2 == 0)
let doubled = numbers.map((n: Int) => n * 2)
let total = numbers.reduce((acc: Int, n: Int) => acc + n, 0)
```

Chained calls are **fused** by the compiler into a single pass, so the chain below allocates one array and walks the input once:

```fa
let total = numbers
  .filter((n: Int) => n %% 2 == 0)
  .map((n: Int) => n * 2)
  .reduce((acc: Int, n: Int) => acc + n, 0)
```

Fusion is an optimisation, not a change in meaning: the result is identical to running each step separately.
