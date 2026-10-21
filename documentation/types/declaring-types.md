# Types

In Fa, types can be manipulated like any other value.

A type is declared with the `type` keyword. Type names are capitalized, and fields are declared with `:`.

```fa
type MyType = {
  foo: String
}
```

Unlike a value, a type is **immutable** and cannot be declared with the `mutable` keyword.

```fa
mutable type MyType = { -- error: a type cannot be mutable
  foo: String
}
```

## Generic types

A type can have type parameters, written in parentheses after its name. Like every type, parameters are capitalized:

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

A generic type is used by giving its parameters: `Point(Integer)`, `Pair(String, Float)`.

## Creating values of a type

When the type is known from the context, an object literal is enough:

```fa
let point: Point(Integer) = { x = 12, y = 13 }
```

When there is no context, write the type before the literal:

```fa
let point = Point(Integer) { x = 12, y = 13 }
```

Type parameters are inferred when the fields are enough to deduce them:

```fa
let point = Point { x = 12, y = 13 } -- Point(Integer)
let floatPoint = Point(Float) { x = 12, y = 13 } -- explicit: the literals would be integers otherwise
```

There is one form per situation: writing the type twice, like `let point: Point(Integer) = Point(Integer) { ... }`, is a warning, and the formatter keeps the plain literal.

Collections follow the same rule, with list literals: see [creating collections](/collections/arrays#creating-collections).

