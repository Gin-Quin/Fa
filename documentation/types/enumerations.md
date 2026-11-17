# Enumerations

An enumeration is a set of named values, of which a value holds exactly one. It is declared with the `Enum` type constructor:

```fa
type Color = Enum { red, green, blue }

let myColor = Color.red
let favoriteColor: Color = .red
```

When the expected type is known, the enumeration name can be left out and the member written as `.red`.

An enumeration is a type of its own, distinct from every other one even with the same members: a `Color` is never a `Light`, even if both are `Enum { red, green, blue }`. A member is also a type with a single value, like a literal: `Color.red` is the type of the value `Color.red`, which a test narrows to.

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

Pinned values are distinct, non-negative integer literals. Without them, members are numbered from 0 in declaration order. Either all members are pinned, or none is.

Members are compared with `==` and `!=` only. An enumeration is not ordered and has no arithmetic: `Color.red < Color.blue` is an error. When the order of the underlying values matters, compare them explicitly:

```fa
let isBefore = Integer(Status.pending) < Integer(Status.archived) -- true
```

## Matching

```fa
let label = when myColor is {
	Color.red => "warm"
	Color.green => "cool"
	Color.blue => "cool"
}
```

`when` on an enumeration is exhaustive: every member must be covered, or an `else` branch provided.

Several members can share a branch:

```fa
let temperature = when myColor is {
	Color.red => "warm"
	Color.green | Color.blue => "cool"
}
```

## Iterating and converting

`Color.members` lists the members in declaration order:

```fa
for color in Color.members {
	console.log(color.toString())
}

assert Color.red.toString() == "red"
assert Integer(Color.red) == 0
```

Converting a string to an enumeration can fail, so `Color.parse` declares its error with `throws`:

```fa
function readColor = (input: String): Color throws ParseError => try Color.parse(input)

let color = Color.parse(userInput) catch error => Color.red
```

`parse` matches the name of a member exactly, case included: `Color.parse("Red")` throws. It throws an error rather than returning an optional, because a bad string is a failure and not an absence. See [errors](/values/errors) and [type conversion](/types/type-conversion).

## Enumerations versus other types

- Need **one** value from a set? Use an `Enum`.
- Need **any combination** of a set? Use [`Fields`](/types/fields).
- Need members that carry **different data**? Use a [`Union`](/types/unions).
- Need names that are really **strings** at runtime, for a wire format or an API? Use a union of string literals instead:

```fa
type Direction = "north" | "south" | "east" | "west"
```

:::implementation
## Implementation details

### Native and WebAssembly

- A member is a declaration index, stored as an `i32`. Names and iteration use
  that index, independently of any pinned integer values.
- An explicit integer conversion reads a private constant table of pinned
  values. For an unpinned enumeration, it widens the unsigned declaration index
  to the destination integer representation. Each compiled unit can emit its
  own private table from the shared nominal type metadata.

### JavaScript

- A member is a JavaScript number, its underlying value. `toString` and `parse` use a table of the member names generated per enumeration, and `members` is a constant array of the values in declaration order.
- Equality compares these primitives directly with `===` (or `!==`), including optional enumerations, where `none` is `undefined`; no structural equality helper is needed.
:::
