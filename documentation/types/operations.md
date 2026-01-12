# Type operations

## Composability

```fa
type TypeA = { foo: String }
type TypeB = { ...TypeA, bar: Integer }

-- TypeB is { foo: String, bar: Integer }
```

You can compose some values from another type using the extract `>>` operator:

```fa
type TypeA = { foo: String, bar: Integer }

type TypeB = {
  ...TypeA >> { foo }
  someNumber: Integer
}

-- TypeB is { foo: String, someNumber: Integer }
```

## Union

```fa
type MyType = { foo: String, bar: Integer } | { foo: Integer, baz: String }
-- MyType is either { foo: String, bar: Integer } or { foo: Integer, baz: String }

type MyType = "hello" | "world"
-- MyType is "hello" or "world"
```

You can check which type a value is with the `is` keyword:

```fa
type MyType = "hello" | "world"

let value: MyType = "hello"

if value is "hello" {
  log("hello")
} else if value is "world" {
  log("world")
}
```

## Intersection

```fa
type TypeA = { foo: String, bar: Integer }
type TypeB = { foo: Integer, baz: String }

type MyType = TypeA & TypeB
-- MyType is { foo: Integer }
```

## Difference

```fa
type TypeA = { foo: String, bar: Integer }
type TypeB = { foo: Integer, baz: String }

type MyType = TypeA - TypeB
-- MyType is { bar: Integer }
```

It works with literal types as well:

```fa
type TypeA = "hello" | "world" | "you" | "fine?"
type TypeB = "hello" | "you"

type MyType = TypeA - TypeB
-- MyType is "world" | "fine?"
```
