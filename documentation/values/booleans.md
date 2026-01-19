# Booleans

Booleans are values that can be either `true` or `false`.

## Basic usage

```fa
let myBoolean = true
let myBoolean: Boolean = true
let isEnabled: Boolean = false
```

## Comparisons

```fa
let age = 21
let isAdult = age >= 18

let temperature = 32
let isFreezing = temperature <= 32
```

## Conditionals

```fa
let isLoggedIn = true

if isLoggedIn {
	console.log("Welcome back")
}
```

## Negation

You can either use `not` or `no` to negate a value.

```fa
let hasAccess = false

if not hasAccess {
	console.log("Access denied")
}

if no hasAccess {
	console.log("Access denied")
}
```

## Functions that return booleans

```fa
function isPositive = (value: Integer): Boolean => value > 0

isPositive(12) -- true
isPositive(-3) -- false
```

## Booleans inside objects

```fa
let featureFlags = {
	debug = true
	telemetry = false
}
```
