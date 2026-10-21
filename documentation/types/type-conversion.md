# Type conversion

Fa converts between types in three ways: implicitly when it is always safe, explicitly with `as` when it is safe but worth stating, and through a fallible function when it can fail.

## Implicit conversions

An implicit conversion happens with no syntax at all. Fa performs one only when **no information can be lost**.

### Structural widening

Because Fa is [structurally typed](../advanced/structural-typing), a type with more fields is assignable to a type with fewer:

```fa
type Human = { name: String }

type SuperHuman = {
  ...Human
  powers: String
}

let human: Human = SuperHuman {
  name = "John"
  powers = "unlimited"
}
```

What happens to `powers` depends on whether you copied or referenced:

```fa
-- a copy: the extra fields are dropped
let human: Human = superHuman
console.log(human) -- { name = "John" }

-- a reference: nothing is dropped, the value is only *viewed* as a Human
use human: Human = superHuman
```

Function parameters are references, so passing a `SuperHuman` to a function expecting a `Human` never copies and never drops anything:

```fa
function logName = (human: Human) => console.log(human.name)

logName(superHuman) -- no copy, no truncation
```

### Numeric widening

An integer type converts implicitly to a wider one, and to a decimal, when every value fits:

```fa
let small: Int8 = 100
let big: Int64 = small     -- fine: every Int8 fits in an Int64
let value: Decimal = small -- fine
```

Narrowing does not happen implicitly:

```fa
let big: Int64 = 100
let small: Int8 = big -- error: an Int64 may not fit in an Int8
```

Ranged integers make narrowing possible when the range proves it:

```fa
let bounded: Int(min = 0, max = 100) = 100
let small: Int8 = bounded -- fine: the range fits in an Int8
```

### Percentages

A `Percentage` converts implicitly to a `Decimal` when combined with one:

```fa
console.log(40 * 50%) -- 20
```

### Optionals

Any value converts implicitly to its optional:

```fa
let value: Int? = 12
```

The reverse does not: an `Int?` is not an `Int` until it has been narrowed. See [optionals](../values/optionals#narrowing).

## Explicit conversions with `as`

`as` performs a conversion the compiler can prove is safe, but which it will not do on its own — usually because the intent matters more than the safety.

```fa
let animal = Animal { name = "Zeus" }

let human = animal as Human
let value = smallInteger as Int64
let text = character as String
```

`as` is not a cast: it cannot reinterpret a value as something it is not. If the conversion is not provably safe, `as` is a compile error.

```fa
let big: Int64 = readNumber()
let small = big as Int8 -- error: this conversion may lose information
```

## Fallible conversions

When a conversion can genuinely fail, it is an ordinary function returning an error union.

```fa
let parsed = Int.fromString("42")        -- Int | ParseError
let narrowed = big.toInt8()              -- Int8 | ArithmeticError
let colour = Color.fromString(userInput) -- Color | ParseError
```

Handle them like any other error:

```fa
function readPort = (text: String): Int | ParseError => {
  let port = Int.fromString(text)!

  if port < 1 or port > 65535 {
    return ParseError("port out of range")
  }

  return port
}
```

Or supply a default:

```fa
let port = when Int.fromString(text) is {
  Int >> value => value
  Error => 8080
}
```

## Conversions between collections

Collections convert through explicit constructors, because the cost is never negligible:

```fa
let array = [1, 2, 2, 3]

let set = Set(Int) { ...array }        -- { 1, 2, 3 }
let bag = Bag(Int) { ...array }
let backToArray = Array(Int) { ...set }
```

## Numbers and strings

```fa
let text = 42.toString()          -- "42"
let text = "Score: " + 42         -- implicit, only inside string concatenation
let number = Int.fromString("42") -- Int | ParseError
```

String interpolation converts automatically:

```fa
console.log("Score: {score}")
```

## Summary

| Situation | Mechanism | Can fail |
| --- | --- | --- |
| More fields to fewer | implicit | no |
| Narrower number to wider | implicit | no |
| `T` to `T?` | implicit | no |
| `Percentage` to `Decimal` | implicit | no |
| Same shape, different name | `as` | no |
| Wider number to narrower, when provable | `as` | no |
| Wider number to narrower, not provable | `toInt8()`, … | yes |
| Text to number | `Int.fromString` | yes |
| Between collections | constructor | no |
