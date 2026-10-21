# Type operations

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

You can check which type a value is with the `is` keyword:

```fa
type MyType = "hello" | "world"

let value: MyType = "hello"

if value is "hello" {
	console.log("hello")
} else if value is "world" {
	console.log("world")
}
```

## Difference

The `-` operator removes members from a union:

```fa
type TypeA = "hello" | "world" | "you" | "fine?"
type TypeB = "hello" | "you"

type MyType = TypeA - TypeB
-- MyType is "world" | "fine?"
```

To remove fields from an object type, use `Omit`.
