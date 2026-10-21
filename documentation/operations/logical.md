# Logical operations

Logical operators compare values, combine conditions and test membership.

## Equality

`==` and `!=` compare **values**.

```fa
assert(score == 42)
assert(score != 0)
```

Equality is structural and deep: two objects are equal when all their fields are equal, two collections when they hold equal items in the same order (or, for sets and bags, the same items).

```fa
assert(Point(1, 2) == Point(1, 2))
assert([1, 2, 3] == [1, 2, 3])
```

Object identity is not observable in Fa, because objects live on the stack and have no address you can compare.

## `is`

`is` tests whether a value **matches** a pattern — a type, a literal, a union member, or a range.

```fa
if status is "ready" {
  start()
}

if value is Animal {
  feed(value)
}

if score is 0..60 {
  console.log("failing")
}
```

The difference between `is` and `==` shows up when the right side is a type:

```fa
let animal = Animal { name = "Zeus" }

assert(animal is Animal)  -- yes: `animal` matches the type Animal
assert(animal != Animal)  -- yes: a value is not equal to a type
```

Use `is not` to negate:

```fa
if value is not String {
  console.log("not a string")
}

if user is not none {
  console.log(user.name)
}
```

`is` also **narrows** the value's type for the rest of the branch. See [narrowing](../control-flow/conditionals#narrowing).

## Membership

`is in` tests whether a value is contained in a collection or a [field set](../types/fields):

```fa
if userId is in allowedIds {
  allow()
}

if role is not in ["admin", "moderator"] {
  deny()
}
```

## Comparisons

`<`, `<=`, `>`, `>=` compare ordered values, and can be chained:

```fa
if 2 < a < 12 {
  console.log("in range")
}
```

A chain evaluates each operand once, and short-circuits: `a < b < c` is `a < b and b < c`, with `b` evaluated a single time.

## Negation

`not` negates a value.

```fa
if not hasPermission {
  deny()
}
```

## Boolean logic

`and` and `or` combine conditions, and short-circuit.

```fa
if isLoggedIn and isAdmin {
  showPanel()
}

if isPreview or isDebug {
  enableVerboseLogs()
}
```

`and` binds tighter than `or`, so `a or b and c` is `a or (b and c)`. Parenthesise when it matters.

## Fallback

`??` produces the left value unless it is `none`, in which case it produces the right one:

```fa
let name = user?.name ?? "anonymous"
let port = readPort() ?? 8080
```

`??` reacts to `none` and to errors, and to nothing else. Unlike a truthiness test, it leaves `0`, `""` and empty collections alone:

```fa
let count: Int? = 0
let value = count ?? 10 -- 0, because `count` is not none
```

## Truthiness

In conditions, these values are **falsy**:

- `false`
- `none`
- `0`
- `""`, the empty string
- empty collections
- errors

Everything else is truthy.

```fa
if users {
  -- runs when `users` is a non-empty collection
}

if not isEnabled {
  -- runs when `isEnabled` is false, or none
}
```

:::caution
Truthiness conflates "absent" with "empty" and with "zero". When a value can legitimately be `0`, `""` or empty, test it explicitly:

```fa
if score is not none { }   -- clear: distinguishes 0 from absent
if score { }               -- ambiguous: false for both none and 0
if users.length > 0 { }    -- clear
if users { }               -- true only for a non-empty collection
```

Use truthiness for booleans and for "did this work" checks; use `is none` / `is not none` everywhere else.
:::

## Operator summary

| Operator | Meaning |
| --- | --- |
| `==` `!=` | value equality |
| `is` `is not` | pattern, type or literal match |
| `is in` `is not in` | membership |
| `<` `<=` `>` `>=` | ordering, chainable |
| `and` `or` | boolean logic, short-circuiting |
| `not` | negation |
| `??` | fallback for `none` and errors |
