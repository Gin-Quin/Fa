# Type functions

A **type function** is a type that takes parameters. Giving it its parameters produces a new type. It is declared like any [generic type](/types/generics), with its parameters right after its name:

```fa
type WithTimestamps(Entity: { id: Integer }) = {
	...Entity
	createdAt: Integer
	updatedAt: Integer
}

type Article = WithTimestamps({ id: Integer, title: String })
-- Article is { id: Integer, title: String, createdAt: Integer, updatedAt: Integer }
```

The right side is any expression that produces a type, evaluated at compile time: an object type, a union, a spread, the result of another type function... and also `if`, `when` and `do` expressions, to compute a type with logic (see [type functions with logic](#type-functions-with-logic)).

```fa
type Response(Data) = {
	status: Integer
	data: Data
}

type Either(Left, Right) = Left | Right

type Public(Entity: { password: String }) = Omit(Entity, { password })
```

Type functions run at compile time only, so they cannot touch runtime values.

> **Warning:** deep or heavily recursive type functions slow compilation down. Keep them shallow.

## Parameters

A capitalized parameter receives a type. A lowercase parameter receives a compile-time value, and its annotation is the type of that value:

```fa
type Score(maximum: Integer(min = 1)) = Integer(min = 0, max = maximum)

type Grade = Score(20)
type Percent = Score(100)
```

Parameters are inferred where possible, and given explicitly right after the name otherwise. They can be constrained and have defaults. See [generics](/types/generics).

## Type functions with logic

Since `if`, `when` and `do` are expressions, a type function can compute its result with logic, like a function that returns a type. It runs at compile time, like the type functions of Zig.

Branch on a type with `when`:

```fa
type Optionalize(Value) = when Value is {
	None => None
	else => Value?
}

type A = Optionalize(Integer) -- Integer?
type B = Optionalize(None) -- None
```

Branch on a compile-time value with `if`:

```fa
type Storage(size: Integer(min = 0)) = if size <= 255 { UInt8 } else { UInt64 }

type Small = Storage(100) -- UInt8
```

Use a `do` block for several steps. Its last expression is the resulting type:

```fa
type CapitalizeLiterals(Input) = do {
	let schema = @typeToSchema(Input)
	let capitalized = capitalizeLiterals(schema)
	TypeFromSchema(capitalized)
}
```

Here, `@typeToSchema` turns the input type into a value, `capitalizeLiterals` is an ordinary function, called at compile time, that transforms the schema, and `TypeFromSchema` turns the result back into a type. See [types at runtime](/types/types-at-runtime) for a complete example.

The `type` keyword tells what a type function produces: a type. A type function can't produce a runtime value, and a `function` can't produce a type.

## Built-in type functions

### `Type(value)`

Returns the type of a value.

```fa
mutable someValue = 12
type TypeOfSomeValue = Type(someValue)
-- TypeOfSomeValue is Integer
```

Don't confuse it with using a compile-time value directly as a type. A [static](/values/declaring-values) value used as a type is its **literal type**, which only accepts this exact value:

```fa
static answer = 42

type LiteralAnswer = answer
-- LiteralAnswer is the literal type 42

type AnswerType = Type(answer)
-- AnswerType is Integer
```

### `Keys(ObjectType)`

Returns the [key set](/types/fields#key-sets) of a type, as a `Fields` type.

```fa
type MyType = { foo: String, bar: Integer }
type KeysOfMyType = Keys(MyType)
-- KeysOfMyType is Fields { foo, bar }
```

For keyed collections, it returns the key type:

```fa
type MyMap = Map(String, Integer)
type KeysOfMyMap = Keys(MyMap)
-- KeysOfMyMap is String
```

### `DeepKeys(ObjectType)`

Like `Keys`, but recursively, including the keys of nested objects.

```fa
type MyType = {
	foo: String
	nested: {
		bar: Integer
	}
}

type DeepKeysOfMyType = DeepKeys(MyType)
-- DeepKeysOfMyType is Fields { foo, nested { bar } }
```

### `Values(Collection)`

Returns the type of the values in a collection.

```fa
type MyArray = Array(String)
type ValuesOfMyArray = Values(MyArray)
-- ValuesOfMyArray is String

type MyMap = Map(String, Integer)
type ValuesOfMyMap = Values(MyMap)
-- ValuesOfMyMap is Integer
```

### `Partial(ObjectType)`

Makes every field optional.

```fa
type MyType = { foo: String, bar: Integer }
type PartialMyType = Partial(MyType)
-- PartialMyType is { foo: String?, bar: Integer? }
```

### `Required(ObjectType)`

Makes every field required.

```fa
type MyType = { foo: String?, bar: Integer? }
type RequiredMyType = Required(MyType)
-- RequiredMyType is { foo: String, bar: Integer }
```

### `Record(Keys, Item)`

Builds an object with the given keys, all holding the same type.

```fa
type RecordMyType = Record({ foo, bar }, Integer)
-- RecordMyType is { foo: Integer, bar: Integer }

type RecordMyType = Record({ foo, bar, nested { baz } }, String)
-- RecordMyType is { foo: String, bar: String, nested: { baz: String } }
```

:::caution
Unlike TypeScript, the keys must be a key set, not a type. To model TypeScript's `Record<string, T>`, use a `Map(String, T)` instead: that is a runtime collection, not an object.
:::

### `Pick(ObjectType, Keys)`

Keeps only the given keys.

```fa
type MyType = { foo: String, bar: Integer }
type PickedType = Pick(MyType, { foo })
-- PickedType is { foo: String }
```

### `Omit(ObjectType, Keys)`

Removes the given keys, including nested ones.

```fa
type MyType = { foo: String, bar: Integer }
type OmittedType = Omit(MyType, { foo })
-- OmittedType is { bar: Integer }
```

```fa
type MyType = {
	foo: {
		bar: Integer
		baz: Integer
	}
}

type OmittedType = Omit(MyType, { foo { baz } })
-- OmittedType is { foo: { bar: Integer } }
```

`Pick` and `Omit` are also how you spread only some fields of a type: see [type operations](/types/operations#spread).

### `Parameters(Function)`

Returns the parameters of a function, as an object.

```fa
function someFunction = (foo: String, bar: Integer): String => ...

type MyType = Parameters(someFunction)
-- MyType is { foo: String, bar: Integer }
```

### `Return(Function)`

Returns the return type of a function.

```fa
function someFunction = (foo: String, bar: Integer): String => ...

type MyType = Return(someFunction)
-- MyType is String
```

### String type functions

```fa
type A = Uppercase("hello") -- "HELLO"
type B = Lowercase("HELLO") -- "hello"
type C = Capitalize("hello") -- "Hello"
```

## Picking fields from a value

The type functions above operate on types. To build a new **value** by picking or omitting fields, use the lowercase `pick` and `omit` functions:

```fa
let myObject = { foo = "foo", bar = 12 }

let picked = pick(myObject, { foo })
-- picked is { foo = "foo" }

let omitted = omit(myObject, { foo })
-- omitted is { bar = 12 }
```

They are the value-level counterparts of `Pick` and `Omit`. The key set is a compile-time value; the object can be a runtime value, or a `static` one, in which case the result is computed at compile time.
