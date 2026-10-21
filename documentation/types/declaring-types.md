# Declaring types

In Fa, a type is a value like any other: you can name it, pass it to a function, return it, and compute it. The only difference is that types exist at compile time only.

A type is declared with the `type` keyword, following the same shape as every other declaration:

```fa
type MyType = {
  foo: String
}
```

A type is always immutable. `mutable type` is not valid.

## What a type is

A type is a **constraint on values**. `Int` constrains a value to be a 64-bit integer; `Int(min = 0)` constrains it further; `"hello"` constrains it to exactly one string.

This is why literals can be used as types:

```fa
type Greeting = "hello"
type Answer = 42

let greeting: Greeting = "hello"
let greeting: Greeting = "goodbye" -- error
```

And why unions of literals are useful:

```fa
type Direction = "north" | "south" | "east" | "west"
```

## Type constructors

Beyond object types, Fa provides four built-in constructors for building types out of members. All four use the `type` keyword:

| Constructor | Use for | Page |
| --- | --- | --- |
| `Enum` | one value out of a named set | [enumerations](./enumerations) |
| `Fields` | any combination of a named set | [fields](./fields) |
| `Union` | one of several differently-shaped values | [unions](./unions) |
| `Errors` | a union whose members are errors | [errors](../values/errors) |

```fa
type Color = Enum { red, green, blue }

type Permissions = Fields { read, write, admin }

type Shape = Union {
  Circle = { radius: Decimal }
  Rect = { width: Decimal, height: Decimal }
}

type ApiError = Errors {
  Timeout = { after: Int }
  Network = { message: String }
}
```

## Anonymous types

A type does not have to be named. Anywhere a type is expected, you can write one inline:

```fa
function move = (point: { x: Int, y: Int }) => ...

let handler: (String) => None = (message: String) => console.log(message)
```

## Type aliases

Because a type is a value, naming an existing type is just a declaration:

```fa
type Byte = UInt8
type UserId = Int
type Handler = (Request) => Response
```

Fa is [structurally typed](../advanced/structural-typing), so `UserId` and `Int` are the *same* type, not two compatible ones. If you need two distinct types with the same shape, add a distinguishing field or use a single-member union.

## Types are values

Types can be stored, passed and returned like anything else, as long as everything happens at compile time:

```fa
type Pair = (First: Type, Second: Type) => {
  first: First
  second: Second
}

type Coordinate = Pair(Decimal, Decimal)
```

A function from types to types is called a [type function](./type-functions). Type functions are how generics work in Fa — see [generics](./generics).

## Naming

By convention, types are written in `PascalCase` and values in `camelCase`. This is a convention only: nothing in the language reads the capitalization of a name. Whether a declaration produces a type is decided by the `type` keyword, and whether a parameter accepts a type is decided by its `Type` annotation.

```fa
type main = { foo: String } -- valid, but do not do this
```
