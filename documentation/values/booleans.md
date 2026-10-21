# Booleans

A `Boolean` is either `true` or `false`.

## Basic usage

```fa
let myBoolean = true
let isVisible: Boolean = true
let isEnabled: Boolean = false
```

## Negation

Use `not` to negate a boolean. `not` is the only negation operator.

```fa
let hasAccess = false

if not hasAccess {
	console.log("Access denied")
}
```

`not` is also used to negate an `is` check:

```fa
if value is not String {
	console.log("Not a string")
}

if userId is not in allowedIds {
	console.log("Not allowed")
}
```

## Combining conditions

Use `and` and `or`. `and` binds tighter than `or`.

```fa
if isLoggedIn and isAdmin {
	showPanel()
}

if isPreview or isDebug {
	enableVerboseLogs()
}
```

Both operators short-circuit: the right operand is evaluated only if it can change the result. In `isLoggedIn and isAdmin`, `isAdmin` is not evaluated when `isLoggedIn` is `false`; in `isPreview or isDebug`, `isDebug` is not evaluated when `isPreview` is `true`.

## Conditions

Conditions (`if`, `while`, `and`, `or`, `not`) only accept booleans. Fa has no "truthy" or "falsy" values: an optional is checked with `is none` / `is not none`, a number is compared explicitly (`count > 0`). See [Logical operators](/operations/logical).
