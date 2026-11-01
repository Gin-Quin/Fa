# Logical operators

Logical operators are used to compare values, combine conditions, and test membership.

## No truthiness

Fa has no "truthy" or "falsy" values. Conditions (`if`, `while`, `and`, `or`, `not`) only accept values of type `Boolean`. Using any other type in a condition is a compile error.

This removes a whole class of ambiguities: `0`, `""`, an empty collection or `none` are never implicitly converted to `false`. Every check is written explicitly:

```fa
if user is not none {
	console.log(user.name) -- `user` is narrowed to `User`
}

if users.isEmpty() {
	console.log("No users")
}

if name == "" {
	console.log("Empty name")
}

if count == 0 {
	console.log("Nothing to count")
}
```

```fa
let user: User? = findUser(id)

if user { -- error: expected a `Boolean`, found `User?`. Use `user is not none`.
	console.log(user.name)
}
```

## Equality

`==` and `!=` compare **values**:

```fa
assert score == 42
assert score != 0
```

Equality is **structural and deep**: two objects are equal when all their fields are equal, and two collections are equal when they hold equal items. Arrays compare their items in order; sets and pools compare their items without order.

```fa
assert Point { x = 1, y = 2 } == Point { x = 1, y = 2 }
assert [1, 2, 3] == [1, 2, 3]
assert [1, 2, 3] != [3, 2, 1]

let a: Set(Integer) = [1, 2, 3]
let b: Set(Integer) = [3, 2, 1]
assert a == b
```

Identity is not observable in Fa: there is no way to ask whether two values are "the same object". Two values are equal exactly when their contents are equal.

The rules of `==` and `!=`:

