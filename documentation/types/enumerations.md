# Enumerations

An enumeration is a set of named values, of which a value holds exactly one. It is declared with the `Enum` type constructor:

```fa
type Color = Enum { red, green, blue }

let myColor = Color.red
let favoriteColor: Color = .red
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

`parse` throws an error rather than returning an optional, because a bad string is a failure and not an absence. See [errors](/values/errors) and [type conversion](/types/type-conversion).

## Enumerations versus other types

- Need **one** value from a set? Use an `Enum`.
- Need **any combination** of a set? Use [`Fields`](/types/fields).
- Need members that carry **different data**? Use a [`Union`](/types/unions).
- Need names that are really **strings** at runtime, for a wire format or an API? Use a union of string literals instead:

```fa
type Direction = "north" | "south" | "east" | "west"
```
