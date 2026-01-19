# Type functions

Type functions are functions that return a type. They can take either a type or a value as an argument; values are automatically converted to types.

Since all parameters are types, they must be capitalized.

Example of a type function:

```fa
type MyTypeFunction = (TypeA, TypeB) => {
  if TypeA is Object and TypeB is Object {
    return TypeA & TypeB
  } else {
    return TypeA
  }
}
```

Type functions live and are executed at compile time only. Thus, they cannot use and manipulate runtime values. Other than that, everything is permitted.

> **Warning:** One should be careful not to overuse complicated type functions, as it can slow down the compilation process.

## Built-in type functions

Fa comes with a set of built-in type functions. 

### Type(value)

Returns the type of the value.

```fa
mutable someValue = 12
type TypeOfSomeValue = Type(someValue)
-- TypeOfSomeValue is Integer
```

Most of the time, you don't need to use this function, as the type inference will automatically infer the type of the value:

```fa
someValue = 12
type TypeOfSomeValue = someValue
-- TypeOfSomeValue is "12"
```

### Keys(Object)

Returns the keys of an object.

```fa
type MyType = { foo: String, bar: Integer }
type KeysOfMyType = Keys(MyType)
-- KeysOfMyType is { foo, bar }
```

It also works with the keyed collections like `Map`:

```fa
type MyMap = Map(String, Integer)
type KeysOfMyMap = Keys(MyMap)
-- KeysOfMyMap is String
```

### Values(Object)

Returns the type of the values of a collection.

```fa
type MyArray = Array(String)
type ValuesOfMyArray = Values(MyArray)
-- ValuesOfMyArray is String

type MyMap = Map(String, Integer)
type ValuesOfMyMap = Values(MyMap)
-- ValuesOfMyMap is Integer
```


### Partial(Object)

Returns a new type with all properties made optional.

```fa
type MyType = { foo: String, bar: Integer }
type PartialMyType = Partial(MyType)
-- PartialMyType is { foo: String?, bar: Integer? }
```

### Required(Object)

Returns a new type with all properties made required.

```fa
type MyType = { foo: String?, bar: Integer? }
type RequiredMyType = Required(MyType)
-- RequiredMyType is { foo: String, bar: Integer }
```

### Record(Type, Keys: KeySet)

Returns a new type with the keys of the original type. It's useful when you want to create an object where all values are of the same type.

:::caution
Unlike TypeScript, keys must be a string literal union. If you need to emulate a `Record<string, Type>` from TypeScript to Fa, you need to use a `Map(String, Type)` instead.
:::

```fa
type RecordMyType = Record(Integer, { foo, bar })
-- RecordMyType is { foo: Integer, bar: Integer }

type RecordMyType = Record(String, { foo, bar, nested { baz } })
-- RecordMyType is { foo: String, bar: String, nested: { baz: String } }
```

### Pick(Object, Keys)

Returns a new type with the keys picked from the original type.

```fa
type MyType = { foo: String, bar: Integer }
type PickedType = Pick(MyType, { foo })
-- PickedType is { foo: String }
```

There is also a `pick` function (lowercase) that constructs a new object from an existing one:

```fa
type MyObject = { foo = "foo", bar = 12 }
type MyPickedObject = Pick(myObject, { foo })
type MyPickedObject = MyObject & { foo }
-- MyPickedObject is { foo: "foo" }
```

### Omit(Object, Keys)

Returns a new type with the keys omitted from the original type.

```fa
type MyType = { foo: String, bar: Integer }
type OmittedType = Omit(MyType, { foo })
-- OmittedType is { bar: Integer }
```

Since we are using flags, we can omit nested values:

```fa
type MyType = {
  foo: {
    bar: Integer,
    baz: Integer
  }
}

type OmittedType = Omit(MyType, { foo { baz } })
-- OmittedType is { foo: { bar: Integer } }
```

There is also an `omit` function (lowercase) that constructs a new object from an existing one by omitting the given keys:

```fa
type myObject = { foo = "foo", bar = 12 }
type myOmittedObject = omit(myObject, { foo })
-- myOmittedObject is { bar: 12 }
```

### Parameters(Function)

Returns the parameters of a type function.

```fa
someFunction = (foo: String, bar: Integer): String => ...

type MyType = Parameters(someFunction)
-- MyType is { foo: String, bar: Integer }
```

### Return(TypeFunction)

Returns the return type of a type function.

```fa
someFunction = (foo: String, bar: Integer): String => ...

type MyType = Return(someFunction)
-- MyType is String
```

### Uppercase(String)

Returns the uppercase version of a string.

```fa
type MyType = Uppercase("hello")
-- MyType is "HELLO"
```

### Lowercase(String)

Returns the lowercase version of a string.

```fa
type MyType = Lowercase("HELLO")
-- MyType is "hello"
```

### Capitalize(String)

Returns the capitalized version of a string.

```fa
type MyType = Capitalize("hello")
-- MyType is "Hello"
```
