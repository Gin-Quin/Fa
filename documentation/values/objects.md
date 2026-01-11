
## Objects

Objects are containers that associate keys with values. They can be mutable or immutable.

### Anonymous objects

```fa
-- anonymous object
let myObject = {
  foo = 1
  bar = 2
}

-- since immutabiliy is deep, you cannot mutate the object
myObject.foo += 1 -- does not work, myObject is immutable

-- complex object
let myComplexObject = {
  -- value with inferred type
  helloString = "hello world"
  
  -- value with explicit type
  withTypeInformation: String | Number = "Hello"
  optionalString: String? = "Hello"
  
  -- private values
  #somePrivateValue = "private"
  
  -- readonly values (still mutable with mutable methods from within the object)
  readonly someReadonlyValue = "readonly"
  
  changeSomeReadonlyValue(mutable self) {
    self.someReadonlyValue = "new value"
  }
  
  -- nested object
  nestedAnonymousObject = {
    someNumber = 3
  }
  
  -- methods (no `=` symbol, not reassignable)
  sayHello(self) {
    return helloString
  }
  
  logSomeString(self) { -- "self" must be the first parameter and cannot have a type annotation
    console.log(self.someString)
  }
  
  -- mutable methods (only accessible by mutable instances)
  helloYou(mutable self) {
    self.helloString = "hello you"
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


### Declaring an object type

Most of the time, you will want to declare the type of an object. This is done with the `type` keyword.

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

let myObject = MyObject {
  someString = "hello"
  point = Point(3, 4)
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

### Methods vs Lambdas

A **method** is a function that is associated with an object. It's defined within the object with a slightly different syntax than regular functions: you must not use the `=` and the `=>` symbols.

```fa
type MyType = {
  -- this is a lambda
  myLambda = () => {
    log("something")
  }

  -- this is a method
  myMethod() {
    log("something")
  }
}
```

Methods and lambdas follow slightly different rules:

1. Methods are not re-created when instantiating a new object.
2. Methods cannot be overridden when creating a new object.
3. Methods must be defined in the object type declaration.

Example:

```fa
myObject = MyType {
  myLambda = () => { -- OK, we can override the function for this specific instance
    log("something else")
  }

  myMethod() { -- WRONG, we cannot override a method when instantiating an object
    log("something else")
  }
}
```

When possible, methods should be preferred over lambdas as they take less memory. Only use lambdas in objects if you want to assign custom values.

> In other languages like C, Fa's object lambdas are called **function pointers**.

### Object inheritance

In Fa, inheritance is done with the `...` syntax.

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

You can pick which fields you want to inherit from the parent using the intersection operator `&`:

```fa
type Child = {
  ...Parent & { fieldA }
  fieldC = 13
}
```

You can also use the subtract operator `-` to exclude fields from the parent object:

```fa
type Child = {
  ...Parent - { fieldA }
  fieldC = 13
}
```

Fa does not allow ambiguous inheritance. If a field is defined in both the parent and child objects with a different type, you must explicitly specify which one to use.

```fa
type ParentA = {
	foo: String
}

type ParentB = {
	foo: Number
}

type Child = {
	...ParentA
	...ParentB -- this will cause a compilation error because the type of 'foo' is ambiguous
}
```
