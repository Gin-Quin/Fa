# Arithmetic

Arithmetic operators work on numbers: integers, floats, big integers and percentages. See [numbers](/values/numbers).

Both operands must have the same type: Fa never converts numbers implicitly. Convert explicitly with a call, and let literals adapt to the expected type:

```fa
let count = 3
let ratio = 0.5

let scaled = count * ratio           -- error: `Integer * Float`. Use `Float(count) * ratio`
let converted = Float(count) * ratio -- 1.5
let doubled = ratio * 2              -- 1.0: the literal `2` adapts to `Float`
```

## Operators

| Operator | Meaning | Example |
| --- | --- | --- |
| `+` | addition | `4 + 6` |
| `-` | subtraction | `10 - 3` |
| `*` | multiplication | `7 * 6` |
| `**` | exponentiation | `7 ** 6` |
| `/` | division, returns a `Float` | `22 / 7` |
| `//` | floor division, returns an `Integer` | `22 // 7` |
| `%%` | remainder | `10 %% 3` |
| `%` | percentage suffix | `50%` |

```fa
let sum = 4 + 6
let difference = 10 - 3
let product = 7 * 6
let power = 7 ** 6
let division = 22 / 7   -- 3.142857... (Float)
let quotient = 22 // 7  -- 3 (Integer)
let remainder = 10 %% 3 -- 1
```

