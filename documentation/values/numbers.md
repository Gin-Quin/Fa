# Numbers

There are three types of numbers: **integers**, **big integers** and **decimals**.

## Integers

Integers are the most common numbers. They follow slightly different rules depending on the target language (Javascript or Zig).

Integers are automatically inferred to three different sizes, depending on their sizes: 32-bit, 48-bit and 64-bit.

```ts
myInteger = 12
myInteger: Integer = 12

// you can specify the size of the integer at bit level
myInteger8: Integer(8) = 128 // 8-bit integer in Zig, `number` in JavaScript
myInteger5: Integer(5) = 128 // 5-bit integer in Zig, `number` in JavaScript

// you can create unsigned integers
myUnsignedInteger8: UnsignedInteger(8) = 128 // 8-bit unsigned integer in Zig, `number` in JavaScript

// the default inferred size is 32 bits
myInteger32 = 128 // 32-bit integer in Zig, `number` in JavaScript
myInteger32: Integer = 128 // 32-bit integer in Zig, `number` in JavaScript

// literal integers between 32 and 48 bits are automatically inferred as 48-bit integers
myInteger48 = 4_294_967_296 // inferred as 48-bit integer in Zig, `number` in JavaScript
myInteger48: Integer(48) = 4_294_967_296 // 48-bit integer in Zig, `number` in JavaScript

// integers larger than 48 bits are transpiled to `BigInt` in JavaScript
myInteger49: Integer(49) = 128 // 49-bit integer in Zig, `BigInt` in JavaScript

// literal integers larger than 48 bits are automatically inferred as 64-bit integers
myInteger64 = 281_474_976_710_656 // inferred as 64-bit integer in Zig, `BigInt` in JavaScript
myInteger64: Integer(64) = 281_474_976_710_656 // 64-bit integer in Zig, `BigInt` in JavaScript
```

When possible, it is recommended to use integer smaller than 48 bits to ensure maximum compatibility and speed with Javascript.

### Big integers

Big integers are integers with an arbitrary size. They are defined with the `BigInteger` type or the `n` suffix.

```ts
myBigInteger = 1234567890123456789012345678901234567890n
myBigInteger: BigInteger = 1234567890123456789012345678901234567890
myBigInteger = BigInteger(1234567890123456789012345678901234567890)
```

## Decimals

Decimals are floating-point numbers. They are defined with the `Number` type.

```ts
myNumber = 12.34
myNumber: Number = 12.34

// by default, numbers with a fractional part are 64-bit floating-point numbers
myNumber64 = 12.34
myNumber64: Number(64) = 12.34

// you can create numbers with a size of 32 bits as well
myNumber32 = 12.34
myNumber32: Number(32) = 12.34
```
