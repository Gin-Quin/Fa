# Arithmetic

Arithmetic operators work on numbers (integers, floats, big integers and percentages). Both operands must have the same type: Fa never converts numbers implicitly. See [numbers](/values/numbers).

## Basic operations

```fa
let sum = 4 + 6
let diff = 10 - 3
let product = 7 * 6
let power = 7 ** 6
let division = 22 / 7 -- 3.142857... (Float)
let quotient = 22 // 7 -- 3 (Integer)
```

The `/` operator always returns a `Float`, even when both operands are integers. Use `//` for integer division: it always returns an `Integer`.

## Modulo

In Fa, `%` is only used to write [percentages](/values/numbers#percentages) (`50%`, `(x + y)%`). The remainder of a division is calculated with the `modulo` keyword.

```fa
let remainder = 10 modulo 3 -- 1
function isEven = (value: Integer): Boolean => value modulo 2 == 0
```

## Order of operations

Use parentheses to make intent explicit.

```fa
let result = (2 + 3) * 4
let ratio = (wins + losses) / total
```

## Compound assignment

```fa
mutable count = 0
count += 1
count -= 2
count *= 3
count //= 4 -- `count` is an Integer, so it uses integer division
```

There is no compound assignment for `modulo`: write `count = count modulo 5`.

## Bitwise operations

Bitwise operations are available on integer types as methods. Fa has no bitwise operators: they are rarely needed, and named methods avoid the precedence pitfalls of symbols (in C or JavaScript, `x & 1 == 0` means `x & (1 == 0)`).

```fa
let a: UInt8 = 0b1100
let b: UInt8 = 0b1010

a.bitAnd(b)      -- 0b1000
a.bitOr(b)       -- 0b1110
a.bitXor(b)      -- 0b0110
a.bitNot()       -- 0b11110011
a.shiftLeft(2)   -- 0b110000
a.shiftRight(2)  -- 0b0011
```

:::tip
To store a set of flags, prefer the [`fields`](/types/fields) type.
:::
