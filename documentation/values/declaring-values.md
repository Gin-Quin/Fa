# Declaring values

## Basic declaration

In Fa, you declare values using a declaration keyword:

- `let` to declare an immutable value
- `mutable` to declare a mutable value
- `use` to give another name to an existing value, without copying it
- `mutable use` to declare a reference that can later designate another value, with `reuse`
- `static` to declare a value computed at compile time
- `function` to declare a function (see [defining functions](/functions/defining-functions))
- `reactive` to declare a mutable value that can be observed for changes
- `derived` to declare a value that is computed from other reactive values

All these statements have the same syntax:

`<declaration> <identifier>[: <type>] = <value>`

The value is mandatory: `let total: Integer` alone is an error. The type annotation is optional if it can be inferred from the value. When it is present, it is a constraint: the value must satisfy the type, and the declared value has the annotated type (see [inferred types](#inferred-types)).

Basic examples:

```fa
-- declaring an immutable integer
-- with a type annotation
let myNumber: Integer = 12

-- the type can be inferred, so annotations are optional
let myInferredNumber = 42

-- declaring an immutable string
let myString: String = "hello"

-- declaring an immutable array
let myArray = [1, 2, 3]
let myTypedArray: Array<Integer> = [1, 2, 3]

-- declaring an immutable object
let myObject = {
	foo = 12
	bar = "hello"
	nested = {
		baz = 42
	}
}

-- you can add type information to fields within an object
let myTypedObject = {
	foo: Integer = 12
	bar: String = "hello"
	nested: { baz: Integer } = {
		baz = 42
	}
}

-- you can declare an optional value with `?` or `Optional`
-- an optional can take the value `none`
let someNumber: Integer? = 12
let noNumber: Integer? = none
let otherNumber: Optional<Integer> = 24
```

## Inferred types

The compiler knows two things about a value: its **type**, and **facts** about its current content.

The type of a value declared without annotation is the **widened** type of its initializer: a literal gives its base type, never a literal type.

```fa
let answer = 42          -- an Integer
let status = "ready"     -- a String
let numbers = [1, 2]     -- an Array<Integer>
let ratio = if fast { 1.5 } else { 2.0 } -- a Float
```

A value that gives no type, like `none` or an empty `[]`, needs an annotation: `mutable best = none` is an error, write `mutable best: Player? = none`.

This is the type that `Type<answer>` returns and that generic inference uses. An annotated value has the annotated type: `let small: Int8 = 42` is an `Int8`. A [`static`](#compile-time-values) value is the exception: it keeps its literal type.

On top of the type, the compiler keeps **flow facts** about each place: its exact literal value, its integer range, the union member it was narrowed to, the length of a collection. Facts make more programs valid, without changing the type:

```fa
let five = 5 -- an Integer, known to be 5

let score: Integer<min = 0, max = 10> = five -- valid: `five` is known to be 5
let items = [10, 20, 30]
let last = items[2] -- no error possible: `items` is known to have 3 items
```

Facts are used to check assignments, exhaustiveness, indices, divisors and [reference invalidation](/advanced/owned-references). They last while the place isn't mutated: an assignment replaces them with the facts of the new value, and any other mutation drops them. See [range analysis](/values/numbers#range-analysis) and [narrowing](/control-flow/conditionals#narrowing).

When an `if` or a `when` has no expected type, its type is the union of the widened types of its branches. A literal branch first adapts to the type of the other branches when it fits: `if valid { level } else { 0 }` with `level: Int8` is an `Int8`. A branch that never finishes, like `return` or `throw`, adds nothing to the type. The union must be a valid type: mixing integer and float literals in the branches without an expected type is an error (`if fast { 1 } else { 2.5 }` needs an annotation, or `1.0`), and so is mixing two integer types whose ranges overlap.

## Assignment

There is no way to introduce a name without a keyword. A bare `name = value` assigns a new value to an existing **mutable** value; it never declares one.

```fa
mutable count = 0
count = 1 -- assignment

total = 12 -- error: `total` is not declared
```

An assignment is a statement, not a value: `a = b = c` is an error. The target is evaluated before the value, from left to right, as in JavaScript: in `items[next()] = compute()`, `next` is called before `compute`.

## Immutable values

By default, all values declared with `let` are constants with "deep immutability":

- you cannot re-assign the value
- you cannot change any fields in the object
- you cannot add or remove items in a collection

```fa
-- by default, "let" declares a constant value
let myValue = 12

myValue += 1 -- error: cannot reassign an immutable value

let myObject = { foo = 12 }

myObject.foo += 1 -- error: cannot change the field of an immutable object
```

## Mutable values

To create a mutable value, declare it with the `mutable` keyword:

```fa
mutable myValue = 12

myValue += 1 -- this works

mutable myObject = { foo = 12 }

myObject.foo += 1 -- this works too

myObject = { foo = 14 } -- and this works as well
```

## References with `use`

`let` and `mutable` copy the value they are given. `use` does not copy: it gives another name to an existing value. Reading or writing through the alias is reading or writing the original value.

```fa
mutable people: Array<Person> = [Person { name = "Ana" }, Person { name = "Bao" }]

let copied = people[1]     -- a copy: changing it leaves the array untouched
use referenced = people[1] -- an alias of the element inside the array

referenced.name = "Cleo"
console.log(people[1].name) -- "Cleo"
```

`use` accepts:

- a **place**: a value, a field, an element (`people[1]`), a [slice of a string](/values/strings#slices), which gives an alias of that place,
- a **call that returns a reference**, which binds that reference: `use ana = people.add(...)`, or `use score = scores.get("ana")`, an optional reference.

Anything else after `use` is an error: `use count = 5`, `use person = makePerson()` and `use chosen = if ready { a } else { b }` have no single place to designate. Use `let` to keep such a value, or `mutable use` and `reuse` to choose between places.

When the place holds a reference, like a relation field, `use` binds the target of that reference: `use next = node.next` designates the node that `node.next` points at, and a later write to `node.next` doesn't change what `next` designates. See [`use` and `mutable use`](/advanced/reference-checking#use-and-mutable-use).

An alias can be written through only if the original value is mutable, and it cannot outlive the value it refers to. If the memory it points into is mutated (for example by `people.push(...)`, which may reallocate the array), or if its place is removed (`people.pop()` removes the last element, `people[1]`), the alias is invalidated: using it afterwards is a compile error until it is declared again. See [owned references](/advanced/owned-references#after-invalidation).

### Changing what a reference designates

A reference declared with `use` designates the same place for its whole life: its address never changes. Assigning it writes into the value it designates.

To declare a reference that can designate another place later, declare it with `mutable use`, then move it with `reuse`:

```fa
mutable players: Array<Player> = [Player { name = "Ana", score = 3 }, Player { name = "Bao", score = 7 }]

mutable use best = players[0]

for player in players {
	if player.score > best.score {
		reuse best = player -- `best` now designates this player: nothing is copied or written
	}
}

best.score += 10 -- writes into the best player of `players`
```

- `reuse` changes the place the reference designates, while `=` writes into the designated value, as with any `use`.
- `reuse` is not a declaration: it moves the reference where it was declared, even from an inner block.
- The type of the reference doesn't change: `best` is a `Player in players`, so it can only designate another element of `players`.
- Only a reference declared with `mutable use` can be moved: `reuse` on a `use` is a compile error.
- `reuse` takes the same forms as `use` (a place, or a call that returns a reference), and must give the declared type, with the same owner. When the type is optional, `reuse best = none` is allowed.
- A `mutable use` can be declared on an immutable collection: it designates read-only places, and can still be moved with `reuse`.
- `use other = best` designates the place `best` designates at that moment: a later `reuse best = ...` doesn't move `other`.

`use` also works with destructuring and imports:

```fa
use { name, age } = person
use { Button as UIButton, Card } = ui.components
```

## Compile-time values

`static` declares a value that is computed by the compiler. It is used plainly, like any other value: every use is replaced by its value in the compiled output.

```fa
static maximumUsers = 1000
static version = @readFile("version.txt")

console.log(version) -- compiled to `console.log("1.4.2")`
```

A `static` value must be initialized with an expression the compiler can evaluate. Compile-time evaluation is deterministic: it cannot read the clock, the network or random numbers. See [metaprogramming](/advanced/metaprogramming).

## Functions

Functions are declared with the `function` keyword:

```fa
function add = (a: Integer, b: Integer): Integer => a + b
```

A lambda such as `(x) => x + 1` is accepted anywhere a value is expected (an argument, a field, a returned value), except as the value of a declaration. Declaring a function with `let` is an error:

```fa
let add = (a: Integer, b: Integer) => a + b -- error: use `function` to declare a function
```

At the top level of a file and inside `type` declarations, functions are hoisted: they can be used before the line that declares them. Inside a block, a function is visible after its declaration. Module-level values are initialized in the order of their dependencies: see [hoisting](/functions/defining-functions#hoisting).

```fa
-- `multiply` can be used even though it is declared below
let product = multiply(6, 7)

function multiply = (a: Integer, b: Integer): Integer => a * b
```

Learn more about [functions](/functions/defining-functions).

## Reactive values

A **reactive value** is a mutable value that can trigger side effects when it changes. Declare it with `reactive`.

`derived` declares a read-only value computed from other reactive values. It is always up to date when read: it is recomputed on read when one of its dependencies changed.

```fa
reactive x = 12

-- derived from a reactive value
derived doubled = x * 2

-- derived from another derived value
derived quadrupled = doubled * 2

x = 4
assert doubled == 8
assert quadrupled == 16
```

Learn more about [reactivity](/advanced/reactivity).

## Function Parameters

Function parameters are **immutable references**: the function reads the value given by the caller without copying it, and cannot change it.

```fa
function increaseFoo = (input: { foo: Integer, bar: Integer }) => {
	-- compiler error: cannot mutate an immutable parameter
	input.foo += input.bar
}
```

To change a parameter, a function declares the mutation as an **effect** in its signature, with `mutates`:

```fa
function increaseFoo = (input: { foo: Integer, bar: Integer }) mutates input => {
	input.foo += input.bar -- this works
}
```

The effect can be as precise as a single field (`mutates input.foo`) or a single index (`mutates items[index]`). See [defining functions](/functions/defining-functions).

Calling a mutating function requires a mutable value:

```fa
let myValue = { foo = 12, bar = 2 }

-- compiler error: `increaseFoo` mutates `input`, but `myValue` is immutable
increaseFoo(myValue)

mutable myMutableValue = { foo = 12, bar = 2 }

-- this works
increaseFoo(myMutableValue)
```

A function can only change the inside of a parameter: it can update the fields of an object or the elements of a collection, but it cannot replace the parameter itself. This way, a parameter compiles to a plain reference on every target, including JavaScript, where a value can't be replaced through a reference.

```fa
function increaseFoo = (input: { foo: Integer, bar: Integer }) mutates input => {
	-- compiler error: cannot reassign a function parameter
	input = {
		foo = input.foo + input.bar
		bar = input.bar
	}

	-- this works
	input.foo += input.bar
}
```

For the same reason, a parameter of a primitive type (numbers, booleans, strings, ...) cannot be mutated:

```fa
function increment = (value: Integer) mutates value => { -- error: a primitive parameter cannot be mutated
	value += 1
}

-- return the new value instead
function incremented = (value: Integer) => value + 1
```

## Module-level values

A value declared at the top of a module (a file of `library/` or `application/`, see [modules](/architecture/modules)), or as a member of a namespace, lives as long as the program. Its place never moves: its address is stable, whether it is modified field by field or replaced as a whole. Unlike a parameter, it can be replaced, even if it is a primitive.

```fa
export mutable: Pool<Person> = []
```

```fa
-- if the value above is in "people.fa", in a function of another file
function restart = () mutates people => {
	use ana = people.add({ name = "Ana" })
	people = [] -- this works too: every person is destroyed, as with `people.clear()`
}
```

The files of `library/` and `application/` contain only declarations, so module-level values are changed from functions. They are initialized once, in the order of their dependencies: see [module-level values and initialization](/architecture/modules#module-level-values-and-initialization).

A function that modifies a module-level value declares it in its effects, with the full path, like a parameter (see [mutating parameters](/functions/defining-functions#mutating-parameters)). The path says exactly what the function may change:

```fa
function register = (name: String) mutates people.size => {
	people.add({ name = name }) -- can only add people
}

function reset = () mutates people => {
	people = [] -- can change anything
}
```

## Shadowing

Declaring a name that already exists in an **enclosing** scope shadows it for the rest of the current scope:

```fa
let value = 1

do {
	let value = 2      -- shadows the outer `value`
	console.log(value) -- 2
}

console.log(value) -- 1
```

The value of a declaration doesn't see the name being declared: in `let value = value + 1`, the `value` on the right is the outer one.

Declaring the same name twice in the **same** scope is a compile error:

```fa
let value = 1
let value = 2 -- error: `value` is already declared in this scope
```

The only exception is an invalidated reference, which is declared again with `use` (see [owned references](/advanced/owned-references#after-invalidation)). A redeclaration inside an inner block is a new name that ends with that block: after the block, the outer reference is still invalidated. To repair a reference from inside a branch or a loop, declare it with `mutable use` and move it with [`reuse`](#changing-what-a-reference-designates). A `mutable use` is never declared again. See [declaring again](/advanced/reference-checking#declaring-again).

## Unused values

A local value, a parameter or the error of a `catch` that is never used is a warning. Start its name with `_` to say that it is unused on purpose: `catch _error => 0`.

## End of scope

Every value is destroyed at the end of the scope that declares it: there is no garbage collector. A `use` alias does not own anything, so it simply stops existing at the end of its scope; the value it refers to is destroyed with its own scope. See [memory management](/advanced/memory-management).

```fa
do {
	let numbers = [1, 2, 3]
	console.log(numbers.length)
} -- `numbers` is destroyed here
```

:::implementation
## Implementation details

### Native

- `let` and `mutable` copy: a flat value is a `memcpy`, an owning value calls the generated `copy<T>`. The copy becomes a move when the source is not used afterwards.
- A `use` alias is a pointer to its place. A slice is a pointer and a length.
- A `mutable use` is a pointer variable; `reuse` stores a new pointer.

### JavaScript

- `let` and `mutable` copy an object or a collection (a generated copy function, not `structuredClone`). The copy is a plain assignment only when the source is not used afterwards (a move), or when both sides are immutable for their whole lives and have the same shape: nothing smarter, so the output is predictable. Primitives and strings need no copy.
- A `use` alias of an object or a collection is the JavaScript reference to it.
- A `use` alias of a primitive place has no JavaScript equivalent: the compiler evaluates the container and the key or index once, into locals, and rewrites each use of the alias into the place (`use score = player.score` then `score += 1` compiles to `player.score += 1`).
- A `mutable use` of a primitive place is a pair of locals (container, key or index), which `reuse` reassigns.
- Facts and inferred types don't exist at runtime: they only remove checks.
:::
