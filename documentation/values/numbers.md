# Numbers

Fa has four families of numbers: **integers**, **big integers**, **decimals** and **percentages**.

:::caution
There is no general `Number` type in Fa. Every numeric value has a concrete type, and arithmetic between different numeric types requires an explicit conversion. See [type conversion](../types/type-conversion).
:::

## Integers

Integers can be signed or unsigned:

- `Int` — alias for `Int64`
- `Int8`, `Int16`, `Int32`, `Int64`, `Int128`

And their unsigned counterparts:

- `UInt` — alias for `UInt64`
- `UInt8`, `UInt16`, `UInt32`, `UInt64`, `UInt128`

```fa
let myInteger = 12
let myInteger: Int = 12

let myUnsignedInteger: UInt = 12
```

An integer literal with no annotation is inferred as `Int`.

Literals may be written in several bases:

```fa
let decimal     = 42
let binary      = 0b101010
let octal       = 0o52
let hexadecimal = 0x2A
```

### Minimum and maximum values

Fa's type system lets you state a minimum and a maximum value:

```fa
let myPositiveInteger: Int(min = 0) = 12
let mySmallInteger: Int(min = 0, max = 200) = 200
```

A minimum or maximum restricts the range of the integer at compile time, so you cannot assign values outside it.

```fa
mutable value: Int(min = 20) = 20

-- compile error: 10 is less than the minimum value of 20
value = 10
```

The compiler propagates the range through your program, which lets it reject impossible code and remove redundant checks:

```fa
let value: Int(min = 20) = get20OrMore()

-- compile warning: this condition is always false
if value < 20 {
  console.log("Value is less than 20")
}
```

```fa
let value: Int(min = 4) = get4OrMore()

let array = [1, 2, 3]

-- compile error: the index is out of bounds
let item = array[value]
```

Assigning a wider range into a narrower one is rejected until you narrow it:

```fa
mutable foo: Int(max = 100) = getIntegerLowerThan(100)
mutable bar: Int(max = 200) = getIntegerLowerThan(200)

-- compile error: `bar` may exceed the maximum value of 100
foo = bar

-- narrow it first
foo = if bar <= 100 { bar } else { 100 }
```

Ranged integers are the mechanism behind Fa's [smart bound checks](../collections/arrays#smart-bound-checks) and its [division-by-zero guarantees](../operations/arithmetic#division-by-zero).

### Overflow

Arithmetic widens the result range. If the result cannot be proven to fit the destination type, the assignment is a compile error:

```fa
let a: Int8 = 100
let b: Int8 = 100

-- compile error: `a + b` has range Int(min = -256, max = 254),
-- which does not fit in Int8
let c: Int8 = a + b
```

When the result may genuinely overflow, pick the behaviour you want explicitly:

```fa
let wrapped    = a.wrappingAdd(b)    -- wraps around
let saturated  = a.saturatingAdd(b)  -- clamps to the type's bounds
let checked    = a.checkedAdd(b)     -- Int8 | ArithmeticError
```

The same three variants exist for `Sub`, `Mul` and `Pow`.

### Compatibility with JavaScript

Beyond 53 bits, JavaScript loses integer precision.

To check whether a number is within the safe range at runtime, compare it against `Int.jsMaxSafeInteger`.

To enforce it at compile time, use a ranged integer:

```fa
type JsInt = Int(min = -Int.jsMaxSafeInteger, max = Int.jsMaxSafeInteger)
```

Alternatively, use a `BigInt`.

## Big integers

Big integers have an arbitrary size. They are written with the `n` suffix, or built with `BigInt`.

```fa
let myBigInteger = 12n
let myBigInteger: BigInt = 12n
let myBigInteger = BigInt(12)
```

## Decimals

Decimals are floating-point numbers:

- `Decimal` — alias for `Float64`
- `Float32` — 32-bit
- `Float64` — 64-bit

```fa
let myDecimal: Decimal = 12.34
let myInferredDecimal = 12.34
```

A literal containing a `.` is inferred as `Decimal`.

## Percentages

A `Percentage` represents a fraction of 100. It is written with the `%` suffix.

```fa
let myPercentage = 50%
let myPercentage: Percentage = 50%
```

The `%` suffix applies to a number literal, an identifier, a field access, or a parenthesised expression:

```fa
let fifty = 50

let a = 50%
let b = fifty%
let c = settings.ratio%
let d = (base + offset)%
```

For any other expression, use the constructor:

```fa
let e = Percentage(computeRatio())
```

Percentages are stored as 64-bit floating-point values, and are implicitly converted to decimals when multiplied by one:

```fa
console.log(40 * 50%) -- 20
```

They keep their own distinct type:

```fa
let percentage = 50%
assert(percentage is not Decimal)
assert(percentage is Percentage)
```

:::tip
Percentages exist to distinguish absolute from relative values — for instance in user interface layouts, where `50` pixels and `50%` of the parent are different things.
:::

:::caution
`%` is a **suffix** that builds a percentage. The remainder operator is `%%`. See [arithmetic](../operations/arithmetic).
:::

## Bytes

`Byte` is an alias for `UInt8`:

```fa
type Byte = UInt8
```
