# Type functions

A **type function** is a function that returns a type. It is declared with `type`, and takes parameters like any other function:

```fa
type Combine = (First: Type, Second: Type) => {
  if First is Object and Second is Object {
    return First & Second
  }

  return First
}

type Result = Combine({ a: Int }, { b: String })
-- Result is { a: Int, b: String }
```

Type functions run at compile time only, so they cannot touch runtime values. Everything else is allowed: conditionals, loops, local values, recursion.

Applying a type function is how generics work in Fa. See [generics](./generics).

> **Warning:** deep or heavily recursive type functions slow compilation down. Keep them shallow.

## Parameters

A parameter annotated `Type` accepts a type. A parameter prefixed with `@` accepts a compile-time value:

```fa
type Repeat = (Item: Type, @count: UInt) => Array(Item, length = count)

type ThreeInts = Repeat(Int, 3)
```

Type arguments are inferred where possible, and passed **by name** when given explicitly. Compile-time value arguments stay positional. See [generics](./generics#passing-type-arguments-explicitly).

## Built-in type functions

### `TypeOf(value)`

Returns the type of a value.

```fa
mutable someValue = 12
type TypeOfSomeValue = TypeOf(someValue)
-- TypeOfSomeValue is Int
```

For a literal, `TypeOf` returns the literal type:

```fa
static someValue = 12
type TypeOfSomeValue = TypeOf(someValue)
-- TypeOfSomeValue is 12
```

:::tip
`TypeOf` returns the type **of a value**. `Type` is the type **of types**, used to declare type parameters. They are different things: `TypeOf(12)` is `Int`, while `Type` is what `Int` itself is an instance of.
:::

### `Keys(Object)`

Returns the [key set](./fields) of a type.

```fa
type MyType = { foo: String, bar: Int }
type KeysOfMyType = Keys(MyType)
-- KeysOfMyType is Fields { foo, bar }
```

For keyed collections, it returns the key type:

```fa
type MyMap = Map(String, Int)
type KeysOfMyMap = Keys(MyMap)
-- KeysOfMyMap is String
```

`DeepKeys(Object)` does the same recursively, including nested objects.

### `Values(Collection)`

Returns the type of the values in a collection.

```fa
type MyArray = Array(String)
type ValuesOfMyArray = Values(MyArray)
-- ValuesOfMyArray is String

type MyMap = Map(String, Int)
type ValuesOfMyMap = Values(MyMap)
-- ValuesOfMyMap is Int
```

### `Partial(Object)`

Makes every field optional.

```fa
type MyType = { foo: String, bar: Int }
type PartialMyType = Partial(MyType)
-- PartialMyType is { foo: String?, bar: Int? }
```

### `Required(Object)`

Makes every field required.

```fa
type MyType = { foo: String?, bar: Int? }
type RequiredMyType = Required(MyType)
-- RequiredMyType is { foo: String, bar: Int }
```

### `Record(Keys, Item)`

Builds an object with the given keys, all holding the same type.

```fa
type RecordMyType = Record(@{ foo, bar }, Int)
-- RecordMyType is { foo: Int, bar: Int }

type RecordMyType = Record(@{ foo, bar, nested { baz } }, String)
-- RecordMyType is { foo: String, bar: String, nested: { baz: String } }
```

:::caution
Unlike TypeScript, the keys must be a key set, not a type. To model TypeScript's `Record<string, T>`, use a `Map(String, T)` instead — that is a runtime collection, not an object.
:::

### `Pick(Object, Keys)`

Keeps only the given keys.

```fa
type MyType = { foo: String, bar: Int }
type PickedType = Pick(MyType, @{ foo })
-- PickedType is { foo: String }
```

### `Omit(Object, Keys)`

Removes the given keys, including nested ones.

```fa
type MyType = { foo: String, bar: Int }
type OmittedType = Omit(MyType, @{ foo })
-- OmittedType is { bar: Int }
```

```fa
type MyType = {
  foo: {
    bar: Int
    baz: Int
  }
}

type OmittedType = Omit(MyType, @{ foo { baz } })
-- OmittedType is { foo: { bar: Int } }
```

### `Parameters(Function)`

Returns the parameters of a function, as an object.

```fa
function someFunction = (foo: String, bar: Int): String => ...

type MyType = Parameters(someFunction)
-- MyType is { foo: String, bar: Int }
```

### `Return(Function)`

Returns the return type of a function.

```fa
function someFunction = (foo: String, bar: Int): String => ...

type MyType = Return(someFunction)
-- MyType is String
```

### String type functions

```fa
type A = Uppercase("hello")   -- "HELLO"
type B = Lowercase("HELLO")   -- "hello"
type C = Capitalize("hello")  -- "Hello"
```

## Picking fields from a value

The type functions above operate on types. To build a new **value** by picking or omitting fields, use the lowercase `pick` and `omit` functions:

```fa
let myObject = { foo = "foo", bar = 12 }

let picked = pick(myObject, @{ foo })
-- picked is { foo = "foo" }

let omitted = omit(myObject, @{ foo })
-- omitted is { bar = 12 }
```

These run at runtime (or at compile time if the object is `static`), and are the value-level counterparts of `Pick` and `Omit`.
