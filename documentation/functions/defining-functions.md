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
[async] function name = <GenericParameters>(parameters): ReturnType throws Errors requires name: Type mutates path invalidates path => body
```

- `<GenericParameters>` are the [generic parameters](#generic-functions) of the function, between angle brackets. They are part of its signature, so a lambda can have them too.

- `async` marks a function that can be awaited. It is never inferred from the body: `await` is only allowed in an `async function`, an async lambda, a [`start do`](/control-flow/do#start-do) block, a test, and at the top level of a standalone file. See [concurrency](/advanced/concurrency).
- `: ReturnType` is the type of the returned value.
- `throws Errors` lists the errors the function can fail with. See [errors](/values/errors).
- `requires name: Type` lists the values the function needs from its caller's environment. See [requirements](/advanced/requirements).
- `mutates path` lists what the function modifies. See [mutating parameters](#mutating-parameters).
- `invalidates path` lists the places the function can remove, or make designate another element. See [invalidating references](#invalidating-references).

Every part except the name, the parameters and the body is optional, following the rules below. When several parts are present, they are always written in this order. [Generic parameters](#generic-functions) start the signature, before the parameters: `function first = <Item>(items: Array<Item>) => ...`.

The return type, `throws`, `requires`, `mutates` and `invalidates` are the **effects** of the function: together, they tell everything a call can do, without reading the body.

### Return type and effects

If the body is a **single expression** (any body that doesn't start with `{`), the return type and the effects are inferred:

```fa
-- valid: the return type is inferred
function addOne = (x: Integer) => x + 1
```

If the body is a **block** (it starts with `{`), they must be declared explicitly. A block without a return type returns nothing: the value of its last expression is dropped.

```fa
-- valid: explicit return type
function addOne = (x: Integer): Integer => {
	let result = x + 1
	result
}

-- warning: the value of `x + 1` is dropped, since the function has no return type
function addOne = (x: Integer) => {
	x + 1
}

-- valid: returns nothing
function greet = (name: String) => {
	console.log("Hello, {name}!")
}
```

This way, the signature of a function with a non-trivial body always documents everything the function can do. Dropping the last value is a warning only when computing it has no effect, like `x + 1`: a block that ends with a call like `people.add(...)` drops its result silently.

A body made of an `if`, a `when` or a `do` is a single expression, even with blocks inside: the return type and the effects of the function are inferred too.

A function that calls itself, directly or through other functions, must declare its return type; its effects are still inferred when its body is a single expression.

A function's declared `throws` and `mutates` must describe the effects of its body. Declaring `throws ParseError` when no `ParseError` can escape, or declaring a mutation that the body never performs, is an error. A declared mutation path can cover a more precise path in the body, as described in [reference checking](/advanced/reference-checking#what-a-body-may-do).

A function whose return type is `Never` never returns: it always throws, stops the program, or loops forever.

The unused pure-value warning also applies to other statements of a block or a file: `x + 1` on its own is a warning.

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

`return` always exits the function, even from inside a [`do` block](/control-flow/do). In a function without a return type, `return` has no value.

## Parameters

Parameters are declared with a name and a type. They can have a default value:

```fa
function greet = (name: String, greeting: String = "Hello") => {
	console.log("{greeting}, {name}!")
}
```

A default value is evaluated at each call that doesn't pass the argument, like in JavaScript, and can use the parameters declared before it:

```fa
function slice = (text: String, start: Integer = 0, end: Integer = text.length) => ...
```

A default value runs during call binding, before the body. For `start`, binding happens when the task is created, with the written arguments in source order; only the body waits for the task's first turn. Its effects, including errors, belong to that call and are declared in the function's signature when the body is a block. A failing started default fails the `start` expression and releases previously evaluated owned arguments.

An omitted argument follows the same rules as a written one: `= shared` borrows an existing value for the call, without copying or finalizing it, while a literal, a call result or `copy shared` builds a temporary owned and cleaned up by the call; a consuming Task-owning parameter's default must build its value, since consuming an existing one on every call is an error (`MEM002`).

```fa
let shared = [1, 2]
function size = (items: Array<Integer> = shared): Integer => items.length
function independent = (items: Array<Integer> = copy shared): Integer => items.length
console.log(size(), independent(), shared.length) -- 2 2 2
```

The parameters and the top-level declarations of the body share one scope: declaring a value with the name of a parameter is an error.

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
move(1, 2, x = 3) -- error: `x` is already given
```

Named arguments and default values belong to the declaration: they work in direct calls, not through a value of a [function type](#function-types). Defaults come from the exact written declaration, including across files and generic instantiations; structural type equality never selects another declaration's defaults. When an anonymous nested annotation has no written default owner, supply the argument explicitly.

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

Inside the function, `values` is an `Array<Integer>`.

To pass the items of an existing collection to a variadic parameter, spread it with `...`:

```fa
let numbers = [1, 2, 3]

sum(...numbers)
sum(0, ...numbers, 4)
```

Any collection whose elements have the type of the variadic parameter can be spread, in the positional arguments that fill the variadic parameter only. A variadic parameter is never passed by name.

### Mutating parameters

Parameters are immutable references: they are never copied when passed to a function, whatever their size and whether the function modifies them or not, and by default the function can't modify them. A copy only happens when the function stores the value somewhere that outlives the call (a field, a collection, a returned value), exactly as an assignment would. See [declaring values](/values/declaring-values#function-parameters).

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

addBonus(Player { name = "Cleo", score = 0 }) -- error: the mutation of a temporary value would be lost
```

A mutation is declared with the precision you need:

- the whole parameter: `mutates player`,
- a field: `mutates player.score`,
- an element, at an index given by another parameter: `mutates scores[index]`,
- the same field of every element: `mutates players[..].score`.

Several mutations are separated by commas: `mutates player.score, scores[index]`. The full grammar of these paths, including `[..]` (every element) and ranges of indices, is given in [places and paths](/advanced/reference-checking#places-and-paths).

```fa
function resetScore = (player: Player) mutates player.score => {
	player.score = 0
	player.name = "" -- error: only `player.score` is declared as mutated
}

function resetAt = (scores: Array<Integer>, index: Integer) throws IndexError mutates scores[index] => {
	try scores[index] = 0 -- the index is not proven in bounds: the assignment can fail
}

function resetAll = (players: Array<Player>) mutates players[..].score => {
	for player in players {
		player.score = 0
	}
}
```

An assignment whose target can fail, like an index that the compiler can't prove in bounds, is prefixed with `try`, or the function handles the error another way. See [errors](/values/errors).

A precise effect tells the caller exactly which parts of a value can change: the other fields, and the other elements, are guaranteed untouched.

A mutation keeps the mutated place valid, but replacing a value destroys what it owned. The compiler deduces from the type of the path which local references the call invalidates: none for a flat value, like `mutates player.score` with an `Integer` score, and every reference into the value otherwise, like the slices of `player.name` for `mutates player.name`. See [mutations](/advanced/owned-references#mutations).

A mutable parameter can only be modified from the inside: its fields or its elements. The parameter itself can't be replaced, so a parameter of a primitive type (numbers, booleans, strings...) can't be mutated. Return the new value instead:

```fa
function increment = (value: Integer) mutates value => { -- error: a primitive parameter can't be mutated
	value += 1
}

function incremented = (value: Integer) => value + 1
```

A function that modifies a [module-level value](/values/declaring-values#module-level-values) declares it the same way, with its full path:

```fa
-- `people` is exported by "people.fa" with `export mutable: Pool<Person> = []`
function register = (name: String) mutates people.size => {
	people.add({ name = name })
	people = [] -- error: only `people.size` is declared as mutated
}
```

Unlike a parameter, a module-level value can be replaced: a function that declares `mutates people` can write `people = []`.

A value passed to a mutated parameter can't overlap anything else the call uses: another argument, a requirement or a module-level value that the function uses, or a reference captured by a callback argument. `swap(team, team)` with two mutated parameters is an error, since both arguments designate the same value; so is any call where the compiler can't prove that they are distinct. `use` aliases can still designate the same value as each other. See [arguments that alias](/advanced/reference-checking#arguments-that-alias).

### Invalidating references

Some operations remove a precise element of a collection, or shift the following ones: deleting an element of a `Pool` or a `Map` removes it, deleting an element of an `Array` shifts the elements after it. The local references to these elements must be declared again before being used.

A function that can do this declares it with the `invalidates` effect:

```fa
type Inventory = {
	items: Pool<String>

	function remove = (self, item: String in self.items) invalidates item => {
		self.items.delete(item)
	}
}
```

`invalidates` implies `mutates`: there is no need to write both for the same path. Invalidating an element of a collection also changes its size: here, `invalidates item` with `item: String in self.items` implies `mutates self.items.size`.

A mutation keeps the place valid, while an invalidation means that the place may have disappeared, or may now designate another element. An invalidation always names precise places:

| Effect | Invalidates the local references to |
| --- | --- |
| `invalidates self[key]` | the element at `key` (`Map.delete`) |
| `invalidates self[index..]` | the elements from `index` on (`Array.delete`) |
| `invalidates element` | the element `element` (`Pool.delete`) |

An operation that affects a whole collection, like `Array.push` (which may reallocate the array) or `Set.add` (which may rehash the set), declares `mutates self`: mutating a collection already invalidates every reference into it.

The paths are evaluated with the values before the call: `Array.pop` declares `invalidates self[self.length - 1]`, the last element before it is removed. They follow the grammar of [places and paths](/advanced/reference-checking#places-and-paths): several invalidations are separated by commas, and `invalidates self[index..]` covers a range of elements.

The compiler uses these effects to track which references are still valid after a call. See [owned references](/advanced/owned-references) and [collections](/collections/arrays).

### Inferred effects

Like the return type, `mutates` and `invalidates` are inferred when the body is a single expression, and must be declared when the body is a block:

```fa
-- inferred: `mutates names`
function addGuest = (names: Array<String>) => names.push("guest")

-- invalid: the block mutates `player`, but the signature doesn't say so
function reset = (player: Player) => {
	player.score = 0
}
```

An inferred effect works exactly like a declared one: `addGuest` can only be called with a `mutable` array. Inferred effects are part of the interface of the function: changing the body can change them.

## Pure functions

A function is **pure** when a call can do nothing but compute its result:

- it doesn't throw (no `throws`),
- it doesn't mutate or invalidate anything but its own local values (no `mutates`, no `invalidates`),
- it has no requirements (no `requires`) and isn't `async`,
- it doesn't read a module-level `mutable` or [reactive](/advanced/reactivity) value,
- it doesn't call, directly or not, a platform primitive that has an effect outside the program: input and output, `console`, the clock, randomness.

Purity is not written: the compiler infers it, from the effects and from the body. It is what allows a pure function to be called at [compile time](/advanced/metaprogramming), a [`derived`](/advanced/reactivity) value to be recomputed when needed, and a chain of [collection methods](/control-flow/loops#iterating-with-methods) to be fused into a single pass.

```fa
function square = (x: Integer) => x * x -- pure
function logSquare = (x: Integer) => console.log(x * x) -- not pure: it prints
```

## Hoisting

At the top level of a file and inside a type, the order of declarations does not matter: a function can be used before it is declared.

Module-level values are initialized in the order of their dependencies, whatever the order of the lines: a cycle between the initializers of module-level values is an error (see [modules](/architecture/modules)).

Inside a block, a function can be called directly before its declaration. The call borrows its captures, which must already be initialized at the call. A function without captures can also be used as a value before its declaration; a function with captures cannot, because its environment is created at the declaration. See [closures](#closures) for the declaration order required when an escaping function captures another function's handle.

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
function repeatText = (text: String, times: Integer<min = 0>): String =>
	if times == 0 { "" } else { text + repeatText(text, times - 1) }

repeatText("ab", 3) -- "ababab"
```

## Anonymous functions

An anonymous function (also called a lambda) uses the same syntax, without the `function` keyword and the name. Parameters always have parentheses.

```fa
let doubled = [1, 2, 3].map((x) => x * 2)

function makeAdd = (y: Integer) => (x: Integer) => x + y
```

A lambda can be written wherever a value is expected (an argument, a return value, a field, an element of a collection), except as the value of a declaration. A named function must always be declared with `function`:

```fa
let addOne = (x: Integer) => x + 1 -- error: use `function addOne = (x: Integer) => x + 1`
```

A declared function can still be given to a value, like any value: `let f = add` is a plain function value, called with positional arguments only. `function f = add` is an [alias](#function-expressions), which keeps the named arguments and the default values.

A lambda takes its types from the function type that is expected where it is written: the types of its parameters, its return type and its effects. What the expected type leaves open, like the result type `Result` of `map`, is inferred from the body. The rule of [declared effects](#return-type-and-effects) only applies to `function` declarations: a lambda with a block body has the return type of the expected function type, and returns nothing when that type has no return type. A lambda with no expected type at all, like a field of an object literal without type, annotates its parameters, and its return type and effects are inferred from its body, whatever its form.

```fa
let lengths = names.map((name) => {
	let trimmed = name.trim()
	trimmed.length -- the value of the lambda: `map` infers `Result` as `Integer`
})
```

A lambda can declare fewer parameters than its expected type, like in JavaScript: `items.map((item) => item.name)` ignores the other arguments that `map` may pass.

The body of a lambda after `=>` is a block when it starts with `{` (see [braces](/getting-started/syntax-basics#braces)). To return an object literal from a lambda, write its type: `(x) => Point { x = x, y = 0 }`.

### Closures

An anonymous function, or a function declared inside a block, can use the values of the scope around it: it **captures** them. How it captures depends on whether it **escapes**, that is, whether it can be called after the current call of the enclosing function, or from another task: a closure escapes when it is stored (in a field, a collection, a module-level value), returned, passed to a parameter that stores it, captured by another escaping closure, or used by a started task.

A closure that **doesn't escape** uses the values themselves, like in JavaScript: it sees their current value, and can modify a captured `mutable` value.

A closure never consumes a capture, including a nonescaping capture. A captured Task cannot be awaited or moved (`MEM002`); pass it explicitly as a Task-owning parameter from its owning scope. A captured structure can be read or explicitly copied, but cannot move out (`MEM004`). Structure and resource parameters remain borrows; `copy` is the fix for storing structure data, while a unique resource must move or be created in its owning scope.

```fa
function main = () => {
	mutable greeting = "Hello"

	function greet = (name: String) => console.log("{greeting}, {name}!")

	greeting = "Bye"
	greet("John") -- "Bye, John!": `greet` doesn't escape, it uses `greeting` itself
}

function sumOf = (numbers: Array<Integer>): Integer => {
	mutable total = 0

	function addToTotal = (value: Integer) => {
		total += value -- valid: `addToTotal` doesn't escape
	}

	for number in numbers {
		addToTotal(number)
	}
	total
}
```

A closure that **escapes** owns immutable captures, taken when it is created. Ordinary values copy; a structure moves at its last use, and otherwise requires an explicit snapshot (`MEM004`). A borrowed parameter, [`use`](/advanced/memory-management#aliases) alias or loop item cannot surrender its owner: first create `let snapshot = copy input`, then capture `snapshot` at its last use. The closure cannot modify its owned captures. Worker transfer also obeys [thread safety](/advanced/concurrency#thread-safety-and-synchronization):

```fa
function makeGreeter = (greeting: String) => (name: String) => "{greeting}, {name}!"

let greetings = [makeGreeter("Hello"), makeGreeter("Hi")]

function main = () => {
	mutable count = 0
	mutable callbacks: Array<() => Void> = []

	use display = settings.display

	-- doesn't escape: `map` calls the closure and doesn't keep it, it reads `display` itself
	let scaled = sizes.map((size) => size * display.scale)

	-- escapes: the closure is stored in `callbacks`, with a copy of the value `display` designates
	callbacks.push(() => console.log(display.scale))

	callbacks.push(() => {
		count += 1 -- error: `count` is captured by copy, since the closure escapes
	})
}
```

An escaping closure captures the field paths it actually uses, including fields of fields and tuple positions of an owned enclosing value. Unused sibling fields are neither captured nor moved. Using a value as a whole, passing it, indexing into a collection inside it, or using it through a reference captures the whole binding. Each captured path follows the ordinary storage rules: an ordinary path copies, a structure path moves at its last use or needs an explicit snapshot, and a unique path cannot be captured.

```fa
function makeCounter = () => {
	let holder = { items = [1], count = 2 }
	function read = () => holder.count -- captures only the ordinary `count` field
	console.log(holder.items.length) -- the sibling collection is still owned here
	read
}
```

A local function that escapes creates its immutable environment once, when execution reaches its declaration. Each capture is one store under the ordinary ownership rules. All handles share that environment; copying a function never copies its captures:

```fa
function main = () => {
	let items = [1]
	function read = () => items.length -- moves `items` into this environment
	let readers = (read, read) -- both handles share the environment
	console.log(readers[0](), readers[1]())
}
```

Before execution reaches its declaration, a local function with captures can only be called directly. That call borrows its captures, which must already be initialized. Taking it as a value there is an error, whether or not the value escapes: move the declaration above the use, or call the function directly. A function without captures has no environment and no such restriction. Reaching the declaration again in another loop iteration or recursive call creates a new environment for that execution.

Calling another local function from an escaping function's body captures that function's handle. If the callee has captures, its declaration must have been reached when the escaping caller is declared: declare the callee first. Two local functions that both capture and call each other cannot both escape; pass their captured data as parameters, or make them methods of one object. Direct calls between nonescaping local functions still borrow at the call; there is no deferred environment initialization.

A value that can't be copied, like a [task](/advanced/concurrency#tasks) or a value of a type with a [`destroy`](/advanced/foreign-code#releasing-c-resources) function, can't be captured by an escaping closure.

Modifying a captured value is not an effect that the closure declares: the value belongs to the enclosing function, which sees the change. To share state with an escaping closure, pass it as a mutated parameter, or return the new value. The compiler can share immutable storage when no write or lifetime difference can be observed; this optimization never makes an otherwise forbidden collection copy legal. A closure can't capture a `mutable use` alias.

Module-level values are not captured: a closure uses them directly, like any function, and declares their mutations in its effects. [Requirements](/advanced/requirements) are not captured either: they are resolved where the closure is called.

## Functions as parameters

A parameter can be a function. The compiler infers, for each parameter of a function type, whether the function **stores** it: in a field, a collection or a module-level value, in a returned or escaping closure, or in a started task. Like inferred effects, this is part of the interface of the function.

- A parameter that is **not stored** is only called during the call. The argument may have any effect: its effects become effects of the call. Passing a callback that throws to `map` makes the call throw.
- A parameter that is **stored** may be called later, from anywhere. Its function type must declare every effect of the argument, as for any [value of a function type](#function-types).

```fa
-- `map` doesn't store its callback: the call throws what the callback throws
let numbers = try texts.map((text) => try Integer.parse(text)) -- throws ParseError

type Button = {
	label: String
	onClick: () => Void
}

-- `handler` is stored, so its type declares every effect it may have: here, none
function setHandler = (button: Button, handler: () => Void) mutates button.onClick => {
	button.onClick = handler
}

setHandler(button, () => try save()) -- error: `onClick` is `() => Void`, which doesn't declare the errors of `save`
```

Inside the function, a call to a parameter that is not stored and whose type has no `throws` needs no `try`: any additional errors supplied by the caller belong to that caller. The function can't catch these additional errors. A `catch` around such calls only is an error, and a `catch` around an expression that also has failing calls of its own only catches the errors of those calls.

If the parameter's type explicitly declares `throws E`, its declared errors follow the ordinary `try` and `catch` rules inside the function. The function can intercept them. Its own signature declares only the errors that can still escape: an error handled without being thrown again is absent from that signature; manually throwing it again makes it part of the signature.

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

A factory call is known at compile time when the factory is [pure](#pure-functions) and its arguments are literals, `static` values or functions known at compile time: the compiler evaluates it during compilation. If the arguments are only known at runtime, the result is a regular value:

```fa
let add = makeAdd(readNumber())
```

A declaration can also give the function type explicitly. The lambda then takes its types from it:

```fa
type Comparator = (a: Integer, b: Integer) => Integer

function ascending: Comparator = (a, b) => a - b
```

### Partial application with `with`

`function with (name = value, ...)` builds a function that has the remaining parameters, in their order and with their defaults:

- parameters are bound by name only, and a bound parameter can't be passed again,
- names come from the visible signature, including a function-valued parameter or field, even when an underlying function uses different names,
- bound values are evaluated once in source order at the `with` expression, and stored under the [escaping-capture rules](/advanced/memory-management#closures): structures require `copy` unless they can move, and unique values cannot be bound (`MEM003`),
- a parameter that the function mutates, and a variadic parameter, can't be bound,
- the new function has the effects of the original one,
- on a generic function, type parameters fixed by the bound values are specialized; residual generic parameters are usable only in the permitted named-function or callback scope, never as stored or returned generic function values (`GN030`).

For example, the visible name below is `amount`:

```fa
type Bound = () => Integer
function increment = (value: Integer): Integer => value + 1
function bind = (operation: (amount: Integer) => Integer): Bound => operation with (amount = 2)
let bound = bind(increment)
console.log(bound()) -- 3
```

`with` binds like a call: `add with (b = 1)` needs no parentheses. With runtime values, `with` gives a regular value: `let addTax = addPercent with (rate = readRate())`.

## Generic functions

A function can take type parameters, written between angle brackets at the start of its signature. A capitalized parameter is a type parameter:

```fa
function first = <Item>(items: Array<Item>): Item? => items.get(0)

first([1, 2, 3])     -- Item is inferred as Integer
first<Float>([1, 2]) -- Item is given explicitly
```

Inferred type parameters get the base type of the arguments: `[1, 2, 3]` is an `Array<Integer>`, so `Item` is `Integer`, not a literal or a ranged integer.

A type parameter can be constrained with `:`:

```fa
function sum = <Value: Integer | Float>(values: Array<Value>): Value => values.reduce((total, value) => total + value, 0)
```

The body of a generic function is checked once, against the constraints of its parameters. See [generics](/types/generics) for constraints, defaults, compile-time value parameters, [generic lambdas and function types](/types/generics#generic-lambdas-and-function-types), and how `<` and `>` are told apart from the comparison operators.

## Function types

The type of a function is written like its signature, without the body:

```fa
type Comparator = (a: Integer, b: Integer) => Integer
type Callback = (message: String) => Void
type Loader = (id: String) => User throws LoadError requires database: Database
type Reset = (player: Player) => Void mutates player
type Handler = async (request: Request) => Response
```

Parameter names are optional in function types: `(Integer, Integer) => Integer`. They are needed when an effect refers to a parameter, like `mutates player`.

A function can be stored in a value of a function type only if the type declares all of its effects, so that a call through the value has the same effects as a direct call:

```fa
function addBonus = (player: Player) mutates player => {
	player.score += 10
}

let reset: Reset = addBonus -- valid: `Reset` declares `mutates player`
let callback: (player: Player) => Void = addBonus -- error: `addBonus` mutates `player`, but the function type doesn't declare it
```

A function `f` fits a function type `T` when:

- they have the same number of parameters, matched by position (except a lambda, which may declare fewer); each parameter of `T` is accepted by the corresponding parameter of `f`,
- the return type of `f` is accepted by the return type of `T`; a `T` returning `Void` accepts any return type, and the value is dropped,
- every effect of `f` is declared by `T`. Effect paths are matched by position: `mutates player` in `T` covers `mutates p` in `f` when `p` is its first parameter. A declared path covers the paths below it: `mutates player` covers `mutates player.score`,
- `f` is `async` only if `T` is: an `async` function can't be stored in a function type without `async`, while a function that isn't `async` can be stored in an `async` function type. A lambda is `async` when its expected function type is (see [async function types](/advanced/concurrency#async-function-types)).

A function type may declare effects that the stored function doesn't have: callers handle them anyway. Inside a function type, a parameter that is itself a function counts as stored: its argument must declare every effect. The last parameter of a function type can be variadic, like `(...values: Integer) => Integer`, which only a function with a variadic parameter of the same type fits. Calling a value of a function type passes the arguments by position only: named arguments and default values don't go through function values.

`Void` is the return type of a function that returns nothing. It's only written in function types: in a function declaration, a block without a return type already returns nothing. `Void` can be a type argument, like `Task<Void>`, but a value of type `Void` can't be bound with `let`. It is distinct from `None`, the type of `none` (see [special types](/types/operations#special-types)).

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
5. A field and a method can't have the same name.
6. Inside the type, an associated function is called through the type (`Self.create(0)` or `Counter.create(0)`), and a method through `self` (`self.double()`).

A method can be used as a function value through its type: `Human.describe` is a function whose first parameter is the instance, as in `humans.map(Human.describe)`. Through an instance, a method must be called: `human.describe` without parentheses is an error, write a lambda, `() => human.describe()`. There is no hidden capture of the instance.

A method call is resolved at compile time, from the type written in the code for the value: a parameter typed `Human` calls the methods of `Human`, even when the argument is a `Hero` with a method of the same name. On a union, a compatible common method is available without narrowing: parameter names and types agree, result types and effects are joined, and the actual alternative supplies the method. Otherwise narrow first. Methods are not part of [structural typing](/advanced/structural-typing): only data fields are compared. In a generic function, a method called on a value of a type parameter is resolved on the actual type, for each instantiation (see [generics](/types/generics)).

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


:::implementation
## Implementation details

### Default and callable ownership

Visible names, default owners, written parameter origins and finalizer/method capabilities use one portable callable metadata graph. Written declarations publish a stable owner recipe with exact generic bindings; an anonymous nested annotation without such an owner retains the explicit-argument diagnostic. A started call binds defaults at creation through the synchronous default entry on every target, with cleanup of earlier owned arguments on failure. Do not add defaults to structural function-type identity or look them up by an equal type.

Callback retention has a per-parameter summary, independent of parameter consumption and the existing callback-error inference. Stored and returned origins seed a dependency graph; forwarding calls and defaults propagate retention without revisiting function bodies. The published parameter flags participate in the interface hash. The query for whether a callee may keep an argument uses its known summary, otherwise it answers yes. The next ownership step replaces that conservative unknown case with `invalidates parameter` in function types; absence of that effect promises borrowing. Capture provenance is recorded from the enclosing flow at creation, while a nonescaping invocation resolves borrowed callable slots at the call. Captured types retain their creation-time narrowing. An escaping environment also retains its captured function handles; an iterative traversal of their saved origins propagates escape checks without consulting later slot assignments. Named sibling/enclosing function references contribute handle dependencies separately from data captures; these dependencies also propagate retained callback parameters across module interfaces. Self calls reuse the running function rather than capturing a new handle.

Retention and its conservative reason are settled before compile-time lowering as well as at file completion. Demand-time checks inspect completed function contexts and use a revision cache; repeated scalar evaluations do not rescan unchanged callable facts. Imported parameter summaries are available through proxies during the walk. A missing optional branch contributes no callable origin to a fallback.

### Native and WebAssembly recursion

A small base case of a direct recursive call is evaluated at the call site when its guard and result use only scalar parameters and operations; an empty node of an arena, whose optional fields are all `none`, is also such a base case. A function whose body is a `when` over a union parameter has the same shortcut for a member whose fields are all scalar: at each recursive call, the member is tested on the evaluated argument and its arm (scalar arithmetic over its fields, the parameters and constants, without calls or `try`) is computed in place; the arms before it may only test other members. Arguments are evaluated once, in order. In speed builds, an integer sum through a final optional recursive call (`prefix + (child?.walk() ?? 0)`) exposes both return paths to LLVM's tail-recursion elimination, so that the last recursive edge can become an accumulator loop. Other effects and calls keep their evaluation order.
:::
