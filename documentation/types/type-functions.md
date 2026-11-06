# Type functions

A **type function** is a type that takes parameters. Giving it its parameters produces a new type. It is declared like any [generic type](/types/generics), with its parameters between angle brackets right after its name:

```fa
type WithTimestamps<Entity: { id: Integer }> = {
	...Entity
	createdAt: Integer
	updatedAt: Integer
}

type Article = WithTimestamps<{ id: Integer, title: String }>
-- Article is { id: Integer, title: String, createdAt: Integer, updatedAt: Integer }
```

The right side is any expression that produces a type, evaluated at compile time: an object type, a union, a spread, the result of another type function... and also `if`, `when` and `do` expressions, to compute a type with logic (see [type functions with logic](#type-functions-with-logic)).

```fa
type Response<Data> = {
	status: Integer
	data: Data
}

type Either<Left, Right> = Left | Right

type Public<Entity: { password: String }> = Omit<Entity, { password }>
```

Type functions run at compile time only, so they cannot touch runtime values.

A type function is checked when it is applied, with its actual parameters, and an error is reported where it is applied (see [checking generic code](/types/generics#checking-generic-code)). Applying it twice to the same parameters computes the result once.

A type function can apply itself, but evaluation is bounded: more than 256 nested applications of type functions is a compile error, so a type function that never stops is reported instead of hanging the compiler.

> **Warning:** deep or heavily recursive type functions slow compilation down. Keep them shallow.

## Parameters

A capitalized parameter receives a type. A lowercase parameter receives a compile-time value, and its annotation is the type of that value:

```fa
type Score<maximum: Integer<min = 1>> = Integer<min = 0, max = maximum>

type Grade = Score<20>
type Percent = Score<100>
```

Parameters are inferred where possible, and given explicitly between angle brackets right after the name otherwise. They can be constrained and have defaults. See [generics](/types/generics).

## Type functions with logic

Since `if`, `when` and `do` are expressions, a type function can compute its result with logic, like a function that returns a type. It runs at compile time, like the type functions of Zig.

Branch on a type with `when`:

```fa
type Optionalize<Value> = when Value is {
	None => None
	else => Value?
}

type A = Optionalize<Integer> -- Integer?
type B = Optionalize<None> -- None
```

On types, `is` tests whether the type satisfies the pattern: `Value is None` is true when every value of `Value` is a `None`. The test applies to the whole type, never member by member: `Optionalize<Integer?>` takes the `else` branch, and gives `Integer?`. In a branch, the parameter is known to satisfy its pattern, so `when Entity is { { id: Integer } => ... }` can use `Entity`'s `id`.

Branch on a compile-time value with `if`:

```fa
type Storage<size: Integer<min = 0>> = if size <= 255 { UInt8 } else { UInt64 }

type Small = Storage<100> -- UInt8
```

Use a `do` block for several steps. Its last expression is the resulting type:

```fa
type CapitalizeLiterals<Input> = do {
	let schema = @typeToSchema(Input)
	let capitalized = @capitalizeLiterals(schema)
	TypeFromSchema<capitalized>
}
```

Here, `@typeToSchema` turns the input type into a value, `capitalizeLiterals` is an ordinary function, called at compile time with `@`, that transforms the schema, and `TypeFromSchema` turns the result back into a type. As everywhere, a call evaluated by the compiler is written with `@`, and the function it calls runs as usual (see [compile-time function calls](/advanced/metaprogramming#compile-time-function-calls)). See [types at runtime](/types/types-at-runtime) for a complete example.

The `type` keyword tells what a type function produces: a type. A type function can't produce a runtime value, and a `function` can't produce a type.

## Built-in type functions

### `Type<value>`

Returns the type of a value.

```fa
mutable someValue = 12
type TypeOfSomeValue = Type<someValue>
-- TypeOfSomeValue is Integer
```

For a function parameter, `Type<parameter>` is the type of the argument actually passed, which can have more fields than the declared type. A function can use it to return the full type of its argument: see [keeping the type of the argument](/advanced/structural-typing#keeping-the-type-of-the-argument). The same holds for a `use` alias, and for the loop variable of a `for`: they keep the type of the value they designate, whatever their annotation.

`Type<value>` gives the [inferred type](/types/declaring-types#inferred-types-and-known-values) of the value, without the facts known about it: after `let five = 5`, `Type<five>` is `Integer`, and inside `if value is String`, `Type<value>` is still the declared union. The expression is not evaluated: `Type<load()>` doesn't call `load`.

Don't confuse it with using a compile-time value directly as a type. A [static](/values/declaring-values) value used as a type is its **literal type**, which only accepts this exact value:

```fa
static answer = 42

type LiteralAnswer = answer
-- LiteralAnswer is the literal type 42

type AnswerType = Type<answer>
-- AnswerType is Integer
```

### `Keys<ObjectType>`

Returns the [key set](/types/fields#key-sets) of an object type, as a `Fields` type. Only the public data fields are keys: methods and `#private` fields are not.

```fa
type MyType = { foo: String, bar: Integer }
type KeysOfMyType = Keys<MyType>
-- KeysOfMyType is Fields { foo, bar }
```

`Keys` only applies to object types. The type of the keys of a `Map` is given by `KeyType`:

```fa
type MyMap = Map<String, Integer>
type KeysOfMyMap = KeyType<MyMap>
-- KeysOfMyMap is String
```

### `DeepKeys<ObjectType>`

Like `Keys`, but recursively, including the keys of nested objects. It stops at a [recursive](/types/unions#unions-and-recursion) field: the field is a key, its own fields are not.

```fa
type MyType = {
	foo: String
	nested: {
		bar: Integer
	}
}

type DeepKeysOfMyType = DeepKeys<MyType>
-- DeepKeysOfMyType is Fields { foo, nested { bar } }
```

### `Values<Collection>`

Returns the type of the values in a collection. It only applies to collections.

```fa
type MyArray = Array<String>
type ValuesOfMyArray = Values<MyArray>
-- ValuesOfMyArray is String

type MyMap = Map<String, Integer>
type ValuesOfMyMap = Values<MyMap>
-- ValuesOfMyMap is Integer
```

### `Partial<ObjectType>`

Makes every field optional: its type accepts `none`, so it can be left out of an object literal. Like `Required`, it is shallow (the fields of a nested object keep their types), and it keeps the defaults and the `readonly` flags.

```fa
type MyType = { foo: String, bar: Integer }
type PartialMyType = Partial<MyType>
-- PartialMyType is { foo: String?, bar: Integer? }

let patch: PartialMyType = { bar = 12 } -- `foo` is none
```

### `Required<ObjectType>`

Makes every field required, by removing `None` from its type.

```fa
type MyType = { foo: String?, bar: Integer? }
type RequiredMyType = Required<MyType>
-- RequiredMyType is { foo: String, bar: Integer }
```

A field whose type is exactly `None` can't be made required: it is an error.

### `Record<keys, Item>`

Builds an object with the keys of a key set, all holding the same type.

```fa
type RecordMyType = Record<{ foo, bar }, Integer>
-- RecordMyType is { foo: Integer, bar: Integer }

type RecordMyType = Record<{ foo, bar, nested { baz } }, String>
-- RecordMyType is { foo: String, bar: String, nested: { baz: String } }
```

:::caution
Unlike TypeScript, the keys must be a key set, not a type. To model TypeScript's `Record<string, T>`, use a `Map<String, T>` instead: that is a runtime collection, not an object.
:::

### `Pick<ObjectType, keys: DeepKeys<ObjectType>>`

Keeps only the given keys, including nested ones. A key that the type doesn't have is an error, since `keys` must be a key set of `ObjectType`; this holds for `Omit` too.

```fa
type MyType = { foo: String, bar: Integer }
type PickedType = Pick<MyType, { foo }>
-- PickedType is { foo: String }
```

### `Omit<ObjectType, keys: DeepKeys<ObjectType>>`

Removes the given keys, including nested ones.

```fa
type MyType = { foo: String, bar: Integer }
type OmittedType = Omit<MyType, { foo }>
-- OmittedType is { bar: Integer }
```

```fa
type MyType = {
	foo: {
		bar: Integer
		baz: Integer
	}
}

type OmittedType = Omit<MyType, { foo { baz } }>
-- OmittedType is { foo: { bar: Integer } }
```

`Pick` and `Omit` are also how you spread only some fields of a type: see [type operations](/types/operations#spread).

### `Parameters<FunctionType>`

Returns the parameters of a function type, as an object. Use [`Type`](#typevalue) to get the type of a function:

```fa
function someFunction = (foo: String, bar: Integer): String => ...

type MyType = Parameters<Type<someFunction>>
-- MyType is { foo: String, bar: Integer }
```

A variadic parameter `...values: Integer` gives a field `values: Array<Integer>`. A parameter without a name in the function type, like in `(Integer, Integer) => Integer`, a destructured parameter and an anonymous structure parameter are errors here.

### `Return<FunctionType>`

Returns the return type of a function type.

```fa
function someFunction = (foo: String, bar: Integer): String => ...

type MyType = Return<Type<someFunction>>
-- MyType is String
```

### String type functions

```fa
type A = Uppercase<"hello"> -- "HELLO"
type B = Lowercase<"HELLO"> -- "hello"
type C = Capitalize<"hello"> -- "Hello"
type D = Uppercase<"north" | "south"> -- "NORTH" | "SOUTH"
```

They take a string literal type, or a union of them, and apply to each literal. Applied to `String`, they give `String`.

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

A function can use a key-set parameter in both its result type and its body, as in `select` in [generics](/types/generics#type-parameters-and-compile-time-value-parameters). This also works when the function is imported, for both `Pick`/`pick` and `Omit`/`omit`.
