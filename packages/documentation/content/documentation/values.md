# Values

In Fa, we talk of "values" instead of "variables".

By default, a value is immutable, unless declared with the `mutable` keyword.

## Mutable values

Mutable values are **variables**, i.e. values that can be reassigned with the `set` keyword.

```ts
mutable myValue = 12

set myValue = 13
```

## Numbers

There are three types of numbers: **integers**, **big integers** and **decimals**.

### Integers

Integers are the most common numbers. They follow slightly different rules depending on the target language (Javascript or Zig).

Integers are automatically inferred to three different sizes, depending on their sizes: 32-bit, 48-bit and 64-bit.

```ts
myInteger = 12
myInteger: Integer = 12

// you can specify the size of the integer at bit level
myInteger8: Integer(8) = 128 // 8-bit integer in Zig, `number` in JavaScript
myInteger5: Integer(5) = 128 // 5-bit integer in Zig, `number` in JavaScript

// you can create unsigned integers
myUnsignedInteger8: UnsignedInteger(8) = 128 // 8-bit unsigned integer in Zig, `number` in JavaScript

// the default inferred size is 32 bits
myInteger32 = 128 // 32-bit integer in Zig, `number` in JavaScript
myInteger32: Integer = 128 // 32-bit integer in Zig, `number` in JavaScript

// literal integers between 32 and 48 bits are automatically inferred as 48-bit integers
myInteger48 = 4_294_967_296 // inferred as 48-bit integer in Zig, `number` in JavaScript
myInteger48: Integer(48) = 4_294_967_296 // 48-bit integer in Zig, `number` in JavaScript

// integers larger than 48 bits are transpiled to `BigInt` in JavaScript
myInteger49: Integer(49) = 128 // 49-bit integer in Zig, `BigInt` in JavaScript

// literal integers larger than 48 bits are automatically inferred as 64-bit integers
myInteger64 = 281_474_976_710_656 // inferred as 64-bit integer in Zig, `BigInt` in JavaScript
myInteger64: Integer(64) = 281_474_976_710_656 // 64-bit integer in Zig, `BigInt` in JavaScript
```

When possible, it is recommended to use integer smaller than 48 bits to ensure maximum compatibility and speed with Javascript.

#### Big integers

Big integers are integers with an arbitrary size. They are defined with the `BigInteger` type or the `n` suffix.

```ts
myBigInteger = 1234567890123456789012345678901234567890n
myBigInteger: BigInteger = 1234567890123456789012345678901234567890
myBigInteger = BigInteger(1234567890123456789012345678901234567890)
```

### Decimals

Decimals are floating-point numbers. They are defined with the `Number` type.

```ts
myNumber = 12.34
myNumber: Number = 12.34

// by default, numbers with a fractional part are 64-bit floating-point numbers
myNumber64 = 12.34
myNumber64: Number(64) = 12.34

// you can create numbers with a size of 32 bits as well
myNumber32 = 12.34
myNumber32: Number(32) = 12.34
```

## Booleans

Booleans are values that can be either `true` or `false`.

```ts
myBoolean = true
myBoolean: Boolean = true
```

## Strings



## Characters

## Bytes

## Functions

Functions are values declared with the `=>` syntax.

```ts
add = (a: Number, b: Number) => a + b

// or

add = (a: Number, b: Number): Number => {
  return a + b
}
```

If it returns a value, it must be specified in the type declaration, unless the function directly returns a value (i.e. no brackets after the `=>`).

```ts
add = (a: Number, b: Number) => a + b // OK, the return type is inferred

add = (a: Number, b: Number) => {  // WRONG, we must specify the return type
  return a + b
}

add = (a: Number, b: Number): Number => {  // OK, we specified the return type
  return a + b
}

logSomething = () => { // OK, the function returns no value
  log("something")
}
```


## Objects

Objects are containers that associate keys with values. They can be mutable or immutable.

### Anonymous objects

```ts
// immutable anonymous object
myObject = {
  foo = 1
  bar = 2
}

// mutable anonymous object
myMutableObject = mutable {
  foo = 1
  bar = 2
}

// complex object
myComplexObject = {
  someString = "hello"
  withTypeInformation: String | Number = "Hello"
  optionalString: String? = "Hello"

  point = Point(3, 4)

  nestedAnonymousObject = {
    someNumber = 3
  }

  someFunction = (a: Number, b: Number) => a + b

  someMethod(a: Number, b: Number) => a + b
}
```

### Accessing object fields

You can access the fields of an object using the `.` operator.

```ts
myObject.someString
```

You can also access the fields of an object using the `[]` operator with a string literal.

```ts
myObject["someString"]
```

You can also pass a path inside the `[]` operator:

```ts
myObject["nested.foo"]

// same as

myObject.nested.foo
```


### Declaring the type of an object

Very often, you will want to declare the type of an object. This is done with the `type` keyword.

```ts
type MyObject = {
  someString: String
  someStringWithDefaultValue = "Hello"
  
  optionalString: String?
  optionalStringWithDefaultValue: String? = "Hello"

  point: Point
  pointWithDefaultValue = Point(3, 4)

  nestedAnonymousObject: {
    someNumber: Number
  }

  someFunction: (Number, Number) => Number

  someMethod(a: Number, b: Number) => a + b
}
```

When creating an object, you must implement all the fields that do not have a default value and are not optional.

