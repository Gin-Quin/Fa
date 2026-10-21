# Type conversion

Fa never converts a value to another kind of value behind your back. There are three situations:

- **no conversion is needed**: the value already satisfies the expected type, because types are [structural](/advanced/structural-typing) constraints,
- **the conversion cannot fail**: it is an explicit call, like `Float(count)`,
- **the conversion can fail**: it is a function that declares its error with `throws`, like `Integer.parse("42")`.

`as` never converts a value: it only gives a name, when [destructuring](/operations/destructuring) or in a `when` branch.

## Structural assignment

Because Fa is [structurally typed](/advanced/structural-typing), a type with more fields satisfies a type with fewer:

```fa
type Human = { name: String }

type SuperHuman = {
	...Human
	powers: String
}

let superHuman = SuperHuman {
	name = "John"
	powers = "unlimited"
}
```

What happens to `powers` depends on whether you copy or reference the value.

**Copying** to a narrower type drops the extra fields:

```fa
let human: Human = superHuman
console.log(human) -- { name = "John" }: `powers` is gone
```

**Referencing** with `use` copies nothing and drops nothing: the value is only *viewed* as a `Human`:

```fa
use human: Human = superHuman
console.log(human.name) -- "John"
console.log(human.powers) -- error: `powers` is not part of Human
```

Function parameters are references, so passing a `SuperHuman` to a function expecting a `Human` never copies and never drops anything:

```fa
function logName = (human: Human) => console.log(human.name)

logName(superHuman) -- no copy, no truncation
```

Two types with the same fields are the same type, so going from one to the other needs nothing at all:

```fa
type Animal = { name: String }

let animal = Animal { name = "Zeus" }
let human: Human = animal -- valid: Human and Animal are the same type
```

## Optionals

A value converts implicitly to its optional, because a `T` satisfies `T | None`:

```fa
let value: Integer? = 12
```

The reverse is not a conversion: an `Integer?` becomes an `Integer` by [narrowing](/values/optionals), or with a fallback:

```fa
function double = (value: Integer?): Integer => {
	if value is none {
		return 0
	}
	value * 2 -- `value` is an Integer here
}

let count = maybeCount ?? 0
```

## Numbers

There are no implicit numeric conversions. Converting between number types is an explicit call to the target type:

```fa
let count: Integer = 3
let ratio: Float = 0.5

let scaled = Float(count) * ratio
let scaled = count * ratio -- error: `Integer` and `Float` cannot be mixed
```

Literals are not conversions: a literal adapts to the expected type.

```fa
let value: Float = 3
let small: Int8 = 100
```

This also holds between integer types of different sizes:

```fa
let small: Int8 = 100
let big: Int64 = small -- error: `Int8` is not `Int64`
let big = Int64(small) -- valid: every Int8 fits in an Int64
```

A conversion call is range-checked at compile time, like any arithmetic result. Converting to a narrower type is valid only when the compiler can prove that the value fits, with a [ranged integer](/values/numbers) or with narrowing:

```fa
let bounded: Integer(min = 0, max = 100) = 100
let small = Int8(bounded) -- valid: the range fits in an Int8

function toByte = (value: Integer): Byte? => {
	if value < 0 or value > 255 {
		return none
	}
	Byte(value) -- `value` is narrowed to Integer(min = 0, max = 255)
}

let small = Int8(readNumber()) -- error: an Integer may not fit in an Int8
```

A ranged integer is an `Integer` with a stronger constraint, so it satisfies `Integer` with no conversion:

```fa
let total: Integer = bounded
```

### Percentages

A `Percentage` is not a `Float`, and is never converted implicitly. The operators between them are explicit:

```fa
let price: Float = 40
let discount = 50%

let reduced = price * discount -- 20.0: Float * Percentage is a Float
let combined = discount + 10% -- 60%: Percentage + Percentage is a Percentage

assert Float(50%) == 0.5
```

To turn a computed value into a percentage, write `(x)%`:

```fa
let share = (part / total)%
```

### Enumerations and fields

[Enumerations](/types/enumerations) and [fields](/types/fields) are stored as unsigned integers. Their integer value is read with an explicit call:

```fa
assert Integer(Color.red) == 0
assert Integer(Permissions { read, write }) == 0b011
```

## Strings

Any value is converted to a string by interpolation, or by calling its `toString` method:

```fa
console.log("Score: {score}")

let text = 42.toString() -- "42"
```

## Fallible conversions

When a conversion can genuinely fail, it is an ordinary function that declares its error with `throws`:

```fa
let parsed = try Integer.parse("42") -- throws ParseError
let color = try Color.parse(userInput) -- throws ParseError
```

Handle them like any other [error](/values/errors): propagate with `try`, or provide a fallback with `catch`:

```fa
type PortError = Errors {
	OutOfRange = { port: Integer }
}

function readPort = (text: String): Integer(min = 1, max = 65535) throws ParseError | PortError => {
	let port = try Integer.parse(text)

	if port < 1 or port > 65535 {
		throw PortError.OutOfRange { port = port }
	}

	port -- narrowed to Integer(min = 1, max = 65535)
}

let port = readPort(text) catch error => 8080
```

## Conversions between collections

Collections convert through `from`, because the cost is never negligible:

```fa
let array = [1, 2, 2, 3]

let set = Set.from(array) -- [1, 2, 3]
let bag = Bag.from(array)
let backToArray = Array.from(set)
```

See [creating collections](/collections/arrays#creating-collections).

## Summary

| Situation | Mechanism | Can fail |
| --- | --- | --- |
| Same fields, different name | none: same type | no |
| More fields to fewer, copied | implicit: extra fields dropped | no |
| More fields to fewer, referenced | implicit: narrowed view with `use` | no |
| `T` to `T?` | implicit | no |
| `T?` to `T` | narrowing with `is none`, or `??` | no |
| Literal to a number type | adapts to the expected type | no |
| Number to another number type, provably fits | explicit call: `Int64(x)`, `Float(x)` | no |
| Number to a narrower type, not provable | narrow first, then call | compile error otherwise |
| `Percentage` to `Float` | explicit call: `Float(50%)` | no |
| Value to string | interpolation or `.toString()` | no |
| String to number or enumeration | `Integer.parse`, `Color.parse` | yes: `throws ParseError` |
| Between collections | `from` | no |
