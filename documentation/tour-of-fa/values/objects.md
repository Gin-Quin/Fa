
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

### Object composition

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
