# Numbers

There are three types of numbers: **integers**, **big integers** and **decimals**.

## Integers

Integers are the most common numbers. They follow slightly different rules depending on the target language (JavaScript or Rust).

Integers are automatically inferred to three different sizes, depending on their size: 32-bit, 48-bit and 64-bit.

```fa
let myInteger = 12
let myInteger: Integer = 12

-- you can specify the size of the integer at bit level
let myInteger8: Integer(8) = 128 -- 8-bit integer in Rust, `number` in JavaScript
let myInteger5: Integer(5) = 128 -- 5-bit integer in Rust, `number` in JavaScript

-- you can create unsigned integers
let myUnsignedInteger8: UnsignedInteger(8) = 128 -- 8-bit unsigned integer in Rust, `number` in JavaScript

-- the default inferred size is 32 bits
let myInteger32 = 128 -- 32-bit integer in Rust, `number` in JavaScript
let myInteger32: Integer = 128 -- 32-bit integer in Rust, `number` in JavaScript

-- literal integers between 32 and 48 bits are automatically inferred as 48-bit integers
let myInteger48 = 4_294_967_296 -- inferred as 48-bit integer in Rust, `number` in JavaScript
let myInteger48: Integer(48) = 4_294_967_296 -- 48-bit integer in Rust, `number` in JavaScript

-- integers larger than 48 bits are transpiled to `BigInt` in JavaScript
let myInteger49: Integer(49) = 128 -- 49-bit integer in Rust, `BigInt` in JavaScript

-- literal integers larger than 48 bits are automatically inferred as 64-bit integers
let myInteger64 = 281_474_976_710_656 -- inferred as 64-bit integer in Rust, `BigInt` in JavaScript
let myInteger64: Integer(64) = 281_474_976_710_656 -- 64-bit integer in Rust, `BigInt` in JavaScript
```

When possible, it is recommended to use integers smaller than 48 bits to ensure maximum compatibility and speed with JavaScript.

### Big integers

Big integers are integers with an arbitrary size. They are defined with the `BigInteger` type or the `n` suffix.

```fa
let myBigInteger = 12n
let myBigInteger: BigInteger = 12
let myBigInteger = BigInteger(12)
```

## Decimals

Decimals are floating-point numbers. They are defined with the `Number` type.

```fa
let myNumber = 12.34
let myNumber: Number = 12.34

-- by default, numbers with a fractional part are 64-bit floating-point numbers
let myNumber64 = 12.34
let myNumber64: Number(64) = 12.34

-- you can create numbers with a size of 32 bits as well
let myNumber32 = 12.34
let myNumber32: Number(32) = 12.34
```

## Percentages

A `Percentage` is a special number value that represents a fraction of 100. It is defined with the `Percentage` type.

```fa
let myPercentage = 50%
let myPercentage: Percentage(Integer) = 50%

let myPercentage64: Percentage(Integer(64)) = 50%

-- you can create percentages of decimals or big integers as well
let myPercentageDecimal = 50.5%
let myPercentageDecimal: Percentage(Decimal) = 50.5%
let myPercentageBigInteger = 12n%
let myPercentageBigInteger: Percentage(BigInteger) = 12n%
```

Percentages have their own type, but can implicitly be converted to numbers:

```fa
console.log(40 * 50%) -- print "20"

let integer: Integer = 50%
console.log(integer) -- print "0.5"
```

They can be created from a variable using parentheses or the `Percentage` constructor:

```fa
let fifty = 50
let myPercentage = (fifty)%
let myPercentage = Percentage(fifty)
```
