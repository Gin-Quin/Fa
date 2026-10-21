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

Equality is **structural and deep**: two objects are equal when all their fields are equal, and two collections are equal when they hold equal items. Arrays compare their items in order; sets and bags compare their items without order.

```fa
assert Point { x = 1, y = 2 } == Point { x = 1, y = 2 }
assert [1, 2, 3] == [1, 2, 3]
assert [1, 2, 3] != [3, 2, 1]

let a: Set(Integer) = [1, 2, 3]
let b: Set(Integer) = [3, 2, 1]
assert a == b
```

Identity is not observable in Fa: there is no way to ask whether two values are "the same object". Two values are equal exactly when their contents are equal.

## Pattern matching with `is`

`is` tests whether a value **matches** a pattern: a literal, a type, a union member or a [range](/values/ranges).

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

### `is` versus `==`

`==` compares two values. `is` matches a value against a pattern. The difference is visible when the right side is a type:

```fa
let animal = Animal { name = "Zeus" }

assert animal is Animal -- valid: `animal` matches the type `Animal`
assert animal == Animal -- error: `==` compares two values, and `Animal` is a type. Use `is`.
```

With a literal, `status == "ready"` and `status is "ready"` give the same result, but only `is` narrows the type of the value.

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

## Comparisons

Comparison operators are `>`, `>=`, `<`, `<=`. They can be chained:

```fa
if 0 < x <= 100 {
	console.log("in range")
}
```

A chain is evaluated like a sequence of comparisons joined by `and`, but **each operand is evaluated once**: `a < f() <= c` calls `f` a single time. The chain short-circuits: if a comparison is `false`, the following operands are not evaluated.

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

`not` has a low precedence: `not a == b` is `not (a == b)`. See [order of operations](/operations/arithmetic#order-of-operations).

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

`??` only works on [optionals](/values/optionals). It reacts to `none` and to nothing else: `0`, `""` and empty collections are regular values.

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
