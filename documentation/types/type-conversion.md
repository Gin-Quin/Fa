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

An object literal can't have extra fields: `let point: Point = { x = 12, y = 13, z = 14 }` is an error. See [object literals](/advanced/structural-typing#object-literals-have-exactly-the-expected-fields).

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

Types with the same fields are structurally compatible, so assignment needs no explicit type conversion. Written alternatives with different methods or destructors still retain their identity at a union join:

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

There are no implicit conversions between kinds of numbers (`Integer`, `Float`, `BigInt`, `Percentage`). Converting from one kind to another is an explicit call to the target type:

```fa
let count: Integer = 3
let ratio: Float = 0.5

let scaled = Float(count) * ratio
let scaled = count * ratio -- error: `Integer` and `Float` cannot be mixed
```

The two float types follow the same rule, with one exception: a `Float32` is accepted where a `Float` is expected, since every `Float32` is exactly a `Float`. But arithmetic, comparisons and equality never mix them, because a `Float32` is rounded after every operation: convert one side with `Float(x)` or `Float32(x)`. Going from `Float` to `Float32` is always the explicit `Float32(x)`, which rounds and never fails.

```fa
let single: Float32 = 0.5
let double: Float = single -- valid: exact widening
let sum = single + double -- error: `Float32` and `Float` can't be mixed, write `Float(single) + double`
```

Literals are not conversions: a literal adapts to the expected type.

```fa
let value: Float = 3
let small: Int8 = 100
```

### Integers compare by range

Every integer type is a range of integers with a storage width: an `Int8` is an `Integer<min = -128, max = 127>` stored in 8 bits. So an integer is accepted, with no conversion, wherever its range fits the expected range:

```fa
let small: Int8 = 100
let big: Int64 = small -- valid: every Int8 fits in an Int64

let bounded: Integer<min = 0, max = 100> = 100
let byte: Byte = bounded -- valid: 0 to 100 fits in a Byte
let total: Integer = bounded -- valid: a ranged integer is an Integer
```

Going to a smaller range is a conversion call. It can't fail when the compiler proves that the value fits, with a [ranged integer](/values/numbers) or with narrowing, and it `throws ArithmeticError` otherwise:

```fa
function toByte = (value: Integer): Byte? => {
	if value < 0 or value > 255 {
		return none
	}
	value -- valid: `value` is narrowed to Integer<min = 0, max = 255>
}

let small = try Int8(readNumber()) -- throws ArithmeticError if the Integer doesn't fit in an Int8
let byte: Byte = readNumber() -- error: an Integer may not fit in a Byte, convert it with `Byte(...)`
```

A `BigInt` converts the same way: `Integer(big)` and `Int8(big)` throw `ArithmeticError` unless the compiler proves that the value fits.

`Integer` has the range and the storage of `Int64`, and the two convert implicitly as values. They are still two types: on JavaScript, an `Integer` is a number and an `Int64` a `BigInt`, so a collection of one is not a collection of the other. Convert the elements explicitly:

```fa
function total = (values: Array<Integer>): Integer => values.sum()

let counters: Array<Int64> = [1, 2, 3]
total(counters) -- error: an `Array<Int64>` is not an `Array<Integer>`
total(counters.map((value) => Integer(value))) -- valid
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

The integer type must hold every value the enumeration or the set may have: `UInt8(status)` is an error when a member of `status`'s type is pinned to `70000`, and the message says to convert to a wider type, like `Integer`. A narrowed value only needs room for its possible members: `UInt8(value)` is valid with `value: Status.active`.

## Strings

Any value is converted to a string by interpolation, or by calling its `toString` method. There is no `String(value)`: these are the two ways.

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

function readPort = (text: String): Integer<min = 1, max = 65535> throws ParseError | PortError => {
	let port = try Integer.parse(text)

	if port < 1 or port > 65535 {
		throw PortError.OutOfRange { port = port }
	}

	port -- narrowed to Integer<min = 1, max = 65535>
}

let port = readPort(text) catch error => 8080
```

## Conversions between collections

Collections convert through `from`, because the cost is never negligible:

```fa
let array = [1, 2, 2, 3]

let set = Set.from(array) -- [1, 2, 3]
let pool = Pool.from(array)
let backToArray = Array.from(set)
```

See [creating collections](/collections/arrays#creating-collections).

## Summary

| Situation | Mechanism | Can fail |
| --- | --- | --- |
| Same fields, different name | none: structurally compatible; written union behavior is preserved | no |
| More fields to fewer, stored | extra fields dropped; structures move or require explicit `copy` | no |
| More fields to fewer, referenced | implicit: narrowed view with `use` | no |
| Object literal with an extra field | none: compile error | compile error |
| Collection of supersets to a parameter | implicit, if the parameter is not `mutates`; otherwise a generic function. Never for `Set` elements and `Map` keys | compile error if `mutates` |
| `T` to `T?` | implicit | no |
| `T?` to `T` | narrowing with `is none`, or `??` | no |
| Literal to a number type | adapts to the expected type | no |
| Integer to an integer type whose range fits | none: ranges compare | no |
| Integer to a smaller range, not proven | explicit call: `Int8(x)` | yes: `throws ArithmeticError` |
| Between kinds of numbers | explicit call: `Float(x)` | no, except `Integer(bigInt)`: `throws ArithmeticError` |
| `Float32` to `Float` | implicit | no |
| `Float` to `Float32` | explicit call: `Float32(x)` | no: rounds |
| `Int64` and `Integer` | implicit, for values; `map` for collections | no |
| `Percentage` to `Float` | explicit call: `Float(50%)` | no |
| Value to string | interpolation or `.toString()` | no |
| String to number or enumeration | `Integer.parse`, `Color.parse` | yes: `throws ParseError` |
| Between collections | `from` | no |
