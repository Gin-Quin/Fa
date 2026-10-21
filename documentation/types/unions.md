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

A member whose definition is a literal (like `Hello = "hello"`) accepts that literal directly. A member whose definition is an object requires the member name.

Members of a named union are distinguished by name: `MyUnion.Car` and `MyUnion.House` stay distinct even if their fields match. This makes a single-member union a way to declare a type that is distinct from its content:

```fa
type UserId = Union { User = Integer }
```

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

Testing a union with `is` narrows it for the rest of the branch:

```fa
function describe = (value: String | Integer): String => {
	if value is String {
		return "string of length {value.length}" -- `value` is a String here
	}

	"number {value}" -- `value` is an Integer here
}
```

## Unions and recursion

A type cannot contain itself directly: its size would be infinite, so it is a compile error.

```fa
type Node = {
	value: String
	child: Node -- error: `Node` contains itself
}
```

Recursion through a union is allowed, including through an optional, which is a union with `None`. The recursive member is then stored on the heap automatically, and each value owns its children:

```fa
type Tree = Union {
	Leaf = { value: String }
	Branch = { left: Tree, right: Tree }
}

type Node = {
	value: String
	child: Node?
}
```

There is no `Box` type: you never allocate the recursive part yourself.

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