- **The operands must have compatible types**: one must be accepted where the other is expected. An optional compares with its base type (`maybeScore == 42` is `false` when `maybeScore` is `none`), and integers of any sizes compare with each other, by their exact values (an `Int8` holding `-1` is not equal to a `UInt8` holding `255`). Comparing a `String` with an `Integer`, a `Float32` with a `Float`, a `Percentage` with a `Float`, or two unrelated object types, is an error.
- **Only data is compared.** Objects compare their data fields; relation fields and `#private` fields are not compared. Functions can't be compared: comparing values whose type contains a function (a function pointer field, for example) is an error. The same goes for tasks, iterators, [references](/advanced/owned-references) and the thread-safe types (`Mutex`, atomics, `Channel`), whose content can change under the reader.
- **Collections**: an `Array` compares its items in order; a `Set`, a `Pool` and a `Map` compare without order (two maps are equal when they have the same keys, with equal values).
- **Floats** follow IEEE 754: `NaN == NaN` is `false` and `-0.0 == 0.0` is `true`. See [floats](/values/numbers#floats).
- **Strings** compare their code points exactly, with no Unicode normalization, as in JavaScript.
- A union value is equal to another when both hold the same member, with equal contents.

`==` and `!=` against a literal or `none` also [narrow](/control-flow/conditionals#narrowing) the value, like `is`.

## Pattern matching with `is`

`is` tests whether a value **matches** a pattern: a literal, `none`, a type, a union member, a [range](/values/ranges) written with `..` or `..=`, or alternatives of these (`answer is "y" | "yes"`). The right side of `is` is always a pattern, never a value: to compare with a value stored in a name, use `==`, and to test whether a value is in a `Range` stored in a name, use `is in`.

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

if score is 90..=100 {
	console.log("excellent")
}
```

Use `is not` to negate an `is` check:

```fa
if status is not "ready" {
	wait()
}

if user is not none {
	console.log(user.name)
}
```

`is` also **narrows** the type of the value in the branch where the match succeeded. See [narrowing](/control-flow/conditionals#narrowing) and [optionals](/values/optionals).

`is` with a type is a test on a union: it narrows a union to one of its members. On a value whose type is not a union, it is decided at compile time: a test that is always true is a warning, and a test that is never true is an error. To be tested at runtime, the members of an anonymous union must be distinguishable: they must have different runtime kinds (number, string, boolean, `none`, collection, object), or their object members must differ by a required field or by a literal field. Otherwise, the union is an error, and the compiler suggests a named [`Union`](/types/unions), whose members carry a tag.

### `is` versus `==`

`==` compares two values. `is` matches a value against a pattern. The difference is visible when the right side is a type:

```fa
let living: Animal | Plant = findLiving()

if living is Animal { } -- valid: `living` matches the type `Animal`
if living == Animal { } -- error: `==` compares two values, and `Animal` is a type. Use `is`.
```

With a literal, `status == "ready"` and `status is "ready"` give the same result, and both narrow the type of the value.

## Membership

Use `is in` to check if a value is contained in a collection:

```fa
if userId is in allowedIds {
	allow()
}
```

Use `is not in` to negate membership:

```fa
if role is not in ["admin", "moderator"] {
	deny()
}
```

The right side of `is in` is an `Array`, a `Set`, a `Pool`, a `Columnar` or one of their growing versions (an element equal to the value), a `Map` (a key equal to the value), or a `Range` (an integer between its bounds). To search a string, use [`includes`](/operations/string). In an array, an element is found when it is `==` to the value, so a NaN is never found; sets and maps use the [SameValueZero](/values/numbers#floats) rule of their keys. Membership in a literal array narrows the value to the union of its literals: inside `if role is in ["admin", "moderator"]`, `role` is `"admin" | "moderator"`.

## Comparisons

Comparison operators are `>`, `>=`, `<`, `<=`. They can be chained:

```fa
if 0 < x <= 100 {
	console.log("in range")
}
```

A chain is evaluated like a sequence of comparisons joined by `and`, but **each operand is evaluated once**: `a < f() <= c` calls `f` a single time. The chain short-circuits: if a comparison is `false`, the following operands are not evaluated.

A chain goes in one direction: `<` and `<=`, or `>` and `>=`. Mixing directions (`a < b > c`) is an error, and so is chaining `==` or `!=` (`a == b == c`).

The ordered types are the numbers (integers between themselves, floats of the same type, big integers, percentages), strings and characters (by Unicode code point, see [comparing strings](/operations/string#comparing)). Booleans, tuples, objects and collections are not ordered: compare their parts explicitly. A comparison in parentheses is a `Boolean`, so `(a < b) < c` is an error, while `(a < b) == c` compares two booleans.

## Negation

`not` negates a `Boolean`. It cannot be used on any other type. It is the only negation operator.

```fa
if not hasPermission {
	deny()
}

if not items.isEmpty() {
	console.log("not empty")
}
```

`not` has a low precedence: `not a == b` is `not (a == b)`. It binds tighter than `and` and `or`: `not a and b` is `(not a) and b`. See [order of operations](/operations/arithmetic#order-of-operations).

## Boolean logic

Use `and` and `or` to combine conditions. Both operands must be `Boolean`.

```fa
if isLoggedIn and isAdmin {
	showPanel()
}

if isPreview or isDebug {
	enableVerboseLogs()
}
```

Both operators **short-circuit**: the right operand is only evaluated when it is needed. `a and b` doesn't evaluate `b` when `a` is `false`, and `a or b` doesn't evaluate `b` when `a` is `true`.

```fa
if user is not none and user.isAdmin {
	-- `user.isAdmin` is only evaluated when `user` is not none
}
```

`and` binds tighter than `or`: `a or b and c` is `a or (b and c)`. Use parentheses when you mean something else, or when it makes the intent clearer.

## Fallback for optionals

`??` returns the left value unless it is `none`, in which case it returns the right one:

```fa
let name = user?.name ?? "anonymous"
let port = readPort() ?? 8080
```

`??` only works on [optionals](/values/optionals): on any other value, it is an error. It reacts to `none` and to nothing else: `0`, `""` and empty collections are regular values. The right side is only evaluated when the left side is `none`. The result has the type of the left side without `None`, joined with the type of the right side: it is optional only when the right side is.

```fa
let count: Integer? = 0
let value = count ?? 10 -- 0, because `count` is not none
```

`??` never handles errors. To provide a fallback for a function that throws, use `catch`:

```fa
let config = loadConfig() catch error => defaultConfig
```

See [errors](/values/errors).

## Summary

| Operator | Meaning |
| --- | --- |
| `==` `!=` | deep structural equality |
| `is` `is not` | pattern, type, literal or range match, with narrowing |
| `is in` `is not in` | membership |
| `<` `<=` `>` `>=` | ordering, chainable |
| `not` | negation of a `Boolean` |
| `and` `or` | boolean logic, short-circuiting, `and` before `or` |
| `??` | fallback for `none` (optionals only) |
