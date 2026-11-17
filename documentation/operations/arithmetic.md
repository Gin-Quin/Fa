# Arithmetic

Arithmetic operators work on numbers: integers, floats, big integers and percentages. See [numbers](/values/numbers).

Both operands must be of the same kind: Fa never converts numbers implicitly. Integers of different sizes mix freely, because every integer type is a [range of integers](/values/numbers#integer-types-are-ranges). Convert explicitly with a call, and let literals adapt to the expected type:

```fa
let count = 3
let ratio = 0.5

let scaled = count * ratio           -- error: `Integer * Float`. Use `Float(count) * ratio`
let converted = Float(count) * ratio -- 1.5
let doubled = ratio * 2              -- 1.0: the literal `2` adapts to `Float`
```

The type of each operation, for each kind of operand, is given in [result types](/values/numbers#result-types).

## Operators

| Operator | Meaning | Example |
| --- | --- | --- |
| `+` | addition | `4 + 6` |
| `-` | subtraction | `10 - 3` |
| `*` | multiplication | `7 * 6` |
| `**` | exponentiation | `7 ** 6` |
| `/` | division, returns a `Float` on integers | `22 / 7` |
| `//` | floor division, returns an `Integer` on integers | `22 // 7` |
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

`/` is true division. On integers, it always returns a `Float`, even when both operands are integers: both are converted to `Float` first (which rounds 64-bit values beyond 2^53). On floats, it returns the same float type:

```fa
let ratio = 22 / 7 -- Float
let half = 4 / 2   -- 2.0, a Float
```

`//` is floor division: it rounds the quotient down, towards negative infinity. On integers, it returns an integer:

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

On floats, `//` and `%%` follow the same rules and return a float: `7.5 // 2.0` is `3.0`.

### Division by zero

Fa has no runtime crash for division by zero. The divisor of an integer `//` or `%%` (and of a `BigInt` one) must be provably non-zero.

When the compiler can prove it, from a literal or from a [ranged integer](/values/numbers#minimum-and-maximum-values), the operation can't fail:

```fa
let a = 10 // 2 -- Integer

let divisor: Integer<min = 1> = readPositive()
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

`/` never throws. On integers, it computes a `Float`, and floats follow IEEE 754: `1 / 0` is an infinity, and `0.0 / 0.0` is NaN. See [floats](/values/numbers#floats).

The proofs follow the rules of [range analysis](/values/numbers#range-analysis).

## Overflow

An integer operation overflows when its result doesn't fit its type, like `200 + 100` in a `UInt8` (maximum `255`).

The compiler tracks the possible range of every arithmetic result: the ranges of the operands propagate through each operation.

- When an overflow is **certain**, it is a compile error.
- When an overflow is **impossible**, no check is generated.
- When an overflow is **possible** but not certain, the operation is checked at runtime in development (`fa run`, `fa test`, `fa develop`): if it overflows, the program [stops](/values/errors#stops) with an error report. A release build (`fa build`) doesn't check it by default, like Rust: the result wraps around. See [overflow](/values/numbers#overflow).

```fa
let a: UInt8 = 200 + 100 -- error: 300 doesn't fit in a UInt8

let small: Integer<min = 0, max = 100> = readPercent()
let bounded: Integer<min = 0, max = 200> = small * 2 -- no check: the result is between 0 and 200

let x: UInt8 = readByte()
let y: UInt8 = readByte()
let sum: Integer = x + y -- no check: the result is between 0 and 510
let byte: UInt8 = x + y  -- checked at runtime: stops the program if the result exceeds 255
```

On JavaScript targets, an `Integer` is checked against ±(2^53 − 1) instead: see [compatibility with JavaScript](/values/numbers#compatibility-with-javascript).

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

See [percentages](/values/numbers#percentages) and the [table of their operations](/values/numbers#result-types).

## Compound assignment

Every arithmetic operator has a compound assignment form: `+=`, `-=`, `*=`, `**=`, `/=`, `//=` and `%%=`. `.=` updates several fields of an object at once: see [updating several fields](/values/objects#updating-several-fields).

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

A compound assignment is subject to the same range and division checks as the operator it uses. When it can fail, because its target is an index or a key that isn't proven, or because its operation throws, it is prefixed with `try`: `try counts[key] += 1`, `try total //= divisor`. A `catch` can't follow an assignment (it would only cover the value on its right): to handle the error, write `total = total // divisor catch error => total`.

Assignments don't chain: `a = b = c` and `x += y += z` are errors.

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
| 1 | access and call: `a.b`, `a?.b`, `a(b)`, `a[b]`, and partial application `a with (b = 1)` |
| 2 | postfix `%` |
| 3 | prefix `-`, `copy` |
| 4 | `**` (right-associative) |
| 5 | `*`, `/`, `//`, `%%` |
| 6 | `+`, `-` |
| 7 | ranges: `..`, `..=` |
| 8 | comparisons: `<`, `<=`, `>`, `>=` (chainable) |
| 9 | `==`, `!=`, `is`, `is not`, `is in`, `is not in` |
| 10 | `not` |
| 11 | `and` |
| 12 | `or` |
| 13 | `??` |
| 14 | `then` |
| 15 | prefix keywords `try`, `await`, `start` |
| 16 | `catch` |
| 17 | `return`, `throw`, `break` |
| 18 | assignments: `=`, `+=` and the other compound assignments, `.=` |

`**` is right-associative (`2 ** 3 ** 2` is `2 ** 9`); the other binary operators are left-associative.

Because the prefix `-` binds tighter than `**`, `-x ** 2` is `(-x) ** 2`. Write `-(x ** 2)` for the opposite of the square.

Ranges bind looser than arithmetic, so `0..count + 1` is `0..(count + 1)`.

`??` binds looser than the boolean operators: `a ?? b or c` is `a ?? (b or c)`.

The keywords at the bottom of the table cover everything on their right:

- `try`, `await` and `start` cover the whole expression on their right, up to a `catch`: `try 10 // divisor` is `try (10 // divisor)`.
- `catch` covers every failing call of its left operand, and its handler extends as far right as possible, like a lambda body: `parse(read(path)) catch error => fallback + 1` handles the errors of `read` and of `parse`, and returns `fallback + 1`.
- Writing `try` and `catch` on the same expression is an error, and so is a `try` inside the left operand of a `catch`.
- `try` also prefixes an assignment that can fail: `try scores[index] = 0`. It is an error before an assignment that can't fail: write `x = try f()`, not `try x = f()`.
- `return`, `throw` and `break` cover everything on their right.

`not` binds tighter than `and` and `or`: `not a and b` is `(not a) and b`.

In the patterns of a `when` branch, `|` is the loosest, and `as` applies to the whole alternative. After `is` and `is not`, the alternatives separated by `|` form one pattern: `x is 1 | 2 and ready` is `(x is (1 | 2)) and ready`.

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

The bitwise methods work on the bits of the receiver's type, and return a value of that type: `bitAnd`, `bitOr` and `bitXor` take an operand of the same type. The result has the full range of the type: ranges don't propagate through bit operations (`bitAnd` on an `Integer<min = 0, max = 10>` returns an `Integer`). Shifts are logical for unsigned integers (the new bits are zeros) and arithmetic for signed integers (`shiftRight` keeps the sign bit). `shiftLeft` drops the bits shifted out, with no overflow check. The shift amount must be between 0 and the width of the type, excluded: an amount proven to be in this range can't fail, an amount proven outside is a compile error, and any other amount `throws ArithmeticError`.

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

| Function | On | Returns |
| --- | --- | --- |
| `round()`, `floor()`, `ceil()`, `truncate()` | floats | the same float type (`2.5.round()` is `3.0`); use `Integer.round(value)` to get an integer |
| `sqrt()` | floats | the same float type; NaN for a negative value |
| `abs()` | integers, floats, `BigInt`, `Percentage` | the same type; on integers, follows the range rules (`Int8.abs()` of `-128` doesn't fit) |
| `Math.max(a, b)`, `Math.min(a, b)` | two numbers of the same kind | that kind, with the range of the result on integers |
| `clamp(min, max)` | integers, floats | the receiver's type; on integers, the result is known to be between `min` and `max` |
| `isNaN()`, `isFinite()` | floats | `Boolean` |

`Math` is a namespace that every file sees. Rounding halfway cases goes away from zero (`(-2.5).round()` is `-3.0`).

:::implementation
## Implementation details

### Native

- `//` and `%%` on signed integers compute the truncated quotient and remainder, then adjust by one when the remainder is non-zero and its sign differs from the divisor's. `sdiv` and `srem` are undefined for the minimum value divided by `-1`: `//` checks it first (an overflow, so a stop), and `%% -1` returns `0` without executing `srem`.
- On native targets, a 64-bit signed remainder in a loop may use an unsigned reciprocal when an immutable divisor declared outside the loop is a positive constant. A guard selects this path only for a nonnegative dividend; negative dividends retain the signed floor remainder. The existing invariant-divisor path handles runtime divisors, with both sign guards. Division-by-zero and minimum/-1 checks precede either path. Wasm retains its ordinary division lowering.
- Narrow arithmetic is permitted only when the storage ranges of every operand and every intermediate fit the selected compute width. The result range is tracked separately: a small remainder never justifies truncating its dividend or divisor, and a later multiplication by zero never justifies overflowing an earlier step. Each nested expression proves its own compute width before its result is converted for its parent; this also applies with overflow checks disabled.
- Overflow checks use the overflow intrinsics (`llvm.sadd.with.overflow`, ...) or a compare against the bounds of the expected type, with a cold branch to the stop routine.
- Shifts by an unproven amount compare the amount with the width first; `shiftLeft` and `shiftRight` are `shl`, `lshr` and `ashr`.

### JavaScript

- `//` and `%%` on numbers are computed together, exactly for every safe integer: `let r = a % b; let q = (a - r) / b; if (r !== 0 && (r < 0) !== (b < 0)) { q -= 1; r += b }`. `q` is `a // b` and `r` is `a %% b`. (`Math.floor(a / b)` and `((a % b) + b) % b` are wrong near 2^53.) On `BigInt` values, `//` is `a / b` adjusted by one when the signs differ and the remainder is non-zero, and `%%` uses the same adjustment.
- `**` on integers is `**` followed by the overflow check; on floats, it is `Math.pow`.
- JavaScript bitwise operators work on 32 bits: on a type of up to 32 bits, the methods compile to `&`, `|`, `^`, `~`, `<<`, `>>` and `>>>`, then wrap the result to the type (`& 0xFF`, `<< 24 >> 24`, ...). On an `Integer`, they go through `BigInt` and `BigInt.asIntN(64, ...)`, then back to a number with an overflow check against ±(2^53 − 1).
- `Math.max`, `Math.min`, `sqrt`, `floor` and `ceil` are the JavaScript functions; `round` rounds halfway cases away from zero (`Math.sign(x) * Math.round(Math.abs(x))`), not like `Math.round`.
:::
