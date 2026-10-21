# Optionals

An **optional** value may hold either a value or `none`.

`none` is the single value of the type `None`, and `T?` is shorthand for the union `T | None`:

```fa
type MaybeInt = Int?
type MaybeInt = Optional(Int)
type MaybeInt = Int | None
```

All three declarations are the same type.

## Declaring optionals

```fa
-- an optional that holds none
let dog: Dog? = none

-- an optional that holds a value
let dog: Dog? = Dog { name = "Rex" }
```

An optional declared without a value holds `none`:

```fa
mutable sum: Int? = none
```

`None` is also the type of a function that returns nothing:

```fa
function logIt = (value: String): None => {
  console.log(value)
}
```

Writing `: None` is optional — a function with no returned value has that type already.

## Testing for none

Use `is none` and `is not none`:

```fa
mutable sum: Int? = none

if sum is none {
  sum = 0
}

console.log(sum) -- 0
```

`none` is falsy, so a bare truthiness test also works — but be careful, because `0` and `""` are falsy too:

```fa
let sum: Int? = 0

assert(sum is 0)         -- yes, sum holds 0
assert(sum is not none)  -- yes, sum is not none
assert(not sum)          -- also yes: 0 is falsy
```

:::caution
When a value can legitimately be `0`, `""` or an empty collection, always test it with `is none` / `is not none` rather than with truthiness. See [truthiness](../operations/logical#truthiness).
:::

## Narrowing

Once you have tested an optional, the compiler narrows its type for the rest of the branch:

```fa
function greet = (user: User?) => {
  if user is none {
    return
  }

  -- here `user` has type `User`, not `User?`
  console.log(user.name)
}
```

`when` narrows the same way:

```fa
when user is {
  none => console.log("nobody")
  User => console.log(user.name)
}
```

## Safe navigation

The `?` operator reads through an optional without unwrapping it first. It comes in three forms:

```fa
user?.name        -- field access
user?.greet()     -- method call
users?[0]         -- collection access
```

Each of them evaluates to `none` when the left side is `none`, and they chain:

```fa
let city = user?.address?.city -- String?
```

Use `??` to supply a fallback:

```fa
let city = user?.address?.city ?? "unknown" -- String
```

## Optionals and errors

`?` deals with **absence**. Errors are a separate mechanism, handled with the `!` operator. See [errors](./errors).

```fa
let name = user?.name   -- none if there is no user
let score = loadScore()! -- propagate the error to the caller
```
