# Booleans

Booleans are values that can be either `true` or `false`.

## Basic usage

```fa
let myBoolean = true
let myBoolean: Boolean = true
let isEnabled: Boolean = false
```

## Negation

Use `not` to negate a boolean.

```fa
let hasAccess = false

if not hasAccess {
	console.log("Access denied")
}
```

## Conditions

Conditions (`if`, `while`, `and`, `or`, `not`) only accept booleans. Fa has no "truthy" or "falsy" values: see [Logical operators](/operations/logical).
