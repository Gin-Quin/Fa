# Optionals

An **optional** value may hold either a value or `none`.

`none` is the single value of the type `None`, and `T?` is shorthand for `Optional(T)`, which is the union `T | None`:

```fa
type MaybeInteger = Integer?
type SameMaybeInteger = Optional(Integer)
type AlsoMaybeInteger = Integer | None
```

All three declarations are the same type.

### Nested optionals

`T??` is `T?`: an optional of an optional is the same optional, since `T | None | None` is `T | None`. There is a single `none`, which can't say which level is absent.

So `get` on a collection of optionals returns a single optional: with `let values: Array(Integer?) = [1, none]`, both `values.get(1)` (a stored `none`) and `values.get(5)` (no element) are `none`. When the difference matters, check the index first (`if index < values.length`), or use a union with a named member instead of `none`.

## Declaring optionals

```fa
-- an optional that holds none
let missingDog: Dog? = none

-- an optional that holds a value
let foundDog: Dog? = Dog { name = "Rex" }
```

A value that is not optional can never be `none`: the compiler rejects `let dog: Dog = none`.

:::note
`None` is the type of an absent value, not the return type of a function that returns nothing. In a function type, a function that returns nothing is written with `Void`: `(message: String) => Void`. `Void` is a unit type that can't hold a value you use; see [tuples](/values/tuples#no-unit-tuple) and [defining functions](/functions/defining-functions).
:::

## Testing for none

Use `is none` and `is not none`:

```fa
mutable sum: Integer? = none

if sum is none {
	sum = 0
}

console.log(sum) -- 0
```

`none` is not `false`, and Fa has no truthiness: an optional cannot be used directly as a condition.

```fa
let dog: Dog? = findDog()

if dog { -- error: expected a `Boolean`, found `Dog?`
}

if dog is not none { -- OK
	dog.bark()
}
```

This also means that `0`, `""` and an empty collection are never confused with `none`:

```fa
let sum: Integer? = 0

assert sum is 0        -- sum holds 0
assert sum is not none -- sum is not none
```

See [logical operators](/operations/logical).

## Narrowing

Once you have tested an optional, with `is none`, `is not none`, `== none` or `!= none`, the compiler narrows its type for the rest of the branch:

```fa
let dog: Dog? = findDog()

if dog is not none {
	dog.bark() -- `dog` is a `Dog` here
}
```

Narrowing also works after an early return: if the `none` case leaves the function, the value is no longer optional for the rest of it.

```fa
function greet = (user: User?) => {
	if user is none {
		return
	}

	-- here `user` has type `User`, not `User?`
	console.log(user.name)
}
```

The rules of narrowing (which places can be narrowed, and what ends a narrowing) are described in [narrowing](/control-flow/conditionals#narrowing). In short: local values, parameters and the paths of their fields are narrowed, and an assignment or a call that may mutate the value ends the narrowing.

`when` narrows the same way, in each branch, when its subject is a plain name:

```fa
when user is {
	none => console.log("nobody")
	User => console.log(user.name)
}
```

## Safe access

The `?.` operator reads through an optional without narrowing it first. It evaluates to `none` when the left side is `none`:

```fa
user?.name    -- field access: String?
user?.greet() -- method call: none if there is no user
```

`?.` short-circuits the whole chain on its right, like in JavaScript: when `user` is `none`, `user?.address.city` is `none`, and `.address.city` is not evaluated. The result is optional once, whatever the length of the chain. A `?.` is only needed where a value of the chain is itself optional:

```fa
let city = user?.address?.city -- String?: `user` and `address` are both optional
```

`?.` is the only safe access operator: there is no `?[...]` or `?(...)`. To index an optional collection or to call an optional function pointer, narrow it first:

```fa
let scores: Array(Integer)? = loadScores()

if scores is not none {
	console.log(scores.get(0))
}
```

## Fallback values

Use `??` to supply a fallback when an optional is `none`:

```fa
let city = user?.address?.city ?? "unknown" -- String
let firstItem = items.get(0) ?? 0           -- Integer
```

The right side is only evaluated when the left side is `none`. The result is optional only when the right side is optional. `??` binds looser than `and` and `or`: see [order of operations](/operations/arithmetic#order-of-operations).

`??` only applies to optionals. It cannot be used on a call that can fail: errors are handled with `try` and `catch`. See [errors](/values/errors).

## Optionals and errors

Optionals and errors are two separate mechanisms:

- an **optional** (`User?`) represents **absence**, which is a normal, expected outcome: a lookup that found nothing,
- an **error** (`throws NotFound`) means that the operation **failed**, and the caller may want to know why.

```fa
function findUser = (id: Integer): User? => ...              -- may simply not exist
function loadUser = (id: Integer): User throws ApiError => ... -- may fail, with a reason

let name = findUser(12)?.name ?? "anonymous" -- absence: `?.` and `??`
let user = try loadUser(12)                  -- failure: propagate the error to the caller
```

Many collection operations come in both forms: `items[index]` throws `IndexError` when the index cannot be proven in bounds (see [range analysis](/values/numbers#range-analysis)), while `items.get(index)` returns an optional. See [arrays](/collections/arrays).
