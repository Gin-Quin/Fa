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

A type is a **constraint on values**. `Integer` constrains a value to be a 64-bit integer; `Integer<min = 0>` constrains it further; `"hello"` constrains it to exactly one string.

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

A type with a single value, like `"hello"` or `42`, is a **literal type**.

## Inferred types and known values

Without an annotation, a value gets the **base type** of its literal, not the literal type:

```fa
let answer = 42 -- Integer
let status = "ready" -- String
let numbers = [1, 2] -- Array<Integer>
```

[`Type<answer>`](/types/type-functions#typevalue) and the inference of [generic parameters](/types/generics#inference) use this type. An annotated value has the annotated type, and a [static](/advanced/metaprogramming#static-values) value keeps its literal type.

Beyond the type, the compiler knows **facts** about each value: its exact literal, its integer range, or the member of a union it holds after a test. These facts make an assignment valid, prove an index, make a `when` exhaustive and keep references precise. They last until the value is mutated:

```fa
let five = 5
let small: Integer<min = 0, max = 10> = five -- valid: `five` is known to be 5

mutable count = 5
count += readNumber()
let other: Integer<min = 0, max = 10> = count -- error: `count` was mutated, it is only known to be an Integer
```

Without an expected type, an `if` or a `when` has the union of the base types of its branches. Mixing `Integer` and `Float` literals is an error, because nothing tells which one is meant:

```fa
let label = if ready { "yes" } else { 0 } -- String | Integer
let ratio = if ready { 1 } else { 0.5 } -- error: write `1.0`, or annotate: `let ratio: Float = ...`
```

See [numbers](/values/numbers) for how ranges are computed, and [narrowing](/control-flow/conditionals#narrowing) for the tests that add facts.

## Type constructors

Beyond object types, Fa provides five built-in type constructors: four build types out of members, and `CStruct` gives a type the memory layout of a C `struct`. They are used with the `type` keyword, like any other type:

| Constructor | Use for | Page |
| --- | --- | --- |
| `Enum` | one value out of a named set | [enumerations](/types/enumerations) |
| `Fields` | any combination of a named set | [fields](/types/fields) |
| `Union` | one of several differently-shaped values | [unions](/types/unions) |
| `Errors` | a union whose members are errors | [errors](/values/errors) |
| `CStruct` | an object with the memory layout of C, shared with C code or a GPU | [foreign code](/advanced/foreign-code#cstruct) |

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

A member of a `Union` or an `Errors` type written without `=` carries no data, like `Cancelled` in `Errors { Timeout = { after: Integer }, Cancelled }`.

## Anonymous types

A type does not have to be named. Anywhere a type is expected, you can write one inline:

```fa
function move = (point: { x: Integer, y: Integer }) => ...

function onMessage = (handler: (message: String) => Void) => ...
```

In a function type, `Void` is the return type of a function that returns nothing. See [special types](/types/operations#special-types).

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

Object types with the same public data field names and types are structurally compatible; methods and defaults do not affect that comparison. Inferred unions still preserve distinct written alternatives when their methods or destructors differ, and dispatch to the actual alternative. See [what is compared](/advanced/structural-typing#what-is-compared).

If you need two distinct types with the same shape, add a distinguishing field, or use a single-member union:

```fa
type UserId = { user: Integer }
type ProductId = { product: Integer }

type OrderId = Union { Order = Integer }

let orderId = OrderId.Order(12)
```

## Generic types

A type can have generic parameters, written between angle brackets right after its name. A capitalized parameter receives a type:

```fa
type Point<NumberType> = {
	x: NumberType
	y: NumberType
}

type Pair<Left, Right> = {
	left: Left
	right: Right
}
```

A generic type is used by giving its parameters: `Point<Integer>`, `Pair<String, Float>`. Giving parameters to a generic type produces a type, so a generic type is a [type function](/types/type-functions):

```fa
type Coordinate = Pair<Float, Float>
```

Parameters can be constrained, have defaults, or receive compile-time values. See [generics](/types/generics).

In an annotation, a generic type is written with its parameters, except those that have a default:

```fa
let point: Point<Integer> = { x = 12, y = 13 }
let other: Point = { x = 12, y = 13 } -- error: `Point` needs its parameter `NumberType`
```

## Creating values of a type

When the type is known from the context, an object literal is enough:

```fa
let point: Point<Integer> = { x = 12, y = 13 }
```

When there is no context, write the type before the literal:

```fa
let point = Point<Integer> { x = 12, y = 13 }
```

Generic parameters are inferred when the fields are enough to deduce them:

```fa
let point = Point { x = 12, y = 13 } -- Point<Integer>
let floatPoint = Point<Float> { x = 12, y = 13 } -- explicit: the literals would be integers otherwise
let mixed = Point { x = 12, y = 1.5 } -- error: `NumberType` is Integer for `x` and Float for `y`
```

Values are always created with braces: there is no positional constructor like `Point(12, 13)`, and no call like `Point(x = 12, y = 13)`. The one exception is a member of a [named union](/types/unions#named-unions) whose data is not an object: it takes its value in parentheses, like `OrderId.Order(12)`.

There is one form per situation: writing the type twice, like `let point: Point<Integer> = Point<Integer> { ... }`, is a warning, and the formatter keeps the plain literal.

Collections follow the same rule, with list literals: see [creating collections](/collections/arrays#creating-collections).

### Types inside a type

A type can declare other types in its body, for example `type Database = { type OpenError = Errors { Failed } }`. Inside the body, use `OpenError` or `Self.OpenError`; outside, use `Database.OpenError`, including through another file (`library.Database.OpenError`). This works for annotations, literals, members, patterns, static functions and enumeration `members`. `Error` is allowed as a nested name (`Farm.Error`); a bare `Error` still names the prelude's type. A value prints with the nested type's own name, such as `Error.Failed`.

## Naming

By convention, types are written in `PascalCase` and values in `camelCase`.

Capitalization carries a meaning in one place only: in a [generic parameter list](/types/generics#type-parameters-and-compile-time-value-parameters), a capitalized parameter receives a type and a lowercase parameter receives a compile-time value. Everywhere else, it is a convention: whether a declaration produces a type is decided by the `type` keyword.

```fa
type main = { foo: String } -- valid, but do not do this
```
