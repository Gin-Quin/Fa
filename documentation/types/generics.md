# Generics

Generics let types and functions work with any type that satisfies a constraint, instead of a single concrete type.

```fa
Array(Integer)
Map(String, User)
Integer(min = 0, max = 200)
```

Fa uses parentheses, not angle brackets: giving parameters to a generic type or function looks like a call, because it is one, evaluated at compile time.

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

Generic parameters are written in parentheses **right after the declared name**, for types and for functions alike:

```fa
type Point(NumberType) = {
	x: NumberType
	y: NumberType
}

function first(Item) = (items: Array(Item)): Item? => items.get(0)
```

Several parameters are separated by commas:

```fa
type Pair(Left, Right) = {
	left: Left
	right: Right
}

function swap(Left, Right) = (pair: Pair(Left, Right)) => Pair(Right, Left) {
	left = pair.right
	right = pair.left
}
```

### Type parameters and compile-time value parameters

The capitalization of a generic parameter tells what it receives:

- a **capitalized** parameter is a **type parameter**: it receives a type,
- a **lowercase** parameter is a **compile-time value parameter**: it receives a value known at compile time, and its annotation is the type of that value.

```fa
type Matrix(Element, rows: Integer, columns: Integer) = {
	values: Array(Element)

	function get = (
		self
		row: Integer(min = 0, max = rows - 1)
		column: Integer(min = 0, max = columns - 1)
	) => self.values[row * columns + column]
}

type Transform = Matrix(Float, 4, 4)
```

Inside the declaration, a compile-time value parameter is used like any other value. Here, `rows` and `columns` bound the indices accepted by `get`, so `transform.get(4, 0)` is a compile error.

