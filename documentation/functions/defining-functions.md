# Functions

## Defining functions

In Fa, there is one way to declare a named function: the `function` keyword.

```fa
function add = (a: Integer, b: Integer): Integer => a + b
```

The syntax is close to TypeScript's arrow functions:

```ts
// in TypeScript
const add = (a: number, b: number): number => a + b
```

The complete form of a function declaration is:

```
[async] function name = (parameters): ReturnType throws Errors requires name: Type mutates path invalidates path => body
```

- `async` marks a function that can be awaited. See [concurrency](/advanced/concurrency).
- `: ReturnType` is the type of the returned value.
- `throws Errors` lists the errors the function can fail with. See [errors](/values/errors).
- `requires name: Type` lists the values the function needs from its caller's environment. See [requirements](/advanced/requirements).
- `mutates path` lists what the function modifies. See [mutating parameters](#mutating-parameters).
- `invalidates path` lists what the function can invalidate references into. See [invalidating references](#invalidating-references).

Every part except the name, the parameters and the body is optional, following the rules below. When several parts are present, they are always written in this order.

The return type, `throws`, `requires`, `mutates` and `invalidates` are the **effects** of the function: together, they tell everything a call can do, without reading the body.

### Return type and effects

If the body is a **single expression**, the return type and the effects are inferred:

```fa
-- valid: the return type is inferred
function addOne = (x: Integer) => x + 1
```

If the body is a **block**, they must be declared explicitly. A block without a return type returns nothing.

```fa
-- valid: explicit return type
function addOne = (x: Integer): Integer => {
	let result = x + 1
	result
}

-- invalid: the block returns a value but has no return type
function addOne = (x: Integer) => {
	x + 1
}

-- valid: returns nothing
function greet = (name: String) => {
	console.log("Hello, {name}!")
}
```

This way, the signature of a function with a non-trivial body always documents everything the function can do.

### Returning a value

The value of a function is the value of the last expression of its body. Use `return` to exit the function early:

```fa
function classify = (value: Integer): String => {
	if value < 0 {
		return "negative"
	}

	if value == 0 {
		return "zero"
	}

	"positive" -- the last expression is the returned value
}
```

`return` always exits the function, even from inside a [`do` block](/control-flow/do).

## Parameters

Parameters are declared with a name and a type. They can have a default value:

```fa
function greet = (name: String, greeting: String = "Hello") => {
	console.log("{greeting}, {name}!")
}
```

### Named arguments

Arguments can be passed by position or by name. Positional arguments come first, named arguments after:

```fa
function move = (x: Integer, y: Integer, speed: Integer = 1) => {
	console.log("Moving to ({x}, {y}) at speed {speed}")
}

move(1, 2)
move(1, 2, speed = 3)
move(1, speed = 3, y = 2)
move(x = 1, y = 2, speed = 3)

move(x = 1, 2) -- error: a positional argument can't follow a named argument
```

### Variadic parameters

A parameter prefixed with `...` collects the remaining arguments into an array. A function has at most one variadic parameter, and it must be the last one.

```fa
function sum = (...values: Integer): Integer => {
	mutable total = 0
	for value in values {
		total += value
	}
	total
}

sum(1, 2, 3) -- 6
sum()        -- 0
```

Inside the function, `values` is an `Array(Integer)`.

To pass the items of an existing collection to a variadic parameter, spread it with `...`:

```fa
let numbers = [1, 2, 3]

sum(...numbers)
sum(0, ...numbers, 4)
```

### Mutating parameters

Parameters are immutable references: they are never copied when passed to a function, and by default the function can't modify them. See [declaring values](/values/declaring-values#function-parameters).

```fa
type Player = {
	name: String
	score: Integer
}

function addBonus = (player: Player) => {
	player.score += 10 -- error: `player` is not declared as mutated. Add `mutates player` to the signature.
}
```

A function that modifies a parameter declares it with the `mutates` effect:

```fa
function addBonus = (player: Player) mutates player => {
	player.score += 10
}
```

Calling a mutating function requires a `mutable` value:

```fa
let alice = Player { name = "Alice", score = 0 }
addBonus(alice) -- error: `addBonus` mutates its argument, but `alice` is immutable

mutable bob = Player { name = "Bob", score = 0 }
addBonus(bob) -- bob.score is now 10
```

A mutation is declared with the precision you need:

- the whole parameter: `mutates player`,
- a field: `mutates player.score`,
- an element, at an index given by another parameter: `mutates scores[index]`.

Several mutations are separated by commas.

```fa
function resetScore = (player: Player) mutates player.score => {
	player.score = 0
	player.name = "" -- error: only `player.score` is declared as mutated
}

function resetAt = (scores: Array(Integer), index: Integer) throws IndexError mutates scores[index] => {
	scores[index] = 0
}
```

A precise effect tells the caller exactly which parts of a value can change: the other fields, and the other elements, are guaranteed untouched.

A mutable parameter can only be modified from the inside: its fields or its elements. The parameter itself can't be replaced, so a parameter of a primitive type (numbers, booleans, strings...) can't be mutated. Return the new value instead:

```fa
function increment = (value: Integer) mutates value => { -- error: a primitive parameter can't be mutated
	value += 1
}

function incremented = (value: Integer) => value + 1
```

### Invalidating references

Some operations can move or remove the elements of a collection: adding an element to an `Array` can reallocate it, deleting an element shifts the others, adding to a `Set` can rehash it, deleting an element of a `Bag` removes it. After such an operation, the local references into the collection are no longer valid, and must be declared again before being used.

A function that can do this declares it with the `invalidates` effect:

```fa
type Inventory = {
	items: Array(String)

	function add = (self, item: String) invalidates self.items => {
		self.items.push(item)
	}
}
```

`invalidates` implies `mutates`: there is no need to write both for the same path.

The effect is as precise as possible: a mutation changes an element, which stays valid, while an invalidation means the element may have moved or disappeared. A path can name a whole collection, one element, or a range of elements:

| Effect | Invalidates the local references to |
| --- | --- |
| `invalidates self` | every element of `self` |
| `invalidates self[key]` | the element at `key` (`Map.delete`) |
| `invalidates self[index..]` | the elements from `index` on (`Array.delete`) |
| `invalidates element` | the element `element` (`Bag.delete`) |

The paths are evaluated with the values before the call: `Array.pop` declares `invalidates self[self.length - 1]`, the last element before it is removed.

The compiler uses these effects to track which references are still valid after a call. See [owned references](/advanced/owned-references) and [collections](/collections/arrays).

### Inferred effects

Like the return type, `mutates` and `invalidates` are inferred when the body is a single expression, and must be declared when the body is a block:

```fa
-- inferred: `invalidates names`
function addGuest = (names: Array(String)) => names.push("guest")

-- invalid: the block mutates `player`, but the signature doesn't say so
function reset = (player: Player) => {
	player.score = 0
}
```

An inferred effect works exactly like a declared one: `addGuest` can only be called with a `mutable` array.

## Hoisting

At the top level of a file and inside a type, the order of declarations does not matter: a function can be used before it is declared.

Inside a block, a function is available after its declaration, and it can use the values declared before it in the block.

```fa
function main = () => {
	let greeting = "Hello"

	function greet = (name: String) => console.log("{greeting}, {name}!")

	greet("John")
}
```

## Recursion

A function can call itself:

```fa
function repeatText = (text: String, times: Integer(min = 0)): String =>
	if times == 0 { "" } else { text + repeatText(text, times - 1) }

repeatText("ab", 3) -- "ababab"
```

## Anonymous functions

An anonymous function (also called a lambda) uses the same syntax, without the `function` keyword and the name. Parameters always have parentheses.

Anonymous functions can only be used where a value is expected: as an argument, a return value or a field value.

```fa
let doubled = [1, 2, 3].map((x) => x * 2)

function makeAdd = (y: Integer) => (x: Integer) => x + y
```

When the types of the parameters are known from the context, they can be omitted.

A named function must always be declared with `function`. Assigning an anonymous function to a value is a compile error:

```fa
let addOne = (x: Integer) => x + 1 -- error: use `function addOne = (x: Integer) => x + 1`
```

### Closures

An anonymous function, or a function declared inside a block, can use the values of the scope around it. It **captures them by copy**, like an assignment: the closure gets its own copy, taken when it is created.

```fa
function main = () => {
	mutable greeting = "Hello"

	function greet = (name: String) => console.log("{greeting}, {name}!")

	greeting = "Bye"
	greet("John") -- "Hello, John!": `greet` has its own copy of `greeting`
}
```

Because the closure owns its copies, it can be stored, returned or sent to another thread without any lifetime question:

```fa
function makeGreeter = (greeting: String) => (name: String) => "{greeting}, {name}!"

let greetings = [makeGreeter("Hello"), makeGreeter("Hi")]
```

A captured copy can't be modified from inside the closure:

```fa
function main = () => {
	mutable total = 0

	function addToTotal = (value: Integer) => {
		total += value -- error: `total` is captured by copy and can't be modified
	}
}
```

To share state, pass it as a mutated parameter, or return the new value.

A closure can capture a **reference** created with [`use`](/advanced/memory-management#aliases) instead of a copy, but only when the compiler can prove that the closure does not escape: it is called immediately, or passed to a function that doesn't store it.

```fa
use display = settings.display

-- valid: `map` calls the closure and doesn't keep it
let scaled = sizes.map((size) => size * display.scale)

mutable callbacks: Array(() => Void) = []

-- error: the closure is stored in `callbacks`, so it can't capture the reference `display`
callbacks.push(() => console.log(display.scale))
```

## Function expressions

The right side of a `function` declaration can be any function that is known at compile time, not only an anonymous function.

An alias of another function:

```fa
function myFunctionClone = myFunction
```

A [composition](/functions/function-composition) of functions:

```fa
function addAndMultiply = add then multiply
```

A partial application, using the `with` keyword to bind some parameters:

```fa
function xyz = (x: Integer, y: Integer, z: Integer) => x + y + z

function xz = xyz with (y = 0)

xz(1, 2) -- 3
```

The result of a function factory, when its arguments are known at compile time:

```fa
function makeAdd = (y: Integer) => (x: Integer) => x + y

function add3 = makeAdd(3)
```

If the arguments are only known at runtime, the result is a regular value:

```fa
let add = makeAdd(readNumber())
```

## Generic functions

A function can take type parameters, written in parentheses right after its name. A capitalized parameter is a type parameter:

```fa
function first(Item) = (items: Array(Item)): Item? => items.get(0)

first([1, 2, 3])     -- Item is inferred as Integer
first(Float)([1, 2]) -- Item is given explicitly
```

A type parameter can be constrained with `:`:

```fa
function sum(Value: Integer | Float) = (values: Array(Value)): Value => values.reduce((total, value) => total + value, 0)
```

See [generics](/types/generics) for constraints, defaults and compile-time value parameters.

## Function types

The type of a function is written like its signature, without the body:

```fa
type Comparator = (a: Integer, b: Integer) => Integer
type Callback = (message: String) => Void
type Loader = (id: String) => User throws LoadError requires database: Database
type Reset = (player: Player) => Void mutates player
```

Parameter names are optional in function types: `(Integer, Integer) => Integer`. They are needed when an effect refers to a parameter, like `mutates player`.

`Void` is the return type of a function that returns nothing. It's only written in function types: in a function declaration, a block without a return type already returns nothing.

## Methods and function pointers

Inside a type, the keyword makes the difference between methods and function pointers:

- `function` declares a function that **belongs to the type**. It is a **method** if its first parameter is `self`, and an **associated function** otherwise.
- `name: Type` declares a **field**, stored in each instance. A field can hold a function: it is then a **function pointer**.

```fa
type Counter = {
	value: Integer

	-- field: a function pointer, stored in each instance
	format: (value: Integer) => String

	-- associated function (no `self` parameter)
	function create = (start: Integer): Counter => Counter {
		value = start
		format = (value) => "{value}"
	}

	-- method
	function double = (self): Integer => self.value * 2

	-- mutating method (only callable on mutable instances)
	function increment = (self) mutates self.value => {
		self.value += 1
	}

	-- method calling a function pointer
	function display = (self) => console.log(self.format(self.value))
}
```

```fa
mutable counter = Counter.create(12)

counter.increment()
counter.display() -- "13"

counter.format = (value) => "Value: {value}"
counter.display() -- "Value: 13"
```

Rules:

1. `self` must be the first parameter and has no type annotation.
2. A method that modifies its instance declares it with `mutates self` (or a field of `self`), and can only be called on a `mutable` instance.
3. Methods are not stored in instances and cannot be replaced. Function pointers are stored in each instance and can be set or replaced like any other field.
4. Methods can only be declared in `type` declarations. Anonymous objects only contain data, which can include function pointers.

## Destructuring parameters

A parameter can be destructured, like in TypeScript: write the extracted fields, then the type of the parameter.

```fa
type SomeObject = {
	foo: String
	bar: Integer
	baz: Boolean
}

function someFunction = (prefix: String, { foo, bar }: SomeObject) => {
	console.log(prefix, foo, bar)
}

someFunction("Values:", { foo = "Hello", bar = 42, baz = true })
```

### Anonymous structure parameter

When the type of the object is not declared elsewhere, write its fields and their types directly in the parameter. Each field becomes a local value, and fields can have default values:

```fa
function greet = ({ name: String, greeting: String = "Hello" }) => {
	console.log("{greeting}, {name}!")
}

greet({ name = "John" })
greet({ name = "Jane", greeting = "Hi" })
```

There is one form per situation:

- with a **named** type, destructure it: `({ x, y }: Point)`,
- with an **anonymous** type, use an anonymous structure parameter: `({ x: Integer, y: Integer })`.

Destructuring a parameter with an inline anonymous type is valid, but the compiler emits a warning, and the formatter rewrites it into an anonymous structure parameter:

```fa
-- warning: use an anonymous structure parameter
function greet = ({ name, greeting }: { name: String, greeting: String }) => {
	console.log("{greeting}, {name}!")
}

-- after formatting
function greet = ({ name: String, greeting: String }) => {
	console.log("{greeting}, {name}!")
}
```

:::note
Unlike TypeScript, `{ foo: String }` in a parameter never renames a field: in Fa, `:` is always followed by a type. Fields are renamed with `as`: `({ foo as bar }: SomeObject)`. See [destructuring](/operations/destructuring#renaming-fields).
:::

Destructured parameters can't keep a name for the whole value. If you need it, take the parameter as a whole and [destructure](/operations/destructuring) it in the body:

```fa
function readPoint = (point: Point) => {
	let { x, y } = point
	console.log(x, y, point)
}
```
