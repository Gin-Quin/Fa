# Type functions

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

### Record(Type, Keys: KeySet)

Returns a new type with the keys of the original type. It's useful when you want to create an object where all values are of the same type.

> **Warning:** Unless in TypeScript, keys must be a string literal union. If you need to emulate a `Record<string, Type>` from Typescript to Fa, you need to use instead a `Map(Type, Key = String)`.

```ts
type MyType = { foo: String, bar: Integer }
type RecordMyType = Record(MyType, { foo, bar })
type RecordMyType = Record(MyType, { foo, bar, nested { baz } })
// RecordMyType is { foo: String, bar: Integer }
```

### Pick(Object, Keys)

Returns a new type with the keys picked from the original type.

```ts
type MyType = { foo: String, bar: Integer }
type PickedType = Pick(MyType, { foo })
// PickedType is { foo: String }
```


### Omit(Object, Keys)

Returns a new type with the keys omitted from the original type.

```ts
type MyType = { foo: String, bar: Integer }
type OmittedType = Omit(MyType, { foo })
// OmittedType is { bar: Integer }
```

Since we are using flags, we can omit nested values:

```ts
type MyType = {
  foo: {
    bar: Integer,
    baz: Integer
  }
}

type OmittedType = Omit(MyType, { foo { baz } })
// OmittedType is { foo: { bar: Integer } }
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
