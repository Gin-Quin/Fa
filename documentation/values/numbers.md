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
There is no general `Number` type in Fa. Every numeric value has a concrete type, and arithmetic between different numeric types requires an explicit conversion. See [conversions](#conversions).
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

### Minimum and maximum values

You can statically indicate a minimum and a maximum value. `min` and `max` are compile-time value parameters of the `Integer` type (see [generics](/types/generics)):

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

let sum = a + b -- Integer(min = 1, max = 15)
let product = a * b -- Integer(min = 0, max = 50)
```

### Overflow

The result of an operation is checked against the destination type at compile time. If the result cannot be proven to fit, the assignment is a compile error:

```fa
let a: Int8 = readInt8()
let b: Int8 = readInt8()

-- compiler error: `a + b` has the range Integer(min = -256, max = 254),
-- which does not fit in Int8
let c: Int8 = a + b
```

When the result may genuinely overflow, choose the behavior you want explicitly:

```fa
let wrapped = a.wrappingAdd(b) -- wraps around
let saturated = a.saturatingAdd(b) -- clamps to the bounds of the type
let checked = a.checkedAdd(b) catch error => 0 -- throws ArithmeticError on overflow
```

The same variants exist for `Sub`, `Mul` and `Pow` (`wrappingSub`, `saturatingMul`, `checkedPow`, ...). See [arithmetic](/operations/arithmetic#overflow).

### Compatibility with JavaScript

When targeting JavaScript, integers greater than 53 bits lose precision.

To check if a number is within the safe range, you can use the `Integer.JS_MAX_SAFE_INTEGER` constant.

If you want to enforce number safety at compile time, you can use a ranged integer:

```fa
type JsInteger = Integer(min = -Integer.JS_MAX_SAFE_INTEGER, max = Integer.JS_MAX_SAFE_INTEGER)
```

Alternatively, you can use a `BigInt`.

## Floats

`Float` is an alias for `Float64`. The available float types are:

- `Float32`: 32-bit floating-point number,
- `Float64`: 64-bit floating-point number.

```fa
let pi: Float = 3.14159
let small: Float32 = 0.5
```

A literal containing a `.` is inferred as `Float`.

## Big integers

Big integers are integers with an arbitrary large size. They are defined with the `BigInt` type or the `n` suffix.

```fa
let myBigInteger = 12n
let myTypedBigInteger: BigInt = 12
let myConvertedBigInteger = BigInt(12)
```

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

Percentages are encoded as 64-bit floating-point numbers, but they have their own type, and they are not implicitly converted to other number types:

```fa
let percentage = 50%
assert percentage is Percentage -- pass
assert percentage is Float -- fail
```

Operations between percentages and other numbers are explicitly defined:

```fa
let size = 40.0 * 50% -- Float * Percentage => Float: 20.0
let total = 20% + 30% -- Percentage + Percentage => Percentage: 50%
let ratio = Float(50%) -- explicit conversion: 0.5
```

## Conversions

Fa never converts a number from one type to another implicitly. Mixing types in an operation is a compile error:

```fa
let count: Integer = 3
let ratio: Float = 0.5

let result = count * ratio -- error: cannot multiply `Integer` by `Float`. Use `Float(count) * ratio`.
let converted = Float(count) * ratio -- 1.5
```

Converting a float to an integer requires choosing how to round it:

```fa
let rounded = Integer.round(2.7) -- 3
let floored = Integer.floor(2.7) -- 2
```

See [arithmetic](/operations/arithmetic) for the available operations and [type conversion](/types/type-conversion) for conversions between types.
