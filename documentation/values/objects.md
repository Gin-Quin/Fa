
## Objects

Objects are containers that associate keys with values. They can be mutable or immutable.

### Anonymous objects

```fa
-- anonymous object
let myObject = {
  foo = 1
  bar = 2
}

-- values can be updated in the current scope
myObject.foo += 1

-- complex object
let myComplexObject = {
  -- value with inferred type
  someString = "hello"
  
  -- value with explicit type
  withTypeInformation: String | Number = "Hello"
  optionalString: String? = "Hello"
  
  -- private values
  #somePrivateValue = "private"
  
  -- readonly values (still mutable with methods)
  -- this is a great replacement for getters
  readonly someReadonlyValue = "readonly"
  
  -- nested object
  nestedAnonymousObject = {
    someNumber = 3
  }
  
  -- methods (no `=` symbol, not reassignable)
  sayHello() {
    return 42
  }
  
  logSomeString(self) {
    console.log(self.someString)
  }
  
  mutateSelf(mutable self) {
    self.someString = "Hello World"
  }

	-- function pointers (declared with an `=` symbol, reassignable)
  someFunction = (a: Number, b: Number) => a + b
}
```

### Accessing object fields

You can access the fields of an object using the `.` operator.

```fa
myObject.someString
```

You can also access the fields of an object using the `[]` operator with a string literal.

```fa
myObject["someString"]
```

You can also pass a path inside the `[]` operator:

```fa
myObject["nested.foo"]

-- same as

myObject.nested.foo
```


### Declaring the type of an object

Very often, you will want to declare the type of an object. This is done with the `type` keyword.

```fa
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

```fa
type MyType = {
  someString: String
  someStringWithDefaultValue = "Hello"
}

-- bracket syntax
myObject = MyType {
  someString = "hello"
}

-- constructor syntax
myObject = MyType("hello", "Hello")

-- you can use labels as well with the constructor syntax
myObject = MyType("hello", someStringWithDefaultValue = "Hello")
myObject = MyType(someString = "hello", someStringWithDefaultValue = "Hello")
```

It's recommended to use the bracket syntax most of the time, and to keep the constructor syntax only for small objects.

### Methods vs Functions

A **method** is a function that is associated with an object. It's defined within the object with a slightly different syntax than regular functions: you must not use the `=` and the `=>` symbols.

```fa
type MyType = {
  -- this is a function
  myFunction = () => {
    log("something")
  }

  -- this is a method
  myMethod() {
    log("something")
  }
}
```

Methods and functions follow slightly different rules:

1. Methods are not stored within the object but separately.
2. Methods cannot be overridden when creating a new object.
3. Methods must be defined in the object type declaration.

Example:

```fa
myObject = MyType {
  myFunction = () => { -- OK, we can override the function for this specific instance
    log("something else")
  }

  myMethod() { -- WRONG, we cannot override a method when instantiating an object
    log("something else")
  }
}
```

When possible, methods should be preferred over functions as they take less memory. Only use functions in objects if you want to assign custom values.

> In other languages like C, Fa's object functions are called **function pointers**.

### Object composition

In Fa, composition is done with the `...` syntax.

```fa
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

```fa
type Child = {
  ...Parent >> { fieldA }
  fieldC = 13
}
```