A compile-time value parameter receives a value whose equality is exact: a boolean, an integer, a string, a character, an enumeration or `Fields` value (such as a [key set](/types/fields#key-sets)), or an array, a tuple or an object made of them. Floats and functions can't be compile-time parameters. Two applications with equal values give the same type: `Matrix(Float, 4, 4)` and `Matrix(Float, 2 + 2, 4)` are one type. A range whose minimum is greater than its maximum, like `Integer(min = 0, max = rows - 1)` when `rows` is 0, is the type `Never`, which no value satisfies.

Ranged integers are a built-in example of compile-time value parameters: in `Integer(min = 0, max = 200)`, `min` and `max` are lowercase because they are values, not types.

Generic parameter lists are the only place where capitalization carries a meaning. Everywhere else, `PascalCase` for types and `camelCase` for values is a convention. See [naming](/types/declaring-types#naming).

## Constraints

A generic parameter can be constrained with `:`, followed by the constraint the argument must satisfy. Without a constraint, any type is accepted.

A **union** means "one of these types": the parameter is bound to exactly one member. An argument whose type satisfies exactly one member binds that member, so an `Array(Integer(min = 0))` gives `Value = Integer`, and is passed as a read-only `Array(Integer)`.

```fa
type Point(NumberType: Integer | Float) = {
	x: NumberType
	y: NumberType
}

function sum(Value: Integer | Float) = (values: Array(Value)): Value => {
	mutable total: Value = 0
	for value in values {
		total += value
	}
	total
}
```

An **object type** means "has at least these fields", following [structural typing](/advanced/structural-typing):

```fa
function findById(Entity: { id: Integer }) = (entities: Array(Entity), id: Integer): Entity? => {
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
function describeAll(Item: { describe: (self) => String }) = (items: Array(Item)) => {
	for item in items {
		console.log(item.describe())
	}
}
```

Any type with a `describe` method returning a `String` is accepted, and `item.describe()` calls the method of the actual type: this is how Fa gets polymorphism without traits or interfaces.

### Checking generic code

A generic type or function is checked **once**, against the constraints of its parameters, not once per use. Inside it, only the operations allowed by the constraints can be used:

```fa
function double(Value) = (value: Value) => value * 2 -- error: `Value` has no constraint allowing `*`
function double(Value: Integer | Float) = (value: Value) => value * 2 -- valid
```

Code that depends on the value of a compile-time parameter is checked after the parameters are given, like in Zig: the body of an [`@for` or an `@if`](/advanced/metaprogramming) over a compile-time parameter, and the right side of a [type function](/types/type-functions). An error found there is reported where the parameters are given, with the declaration it comes from.

Each use of a generic function is compiled for its actual parameters, so a method called on a type parameter is the method of the actual type.

### Constraints that use earlier parameters

A constraint can reference the parameters declared before it:

```fa
function select(Entity, keys: Keys(Entity)) = (entity: Entity): Pick(Entity, keys) => pick(entity, keys)

type User = { name: String, email: String, password: String }

let user = User { name = "Ada", email = "ada@example.com", password = "secret" }
let publicUser = select(User, { name, email })(user) -- { name: String, email: String }
```

Here, `keys` must be a [key set](/types/fields#key-sets) of `Entity`: `select(User, { age })` is a compile error. See [type functions](/types/type-functions) for `Keys`, `Pick` and `pick`.

## Default parameters

Generic parameters can have defaults, written with `=`:

```fa
type Collection(Item, Index = Integer) = {
	get: (index: Index) => Item?
}

type Names = Collection(String) -- Index is Integer
type NamesByKey = Collection(String, String) -- Index is String
```

## Using generics

### Inference

Generic parameters are inferred from the arguments, and from the expected type:

```fa
let point = Point { x = 12, y = 13 } -- Point(Integer)
let firstName = first(["Ada", "Alan"]) -- Item is String
let total = sum([1, 2, 3]) -- Value is Integer

let ratio: Float = sum([1, 2, 3]) -- Value is Float, inferred from the expected type
```

A parameter is found from, in this order:

1. the parameters given explicitly,
2. the expected type, to which literals adapt (`ratio` above),
3. the types of the arguments, where a literal has its [base type](/types/declaring-types#inferred-types-and-known-values): `["Ada", "Alan"]` is an `Array(String)`, and `[1, 2, 3]` an `Array(Integer)`,
4. the default of the parameter.

A parameter can also be found from another one: in `select(Entity, keys: Keys(Entity))` below, `Entity` gives the type of `keys`. A generic function passed as a value is given its parameters by the expected function type, as in `["Ada", "Alan"].map(identity)`.

Inference never guesses between two valid answers: when a parameter is ambiguous, or when nothing allows inferring it, it is a compile error, and you have to write it explicitly.

```fa
function parse(Output: Integer | Float) = (input: String): Output throws ParseError => ...

let count: Integer = try parse("42") -- inferred from the expected type
let count = try parse("42") -- error: `Output` is ambiguous
```

### Explicit parameters

To give the parameters explicitly, write them right after the name, the same way they are declared:

```fa
let point = Point(Float) { x = 12, y = 13 }
let total = sum(Float)([1, 2, 3])
let count = try parse(Integer)("42")
let numbers = Set(Float).from([1, 2, 3])
```

`sum(Float)` is itself a function, which is then called with `([1, 2, 3])`.

### Generic application or call

Both look like a call, so the compiler tells them apart with one rule. The first list of arguments given to a generic function or type is its **generic parameters** when:

- the declaration has a compile-time value parameter (a lowercase one): `repeatText(3)("ab")`, `select(User, { name, email })(user)`,
- or the list has at least one argument, and every argument is a type: `sum(Float)([1, 2, 3])`, `Point(Float) { x = 1, y = 2 }`.

Otherwise, it is the call itself, and the generic parameters are inferred: `first([1, 2, 3])`, `empty()`. Compile-time value parameters are never inferred from the arguments of the call: they are always given.

On a built-in scalar type, positional values are a [conversion](/types/type-conversion) and named values are a refinement:

```fa
let small = Int8(reading) catch error => 0 -- a conversion, which throws when `reading` may not fit
type Small = Integer(min = 0, max = 100) -- a ranged type
```

Giving an argument that doesn't satisfy the constraint is a compile error:

```fa
let total = sum(String)(["a", "b"]) -- error: `String` doesn't satisfy the constraint `Integer | Float` of `Value`
```

### Empty values

An empty literal gives nothing to infer from. It needs a context, or explicit parameters:

```fa
function empty(Item) = (): Array(Item) => []

let numbers: Array(Integer) = empty() -- inferred from the expected type
let numbers = empty(Integer)() -- explicit
let numbers = empty() -- error: `Item` cannot be inferred

mutable names: Array(String) = []
let scores = Array(Integer).from([])
```

## Types versus values

Giving parameters to a generic type produces a **type**. Writing an object literal after a type produces a **value**:

```fa
type Pair(Left, Right) = {
	left: Left
	right: Right
}

type Entry = Pair(String, Integer) -- a type

let entry = Pair(String, Integer) { left = "age", right = 42 } -- a value of that type
```

A generic type is a [type function](/types/type-functions): a function, evaluated at compile time, from its parameters to a type. Values are always created with braces: there is no positional constructor like `Pair("age", 42)`. See [creating values of a type](/types/declaring-types#creating-values-of-a-type).

## Generic methods

Methods declared in a generic type can use the parameters of the type:

```fa
type Stack(Item) = {
	#items: Array(Item)

	function push = (self, item: Item) mutates self => {
		self.#items.push(item)
	}

	function pop = (self) => self.#items.pop() -- inferred: `invalidates self.#items[self.#items.length - 1]`
}
```

A method can also declare its own generic parameters, after its name:

```fa
type Stack(Item) = {
	#items: Array(Item)

	function map(Result) = (self, transform: (item: Item) => Result) => Stack(Result) {
		#items = self.#items.map(transform)
	}
}
```

## Generic errors

A generic function can be generic over the errors of the functions it receives. See [errors](/values/errors).

```fa
function retry(Result, Failure) = (
	attempts: Integer(min = 1)
	action: () => Result throws Failure
): Result throws Failure => ...
```

## Lambdas are not generic

A lambda cannot declare generic parameters: generic parameters are written after a name, and a lambda has none. Declare a generic `function` instead, and pass it where a function is expected:

```fa
function identity(Value) = (value: Value): Value => value

let names = ["Ada", "Alan"].map(identity) -- Value is String
```

A lambda written inside a generic function or type can use the generic parameters of its enclosing declaration.

## What Fa deliberately leaves out

To keep the type system predictable, Fa has no:

- user-defined specialization or overloading,
- higher-kinded types,
- variance annotations,
- implicit conversions used to satisfy a constraint,
- overload resolution based on the return type.

Generic types, generic functions, structural constraints, compile-time value parameters, defaults and inference cover the cases these features are usually reached for.
