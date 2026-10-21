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

## Integers

`Integer` is an alias for `Int64`. When you need a specific size, use one of the sized integer types:

- `Int8`, `Int16`, `Int32`, `Int64`, `Int128`: signed integers,
- `UInt8`, `UInt16`, `UInt32`, `UInt64`, `UInt128`: unsigned integers.

```fa
let pixel: UInt8 = 255
let fileSize: UInt64 = 1000000
```

The `Byte` type is an alias for `UInt8`:

```fa
type Byte = UInt8
```

### Minimum and maximum values

You can statically indicate a minimum and a maximum value:

```fa
let myPositiveInteger: Integer(Min = 0) = 12
let mySmallInteger: Integer(Min = 0, Max = 200) = 200
```

Indicating a minimum or a maximum value restricts the range of the integer at compile time, which means you won't be able to assign values outside of the specified range.

To declare a non-negative integer, prefer `Integer(Min = 0)` over an unsigned type: unsigned types are meant for binary data and fixed-size formats.

Some examples:

```fa
mutable value: Integer(Min = 20) = 20

-- compiler error: 10 is less than the minimum value of 20
value = 10
```

```fa
let value: Integer(Min = 20) = get20OrMore()

-- compiler warning: the condition is always false
if value < 20 {
	console.log("Value is less than 20")
}
```

```fa
let value: Integer(Min = 4) = get4OrMore()

let array = [1, 2, 3]

-- compiler error: the index is out of bounds
let item = array[value]
```

```fa
mutable foo: Integer(Max = 100) = getIntegerLowerThan(100)
mutable bar: Integer(Max = 200) = getIntegerLowerThan(200)

-- compiler error: bar may exceed the maximum value of 100
foo = bar

-- you first have to check that bar is lower than 100
foo = if bar <= 100 { bar } else { 100 }
```

### Compatibility with JavaScript

When targeting JavaScript, integers greater than 53 bits lose precision.

To check if a number is within the safe range, you can use the `Integer.JS_MAX_SAFE_INTEGER` constant.

If you want to enforce number safety at compile time, you can use the `JsInteger` type:

```fa
type JsInteger = Integer(Min = -Integer.JS_MAX_SAFE_INTEGER, Max = Integer.JS_MAX_SAFE_INTEGER)
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

## Big integers

Big integers are integers with an arbitrary large size. They are defined with the `BigInt` type or the `n` suffix.

```fa
let myBigInteger = 12n
let myBigInteger2: BigInt = 12
let myBigInteger3 = BigInt(12)
```

## Percentages

A `Percentage` is a number that represents a fraction of 100. Percentages are mostly used to distinguish relative values from absolute values, for example in user interfaces.

A percentage is written with the `%` suffix:

```fa
let half = 50%
let half2: Percentage = 50%
```

The `%` suffix can also be applied to an expression in parentheses:

```fa
let width = 30
let margin = 10

let total = (width + margin)% -- 40%
```

`Percentage(value)` creates a percentage from a value as well, but the `(value)%` syntax is preferred: the formatter rewrites `Percentage(width + margin)` into `(width + margin)%`.

:::note
This is a deliberate exception to the "one way to write things" rule, to keep user interface code readable.
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
let result2 = Float(count) * ratio -- 1.5
```

Converting a float to an integer requires choosing how to round it:

```fa
let rounded = Integer.round(2.7) -- 3
let floored = Integer.floor(2.7) -- 2
```

See [arithmetic](/operations/arithmetic) for the available operations.
