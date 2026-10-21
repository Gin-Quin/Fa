# Logical operators

Logical operators are used to compare values, combine conditions, and test membership.

## No truthiness

Fa has no "truthy" or "falsy" values. Conditions (`if`, `while`, `and`, `or`, `not`) only accept values of type `Boolean`. Using any other type in a condition is a compile error.

This removes a whole class of ambiguities: `0`, `""`, an empty collection or `none` are never implicitly converted to `false`. Every check is written explicitly:

```fa
if user is not none {
  console.log(user.name) -- `user` is narrowed to `User`
}

if users.isEmpty {
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

## Equality and type checks

Use `is` to check value equality or type membership.

```fa
if status is "ready" {
  start()
}

if value is Animal {
  feed(value)
}
```

Use `is not` to negate an `is` check.

```fa
if status is not "ready" {
  wait()
}

if user is not none {
  console.log(user.name)
}
```

Use `==` and `!=` for direct value equality and inequality.

```fa
assert score == 42
assert score != 0

let animal = Animal()

assert animal is Animal -- works, animal is an instance of Animal
assert animal == Animal -- does not work, animal does not equal Animal
```

## Negation

`not` negates a `Boolean`. It cannot be used on any other type.

```fa
if not hasPermission {
  deny()
}

if not items.isEmpty {
  console.log("not empty")
}
```

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

## Membership

Use `is in` to check if a value is contained in a collection.

```fa
if userId is in allowedIds {
  allow()
}
```

Use `is not in` to negate membership.

```fa
if role is not in ["admin", "moderator"] {
  deny()
}
```

## Comparisons

Comparison operators are `>`, `>=`, `<`, `<=`. They can be chained.

```fa
if 2 < a < 12 {
  console.log("in range")
}
```
