# Arithmetic

Arithmetic operators work on numbers (integers, decimals, and big integers).

## Basic operations

```fa
let sum = 4 + 6
let diff = 10 - 3
let product = 7 * 6
let power = 7 ** 6
let division = 22 / 7
let quotient = 22 // 7
```

## Modulo

In Fa, the `%` operator is used to calculate a percentage, and the `modulo` keyword is used to calculate the remainder.

```fa
let remainder = 10 modulo 3 -- 1
let isEven = (value: Integer): Boolean => value modulo 2 == 0
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
count /= 4
count %= 5
```
