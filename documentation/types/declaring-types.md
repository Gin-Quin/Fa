# Declaring types

In Fa, types can be named, passed to [generic](/types/generics) types and functions, and computed by [type functions](/types/type-functions). Unlike values, types exist at compile time only.

A type is declared with the `type` keyword, following the same shape as every other declaration. Fields are declared with `:`, followed by their type:

```fa
type MyType = {
	foo: String
}
```

A type is always immutable:

```fa
mutable type MyType = { -- error: a type cannot be mutable
	foo: String
}
```

## What a type is

A type is a **constraint on values**. `Integer` constrains a value to be a 64-bit integer; `Integer(min = 0)` constrains it further; `"hello"` constrains it to exactly one string.

Writing `value: Type` means that `value` **satisfies** `Type`. This is why literals can be used as types:

```fa
type Greeting = "hello"
type Answer = 42

let greeting: Greeting = "hello"
let greeting: Greeting = "goodbye" -- error: "goodbye" doesn't satisfy `Greeting`
```

And why unions of literals are useful:

```fa
type Direction = "north" | "south" | "east" | "west"
```

## Type constructors

Beyond object types, Fa provides four built-in type constructors for building types out of members. They are used with the `type` keyword, like any other type:

| Constructor | Use for | Page |
| --- | --- | --- |
| `Enum` | one value out of a named set | [enumerations](/types/enumerations) |
| `Fields` | any combination of a named set | [fields](/types/fields) |
| `Union` | one of several differently-shaped values | [unions](/types/unions) |
| `Errors` | a union whose members are errors | [errors](/values/errors) |

```fa
type Color = Enum { red, green, blue }

type Permissions = Fields { read, write, admin }

type Shape = Union {
	Circle = { radius: Float }
	Rectangle = { width: Float, height: Float }
}

type ApiError = Errors {
	Timeout = { after: Integer }
	Network = { message: String }
}
```

## Anonymous types

A type does not have to be named. Anywhere a type is expected, you can write one inline:

```fa
function move = (point: { x: Integer, y: Integer }) => ...

function onMessage = (handler: (message: String) => Void) => ...
```

In a function type, `Void` is the return type of a function that returns nothing.

## Type aliases

Naming an existing type is just a declaration:

```fa
type UserId = Integer
type Handler = (request: Request) => Response
```

Fa is [structurally typed](/advanced/structural-typing), so `UserId` and `Integer` are the *same* type, not two compatible ones:

```fa
let userId: UserId = 12
let count: Integer = userId -- valid: UserId is Integer
```

If you need two distinct types with the same shape, add a distinguishing field, or use a single-member union:

```fa
type UserId = { user: Integer }
type ProductId = { product: Integer }

type OrderId = Union { Order = Integer }
```

## Generic types

A type can have generic parameters, written in parentheses right after its name. A capitalized parameter receives a type:

```fa
type Point(NumberType) = {
	x: NumberType
	y: NumberType
}

type Pair(Left, Right) = {
	left: Left
	right: Right
}
```

A generic type is used by giving its parameters: `Point(Integer)`, `Pair(String, Float)`. Giving parameters to a generic type produces a type, so a generic type is a [type function](/types/type-functions):

```fa
type Coordinate = Pair(Float, Float)
```

Parameters can be constrained, have defaults, or receive compile-time values. See [generics](/types/generics).

## Creating values of a type

When the type is known from the context, an object literal is enough:

```fa
let point: Point(Integer) = { x = 12, y = 13 }
```

When there is no context, write the type before the literal:

```fa
let point = Point(Integer) { x = 12, y = 13 }
```

Generic parameters are inferred when the fields are enough to deduce them:

```fa
let point = Point { x = 12, y = 13 } -- Point(Integer)
let floatPoint = Point(Float) { x = 12, y = 13 } -- explicit: the literals would be integers otherwise
```

Values are always created with braces: there is no positional constructor like `Point(12, 13)`, and no call like `Point(x = 12, y = 13)`.

There is one form per situation: writing the type twice, like `let point: Point(Integer) = Point(Integer) { ... }`, is a warning, and the formatter keeps the plain literal.

Collections follow the same rule, with list literals: see [creating collections](/collections/arrays#creating-collections).

## Naming

By convention, types are written in `PascalCase` and values in `camelCase`.

Capitalization carries a meaning in one place only: in a [generic parameter list](/types/generics#type-parameters-and-compile-time-value-parameters), a capitalized parameter receives a type and a lowercase parameter receives a compile-time value. Everywhere else, it is a convention: whether a declaration produces a type is decided by the `type` keyword.

```fa
type main = { foo: String } -- valid, but do not do this
```
