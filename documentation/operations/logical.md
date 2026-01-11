# Logical operators

Logical operators are used to compare values, combine conditions, and test membership.

## Truthiness

In `if` and other conditional contexts, these values are falsy:

- `false`
- `null`
- `0`
- `""` (empty string)
- empty collections
- errors

Everything else is truthy.

For optional collections, strings, numbers or boolans, use `is not null` when you want to accept falsy values but reject `null`.

```fa
let users: Array(User)?

if users is not null {
  -- users can still be an empty array here
}
```

```fa
if user {
  -- user is truthy (not null)
}

if not isEnabled {
  -- runs when isEnabled is false or null
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

Use `is not` or `is no` to negate an `is` check.

```fa
if user is not null {
  console.log(user.name)
}
```

Use `==` and `!=` for direct value equality and inequality.

```fa
assert(score == 42)
assert(score != 0)

let animal = Animal()

assert(animal is Animal) -- works, animal is an instance of Animal
assert(animal == Animal) -- does not work, animal does not equal Animal
```

## Negation

`not` and `no` have the same effect and negate a value.

```fa
if not hasPermission {
  deny()
}

if no items {
  console.log("empty")
}
```

## Boolean logic

Use `and` and `or` to combine conditions.

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
