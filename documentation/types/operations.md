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

The rules of a spread:

- Only object types can be spread. Their public data fields are copied, with their defaults and `readonly`; their methods and `#private` fields are not.
- A field defined twice with the same type is kept once. With different types, it's a compile error, including when one of them is written by hand: a field is never overridden.
- Fields keep the order in which they are written, spreads included. This order is only visible when the type is printed or turned into a [schema](/types/types-at-runtime#typetoschematype-schema).
- A type parameter can be spread, like `...Entity` in a [type function](/types/type-functions). A conflict is then reported where the type function is applied.

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

`|` builds a type accepting a value of **either** side. The sides must be [distinguishable at runtime](/types/unions#members-must-be-distinguishable). Check which side a value is on with the `is` keyword:

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

The rules of a difference:

- The left side is a union, anonymous or [named](/types/unions#named-unions). Each of its members that satisfies the right side is removed. The right side can itself be a union, whose members are removed one by one.
- Removing a member that isn't there is an error.
- From a ranged integer, removing values keeps the remaining ranges: `Integer(min = 0, max = 10) - 0` is `Integer(min = 1, max = 10)`.
- Other types can't be split: `String - "hello"` is an error, since no type means "every string but one".
- A difference of a named union is a named union with the same identity: `Shape - Shape.Point` is a `Shape`, without its `Point` member.
- Removing every member gives `Never`.

To remove fields from an object type, use `Omit`.

## Optional

`?` adds `None` to a type:

```fa
type MaybeInteger = Integer?
-- the same as Optional(Integer), and as Integer | None
```

Since `T?` is a union with `None`, adding `None` twice changes nothing: `Integer??` is `Integer?`.

See [optionals](/values/optionals).

## Special types

| Type | Meaning |
| --- | --- |
| `Never` | no value at all |
| `None` | exactly one value, `none` |
| `Error` | any error |
| `Void` | the result of a function that returns nothing |

There is no type accepting any value: a [generic parameter](/types/generics) or a [union](/types/unions) says which values are expected.

`Never` is the type of an expression that never produces a value: `throw`, `return`, `break` and `continue`, a `loop` without `break`, and a call to a function that returns `Never`. `Never` satisfies every type, so these expressions fit anywhere, and it disappears from a union: `if ready { 1 } else { throw error }` is an `Integer`.

```fa
function runForever = (): Never => {
	loop {
		console.log("still running")
	}
}
```

`Void` is a type with a single, empty value, distinct from `None`. A function that returns nothing returns `Void`. A function declaration simply omits its return type, but a function type writes it, like `(message: String) => Void`, and `Void` can be given to a generic parameter, like `Task(Void)`. A `Void` result can't be stored: `let result = log("done")` is an error. See [defining functions](/functions/defining-functions).

Every member of an [`Errors`](/values/errors) type satisfies `Error`.

## Precedence

From tightest to loosest:

1. `?` (optional suffix)
2. `-` (difference)
3. `|` (union)

`...` is not an operator between types: it only appears as a member of an object type.

Use parentheses when the intent is not obvious:

```fa
type A = (String | Integer)? -- optional of a union
type B = String | Integer? -- String | Integer | None: the same type as A
type C = (TypeA | TypeB) - TypeB -- the difference applies to the whole union
type D = TypeA | TypeB - TypeB -- TypeA | (TypeB - TypeB), so TypeA
```
