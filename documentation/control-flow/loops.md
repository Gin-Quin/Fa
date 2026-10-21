# Loops

Fa has three loop statements: `for`, `while` and `loop`.

## `for`

```
for [<pattern> in] <iterable> { <code> }
```

The iterable must be a range, a collection, a string, or a generator.

The pattern is a name, a name and an index (`item, index`), or a [destructuring](/operations/destructuring) pattern (`{ name, age }`, `(key, value)`). It is omitted when the elements are not used.

```fa
for 0..10 {
	console.log("Hello, world!")
}

for count in 0..10 {
	console.log("Count: {count}")
}

let fruits = ["apple", "banana", "cherry"]

for fruit in fruits {
	console.log("Fruit: {fruit}")
}
```

[Ranges](/values/ranges) follow the usual rule: `0..10` runs ten times, with `count` going from 0 to 9. Use `0..=10` for eleven.

### Patterns

A second name receives the index:

```fa
for fruit, index in fruits {
	console.log("{index}: {fruit}")
}
```

Iterating a collection of tuples, a [map](/collections/maps) for instance, destructures them:

```fa
for (id, name) in usersById {
	console.log("{id} -> {name}")
}
```

Iterating objects can destructure their fields, which is what makes [columnars](/collections/columnars) fast:

```fa
for { name, age } in people {
	console.log("{name} is {age}")
}
```

### Loop variables are references

The values extracted by a `for` loop are **references** to the elements of the collection: elements are never copied.

- If the collection is immutable, the references are immutable.
- If the collection is `mutable`, the references are mutable: writing through one writes into the collection.

```fa
mutable players: Array(Player) = [
	{ name = "John", score = 10 }
	{ name = "Jane", score = 12 }
]

for player in players {
	player.score += 1 -- modifies the element in place
}

mutable scores = [1, 2, 3]

for score in scores {
	score += 1
}

console.log(scores) -- [2, 3, 4]
```

However, the structure of a collection cannot be modified while iterating over it: adding or removing elements inside a loop over the same collection is a compile error, since it would invalidate the references of the loop.

```fa
for player, index in players {
	if player.score == 0 {
		players.delete(index) -- error: cannot remove elements from `players` while iterating over it
	}
}

for score in scores {
	scores.push(0) -- error: cannot add elements to `scores` while iterating over it
}
```

To remove elements, use a dedicated method instead of a loop:

```fa
players.deleteWhere((player) => player.score == 0)
```

### Compile-time `for`

`@for` unrolls a loop at compile time. Its iterable must be a compile-time value, declared with `static` (see [metaprogramming](/advanced/metaprogramming)).

```fa
static fruits = ["apple", "banana", "cherry"]

@for fruit in fruits {
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

`parallel` distributes the iterations across CPU cores. Use `parallel(threads = ...)` to choose the number of threads. See [concurrency](/advanced/concurrency).

```fa
let numbers = [1, 2, 3, 4]
mutable results: Array(Integer) = [0, 0, 0, 0]

for parallel number, index in numbers {
	results[index] = expensiveComputation(number)
}

for parallel(threads = 2) number in numbers {
	console.log(expensiveComputation(number))
}
```

A parallel loop may write to its own index of a mutable array: the compiler proves that two iterations never write to the same element.

## `while`

```
while <expression> { <code> }
```

The body runs as long as the expression is `true`. Like every condition, it must be a `Boolean` (see [logical operators](/operations/logical#no-truthiness)).

```fa
mutable remaining = 10

while remaining > 0 {
	remaining -= 1
}
```

### Looping over an optional value

Use `while let` to run a loop as long as an optional expression is not `none`. The value is available in the loop, narrowed to its non-optional type, and the loop stops at the first `none`:

```fa
while let value = stack.pop() {
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

`loop` repeats forever, like `while true`. It must contain a `break` or a `return`.

```fa
loop {
	let line = readLine() -- String?
	if line is none {
		break
	}
	console.log(line)
}
```

### `loop` as an expression

`break` can carry a value, which becomes the value of the loop:

```fa
mutable candidate = 1

let root = loop {
	if candidate * candidate >= target {
		break candidate
	}
	candidate += 1
}
```

:::tip
Only `loop` can be used as an expression. `for` and `while` cannot, because they may run zero times and would have no value.
:::

## `break` and `continue`

`break` leaves the innermost loop; `continue` skips to its next iteration.

```fa
for number in numbers {
	if number < 0 {
		continue
	}
	if number > 100 {
		break
	}
	console.log(number)
}
```

Fa has no labeled loops: `break` and `continue` always act on the innermost loop. To leave several nested loops at once, put them in a function and use `return`:

```fa
function findEmptyCell = (rows: Array(Array(Cell?))): Tuple(Integer, Integer)? => {
	for row, rowIndex in rows {
		for cell, columnIndex in row {
			if cell is none {
				return (rowIndex, columnIndex)
			}
		}
	}
	return none
}
```

## Iterating with methods

Collection methods are often clearer than an explicit loop:

```fa
let evens = numbers.filter((n) => n %% 2 == 0)
let doubled = numbers.map((n) => n * 2)
let total = numbers.reduce((sum, n) => sum + n, 0)
```

Chained calls are **fused** by the compiler into a single pass, so the chain below walks the input once and allocates no intermediate array:

```fa
let total = numbers
	.filter((n) => n %% 2 == 0)
	.map((n) => n * 2)
	.reduce((sum, n) => sum + n, 0)
```

Fusion is an optimisation, not a change in meaning: the result is identical to running each step separately.
