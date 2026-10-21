# Enumerations

An enumeration is a set of named values, of which a value holds exactly one.

```fa
type Color = Enum { red, green, blue }

let myColor = Color.red
let myColor: Color = .red
```

When the expected type is known, the enumeration name can be left out and the member written as `.red`.

## Under the hood

Enumeration members are stored as unsigned integers, using the smallest integer type that fits. An enumeration is, in effect, a union of integers with names attached.

You can pin the underlying values:

```fa
type Status = Enum {
  pending = 0
  active = 1
  archived = 9
}
```

## Matching

```fa
let label = when myColor is {
  red => "warm"
  green => "cool"
  blue => "cool"
}
```

`when` on an enumeration is exhaustive: every member must be covered, or an `else` branch provided.

## Iterating and converting

```fa
for Color.members >> color {
  console.log(Color.toString(color))
}

assert(Color.toString(Color.red) == "red")
assert(Color.fromString("red") == Color.red)  -- Color | ParseError
assert(Color.red.toInt() == 0)
```

`fromString` returns an error rather than an optional, because a bad string is a failure and not an absence.

## Enumerations versus other types

- Need **one** value from a set? Use an `Enum`.
- Need **any combination** of a set? Use [`Fields`](./fields).
- Need members that carry **different data**? Use a [`Union`](./unions).
- Need names that are really **strings** at runtime, for a wire format or an API? Use a union of string literals instead:

```fa
type Direction = "north" | "south" | "east" | "west"
```
