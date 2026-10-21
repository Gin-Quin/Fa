# Booleans

A `Boolean` is either `true` or `false`.

```fa
let myBoolean = true
let myBoolean: Boolean = true
let isEnabled: Boolean = false
```

## Negation

Use `not` to negate a value:

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

Use `and` and `or`:

```fa
if isLoggedIn and isAdmin {
  showPanel()
}

if isPreview or isDebug {
  enableVerboseLogs()
}
```

Both operators short-circuit: the right operand is evaluated only if it can change the result.

See [logical operations](../operations/logical) for comparisons, membership and truthiness.