:::caution
`%` is not the remainder operator in Fa: it is the suffix that builds a [percentage](/values/numbers#percentages) (`50%`, `(x + y)%`). The remainder operator is `%%`.

```fa
function isEven = (value: Integer): Boolean => value %% 2 == 0

let half = 50%
```
:::

Always put spaces around a binary `-`. Because `--` starts a comment, `a --b` is `a` followed by a comment, while `a - b` is a subtraction.

## Division

`/` is true division. It always returns a `Float`, even when both operands are integers:

```fa
let ratio = 22 / 7 -- Float
let half = 4 / 2   -- 2.0, a Float
```

`//` is floor division: it rounds the quotient down, towards negative infinity, and returns an `Integer`:

```fa
let pages = 22 // 7 -- 3
let below = -7 // 2 -- -4
```

`%%` is the remainder of the floor division. It is consistent with `//`: for any `a` and `b`, `a == (a // b) * b + a %% b`. The result has the sign of the divisor:

```fa
let a = 7 %% 3   -- 1
let b = -7 %% 3  -- 2
let c = 7 %% -3  -- -2
```

### Division by zero

Fa has no runtime crash for division by zero. The divisor of `/`, `//` and `%%` must be provably non-zero.

When the compiler can prove it, from a literal or from a [ranged integer](/values/numbers#minimum-and-maximum-values), the operation can't fail:

```fa
let a = 10 // 2 -- Integer

let divisor: Integer(min = 1) = readPositive()
let b = 10 // divisor -- Integer
```

When it cannot, the operation `throws DivisionByZero`, and the error must be handled or propagated like any other [error](/values/errors):

```fa
let divisor = readNumber()

let c = 10 // divisor catch error => 0
let d = try 10 // divisor -- the enclosing function declares `throws DivisionByZero`
```

Narrowing the divisor removes the error:

```fa
if divisor != 0 {
	let e = 10 // divisor -- can't fail: `divisor` is narrowed here
}
```

## Overflow

An integer operation overflows when its result doesn't fit its type, like `200 + 100` in a `UInt8` (maximum `255`).

The compiler tracks the possible range of every arithmetic result: the ranges of the operands propagate through each operation.

- When an overflow is **certain**, it is a compile error.
- When an overflow is **impossible**, no check is generated.
- When an overflow is **possible** but not certain, the operation is checked at runtime: if it overflows, the program stops with an error report. It never silently produces a wrong result.

```fa
let a: UInt8 = 200 + 100 -- error: 300 doesn't fit in a UInt8

let small: Integer(min = 0, max = 100) = readPercent()
let bounded: Integer(min = 0, max = 200) = small * 2 -- no check: the result is between 0 and 200

let x: UInt8 = readByte()
let y: UInt8 = readByte()
let sum: Integer = x + y -- no check: the result is between 0 and 510
let byte: UInt8 = x + y  -- checked at runtime: stops the program if the result exceeds 255
```

Most checks disappear in practice: a loop index bounded by the length of a collection, a ranged type or a value read from a smaller type give the compiler the bounds it needs. The remaining checks cost a single, perfectly predicted branch.

When an overflow is expected, ask for a specific behavior explicitly:

```fa
let x: UInt8 = 250
let y: UInt8 = 10

x.wrappingAdd(y)   -- 4: wraps around, like a clock: 250 + 10 = 260, minus 256
x.saturatingAdd(y) -- 255: clamps to the bounds of the type
x.checkedAdd(y) catch error => 0 -- `checkedAdd` throws ArithmeticError on overflow
```

`Sub`, `Mul` and `Pow` have the same three variants: `wrappingSub`, `saturatingMul`, `checkedPow`... The `wrapping` variants are also the way to remove a runtime check from a hot loop, when wrapping around is the intended behavior (hashes, checksums, random number generators).

## Percentages

Operations between percentages and other numbers are explicit, with no implicit conversion:

```fa
let price = 80.0
let discount = 25%

let reduced = price * discount -- 20.0: `Float * Percentage` returns a `Float`
let total = discount + 10%     -- 35%: `Percentage + Percentage` returns a `Percentage`

assert Float(50%) == 0.5
```

See [percentages](/values/numbers#percentages).

## Compound assignment

Every arithmetic operator has a compound assignment form: `+=`, `-=`, `*=`, `**=`, `/=`, `//=` and `%%=`. To update several fields of an object at once, see [`.=`](/values/objects#updating-several-fields).

```fa
mutable count = 10
count += 1  -- 11
count -= 2  -- 9
count *= 3  -- 27
count /= 4  -- error: the result is a Float, not an Integer
count //= 4 -- 6
count %%= 5 -- 1
count **= 2 -- 1
```

A compound assignment is subject to the same range and division checks as the operator it uses.

## Comparison

Comparison operators are `<`, `<=`, `>`, `>=`, `==` and `!=`. Comparisons can be chained, and each operand is evaluated once:

```fa
if 0 < score <= 100 {
	console.log("valid score")
}
```

See [logical operations](/operations/logical).

## Order of operations

From tightest to loosest:

| Precedence | Operators |
| --- | --- |
| 1 | access and call: `a.b`, `a?.b`, `a(b)`, `a[b]` |
| 2 | postfix: `?`, `%` |
| 3 | prefix `-` |
| 4 | `**` |
| 5 | `*`, `/`, `//`, `%%` |
| 6 | `+`, `-` |
| 7 | ranges: `..`, `..=` |
| 8 | comparisons: `<`, `<=`, `>`, `>=` |
| 9 | `==`, `!=`, `is` |
| 10 | `not` |
| 11 | `and` |
| 12 | `or` |
| 13 | `then` |

`**` is right-associative (`2 ** 3 ** 2` is `2 ** 9`); the other binary operators are left-associative.

Because the prefix `-` binds tighter than `**`, `-x ** 2` is `(-x) ** 2`. Write `-(x ** 2)` for the opposite of the square.

Ranges bind looser than arithmetic, so `0..count + 1` is `0..(count + 1)`.

Use parentheses whenever the intent is not immediately obvious:

```fa
let result = (2 + 3) * 4
let area = (width + margin) * height
```

## Bitwise operations

Fa has no bitwise operators. Bitwise operations are methods on integer types:

- symbols like `&`, `|` and `^` have surprising precedences: in C or JavaScript, `x & 1 == 0` means `x & (1 == 0)`. A method call can't be misread,
- bitwise operations are rarely needed, and dedicated symbols would clutter the language for everyone else.

```fa
let a: UInt8 = 0b1100
let b: UInt8 = 0b1010

a.bitAnd(b)     -- 0b1000
a.bitOr(b)      -- 0b1110
a.bitXor(b)     -- 0b0110
a.bitNot()      -- 0b11110011
a.shiftLeft(2)  -- 0b110000
a.shiftRight(2) -- 0b0011
```

Shifts are logical for unsigned integers (the new bits are zeros) and arithmetic for signed integers (`shiftRight` keeps the sign bit). Shifting by the width of the type or more is a compile error when the amount is known at compile time, and throws `ArithmeticError` otherwise.

:::tip
To store a set of flags, prefer the [`Fields`](/types/fields) type.
:::

## Math functions

```fa
let rounded = value.round()
let down = value.floor()
let up = value.ceil()
let magnitude = value.abs()
let root = value.sqrt()
let biggest = Math.max(a, b)
let smallest = Math.min(a, b)
let bounded = value.clamp(0, 100)
```
