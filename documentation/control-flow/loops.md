# Loops

Fa has three loops: `for`, `while` and `loop`. `for` and `while` are statements; `loop` can also be an [expression](#loop-as-an-expression).

## `for`

```
for [<pattern> in] <iterable> { <code> }
```

The iterable must be a range with a start, a collection, a string, or an [iterator](#element-by-element-with-values). It is evaluated once, before the first iteration.

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

[Ranges](/values/ranges) follow the usual rule: `0..10` runs ten times, with `count` going from 0 to 9. Use `0..=10` for eleven. The loop variable of a range is an immutable copy, known to be in the range: `count` is an `Integer<min = 0, max = 9>`. A range without an end, like `0..`, runs until a `break` or a `return` (the program [stops](/values/errors#stops) if the variable would go past the maximum `Integer`); a range without a start, like `..10`, can't be iterated. An inclusive end at the maximum, like `0..=Int8.maxValue` over an `Int8`, ends normally.

### Patterns

A second name receives the index:

```fa
for fruit, index in fruits {
	console.log("{index}: {fruit}")
}
```

Only the collections with positions have an index: `Array`, `GrowingArray`, `OrderedColumnar` and `String`. The index is an immutable `Integer`, and the compiler knows that it is in bounds: `fruits[index]` can't fail inside the loop (see [numbers](/values/numbers)).

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

A `for` pattern can destructure a tuple, but cannot destructure an array (`E-LOOP-2`).

### Loop variables are references

The values extracted by a `for` loop are **references** to the elements of the collection: elements are never copied.

- If the collection is immutable, the references are immutable.
- If the collection is a mutable place (a `mutable` value, a parameter declared with `mutates`, a `mutable use` alias), the references are mutable: writing through one writes into the collection.

| Iterating | The loop variables are |
| --- | --- |
| an `Array`, a `GrowingArray`, a `Pool`, a `Columnar` | references to the elements, mutable if the collection is |
| a `Map`, a `GrowingMap`, with `(key, value)` | a read-only reference to the key, and a reference to the value, mutable if the map is |
| a `Set`, a `GrowingSet` | read-only references: changing an element would change its place in the set |
| a destructured element, like `{ name, age }` | references to the fields, mutable if the collection is |
| a range, a `String` (even a `mutable` one) | immutable copies (an `Integer`; a `String` holding one [code point](/values/strings#iterating-over-a-string)) |
| an iterator | the values the iterator gives |

Writing through a loop variable is a mutation of the collection: in a function, iterating a parameter and writing to its elements needs the effect `mutates players` (or a precise path like `mutates players[..].score`, see [mutating parameters](/functions/defining-functions#mutating-parameters)).

```fa
mutable players: Array<Player> = [
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

However, the structure of a collection cannot be modified while iterating over it: mutating the whole collection or its length or size, adding or removing elements, inside a loop over the same collection is a compile error, since it would invalidate the references of the loop or change what it visits.

The rule applies to every effect of the body: the direct operations, the operations through another alias of the collection, and the effects of the functions it calls, including the effects implied by [relations](/advanced/relations) (deleting an element also writes the relation fields that point at it). Writing to an element, through the loop variable or by its index, is allowed: the element stays in place. See [iteration](/advanced/reference-checking#iteration) for the exact rule.

```fa
for player, index in players {
	if player.score == 0 {
		players.delete(index) -- error: cannot remove elements from `players` while iterating over it
	}
}

for score in scores {
	scores.push(0) -- error: cannot add elements to `scores` while iterating over it
}

function register = (name: String) mutates people.size => {
	people.add({ name = name })
}

for person in people {
	register("{person.name}'s friend") -- error: `register` mutates `people.size` while the loop iterates over `people`
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

`parallel` distributes the iterations across CPU cores. Use `parallel(threads = ...)` to choose the number of threads. What the iterations may share and write is given in [parallel iterations](/advanced/concurrency#1-parallel-iterations).

```fa
let numbers = [1, 2, 3, 4]
mutable results: Array<Integer> = [0, 0, 0, 0]

for parallel number, index in numbers {
	results[index] = expensiveComputation(number)
}

for parallel(threads = 2) number in numbers {
	console.log(expensiveComputation(number))
}
```

A parallel loop may write to its own index of a mutable array: the compiler proves that two iterations never write to the same element. Here, `index` is in bounds of `results` because both arrays have four elements; when the compiler can't prove it, the write is written `try results[index] = ...` (see [errors](/values/errors)).

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

Use `while let` to run a loop as long as an optional expression is not `none`. The expression is evaluated before each iteration. Its value is copied into a new name (a name only, no destructuring), available in the loop and narrowed to its non-optional type, and the loop stops at the first `none`. The expression must be optional:

```fa
while let value = stack.pop() {
	console.log("Popped: {value}")
	console.log("Stack length: {stack.length}")
}
```

:::tip
This is Fa's equivalent of Rust's `while let`.
:::

Since `T??` is `T?` (see [optionals](/values/optionals)), the loop can't tell an absent value from an element that is `none`: on an `Array<Integer?>`, `while let value = stack.pop()` stops at the first `none` element. Test `stack.length` instead when the elements are optional.

The condition of a `while` also [narrows](/control-flow/conditionals#narrowing) the values it tests inside the body.

## `loop`

```
loop { <code> }
```

`loop` repeats forever, like `while true`, until a `break`, a `return` or a `throw` leaves it. A `loop` without a `break` never ends normally, like the main loop of a server: its type is `Never`.

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

`break` can carry a value, which becomes the value of the loop. The type of a `loop` expression is the union of the types of its `break` values (widened like the branches of an [`if`](/control-flow/conditionals#if-as-an-expression)); a `break` without a value inside a `loop` used as an expression is an error.

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
Only `loop` can be used as an expression. `for` and `while` cannot, because they may run zero times and would have no value: `break value` inside them is an error. In a `loop` used as a statement, `break value` is a warning: the value is dropped.
:::

`break`, `continue`, `return` and `throw` have the type `Never`: they never produce a value, so they can be used wherever a value is expected (`let size = if text is none { break } else { text.length }`).

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

`break` and `continue` act on the innermost loop of the current function, including from a [`do` block](/control-flow/do). They don't cross a function boundary: inside a lambda or a function declared in the loop, they are an error.

Fa has no labeled loops: `break outer` is an error (`E-LOOP-14`), and `break` and `continue` always act on the innermost loop. To leave several nested loops at once, put them in a function and use `return`:

```fa
function findEmptyCell = (rows: Array<Array<Cell?>>): Tuple<Integer, Integer>? => {
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

Each method runs over the whole collection before the next one starts, as with JavaScript arrays. When the callbacks of a chain are [**pure**](/functions/defining-functions#pure-functions) (they modify nothing outside themselves, throw nothing, and do no input or output), the compiler **fuses** the chain into a single pass, so the chain below walks the input once and allocates no intermediate array:

```fa
let total = numbers
	.filter((n) => n %% 2 == 0)
	.map((n) => n * 2)
	.reduce((sum, n) => sum + n, 0)
```

Fusion is an optimisation, not a change in meaning: it only happens when it can't be seen. A callback with an effect keeps the chain step by step:

```fa
let doubled = numbers
	.filter((n) => {
		console.log("filter {n}")
		n %% 2 == 0
	})
	.map((n) => n * 2)
-- not fused: every "filter" line is printed before `map` runs, as in JavaScript
```

### Element by element with `values()`

To run a chain element by element, ask for it explicitly with `values()`, as with the iterator helpers of JavaScript. `values()` returns an **iterator** over the collection: each element goes through the whole chain before the next one is read, and nothing is computed until the chain is consumed, by `toArray()`, `reduce`, `find` or a `for` loop.

```fa
let firstLarge = numbers
	.values()
	.map((n) => n * 1000)
	.find((n) => n > 5000) -- stops at the first match: the rest of `numbers` is never read
```

`firstValues(count = 1)` keeps the first `count` elements of an iterator, and stops reading the collection once it has them. As its plural name says, it returns an iterator, never a single element (use `find` for that):

```fa
let firstEvens = numbers
	.values()
	.filter((n) => n %% 2 == 0)
	.firstValues(3)
	.toArray() -- at most 3 items: the collection is read up to the third even number
```

The order of the effects follows the elements:

```fa
let doubled = numbers
	.values()
	.filter((n) => {
		console.log("filter {n}")
		n %% 2 == 0
	})
	.map((n) => {
		console.log("map {n}")
		n * 2
	})
	.toArray()
-- prints "filter 1", "filter 2", "map 2", "filter 3"...
```

Every collection has `values()`. An iterator is a reference into its collection: modifying the collection invalidates it, like a reference declared with `use`. While an iterator is used, the collection follows the same rules as during a `for` loop: its structure can't be modified, and its elements can be written in place.
