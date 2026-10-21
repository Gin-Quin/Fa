# Type operations

Types can be combined with operators. Every operator below works at compile time and produces a new type.

## Composition

Spread a type into another with `...`:

```fa
type TypeA = { foo: String }
type TypeB = { ...TypeA, bar: Int }

-- TypeB is { foo: String, bar: Int }
```

Pick which fields to spread with the extract operator `>>` and a [key set](./fields):

```fa
type TypeA = { foo: String, bar: Int }

type TypeB = {
  ...TypeA >> @{ foo }
  someNumber: Int
}

-- TypeB is { foo: String, someNumber: Int }
```

## Union

`|` builds a type accepting a value of **either** side:

```fa
type MyType = { foo: String, bar: Int } | { foo: Int, baz: String }

type Greeting = "hello" | "world"
```

Check which side a value is on with `is`:

```fa
type Greeting = "hello" | "world"

let value: Greeting = "hello"

if value is "hello" {
  console.log("hello")
} else if value is "world" {
  console.log("world")
}
```

See [unions](./unions).

## Intersection

`&` builds a type accepting a value satisfying **both** sides.

For object types, this means a value must carry every field of both:

```fa
type TypeA = { foo: String, bar: Int }
type TypeB = { foo: String, baz: String }

type MyType = TypeA & TypeB
-- MyType is { foo: String, bar: Int, baz: String }
```

When both sides declare the same field with **incompatible** types, no value can satisfy the result, and the field's type becomes `Never`:

```fa
type TypeA = { foo: String }
type TypeB = { foo: Int }

type MyType = TypeA & TypeB
-- MyType is { foo: Never }: no value can inhabit this type
```

For unions, `&` keeps the members present on both sides:

```fa
type TypeA = "hello" | "world" | "you"
type TypeB = "world" | "you" | "fine?"

type MyType = TypeA & TypeB
-- MyType is "world" | "you"
```

:::caution
`&` is intersection, and only intersection. To *pick* fields out of a type, use `>>` with a key set: `TypeA >> @{ foo }`.
:::

## Difference

`-` removes from the left side whatever the right side describes.

For object types, it removes fields:

```fa
type TypeA = { foo: String, bar: Int }
type TypeB = { foo: Int, baz: String }

type MyType = TypeA - TypeB
-- MyType is { bar: Int }
```

It works with key sets too:

```fa
type MyType = TypeA - @{ foo }
-- MyType is { bar: Int }
```

And with unions, where it removes members:

```fa
type TypeA = "hello" | "world" | "you" | "fine?"
type TypeB = "hello" | "you"

type MyType = TypeA - TypeB
-- MyType is "world" | "fine?"
```

## Optional

`?` adds `None` to a type:

```fa
type MaybeInt = Int?
-- the same as Int | None
```

## Special types

| Type | Meaning |
| --- | --- |
| `Any` | any value at all |
| `Object` | any object type |
| `Never` | no value; the result of an impossible intersection |
| `None` | exactly one value, `none` |
| `Type` | the type of types |
| `Error` | any error |

`Never` is what a function returns when it cannot return at all:

```fa
function loopForever = (): Never => {
  loop { }
}
```

## Precedence

From tightest to loosest:

1. `?` (optional suffix)
2. `...` (spread)
3. `>>` (pick), `-` (difference)
4. `&` (intersection)
5. `|` (union)

Use parentheses when the intent is not obvious:

```fa
type A = (String | Int)?           -- optional of a union
type B = String | Int?             -- String | Int | None (the same type)
type C = (TypeA | TypeB) & TypeC   -- parentheses required to distribute
```
