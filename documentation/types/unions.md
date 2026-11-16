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

A union is flattened, and a member that another member already covers disappears: `"north" | String` is `String`, `Int8 | Integer` is `Integer`, `Integer? | None` is `Integer?`, and `Never` disappears from any union. This applies to literals, ranges, `None`, enumeration and named union members, `Fields` and collections; object, tuple and function members never disappear this way: they must be [distinguishable](#members-must-be-distinguishable), including preserved written origins as described below.

A member (field or method) that every alternative has with compatible types is available on the union without narrowing. Reading a field returns the union of its alternatives' field types; writing requires a type accepted by every alternative (the same writable field type). A common method has the same visible call parameter names and types in every alternative (apart from its implicit receiver); its result is the union of the results, and its effects are the union of the effects. The call dispatches to the actual alternative. Otherwise narrow first: the diagnostic identifies the alternative missing the member or the differing signatures.

```fa
type Circle = { radius: Float, function area = (self): Float => self.radius * self.radius * 3.14 }
type Square = { side: Float, function area = (self): Float => self.side * self.side }
function area = (shape: Circle | Square): Float => shape.area()
console.log(area(Circle { radius = 2.0 }), area(Square { side = 2.0 }))
```

### Members must be distinguishable

A value of a union must tell, at runtime, which member it holds. So the members of an anonymous union must differ:

- by their runtime kind: number (one of `Integer`, a sized or ranged integer, `Float`, `Percentage`, an enumeration or a `Fields` type), big integer (one of `BigInt`, an `Int64`, `UInt64`, `Int128` or `UInt128` written explicitly, or a `Fields` type of more than 32 flags), string, boolean, `none`, function, array (one of `Array`, `GrowingArray`, `OrderedColumnar` or a tuple), each other collection (`Set`, `Map`, `Pool`, ...), object;
- or by belonging to different named unions: `ApiError | StorageError` is valid, whatever their members;
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

### Written alternatives with identical fields

A join of written types with identical fields but different methods or destructors preserves those alternatives. It infers `Person | Greeting`, rather than erasing one member because its data layout matches. The actual member selects its method or destructor; destruction keeps the exact written owner even through generic calls, imports and containers. Structural assignability remains based on data fields: this rule preserves origin at a join, rather than making every named object nominal.

```fa
type Person = { name: String, function describe = (self): String => self.name }
type Greeting = { name: String, function describe = (self): String => "Hello {self.name}" }
function choose = (person: Person, greeting: Greeting, flag: Boolean) => if flag { person } else { greeting }
```

The result has both written alternatives. `choose(person, greeting, flag).describe()` uses the selected member's method under the common-member rule above. Destructors always belong to the actual alternative. The distinguishability error above still applies to anonymous shapes without distinct written behavior to preserve. Copyability follows every alternative: a possible unique member prevents copying until narrowed; any possible structure requires explicit `copy` unless moved.

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

Without annotation, `myCar` is a `MyUnion` known to hold a `MyUnion.Car`: its fields can be read as long as this fact holds, and a `mutable` value can later receive another member.

A member whose definition is a literal (like `Hello = "hello"`) accepts that literal directly; two members of the same union can't accept the same literal. A member whose definition is an object requires the member name. A member whose definition is another type, including a named object type, takes its value in parentheses, and a member without definition carries no data:

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

Members of a named union are distinguished by name: `MyUnion.Car` and `MyUnion.House` stay distinct even if their fields match. A named union is distinct from every other type, even one with the same members, and each member is a type of its own: a `Shape.Circle` is a `Shape`, and it satisfies `{ radius: Float }` (its fields can be read, and it can be passed where a `{ radius: Float }` is expected), but a plain `{ radius: Float }` is not a `Shape.Circle`. A member whose data is not an object is not its data: bind it with `as` to reach it. This makes a single-member union a way to declare a type that is distinct from its content:

```fa
type UserId = Union { User = Integer }

let userId = UserId.User(12)
let raw: Integer = userId -- error: a `UserId` is not an `Integer`
```

Member names are only reachable through the union, as `Shape.Point`: inside the declaration, `String` is still the type `String`, even if a member is called `String`.

A named union can be [generic](/types/generics). Its members are written without the parameters, which are inferred like those of a typed literal; the applied form, like `Result<Integer, String>.Ok(1)`, is only needed when nothing gives them:

```fa
type Result<Value, Failure> = Union {
	Ok = Value
	Failed = Failure
}

let result: Result<Integer, String> = Result.Ok(42)
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

Testing a union with `is` narrows it in the branch where the test succeeded:

```fa
function describe = (value: String | Integer): String => {
	if value is String {
		return "string of length {value.length}" -- `value` is a String here
	}

	"number {value}" -- `value` is an Integer here
}
```

Comparing the discriminant of an anonymous union of objects narrows the union too, as in TypeScript:

```fa
type Shape = { kind: "circle", radius: Float } | { kind: "square", side: Float }

function area = (shape: Shape): Float => {
	if shape.kind == "circle" {
		return 3.14 * shape.radius * shape.radius -- `shape` is the circle member here
	}
	shape.side * shape.side
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
	folders: Array<Folder>
}
```

There is no `Box` type: you never allocate the recursive part yourself. A recursive value is a [structure](/advanced/memory-management#structures): moving its last use transfers its children; keeping two owners requires explicit `copy`. This applies even when every leaf is ordinary data.

The rules of recursive types:

- A type can only refer to itself through its name: recursion goes through a named `type` declaration, and two declarations can refer to each other.
- Two recursive types are the same type when unfolding them gives the same fields forever: `Node` and another type `Link = { value: String, child: Link? }` are the same type.
- A generic type can't refer to itself with other parameters than its own: `type List<Item> = { head: Item, tail: List<Array<Item>>? }` is an error, since every use would need a new type.
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