There are two available syntaxes to create a new instance of an object:

1. The bracket syntax.
2. The constructor syntax.

```ts
type MyType = {
  someString: String
  someStringWithDefaultValue = "Hello"
}

// bracket syntax
myObject = MyType {
  someString = "hello"
}

// constructor syntax
myObject = MyType("hello", "Hello")

// you can use labels as well with the constructor syntax
myObject = MyType("hello", someStringWithDefaultValue = "Hello")
myObject = MyType(someString = "hello", someStringWithDefaultValue = "Hello")
```

It's recommended to use the bracket syntax most of the time, and to keep the constructor syntax only for small objects.

### Methods vs Functions

A **method** is a function that is associated with an object. It's defined within the object with a slightly different syntax than regular functions: you must not use the `=` and the `=>` symbols.

```ts
type MyType = {
  // this is a function
  myFunction = () => {
    log("something")
  }

  // this is a method
  myMethod() => {
    log("something")
  }
}
```

Methods and functions follow slightly different rules:

1. Methods are not stored within the object but separately.
2. Methods cannot be overridden when creating a new object.
3. Methods must be defined in the object type declaration.

Example:

```ts
myObject = MyType {
  myFunction = () => { // OK, we can override the function
    log("something else")
  }

  myMethod() => { // WRONG, we cannot override a method
    log("something else")
  }
}
```

When possible, methods should be preferred over functions as they take less memory. Only use functions in objects if you want to assign custom values.

> In other languages like C, Fa's object functions are called **function pointers**.

### Composition

In Fa, composition is done with the `...` syntax.

```ts
type Parent = {
  fieldA: Number
  fieldB = 13
}

type Child = {
  ...Parent
  fieldC: String
}
```

You can select which fields you want to inherit from the parent using the extract operator `>>`.

```ts
type Child = {
  ...Parent >> { fieldA }
  fieldC = 13
}
```


## Arrays

Arrays are the most common container to store a list of values. They can be mutable or immutable.

```ts
myArray = [1, 2, 3] // immutable array, you cannot modify, add or remove elements
myArray: Array(Integer) = [1, 2, 3]
myArray = Array(Integer) [1, 2, 3]

myMutableArray = mutable [1, 2, 3] // mutable array, you can modify, add and remove elements
myMutableArray: mutable Array(Integer) = [1, 2, 3]
myMutableArray = mutable Array(Integer) [1, 2, 3]
```

## Vectors

Vectors are arrays with a fixed length.

```ts
myFixedArray = Vector(4, Integer) [1, 2, 3, 4]

type Matrix4x4 = Vector(4, Vector(4, Number))

myMatrix = mutable Matrix4x4 [
  [1, 2, 3, 4],
  [1, 2, 3, 4],
  [1, 2, 3, 4],
  [1, 2, 3, 4],
]
```

## Tuples

Unlike other languages, Fa does not have tuples. Instead, you should use objects with the constructor notation.

For example, in Rust:

```rs
struct MyTuple(f32, String, i32);

let myTuple = MyTuple(1.0, "Hello", 3);
```

In Fa, you should declare a type object and use the constructor notation:

```ts
type MyTuple = {
  x: Number
  y: String
  z: Integer
}

myTuple = MyTuple(1.0, "Hello", 3)
```

## Maps

Maps are containers that associate keys with values. They can be mutable or immutable.

```ts
myMap = Map(String, Integer) {
  "foo" => 12
  "bar" => 13
}

myMap = Map(Point, Integer) {
  (1, 2) => 12
  (3, 4) => 13
  Point(5, 6) => 14
}
```

## Sets

Sets are containers that store unique values. They can be mutable or immutable.

```ts
mySet = Set(Integer) { 1, 2, 3 }
```

### Key sets

You can create a set containing a fixed number of values:

```ts
myKeySet = Set("hello" | "world" | "!") { hello, "!" }
```

This is useful when you want to get some keys from an object:

```ts
logProperties = (object: Object, @keys: Set(DeepKeys(object))) {
  @for @keys as @key {
    log("{@key} = {object[@key]}")
  }
}

myObject = {
  foo = 12
  bar = 13
  baz = 14
  nested = {
    foo = 15
    bar = 16
  }
}

logProperties(myObject, { foo, baz, nested { foo } })
logProperties(myObject, "foo" | "baz" | "nested.foo")
```


## Enums

Enums are a set of named numbers that can be used to represent a value among this set.

```ts
type MyEnum = Enum { foo, bar, baz }

myEnum = MyEnum.foo
```

## Flags

Flags are like enums, but they can be combined.

```ts
type MyFlag = Flag { foo, bar, baz }

myFlag = MyFlag.foo & MyFlag.bar
```

## Ranges

Ranges are a set of numbers. They can be mutable or immutable.

```ts
myRange = 1..3
myRange: Range = 1..3
myRange = Range { start = 1, end = 3, step = 1 }
myRange = Range(1, 3)

myMutableRange = mutable 1..3

myMutableRange.start = 2
myMutableRange.end = 4
```

You can create ranges without lower or upper bounds:

```ts
myRange = 1.. // 1 to infinite
myRange = ..3 // -infinite to 3
myRange = ..-1..3 // +infinite to 3 (reverse order)
myRange = .. // everything
```

You can also create ranges with a step:

```ts
myRange = 1..+3..10
myRange = 1..-4..-16
```
