# Numbers

There are three types of numbers: **integers**, **big integers** and **decimals**.

## Integers

Integers can be positive or negative, and signed or unsigned. The different integer types are:

- `Int`: alias for `Int64`
- `Int8`: 8-bit signed integer
- `Int16`: 16-bit signed integer
- `Int32`: 32-bit signed integer
- `Int64`: 64-bit signed integer
- `Int128`: 128-bit signed integer

And their unsigned counterparts:

- `UInt`: alias for `UInt64`
- `UInt8`: 8-bit unsigned integer
- `UInt16`: 16-bit unsigned integer
- `UInt32`: 32-bit unsigned integer
- `UInt64`: 64-bit unsigned integer
- `UInt128`: 128-bit unsigned integer

Examples:

```fa
let myInteger = 12
let myInteger: Int = 12

let myUnsignedInteger: UInt = 12
```

### Minimum and Maximum Values

One great feature of the Fa's type system is that you can statically indicate a minimum and maximum value:

```fa
let myPositiveInteger: Int(Min = 0) = 12
let mySmallInteger: Int(Min = 0, Max = 200) = 200
```

Indicating a minimum or a maximum value will restrict the range of the integer at compile time, which means you won't be able to assign values outside of the specified range.

Some examples:


```fa
mutable value: Int(Min = 20) = 0

-- compiler error: 10 is less than the minimum value of 20
value = 10
```

```fa
let value: Int(Min = 20) = get20OrMore()

-- compiler warning: the condition is always false
if value < 20 {
  console.log("Value is less than 20")
}
```

```fa
let value: Int(Min = 4) = get4OrMore()

let array = [1, 2, 3]

-- compiler error: the index is out of bounds
let item = array[value]
```

```fa
mutable foo: Int(Max = 100) = getIntegerLowerThan(100)
mutable bar: Int(Max = 200) = getIntegerLowerThan(200)

-- this will raise cause an error, as bar may exceed
-- the maximum value of 100
foo = bar

-- you first have to check that bar is lower than 100
foo = if bar <= 100 { bar } else { 100 }
```


### Compatibility with Javascript

For integers greater than 53 bits, Javascript will start to lose precision.

To check if a number is within the safe range, you can use the `Number.JS_MAX_SAFE_INTEGER` constant.

Or, if you want to enforce number safety at compile time, you can use the `JsInt` type:

```fa
type JsInt = Integer(Max = Number.JS_MAX_SAFE_INTEGER)
```

Alternatively, you can use a `BigInt`.

## Big Integers

Big integers are integers with an arbitrary large size. They are defined with the `BigInt` type or the `n` suffix.

```fa
let myBigInteger = 12n
let myBigInteger: BigInt = 12
let myBigInteger = BigInt(12)
```

## Decimal Numbers

Decimal numbers are floating-point numbers. The different types are:

- `Decimal`: alias for `Float64`
- `Float32`: 32-bit signed
- `Float64`: 64-bit signed

```fa
let myDecimal: Decimal = 12.34
let myInferredDecimal: Decimal = 12.34
```

## Percentages

A `Percentage` is a special number value that represents a fraction of 100. It is defined with the `Percentage` type.

```fa
let myPercentage = 50%
let myPercentage: Percentage = 50%

let fifty = 50

let myPercentage = (fifty)%
let myPercentage = Percentage(fifty)
```

Percentages are encoded as decimal numbers (64-bit floating-point).


They are implicitly castable into decimal numbers:

```fa
console.log(40 * 50%) -- print "20"
```

However, they have their own type:

```fa
let percentage = 50%
assert(percentage is Decimal) -- fail
assert(percentage is Percentage) -- pass
```

:::tip
Percentages are mostly used to differentiate between absolute and relative values, for example in user interfaces.
:::

## Bytes

The `Byte` type is an alias for `UInt8`:

```fa
type Byte = UInt8
```
