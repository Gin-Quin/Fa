# Generics

Generics let types and functions work with any type that satisfies a constraint, instead of a single concrete type.

```fa
Array<Integer>
Map<String, User>
Integer<min = 0, max = 200>
```

Generic parameters are written between angle brackets, `<` and `>`. Parentheses are only used for calls, so `first<Float>([1, 2])` reads at a glance: the generic parameters, then the call. How `<` and `>` are told apart from the comparison operators is explained in [angle brackets and comparisons](#angle-brackets-and-comparisons).

## A type is a constraint

In Fa, a type annotation is a **constraint**. Writing `value: Type` does not mean that `value` is exactly of type `Type`: it means that `value` **satisfies** the constraint `Type`.

```fa
type Named = { name: String }
type User = { name: String, email: String }

function greet = (someone: Named) => console.log("Hello, {someone.name}!")

greet(User { name = "Ada", email = "ada@example.com" }) -- valid: a User satisfies `Named`
```

Generic parameters follow the same rule: their annotation is the constraint that an argument must satisfy.

## Generic parameters

A type declares its generic parameters between angle brackets **right after its name**. A function declares them **at the start of its signature**, before its parameters:

```fa
type Point<NumberType> = {
	x: NumberType
	y: NumberType
}

function first = <Item>(items: Array<Item>): Item? => items.get(0)
```

The generic parameters of a function are part of its signature, not of its name: this is why a lambda and a function type can have them too (see [generic lambdas and function types](#generic-lambdas-and-function-types)).

Several parameters are separated by commas:

```fa
type Pair<Left, Right> = {
	left: Left
	right: Right
}

function swap = <Left, Right>(pair: Pair<Left, Right>) => Pair<Right, Left> {
	left = pair.right
	right = pair.left
}
```

### Type parameters and compile-time value parameters

The capitalization of a generic parameter tells what it receives:

- a **capitalized** parameter is a **type parameter**: it receives a type,
- a **lowercase** parameter is a **compile-time value parameter**: it receives a value known at compile time, and its annotation is the type of that value.

```fa
type Matrix<Element, rows: Integer, columns: Integer> = {
	values: Array<Element>

	function get = (
		self
		row: Integer<min = 0, max = rows - 1>
		column: Integer<min = 0, max = columns - 1>
	) => self.values[row * columns + column]
}

type Transform = Matrix<Float, 4, 4>
```

Inside the declaration, a compile-time value parameter is used like any other value. Here, `rows` and `columns` bound the indices accepted by `get`, so `transform.get(4, 0)` is a compile error.

A compile-time value parameter receives a value whose equality is exact: a boolean, an integer, a string, a character, an enumeration or `Fields` value (such as a [key set](/types/fields#key-sets)), or an array, a tuple or an object made of them. Floats and functions can't be compile-time parameters. Two applications with equal values give the same type: `Matrix<Float, 4, 4>` and `Matrix<Float, 2 + 2, 4>` are one type. A range whose minimum is greater than its maximum, like `Integer<min = 0, max = rows - 1>` when `rows` is 0, is the type `Never`, which no value satisfies.

Ranged integers are a built-in example of compile-time value parameters: in `Integer<min = 0, max = 200>`, `min` and `max` are lowercase because they are values, not types.

Generic parameter lists are the only place where capitalization carries a meaning. Everywhere else, `PascalCase` for types and `camelCase` for values is a convention. See [naming](/types/declaring-types#naming).

## Constraints

A generic parameter can be constrained with `:`, followed by the constraint the argument must satisfy. Without a constraint, any type is accepted.

A **union** means "one of these types": the parameter is bound to exactly one member. An argument whose type satisfies exactly one member binds that member, so an `Array<Integer<min = 0>>` gives `Value = Integer`, and is passed as a read-only `Array<Integer>`.

```fa
type Point<NumberType: Integer | Float> = {
	x: NumberType
	y: NumberType
}

function sum = <Value: Integer | Float>(values: Array<Value>): Value => {
	mutable total: Value = 0
	for value in values {
		total += value
	}
	total
}
```

An **object type** means "has at least these fields", following [structural typing](/advanced/structural-typing):

```fa
function findById = <Entity: { id: Integer }>(entities: Array<Entity>, id: Integer): Entity? => {
	for entity in entities {
		if entity.id == id {
			return entity
		}
	}
	none
}
```

Any type with an `id: Integer` field is accepted, with no declaration of intent.

A constraint can also require a method, written like a function field whose first parameter is `self`:

```fa
function describeAll = <Item: { describe: (self) => String }>(items: Array<Item>) => {
	for item in items {
		console.log(item.describe())
	}
}
```

Any type with a `describe` method returning a `String` is accepted, and `item.describe()` calls the method of the actual type: this is how Fa gets polymorphism without traits or interfaces. This includes the types with `#private` fields: they are distinct from every other type, but they satisfy an object constraint through their public fields and methods.

### Copy conditions

Generic code is checked once with symbolic type parameters; storing or copying a parameter adds a semantic copy condition, checked at each instantiation and carried through imports, wrappers and permitted callbacks. No `Copyable` annotation is needed:

```fa
function duplicate = <Item>(value: Item): Item => copy value
let values = [1, 2]
let independent = duplicate(values)
console.log(values, independent)
```

An explicit copy accepts ordinary values and copyable structures; a unique Task/resource instantiation is `MEM003`. Returning a borrowed `Item` without `copy` works for ordinary types and is `MEM004` for a structure. Merely reading or passing a borrowed `Item` adds no copying requirement. Conditions depending on generic synchronized payloads and mutation are resolved at specialization; reject only the instantiation whose effects or transfer cannot be proven, never all generic uses. See [copies and moves](/advanced/memory-management#explicit-copy).

### Checking generic code

A generic type or function is checked **once**, against the constraints of its parameters, not once per use. Inside it, only the operations allowed by the constraints can be used:

```fa
function double = <Value>(value: Value) => value * 2 -- error: `Value` has no constraint allowing `*`
function double = <Value: Integer | Float>(value: Value) => value * 2 -- valid
```

Under a union constraint, each operation is checked once per member, and must give the same kind of result for every member: in `double` above, `value * 2` is an `Integer` for `Integer` and a `Float` for `Float`, so it is a `Value`, while `value / 2` is a `Float` for both. An operation valid for one member only, like `value.shiftLeft(1)`, is an error.

Code that depends on the value of a compile-time parameter is checked after the parameters are given, like in Zig: the body of an [`@for` or an `@if`](/advanced/metaprogramming) over a compile-time parameter, and the right side of a [type function](/types/type-functions). An error found there is reported where the parameters are given, with the declaration it comes from.

Each use of a generic function is compiled for its actual parameters, so a method called on a type parameter is the method of the actual type.

### Constraints that use earlier parameters

A constraint can reference the parameters declared before it:

```fa
function select = <Entity, keys: Keys<Entity>>(entity: Entity): Pick<Entity, keys> => pick(entity, keys)

type User = { name: String, email: String, password: String }

let user = User { name = "Ada", email = "ada@example.com", password = "secret" }
let publicUser = select<User, { name, email }>(user) -- { name: String, email: String }
```

Here, `keys` must be a [key set](/types/fields#key-sets) of `Entity`: `select<User, { age }>` is a compile error. See [type functions](/types/type-functions) for `Keys`, `Pick` and `pick`.

The same call works when `select` is imported, preserving the exact selected fields. Its derived result is computed by the compiler; an extra type argument cannot replace it (`GN007`).

## Default parameters

Generic parameters can have defaults, written with `=`:

```fa
type Collection<Item, Index = Integer> = {
	get: (index: Index) => Item?
}

type Names = Collection<String> -- Index is Integer
type NamesByKey = Collection<String, String> -- Index is String
```

## Using generics

### Inference

Generic parameters are inferred from the arguments, and from the expected type:

```fa
let point = Point { x = 12, y = 13 } -- Point<Integer>
let firstName = first(["Ada", "Alan"]) -- Item is String
let total = sum([1, 2, 3]) -- Value is Integer

let ratio: Float = sum([1, 2, 3]) -- Value is Float, inferred from the expected type
```

A parameter is found from, in this order:

1. the parameters given explicitly,
2. the expected type, to which literals adapt (`ratio` above),
3. the types of the arguments, where a literal has its [base type](/types/declaring-types#inferred-types-and-known-values): `["Ada", "Alan"]` is an `Array<String>`, and `[1, 2, 3]` an `Array<Integer>`,
4. the default of the parameter.

A parameter can also be found from another one: in `select = <Entity, keys: Keys<Entity>>` above, `Entity` gives the type of `keys`. A generic function passed as a value is given its parameters by the expected function type, as in `["Ada", "Alan"].map(identity)`.

Inference never guesses between two valid answers: when a parameter is ambiguous, or when nothing allows inferring it, it is a compile error, and you have to write it explicitly.

```fa
function parse = <Output: Integer | Float>(input: String): Output throws ParseError => ...

let count: Integer = try parse("42") -- inferred from the expected type
let count = try parse("42") -- error: `Output` is ambiguous
```

### Explicit parameters

To give the parameters explicitly, write them between angle brackets right after the name of the type or of the function:

```fa
let point = Point<Float> { x = 12, y = 13 }
let total = sum<Float>([1, 2, 3])
let count = try parse<Integer>("42")
let numbers = Set<Float>.from([1, 2, 3])
```

`sum<Float>` is itself a function, which is then called with `([1, 2, 3])`. Since generic parameters have their own brackets, a chain of calls is never ambiguous, even when a generic function returns a function:

```fa
function makeAdder = <Value: Integer | Float>(base: Value) => (value: Value) => base + value

makeAdder<Float>(5)(3) -- 8.0: the generic parameters, then two calls
makeAdder(5)(3) -- 8: `Value` is inferred, then two calls
```

Compile-time value parameters are never inferred from the arguments of the call: they are always given, like in `repeatText<3>("ab")` or `select<User, { name, email }>(user)`.

On a built-in scalar type, parentheses are a [conversion](/types/type-conversion) and angle brackets a refinement:

```fa
let small = Int8(reading) catch error => 0 -- a conversion, which throws when `reading` may not fit
type Small = Integer<min = 0, max = 100> -- a ranged type
```

Giving an argument that doesn't satisfy the constraint is a compile error:

```fa
let total = sum<String>(["a", "b"]) -- error: `String` doesn't satisfy the constraint `Integer | Float` of `Value`
```

### Angle brackets and comparisons

`<` and `>` are also the comparison operators. The tokenizer tells them apart without lookahead, with four rules that a reader can apply as easily:

1. `<` followed by whitespace (a space, a tab or a line break) is the comparison operator. Any other `<` opens a generic list, whatever comes before it.
2. Inside an open generic list, `>` always closes it. Outside one, `>` is the comparison operator.
3. A comparison inside a generic list goes in parentheses: `Choose<(count > 255), UInt16, UInt8>`. Parentheses, brackets and braces opened inside a generic list suspend it until they close, so `>` is the comparison operator again inside them.
4. `<=`, `>=`, `=>` and `->` are tokens of their own, so they are never generic brackets: in `Pool<people->bestFriend>`, `->` doesn't close the list.

```fa
type Choose<condition: Boolean, Then, Else> = if condition { Then } else { Else }
static count = 300

if count < limit { ... } -- `<` followed by a space: a comparison
let pairs: Array<Tuple<Integer, String>> = [] -- `>>` closes two generic lists
let handlers: Array<(event: Event) => Void> = [] -- `=>` is its own token
type Index = Choose<(count > 255), UInt16, UInt8> -- a comparison in a generic list, in parentheses: `UInt16`

if count<limit { ... } -- error: `<limit` opens a generic list; write `count < limit`
```

The mistake is reported once: the compiler then reads this `<` as a comparison, so the rest of the line and the block of the `if` are checked as usual.

So a comparison is always written with whitespace after `<`, which the formatter does anyway, and a generic list starts on the line of its `<`: a `<` at the end of a line is a comparison. Its closing `>` can be on its own line:

```fa
type Lookup = Map<String, {
	id: Integer
	name: String
}>
```

A line break inside a generic list never separates two items: the items are separated by commas. A list too long for its line keeps its first item on the line of `<`, and continues after each comma, with `>` on its own line. This is how the formatter writes it, once breaking the object types and the parameters of the line is not enough:

```fa
type Registry<Key: String | Integer,
	Entry: { id: Key, name: String },
	Fallback: { name: String } = Entry
> = {
	entries: Map<Key, Entry>
	fallback: Fallback?
}
```

Rust has the same ambiguity and solves it with the turbofish (`sum::<f64>(...)`) in expressions; TypeScript and C# guess by looking ahead. Fa decides from the character that follows `<`, and then from the open generic lists, with no lookahead.

### Empty values

An empty literal gives nothing to infer from. It needs a context, or explicit parameters:

```fa
function empty = <Item>(): Array<Item> => []

let numbers: Array<Integer> = empty() -- inferred from the expected type
let numbers = empty<Integer>() -- explicit
let numbers = empty() -- error: `Item` cannot be inferred

mutable names: Array<String> = []
let scores = Array<Integer>.from([])
```

## Types versus values

Giving parameters to a generic type produces a **type**. Writing an object literal after a type produces a **value**:

```fa
type Pair<Left, Right> = {
	left: Left
	right: Right
}

type Entry = Pair<String, Integer> -- a type

let entry = Pair<String, Integer> { left = "age", right = 42 } -- a value of that type
```

A generic type is a [type function](/types/type-functions): a function, evaluated at compile time, from its parameters to a type. Values are always created with braces: there is no positional constructor like `Pair("age", 42)`. See [creating values of a type](/types/declaring-types#creating-values-of-a-type).

## Generic methods

Methods declared in a generic type can use the parameters of the type:

```fa
type Stack<Item> = {
	#items: Array<Item>

	function push = (self, item: Item) mutates self => {
		self.#items.push(item)
	}

	function pop = (self) => self.#items.pop() -- inferred: `invalidates self.#items[self.#items.length - 1]`
}
```

A method can also declare its own generic parameters, at the start of its signature:

```fa
type Stack<Item> = {
	#items: Array<Item>

	function map = <Result>(self, transform: (item: Item) => Result) => Stack<Result> {
		#items = self.#items.map(transform)
	}
}
```

## Generic errors

A generic function can be generic over the errors of the functions it receives. See [errors](/values/errors).

```fa
function retry = <Result, Failure>(
	attempts: Integer<min = 1>
	action: () => Result throws Failure
): Result throws Failure => ...
```

## Generic lambdas and function types

The generic parameters of a function are part of its signature. So a lambda can declare them too, and a function type can have them: `<Value>(value: Value) => Value` is the type of a function that works for any `Value`, given at each call.

```fa
function identity = <Value>(value: Value): Value => value

function applyToBoth = (transform: <Value>(value: Value) => Value, number: Integer, text: String) => {
	(transform(number), transform(text)) -- `Value` is `Integer`, then `String`
}

applyToBoth(identity, 1, "one") -- (1, "one")
applyToBoth(<Value>(value: Value) => value, 1, "one") -- a generic lambda
applyToBoth((value) => value, 1, "one") -- valid: the lambda is generic, from its expected type
applyToBoth((value: Integer) => value, 1, "one") -- error: the lambda isn't generic
```

- **A value of a generic function type is known at compile time.** It is a named function or a lambda, given directly as an argument (see [where a generic function type is written](#where-a-generic-function-type-is-written)). A function that receives one is compiled for each function it receives, like a generic function for each of its parameters, so `transform(number)` and `transform(text)` call two compiled versions of `identity`, without boxing.
- **Generic function types are never inferred**: they come from an annotation, like `transform` above. A lambda without generic parameters is generic only when its expected type is.
- A generic function passed where a function type that isn't generic is expected is given its parameters by that type: in `["Ada", "Alan"].map(identity)`, `Value` is `String`.

### Where a generic function type is written

A generic function type is written in one place: as the type of a parameter of a function declared with `function` (a method included). Such a parameter receives:

- a named generic function, like `identity`,
- a lambda written as the argument,
- a parameter of the calling function that has a generic function type itself.

```fa
function applyTwice = (transform: <Value>(value: Value) => Value, number: Integer, text: String) => {
	applyToBoth(transform, number, text) -- valid: `transform` is known wherever `applyTwice` is compiled
}
```

Everywhere else, a generic function type is an error: a `let`, `static` or `mutable` declaration, a return type, a field, an element of a collection, or a parameter of a lambda or of another function type.

```fa
let alias: <Value>(value: Value) => Value = identity -- error: use `identity` itself
let same = <Value>(value: Value) => value -- error: use `function` to declare a function
let numberIdentity: (value: Integer) => Integer = identity -- valid: not generic, `Value` is `Integer`

```

A function is named by its `function` declaration only, generic or not, so there is nothing an alias could add. A function that would return a generic function is written as one generic function with more parameters.

### Async generic lambdas

`async` comes first, then the generic parameters, as in a declaration, where `async` comes before `function` and the generic parameters start the signature. The same order is used for a function type:

```fa
async function fetchAll = <Item>(urls: Array<String>): Array<Item> => ...

function forEach = (urls: Array<String>, action: async <Item>(item: Item) => Void) => ...

forEach(urls, async <Item>(item: Item) => {
	console.log("received")
})
```

`<Item> async (item: Item) => ...` is a syntax error. `async` is the only keyword written before a lambda: its effects (`throws`, `mutates`...) are written after its return type, with or without generic parameters.

### Enclosing generic parameters

A lambda written inside a generic function or type can use the generic parameters of its enclosing declaration.

A function satisfies a generic function type with the same number of parameters when each constraint of the type satisfies the function's constraint. A narrower constraint is `E-GEN-1`; a function that is not generic is `E-GEN-2`.

## What Fa deliberately leaves out

To keep the type system predictable, Fa has no:

- user-defined specialization or overloading,
- higher-kinded types,
- variance annotations,
- implicit conversions used to satisfy a constraint,
- overload resolution based on the return type.

Generic types, generic functions, structural constraints, compile-time value parameters, defaults and inference cover the cases these features are usually reached for.

:::implementation
## Implementation details

### JavaScript

- Generic functions are erased: one JavaScript function serves every use, without boxing.
- A parameter bound to a union whose members behave differently at runtime (`Integer | Float` when the operations differ, an explicit `Int64` represented as a `BigInt`, overflow checks) gets one JavaScript instance per member actually used.
- Operations that depend on the actual type of another parameter (static methods, copies, `==`, hashing, formatting) receive hidden dictionaries of functions.
- Imported calls use the same published generic order and runtime requirements as local calls. Each erased type parameter also receives an object-versus-slot flag for references; primitive and mixed-union slots keep their cached carrier even when the current value is an object.
- Explicit applications and implicit conversions to a concrete function type bind hidden arguments once when the function value is created. A written generic owner binds its own arguments while preserving a method's remaining generic list. The bound function preserves an asynchronous entry's task-first convention.
- A parameter of a generic function type receives the JavaScript function itself: the function that receives it is erased too, and needs one instance per argument only when the argument needs one of the instances or dictionaries above.

### Native

- Every use is monomorphized for its actual parameters.
- A generic function value is the ordinary function pair: a pointer to a constant table of its required instances and its closure environment. Whole-program compilation fills the table from the types at which the value is called; no box is allocated. Named capturing functions reuse their declaration environment. A producer retains its enclosing type, method and compile-time value bindings when its entries are instantiated, and those bindings distinguish its cached tables even when it has no data captures. An instance set that keeps growing is polymorphic recursion, reported with its first types.
- Tokenizer: a stack of the open brackets. `<` followed by a non-whitespace character pushes a generic list; `>` pops it when the top of the stack is a generic list, and is a comparison otherwise; `(`, `[` and `{` push, and their closing tokens pop. `<=`, `>=`, `=>` and `->` are matched first, as longer tokens.
:::
