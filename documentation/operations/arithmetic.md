# Arithmetic

Arithmetic operators work on integers, decimals and big integers.

## Operators

| Operator | Meaning | Example |
| --- | --- | --- |
| `+` | addition | `4 + 6` |
| `-` | subtraction | `10 - 3` |
| `*` | multiplication | `7 * 6` |
| `**` | exponentiation | `7 ** 6` |
| `/` | division | `22 / 7` |
| `//` | integer division | `22 // 7` |
| `%%` | remainder | `10 %% 3` |
| `%` | percentage suffix | `50%` |

```fa
let sum = 4 + 6
let difference = 10 - 3
let product = 7 * 6
let power = 7 ** 6
let division = 22 / 7   -- 3.142857…  (Decimal)
let quotient = 22 // 7  -- 3          (Int)
let remainder = 10 %% 3 -- 1
```

:::caution
`%` is not the remainder operator in Fa — it is the suffix that builds a [percentage](../values/numbers#percentages). The remainder operator is `%%`.

```fa
let isEven = (value: Int): Boolean => value %% 2 == 0
let half = 50%
```
:::

Always put spaces around a binary `-`. Because `--` starts a comment, `a --b` is `a` followed by a comment, while `a - b` is a subtraction.

## Division

`/` is true division. Dividing two integers produces a `Decimal`:

```fa
let ratio = 22 / 7 -- Decimal
```

`//` is integer division, truncating towards negative infinity, and produces an integer:

```fa
let pages = 22 // 7 -- 3
```

### Division by zero

Fa has no runtime crash for division by zero. Instead, the compiler requires the divisor to be provably non-zero.

When it can prove it — from a literal, or from a [ranged integer](../values/numbers#minimum-and-maximum-values) — the result is a plain number:

```fa
let a = 10 // 2                      -- Int

let divisor: Int(min = 1) = readPositive()
let b = 10 // divisor                -- Int
```

When it cannot, the result is an error union you must handle:

```fa
let divisor = readNumber()

let c = 10 // divisor -- Int | DivisionByZero
let d = 10 // divisor ?? 0
```

Narrowing the divisor removes the error:

```fa
if divisor != 0 {
  let e = 10 // divisor -- Int, because `divisor` is narrowed here
}
```

The same rule applies to `/` and `%%`.

## Overflow

The compiler tracks the possible range of every arithmetic result. If a result cannot be proven to fit its destination, the assignment is rejected:

```fa
let a: Int8 = 100
let b: Int8 = 100

let c: Int8 = a + b -- error: the result may not fit in an Int8
let c: Int16 = a + b -- fine
```

When overflow is genuinely possible and you need a specific behaviour, ask for it:

```fa
a.wrappingAdd(b)   -- Int8, wraps around
a.saturatingAdd(b) -- Int8, clamps to the type's bounds
a.checkedAdd(b)    -- Int8 | ArithmeticError
```

`Sub`, `Mul` and `Pow` have the same three variants.

## Compound assignment

```fa
mutable count = 10
count += 1  -- 11
count -= 2  -- 9
count *= 3  -- 27
count /= 4  -- error: the result is a Decimal, not an Int
count //= 4 -- 6
count %%= 5 -- 1
count **= 2 -- 1
```

A compound assignment is subject to the same range and division checks as the operator it uses.

## Comparison

Comparison operators are `<`, `<=`, `>`, `>=`, `==` and `!=`, and they can be chained:

```fa
if 2 < a < 12 {
  console.log("in range")
}
```

See [logical operations](./logical).

## Order of operations

From tightest to loosest:

1. `**`
2. `*`, `/`, `//`, `%%`
3. `+`, `-`
4. comparisons
5. `and`
6. `or`

`**` is right-associative (`2 ** 3 ** 2` is `2 ** 9`); everything else is left-associative.

Use parentheses whenever the intent is not immediately obvious:

```fa
let result = (2 + 3) * 4
let ratio = (wins + losses) / total
```

## Bitwise operations

Because `>>` and `<<` are the [extract and insert](./extract-insert) operators, bitwise operations are methods on integers:

```fa
let a = 0b1100
let b = 0b1010

a.bitAnd(b)     -- 0b1000
a.bitOr(b)      -- 0b1110
a.bitXor(b)     -- 0b0110
a.bitNot()      -- inverts every bit
a.shiftLeft(2)  -- 0b110000
a.shiftRight(2) -- 0b0011
```

Shifts are logical for unsigned integers and arithmetic for signed ones. Shifting by more than the width of the type is a compile error when the amount is known, and returns `ArithmeticError` when it is not.

## Math functions

```fa
let rounded = value.round()
let down = value.floor()
let up = value.ceil()
let magnitude = value.abs()
let root = value.sqrt()      -- Decimal | ArithmeticError for negatives
let biggest = Math.max(a, b)
let smallest = Math.min(a, b)
let bounded = value.clamp(0, 100)
```
