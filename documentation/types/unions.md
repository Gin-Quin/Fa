# Unions

A union is a type whose values are one of several alternatives.

## Anonymous unions

The `|` operator builds a union out of existing types:

```fa
type Value = String | Integer

type Direction = "north" | "south" | "east" | "west"

type MaybeUser = User | None -- the same as User?
```

Anonymous unions are structural: `String | Integer` and `Integer | String` are the same type, and a `String` satisfies both.

A union is flattened, and a member that another member already covers disappears: `"north" | String` is `String`, `Int8 | Integer` is `Integer`, `Integer? | None` is `Integer?`, and `Never` disappears from any union.

### Members must be distinguishable

A value of a union must tell, at runtime, which member it holds. So the members of an anonymous union must differ:

- by their runtime kind: number (`Integer`, `Float` or `Percentage`, one of them), `BigInt`, string, boolean, `none`, function, collection, object;
- or, for members of the same kind, by their values: literals (`"north" | "south"`), or integer ranges that don't overlap;
- or, for object members, by their fields: each one has a required field that the other lacks, or both have the same field with literal types that don't overlap (a discriminant).

```fa
type Value = String | Integer -- valid: a string and a number
type Size = "small" | "large" -- valid: two literals
type Shape = { kind: "circle", radius: Float } | { kind: "square", side: Float } -- valid: a discriminant

type Number = Integer | Float -- error: both are numbers, use a named union
type Person = { name: String } | { name: String, age: Integer } -- error: the first has no field of its own, use a named union
```

Members that can't be told apart are an error that suggests a [named union](#named-unions), whose members carry their name at runtime. A value that satisfies several object members at once can't be stored in the union: copy it into the member type first.

## Named unions

When the alternatives need names or carry different data, declare the union with the `Union` type constructor:

```fa
type MyUnion = Union {
	Hello = "hello"

	World = "world"

	Car = {
		brand: String
		model: String
		year: Integer
	}

	House = {
		address: String
		price: Integer
	}
}
```

Create a value by naming its member:

```fa
let myCar = MyUnion.Car {
	brand = "Toyota"
	model = "Corolla"
	year = 2020
}

let myGreeting: MyUnion = "hello"
```

A member whose definition is a literal (like `Hello = "hello"`) accepts that literal directly; two members of the same union can't accept the same literal. A member whose definition is an object requires the member name. A member whose definition is another type takes its value in parentheses, and a member without definition carries no data:

```fa
type Shape = Union {
	Circle = { radius: Float }
	Labelled = String
	Point
}

let circle = Shape.Circle { radius = 1.0 }
let label = Shape.Labelled("north")
let point = Shape.Point
```

Members of a named union are distinguished by name: `MyUnion.Car` and `MyUnion.House` stay distinct even if their fields match. A named union is distinct from every other type, even one with the same members, and each member is a type of its own: a `Shape.Circle` is a `Shape`, and its fields can be read like those of `{ radius: Float }`, but a plain `{ radius: Float }` is not a `Shape.Circle`. This makes a single-member union a way to declare a type that is distinct from its content:

```fa
type UserId = Union { User = Integer }

let userId = UserId.User(12)
let raw: Integer = userId -- error: a `UserId` is not an `Integer`
```

Member names are only reachable through the union, as `Shape.Point`: inside the declaration, `String` is still the type `String`, even if a member is called `String`.

## Matching a union

Use `when` to branch on which member a value holds:

```fa
function describe = (value: MyUnion) => when value is {
	MyUnion.Car => console.log("It's a car")
	MyUnion.House => console.log("It's a house")
	MyUnion.Hello => console.log("It's hello")
	MyUnion.World => console.log("It's world")
}
```

Several members can share a branch:

```fa
function classify = (value: MyUnion) => when value is {
	MyUnion.Car | MyUnion.House => console.log("It's a thing")
	MyUnion.Hello | MyUnion.World => console.log("It's a word")
}
```

### Binding the payload

Use `as` to give a name to the matched value, or to destructure it:

```fa
type Shape = Union {
	Circle = { radius: Float }
	Rectangle = { width: Float, height: Float }
}

let description = when shape is {
	Shape.Circle as circle => "circle of radius {circle.radius}"
	Shape.Rectangle as { width, height } => "rectangle {width}x{height}"
}
```

Inside the branch, the bound value has the type of the member: `circle` is a `Shape.Circle`.

`when` has no guards: to test a condition on the payload, use an `if` inside the branch.

```fa
let description = when shape is {
	Shape.Circle as circle => if circle.radius == 0 { "point" } else { "circle" }
	Shape.Rectangle => "rectangle"
}
```

### Exhaustiveness

`when` must cover every member of the union. A missing member is a compile error, not a silent fallthrough:

```fa
when shape is {
	Shape.Circle => "circle"
	-- error: `Shape.Rectangle` is not handled
}
```

Add an `else` branch to handle the rest:

```fa
when shape is {
	Shape.Circle => "circle"
	else => "something else"
}
```

:::tip
Prefer listing every member over writing `else`. When you later add a member to the union, the compiler will point you at every `when` that needs updating, which is the main reason to use a named union in the first place.
:::

See [conditionals](/control-flow/conditionals) for everything `when` can match.

## Narrowing

Testing a union with `is` narrows it in the branch where the test succeeded:

```fa
function describe = (value: String | Integer): String => {
	if value is String {
		return "string of length {value.length}" -- `value` is a String here
	}

	"number {value}" -- `value` is an Integer here
}
```

`is` with a type only tests at runtime which member of a union a value holds. On a value that is not a union, the answer is known while compiling: a test that is always true is a warning, and a test that is never true is an error. See [narrowing](/control-flow/conditionals#narrowing) for every test that narrows.

## Unions and recursion

A type cannot contain itself directly: its size would be infinite, so it is a compile error.

```fa
type Node = {
	value: String
	child: Node -- error: `Node` contains itself
}
```

Recursion is allowed through a union, including an optional (a union with `None`), and through a collection. The recursive member is then stored on the heap automatically, and each value owns its children:

```fa
type Tree = Union {
	Leaf = { value: String }
	Branch = { left: Tree, right: Tree }
}

type Node = {
	value: String
	child: Node?
}

type Folder = {
	name: String
	folders: Array(Folder)
}
```

There is no `Box` type: you never allocate the recursive part yourself.

The rules of recursive types:

- A type can only refer to itself through its name: recursion goes through a named `type` declaration, and two declarations can refer to each other.
- Two recursive types are the same type when unfolding them gives the same fields forever: `Node` and another type `Link = { value: String, child: Link? }` are the same type.
- A generic type can't refer to itself with other parameters than its own: `type List(Item) = { head: Item, tail: List(Array(Item))? }` is an error, since every use would need a new type.
- [`DeepKeys`](/types/type-functions#deepkeysobjecttype) stops at a recursive field.

For structures that genuinely need to share nodes, use a collection and [relations](/advanced/relations) instead.

## Unions of errors

An error type is a union whose members are errors. Declare it with the `Errors` type constructor rather than `Union`, so that it can be used with `throws`, `throw` and `catch`:

```fa
type ApiError = Errors {
	Timeout = { after: Integer }
	Network = { message: String }
}

function load = (): String throws ApiError => ...
```

See [errors](/values/errors).
