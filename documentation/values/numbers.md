# Numbers

Fa has four kinds of numbers: **integers**, **floats**, **big integers** and **percentages**.

Most of the time, you only need two types:

- `Integer`: a 64-bit signed integer. It's the default type of integer literals like `12`.
- `Float`: a 64-bit floating-point number. It's the default type of literals like `12.5`.

```fa
let count = 12 -- Integer
let ratio = 0.75 -- Float

let explicitCount: Integer = 12
let explicitRatio: Float = 3 -- a literal adapts to its expected type
```

:::caution
There is no general `Number` type in Fa. Every numeric value has a concrete type, and arithmetic between different kinds of numbers (integers, floats, big integers, percentages) requires an explicit conversion. Integer types of different sizes are not different kinds: they are [ranges](#integer-types-are-ranges) of the same integers. See [conversions](#conversions).
:::

## Literals

Integer literals can be written in decimal, binary (`0b`), octal (`0o`) or hexadecimal (`0x`):

```fa
let decimal = 42
let binary = 0b101010
let octal = 0o52
let hexadecimal = 0x2A
```

Underscores can be used as separators to make long literals readable. They have no effect on the value:

```fa
let population = 8_100_000_000
let mask = 0b1111_0000
let color = 0xFF_88_00
```

A float literal has a `.` with digits on both sides, an exponent, or both. A literal with an exponent is always a float:

```fa
let half = 0.5
let large = 1.5e9
let tiny = 2.5e-3
let million = 1e6 -- a Float
```

The rules of number literals:

- `.5` and `5.` are errors: write `0.5` and `5.0`. (`42.toString()` is a method call on `42`.)
- An underscore is only allowed between two digits: `_1`, `1_` and `1__0` are errors.
- The `n` suffix makes a [big integer](#big-integers), on any integer literal: `12n`, `0xFFn`. The `%` suffix makes a [percentage](#percentages), on any integer or float literal: `50%`, `12.5%`.
- A minus sign written before a literal is part of the constant: `let lowest: Int8 = -128` is valid, even though `128` doesn't fit in an `Int8`.

A literal **adapts to its expected type**: an integer literal to any integer type whose range contains it, to a float type or to `BigInt`; a float literal to `Float32` or `Float64`. In an arithmetic operation, a literal operand adapts to the type of the other operand (`ratio * 2` is a `Float`). A literal never becomes a `Percentage` without its `%` suffix. Without an expected type, an integer literal is an `Integer` and a float literal is a `Float`.

A literal that doesn't fit its type is a compile error:

```fa
let small: UInt8 = 300 -- error: 300 doesn't fit in a UInt8
let huge = 10_000_000_000_000_000_000 -- error: doesn't fit in an Integer. Use `10_000_000_000_000_000_000n`.
```

## Integers

`Integer` is an alias for `Int64`. When you need a specific size, use one of the sized integer types:

- `Int8`, `Int16`, `Int32`, `Int64`, `Int128`: signed integers,
- `UInt8`, `UInt16`, `UInt32`, `UInt64`, `UInt128`: unsigned integers.

```fa
let pixel: UInt8 = 255
let fileSize: UInt64 = 1_000_000
```

The `Byte` type is an alias for `UInt8`:

```fa
type Byte = UInt8
```

### Integer types are ranges

Every integer type is a **range of integers** with a storage width:

| Type | Range | Storage |
| --- | --- | --- |
| `Int8` | `-128..=127` | 8 bits |
| `Int16` | `-32_768..=32_767` | 16 bits |
| `Int32` | `-2_147_483_648..=2_147_483_647` | 32 bits |
| `Int64`, `Integer` | −2^63 to 2^63 − 1 | 64 bits |
| `Int128` | −2^127 to 2^127 − 1 | 128 bits |
| `UInt8`, `Byte` | `0..=255` | 8 bits |
| `UInt16` | `0..=65_535` | 16 bits |
| `UInt32` | `0..=4_294_967_295` | 32 bits |
| `UInt64` | 0 to 2^64 − 1 | 64 bits |
| `UInt128` | 0 to 2^128 − 1 | 128 bits |
| `Integer(min = a, max = b)` | `a..=b` | 64 bits |
| `Int8(min = a, max = b)`, ... | `a..=b`, within the range of the type | the width of the type |

An `Int8` is an `Integer(min = -128, max = 127)` stored in 8 bits. So integer types compare by range:

- An integer value is accepted wherever its range fits the expected range, with no conversion: an `Int8` is accepted as an `Int16`, an `Integer(min = 0, max = 100)` as a `UInt8`.
- Going to a smaller range needs a conversion call, like `Int8(value)`. It can't fail when the compiler proves that the value fits, and `throws ArithmeticError` otherwise. A [narrowing](/control-flow/conditionals#narrowing) is also a proof.
- Conversions between kinds (`Integer`, `Float`, `BigInt`, `Percentage`) are always explicit.

```fa
let a: Int8 = readInt8()
let b: Int8 = readInt8()

let wide: Int16 = a + b -- valid: the result is between -256 and 254
let byte: UInt8 = a -- error: `a` may be negative
let converted = UInt8(a) catch error => 0 -- throws ArithmeticError when `a` is negative

if a >= 0 {
	let positive: UInt8 = a -- valid: here, `a` is between 0 and 127
}
```

### Minimum and maximum values

You can statically indicate a minimum and a maximum value. `min` and `max` are compile-time value parameters of the integer types (see [generics](/types/generics)):

```fa
let myPositiveInteger: Integer(min = 0) = 12
let mySmallInteger: Integer(min = 0, max = 200) = 200
```

Indicating a minimum or a maximum value restricts the range of the integer at compile time, which means you won't be able to assign values outside of the specified range.

To declare a non-negative integer, prefer `Integer(min = 0)` over an unsigned type: unsigned types are meant for binary data and fixed-size formats.

Some examples:

```fa
mutable value: Integer(min = 20) = 20

-- compiler error: 10 is less than the minimum value of 20
value = 10
```

The compiler propagates the range through your program, which lets it reject impossible code and remove redundant checks:

```fa
let value: Integer(min = 20) = get20OrMore()

-- compiler warning: the condition is always false
if value < 20 {
	console.log("Value is less than 20")
}
```

```fa
let value: Integer(min = 4) = get4OrMore()

let array = [1, 2, 3]

-- compiler error: the index is out of bounds
let item = array[value]
```

Assigning a wider range into a narrower one is rejected until you narrow it:

```fa
mutable foo: Integer(max = 100) = getIntegerLowerThan(100)
mutable bar: Integer(max = 200) = getIntegerLowerThan(200)

-- compiler error: bar may exceed the maximum value of 100
foo = bar

-- you first have to check that bar is lower than 100
foo = if bar <= 100 { bar } else { 100 }
```

Ranged integers are the mechanism behind Fa's [smart bound checks](/collections/arrays#smart-bound-checks) and its [division by zero guarantees](/operations/arithmetic#division-by-zero).

### Ranges through arithmetic

Ranges propagate through arithmetic: the result of an operation has a range computed from the ranges of its operands.

```fa
let a: Integer(min = 0, max = 10) = getSmallNumber()
let b: Integer(min = 1, max = 5) = getTinyNumber()

let sum = a + b -- an Integer, known to be between 1 and 15
let product = a * b -- an Integer, known to be between 0 and 50
```

The range of a result is checked against the type it goes to:

- When the result is assigned to an annotated value, or passed where a type is expected, its range is checked against that type, as described in [overflow](#overflow).
- An unannotated value gets the **base type** of its operands: `let s = x + y` with two `Int8` operands is an `Int8`, checked at runtime when the result range doesn't fit. The base type of a ranged integer is the type it refines (`Integer` for `Integer(min = 0)`). When the operands have different base types, the result gets the one whose range contains the other (`Int8 + Int32` is an `Int32`); when neither does (`Int8 + UInt8`), annotate the result.
- The value also keeps the computed range as a fact (see [inferred types](/values/declaring-values#inferred-types)), as long as it isn't mutated.

### Range analysis

To decide whether an index is in bounds, a divisor is non-zero or an operation can overflow, the compiler uses a **fixed set of rules**. Every version of the compiler, on every target, accepts exactly the programs these rules accept. An index or a divisor that these rules can't prove needs `try` or `catch`; an overflow they can't exclude is checked at runtime.

1. **One interval per integer place.** It comes from the type of the place, from the value it was initialized with or assigned (a literal is an exact value) and from interval arithmetic on the operators.
2. **Conditions refine the interval** in the branches of `if`, `while` and `when`: a comparison with a constant or with a place whose interval is known, an `is` with a range, and `==` or `!=` with a literal. An excluded value (`if divisor != 0`) is kept as such.
3. **Lengths.** The length of a collection is known from its literal (`[1, 2, 3]` has 3 items) until the collection is mutated. These relational facts are kept:
	- `index < collection.length` inside `for index in 0..collection.length` and `for item, index in collection`,
	- `index < collection.length` after a guard `if index < collection.length` (or in the `else` of its negation); the guarded expression may be a place or a place plus or minus a constant (`index + 1 < items.length`),
	- equal lengths for the collections created from the same length: a copy, or `map` over a collection.
4. **Loops** are analysed until the facts stop changing. A bound that keeps growing from one iteration to the next becomes unknown: after `mutable count = 0` and a loop that runs `count += 1`, `count` is only known to be at least 0.
5. **Mutations drop facts.** An assignment replaces the facts of a place with those of the new value; any other mutation of a place drops its facts. A mutation that may change the length of a collection drops the facts about its length.

Nothing else is used: no reasoning on the body of a called function (only its signature, including the ranges of its return type), and no other relation between places.

```fa
let scores = [10, 20, 30]

for index in 0..scores.length {
	console.log(scores[index]) -- proven by the loop
}

function at = (items: Array(Integer), index: Integer): Integer? => {
	if index >= 0 and index < items.length {
		return items[index] -- proven by the guard
	}
	none
}
```

### Overflow

The compiler tracks the range of every result. A certain overflow is a compile error, an impossible one generates no check, and a possible one is checked at runtime: the program [stops](/values/errors#stops) instead of producing a wrong result.

```fa
let a: Int8 = readInt8()
let b: Int8 = readInt8()

let c: Int16 = a + b -- no check: `a + b` is between -256 and 254
let d: Int8 = a + b  -- checked at runtime: stops the program if the result doesn't fit
let e: Int8 = 100 + 100 -- compiler error: 200 doesn't fit in an Int8
```

When the result may genuinely overflow, choose the behavior you want explicitly:

```fa
let wrapped = a.wrappingAdd(b) -- wraps around
let saturated = a.saturatingAdd(b) -- clamps to the bounds of the type
let checked = a.checkedAdd(b) catch error => 0 -- throws ArithmeticError on overflow
```

The same variants exist for `Sub`, `Mul` and `Pow` (`wrappingSub`, `saturatingMul`, `checkedPow`, ...). See [arithmetic](/operations/arithmetic#overflow).

### Compatibility with JavaScript

On JavaScript targets, an `Integer` is a JavaScript number, which is exact up to 2^53 − 1. Its overflow checks use these bounds instead of the bounds of an `Int64`: a program [stops](/values/errors#stops) instead of losing precision, so a result is never silently wrong. A certain overflow of these bounds is a compile error when compiling for JavaScript.

`Integer.jsMaxSafeInteger` is 2^53 − 1. If you want the same behavior on every target, use a ranged integer:

```fa
type JsInteger = Integer(min = -Integer.jsMaxSafeInteger, max = Integer.jsMaxSafeInteger)
```

The 64-bit and wider types written explicitly (`Int64`, `UInt64`, `Int128`, `UInt128`) and `BigInt` are exact on JavaScript too, but slower: they compile to JavaScript `BigInt` values.

## Floats

`Float` is an alias for `Float64`. The available float types are:

- `Float32`: 32-bit floating-point number,
- `Float64`: 64-bit floating-point number.

```fa
let pi: Float = 3.14159
let small: Float32 = 0.5
```

A literal containing a `.` or an exponent is inferred as `Float`.

Floats follow the IEEE 754 standard, on every target:

- `/` on floats never fails: dividing by zero gives an infinity, or NaN for `0.0 / 0.0`. Overflowing gives an infinity. Floats don't stop the program and don't throw.
- `NaN == NaN` is `false`, and every ordering comparison with NaN is `false`. `-0.0 == 0.0` is `true`. Test NaN with `value.isNaN()`, and infinities with `value.isFinite()`.
- As a `Set` element or a `Map` key, floats are compared with the *SameValueZero* rule, like in a JavaScript `Map`: NaN finds NaN, and `-0.0` and `0.0` are the same key.

## Big integers

Big integers are integers with an arbitrary large size. They are defined with the `BigInt` type or the `n` suffix.

```fa
let myBigInteger = 12n
let myTypedBigInteger: BigInt = 12
let myConvertedBigInteger = BigInt(12)
```

A big integer never overflows. Its `//` and `%%` throw `DivisionByZero` like the integer ones, and its `**` needs an exponent that is not negative.

## Percentages

A `Percentage` is a number that represents a fraction of 100. Percentages are mostly used to distinguish relative values from absolute values.

:::tip
In user interface layouts, `50` pixels and `50%` of the parent are different things. Giving percentages their own type makes it impossible to mix them up.
:::

A percentage is written with the `%` suffix after a number literal:

```fa
let half = 50%
let typedHalf: Percentage = 50%
```

The `%` suffix can also be applied to an expression in parentheses:

```fa
let width = 30
let margin = 10

let total = (width + margin)% -- 40%
```

The suffix only applies to a number literal or to a parenthesized expression. To turn a value stored in a name into a percentage, wrap it in parentheses:

```fa
let fifty = 50

let a = fifty% -- error: `%` only applies to a literal or a parenthesized expression
let b = (fifty)% -- 50%
```

`Percentage(value)` creates a percentage from a value as well, but the `(value)%` syntax is preferred: the formatter rewrites `Percentage(width + margin)` into `(width + margin)%`.

:::note
This is a deliberate exception to the "one way to write things" rule, to keep user interface code readable.
:::

:::caution
`%` is **not** the remainder operator: it only builds percentages. The remainder operator is `%%`. See [arithmetic](/operations/arithmetic).
:::

A percentage is stored as the fraction it represents, in a 64-bit float: `50%` is stored as `0.5`, and `(value)%` divides an integer or a float by 100. A percentage has its own type, and it is not implicitly converted to other number types:

```fa
let percentage = 50%

let ratio: Float = percentage -- error: a `Percentage` is not a `Float`
let converted = Float(percentage) -- 0.5
```

Operations between percentages and other numbers are explicitly defined (see [result types](#result-types)):

```fa
let size = 40.0 * 50% -- Float * Percentage => Float: 20.0
let total = 20% + 30% -- Percentage + Percentage => Percentage: 50%
let ratio = Float(50%) -- explicit conversion: 0.5
```

## Result types

The type of each arithmetic operation, by kind of operands. Both operands must be of the same kind; a literal operand adapts to the other one.

| Operands | `+` `-` `*` | `/` | `//` | `%%` | `**` |
| --- | --- | --- | --- | --- | --- |
| integers | integer, by [range](#ranges-through-arithmetic) | `Float` | integer | integer | integer |
| `Float32` | `Float32` | `Float32` | `Float32` | `Float32` | `Float32` |
| `Float64` | `Float64` | `Float64` | `Float64` | `Float64` | `Float64` |
| `BigInt` | `BigInt` | `Float` | `BigInt` | `BigInt` | `BigInt` |

- On floats, `//` is the floor of the quotient, as a float (`7.5 // 2.0` is `3.0`), and `%%` is the matching remainder.
- An integer `//` or `%%` whose divisor isn't proven non-zero `throws DivisionByZero`. See [division by zero](/operations/arithmetic#division-by-zero).
- An integer or big integer `**` whose exponent isn't proven to be non-negative `throws ArithmeticError`.
- The prefix `-` keeps the type of its operand, and follows the range rules on integers (`-x` with `x: UInt8` is only valid when the result fits).

With percentages:

| Operation | Result |
| --- | --- |
| `Percentage + Percentage`, `Percentage - Percentage` | `Percentage` (`20% + 30%` is `50%`) |
| `Percentage * Percentage` | `Percentage` (`50% * 50%` is `25%`) |
| `Percentage / Percentage` | `Float` (`50% / 25%` is `2.0`) |
| `Float * Percentage`, `Percentage * Float` | the float type (`40.0 * 50%` is `20.0`) |
| `-percentage` | `Percentage` |

Any other mix, like `Integer * Percentage`, is an error: convert first (`Float(count) * 50%`). Percentages are ordered with `<`, `<=`, `>` and `>=` between themselves.

## Conversions

Fa never converts a number from one kind to another implicitly. Mixing kinds in an operation is a compile error:

```fa
let count: Integer = 3
let ratio: Float = 0.5

let result = count * ratio -- error: cannot multiply `Integer` by `Float`. Use `Float(count) * ratio`.
let converted = Float(count) * ratio -- 1.5
```

The conversions between numbers:

| Conversion | Result |
| --- | --- |
| `Float(integer)`, `Float32(...)` | the nearest float: never fails |
| `Float(percentage)` | the fraction: `Float(50%)` is `0.5` |
| `BigInt(integer)` | exact: never fails |
| `Int8(integer)`, `Integer(bigInt)`, ... | exact; `throws ArithmeticError` when the value isn't proven to fit |
| `Percentage(number)` | the number divided by 100 (prefer `(number)%`) |
| `Integer.round(float)`, `Integer.floor`, `Integer.ceil`, `Integer.truncate` | the rounded integer; `throws ArithmeticError` when the float is NaN, infinite or out of range, unless it is a literal |

Converting a float to an integer requires choosing how to round it:

```fa
let rounded = Integer.round(2.7) -- 3
let floored = Integer.floor(2.7) -- 2
```

See [arithmetic](/operations/arithmetic) for the available operations and [type conversion](/types/type-conversion) for conversions between types.

:::implementation
## Implementation details

### Native

- Each integer type is stored in its width. An arithmetic operation is computed in a width large enough for the range of its result (up to 128 bits), then range-checked if needed: a possible overflow is one compare (or the overflow flag, with `llvm.sadd.with.overflow` and friends) and a cold branch to the stop routine.
- A ranged `Integer(min, max)` is stored in 64 bits; a ranged sized type (`Int8(min = 0)`) in the width of its type.
- `BigInt` is an arbitrary-precision integer: inline when it fits in 64 bits, heap-allocated limbs otherwise.
- A `Percentage` is an `f64` holding the fraction (`0.5` for `50%`).
- Floats are `f32` and `f64`, with IEEE semantics: no fast-math flags that would change the results.

### JavaScript

- `Integer`, the sized types up to 32 bits and the ranged integers are JavaScript numbers. A possibly overflowing `Integer` operation is followed by a check against ±(2^53 − 1) (`Number.isSafeInteger`-style compare). Sized types check against their own bounds.
- `Int64`, `UInt64`, `Int128`, `UInt128` written explicitly, and `BigInt`, are JavaScript `BigInt` values; the sized ones check their bounds after each operation (or use `BigInt.asIntN`/`BigInt.asUintN` for the `wrapping` variants). Passing an `Integer` where an `Int64` is expected is a `BigInt(value)` call.
- `Float32` operations are rounded with `Math.fround`.
- A `Percentage` is a number holding the fraction.
- A stop is a thrown `FaStop` error (see [errors](/values/errors#stops)).
:::
