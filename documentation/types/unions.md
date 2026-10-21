# Unions

A union is a type whose values are one of several alternatives.

## Anonymous unions

The `|` operator builds a union out of existing types:

```fa
type Value = String | Int

type Direction = "north" | "south" | "east" | "west"

type MaybeUser = User | None
```

Anonymous unions are structural: `String | Int` and `Int | String` are the same type, and a `String` is assignable to both.

## Named unions

When the alternatives need names or carry different data, declare the union with the `Union` constructor:

```fa
type MyUnion = Union {
  Hello = "hello"

  World = "world"

  Car = {
    brand: String
    model: String
    year: Int
  }

  House = {
    address: String
    price: Int
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

## Matching a union

Use `when` to branch on which member a value holds:

```fa
when myCar is {
  Car => console.log("It's a car")
  House => console.log("It's a house")
  Hello => console.log("It's hello")
  World => console.log("It's world")
}
```

Inside a `when` on a named union, members can be written unqualified. Qualify them when the union is not obvious from context:

```fa
when value is {
  MyUnion.Car => console.log("It's a car")
  Direction => console.log("It's a direction")
}
```

### Extracting the payload

Use `>>` to bind the member's data:

```fa
let description = when shape is {
  Circle >> circle => "circle of radius {circle.radius}"
  Rect >> { width, height } => "rect {width}x{height}"
}
```

### Exhaustiveness

`when` must cover every member of the union. A missing member is a compile error, not a silent fallthrough:

```fa
when shape is {
  Circle => "circle"
  -- error: `Rect` is not handled
}
```

Add an `else` branch to handle the rest:

```fa
when shape is {
  Circle => "circle"
  else => "something else"
}
```

:::tip
Prefer listing every member over writing `else`. When you later add a member to the union, the compiler will point you at every `when` that needs updating — which is the main reason to use a named union in the first place.
:::

## Narrowing

Testing a union narrows it for the rest of the branch:

```fa
function describe = (value: String | Int): String => {
  if value is String {
    return "string of length {value.length}" -- `value` is a String here
  }

  return "number {value}" -- `value` is an Int here
}
```

## Unions and recursion

An object cannot contain itself, because objects live on the stack and would have infinite size. A union can, because only one member exists at a time and the compiler can size it:

```fa
type Node = Union {
  Text = { value: String }
  Parent = { child: Node }
}
```

For structures that genuinely need to share nodes, use a collection and [relations](../advanced/relations) instead.

## Unions of errors

An error type is a union whose members are errors. Declare it with `Errors` rather than `Union`, so that its members become falsy and work with the `!` operator:

```fa
type ApiError = Errors {
  Timeout = { after: Int }
  Network = { message: String }
}
```

See [errors](../values/errors).
