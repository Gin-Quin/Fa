# Type operations

Types can be combined with operators. Every operator below works at compile time and produces a new type.

## Spread

Object types are composed with the spread syntax `...`, the same syntax used to spread values:

```fa
type TypeA = { foo: String }
type TypeB = { ...TypeA, bar: Integer }

-- TypeB is { foo: String, bar: Integer }
```

Several types can be spread, one per line:

```fa
type Timestamp = { timestamp: Integer }
type Auth = { token: String }

type AuthenticatedRequest = {
	...Request
	...Timestamp
	...Auth
	retries: Integer
}
```

Fields are added in the order of the spreads. If two spread types define the same field with different types, it's a compile error.

Spread is the only way to compose object types: Fa has no intersection operator.

To spread only some fields of a type, use the `Pick` and `Omit` [type functions](/types/type-functions):

```fa
type TypeA = { foo: String, bar: Integer }

type TypeB = {
	...Pick(TypeA, { foo })
	someNumber: Integer
}
-- TypeB is { foo: String, someNumber: Integer }

type TypeC = {
	...Omit(TypeA, { foo })
	baz: Boolean
}
-- TypeC is { bar: Integer, baz: Boolean }
```

## Union

```fa
type MyType = { foo: String, bar: Integer } | { foo: Integer, baz: String }
-- MyType is either { foo: String, bar: Integer } or { foo: Integer, baz: String }

type MyType2 = "hello" | "world"
-- MyType2 is "hello" or "world"
```

`|` builds a type accepting a value of **either** side. Check which side a value is on with the `is` keyword:

```fa
type MyType = "hello" | "world"

let value: MyType = "hello"

if value is "hello" {
	console.log("hello")
} else if value is "world" {
	console.log("world")
}
```

See [unions](/types/unions).

## Difference

The `-` operator removes members from a union:

```fa
type TypeA = "hello" | "world" | "you" | "fine?"
type TypeB = "hello" | "you"

type MyType = TypeA - TypeB
-- MyType is "world" | "fine?"
```

To remove fields from an object type, use `Omit`.

## Optional

`?` adds `None` to a type:

```fa
type MaybeInteger = Integer?
-- the same as Optional(Integer), and as Integer | None
```

See [optionals](/values/optionals).

## Special types

| Type | Meaning |
| --- | --- |
| `Any` | any value at all |
| `Never` | no value at all |
| `None` | exactly one value, `none` |
| `Error` | any error |
| `Void` | in a function type, the return type of a function that returns nothing |

`Never` is the return type of a function that never returns:

```fa
function runForever = (): Never => {
	while true {
		console.log("still running")
	}
}
```

`Void` only appears in function types, like `(message: String) => Void`. A function declaration that returns nothing simply omits its return type. See [defining functions](/functions/defining-functions).

## Precedence

From tightest to loosest:

1. `?` (optional suffix)
2. `...` (spread)
3. `-` (difference)
4. `|` (union)

Use parentheses when the intent is not obvious:

```fa
type A = (String | Integer)? -- optional of a union
type B = String | Integer? -- String | Integer | None: the same type as A
type C = (TypeA | TypeB) - TypeB -- the difference applies to the whole union
```
