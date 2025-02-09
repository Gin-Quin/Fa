# Types

In Fa, types can be manipulated like any other value.

A type is declared with the `type` keyword.

```ts
type MyType = {
  foo: String
}

MyType = type {
  foo: String
}
```

Unlike a value, a type is **immutable** and cannot have the `mutable` keyword.

```ts
type main = {
  foo: String
}
```

## Type operations

### Type union

```ts
type MyType = { foo: String, bar: Integer } | { foo: Integer, baz: String }

// MyType is either { foo: String, bar: Integer } or { foo: Integer, baz: String }
```

### Type intersection

```ts
type MyType = { foo: String, bar: Integer } & { foo: Integer, baz: String }

// MyType is { foo: Integer }
```

### Type extension

```ts
type TypeA = { foo: String }
type TypeB = { ...TypeA, bar: Integer }

// TypeB is { foo: String, bar: Integer }
```

### Type difference

```ts
type TypeA = { foo: String, bar: Integer }
type TypeB = { foo: Integer, baz: String }

type MyType = TypeA - TypeB
// MyType is { bar: Integer }
```

It works with literal types as well:

```ts
type TypeA = "hello" | "world" | "you" | "fine?"
type TypeB = "hello" | "you"

type MyType = TypeA - TypeB
// MyType is "world" | "fine?"
```

## Type functions

Type functions are functions that return a type. They can take either a type or a value as an argument; values are automatically converted to types.

Since all parameters are types, they must be capitalized.

Example of a type function:

```ts
type MyTypeFunction = (TypeA, TypeB) => {
  if TypeA is Object and TypeB is Object {
    return TypeA & TypeB
  } else {
    return TypeA
  }
}
```

Type functions live and are executed at compile time only. Thus, they cannot use and manipulate runtime values. Other than that, everything is permitted.

> **Warning:** One should be careful to not overuse complicated type functions, as it can slow down the compilation process.

## Built-in type functions

Fa comes with a set of built-in type functions. 

### Type(value)

Returns the type of the value.

```ts
mutable someValue = 12
type TypeOfSomeValue = Type(someValue)
// TypeOfSomeValue is Integer
```

### Keys(Object)

Returns the keys of a type object or array.

```ts
type MyType = { foo: String, bar: Integer }
type KeysOfMyType = Keys(MyType)
// KeysOfMyType is "foo" | "bar"

type MyArray = Array(String)
type KeysOfMyArray = Keys(MyArray)
// KeysOfMyArray is Integer
```

### Partial(Object)

Returns a new type with all properties made optional.

```ts
type MyType = { foo: String, bar: Integer }
type PartialMyType = Partial(MyType)
// PartialMyType is { foo?: String, bar?: Integer }
```

### Required(Object)

Returns a new type with all properties made required.

```ts
type MyType = { foo?: String, bar?: Integer }
type RequiredMyType = Required(MyType)
// RequiredMyType is { foo: String, bar: Integer }
```

### Record(Keys: StringLiteralUnion, Type)

Returns a new type with the keys of the original type. It's useful when you want to create an object where all values are of the same type.

> **Warning:** Unless in TypeScript, keys must be a string literal union. If you need to emulate a `Record<string, Type>` from Typescript to Fa, you need to use instead a `Map(Type, Key = String)`.

```ts
type MyType = { foo: String, bar: Integer }
type RecordMyType = Record("foo" | "bar", MyType)
// RecordMyType is { foo: String, bar: Integer }
```

### Pick(Object, Keys)

Returns a new type with the keys picked from the original type.

```ts
type MyType = { foo: String, bar: Integer }
type PickedType = Pick(MyType, "foo")
// PickedType is { foo: String }
```


### Omit(Object, Keys)

Returns a new type with the keys omitted from the original type.

```ts
type MyType = { foo: String, bar: Integer }
type OmittedType = Omit(MyType, "foo")
// OmittedType is { bar: Integer }
```

### Parameters(Function)

Returns the parameters of a type function.

```ts
someFunction = (foo: String, bar: Integer): String => ...

type MyType = Parameters(someFunction)
// MyType is { foo: String, bar: Integer }
```

### Return(TypeFunction)

Returns the return type of a type function.

```ts
someFunction = (foo: String, bar: Integer): String => ...

type MyType = ReturnType(someFunction)
// MyType is String
```

### Uppercase(String)

Returns the uppercase version of a string.

```ts
type MyType = Uppercase("hello")
// MyType is "HELLO"
```

### Lowercase(String)

Returns the lowercase version of a string.

```ts
type MyType = Lowercase("HELLO")
// MyType is "hello"
```

### Capitalize(String)

Returns the capitalized version of a string.

```ts
type MyType = Capitalize("hello")
// MyType is "Hello"
```

## Types to values

Some functions allow to create runtime values from types.

These functions must be compile-time compatible, i.e. either **pure** or relying on external compile-time values only.

### @typeToSchema(Type): Schema

Returns the schema of a type.

```ts
type MyType = { foo = "hello", bar?: Integer }

myTypeSchema = @typeToSchema(MyType)

assert myTypeSchema == {
  type: "Object",
  properties: {
    foo: { definition: { type: "String" }, default: "hello" },
    bar: { definition: { type: "Integer" }, optional: true }
  }
}
```

In conjunction with the `TypeFromSchema` type function, it can be used to generate complex types:

```ts
type CapitalizeSecondLetter = (Type) => {
  schema = @typeToSchema(Type)

  capitalizeSecondLetter = (string: String) => string[0] + string[1].uppercase() + string[2..]

  when schema.type is {
    "StringLiteral" => TypeFromSchema({
      type = "StringLiteral"
      value = capitalizeSecondLetter(schema.value)
    })

    "Union" => {
      newItems = Array(Schema::StringLiteral, capacity = schema.items.length)
      
      for schema.items as item {
        assert item.type is "StringLiteral"
        
        newItems.push({
          type = "StringLiteral"
          value = capitalizeSecondLetter(item.value)
        })
      }

      return TypeFromSchema({
        type = "Union"
        items = newItems
      })
    }

    else => {
      return Error("Unsupported type argument: {Type}")
    }
  }
}
```

### @typeToString(Type): String

Returns the string representation of a type.

```ts
type MyType = @typeToString(String)
// MyType is "String"
```
