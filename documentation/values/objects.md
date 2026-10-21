# Objects

Objects are containers that associate keys with values. They can be mutable or immutable.

## Anonymous objects

```fa
-- anonymous object
let myObject = {
	foo = 1
	bar = 2
}

-- since immutability is deep, you cannot mutate the object
myObject.foo += 1 -- error: myObject is immutable

-- complex object
let myComplexObject = {
	-- value with inferred type
	helloString = "hello world"

	-- value with explicit type
	withTypeInformation: String | Integer = "Hello"
	optionalString: String? = "Hello"

	-- private values
	#somePrivateValue = "private"

	-- nested object
	nestedAnonymousObject = {
		someNumber = 3
	}

	-- function pointer (a field that holds a function)
	add = (a: Integer, b: Integer) => a + b
}
```

Anonymous objects only contain data. They can contain function pointers, but no methods: methods are declared in [object types](#declaring-an-object-type).

## Accessing object fields

You can access the fields of an object using the `.` operator.

```fa
myComplexObject.helloString
```

## Declaring an object type

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
		someNumber: Integer
	}

	-- function pointer
	someFunction: (Integer, Integer) => Integer

	-- method
	function someMethod = (self, a: Integer, b: Integer) => a + b
}

let myObject = MyObject {
	someString = "hello"
	point = Point(3, 4)
	someFunction = (a, b) => a * b
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
let myObject = MyType {
	someString = "hello"
}

-- constructor syntax
let myObject2 = MyType("hello", "Hello")

-- you can use labels as well with the constructor syntax
let myObject3 = MyType("hello", someStringWithDefaultValue = "Hello")
let myObject4 = MyType(someString = "hello", someStringWithDefaultValue = "Hello")
```

It's recommended to use the bracket syntax most of the time, and to keep the constructor syntax only for small objects.

## Methods and function pointers

A **method** is a function that belongs to a type. It is declared with the `function` keyword inside the type, and its first parameter is `self`.

A **function pointer** is a field that holds a function. It is declared like any other field, with a name and a type.

```fa
type MyType = {
	-- function pointer
	onClick: () => Void

	-- method
	function describe = (self): String => "I am a MyType"

	-- mutating method (only callable on mutable instances)
	function reset = (mutable self) => {
		self.onClick = () => console.log("clicked")
	}
}
```

Methods and function pointers follow different rules:

1. Methods are not stored in instances. Function pointers are stored in each instance.
2. Methods cannot be replaced. Function pointers can be set when creating an object, and replaced on mutable instances.
3. Methods can only be declared in `type` declarations.

```fa
mutable myObject = MyType {
	onClick = () => console.log("hello") -- OK, function pointers are set per instance
}

myObject.onClick = () => console.log("hello again") -- OK, myObject is mutable

myObject.describe = () => "something else" -- error: `describe` is a method and cannot be replaced
```

When possible, prefer methods over function pointers, as they take less memory. Only use function pointers if each instance needs its own function.

See [defining functions](/functions/defining-functions) for the complete syntax.

## Readonly fields

A `readonly` field cannot be modified from outside the type, even on a mutable instance. It can only be modified by the mutating methods of its type.

```fa
type Account = {
	readonly balance: Integer = 0

	function deposit = (mutable self, amount: Integer(Min = 0)) => {
		self.balance += amount
	}
}

mutable account = Account {}

account.deposit(100) -- OK
account.balance = 1000 -- error: `balance` is readonly
```

## Object inheritance

In Fa, inheritance is done with the `...` syntax.

```fa
type Parent = {
  fieldA: Integer
  fieldB = 13
}

type Child = {
  ...Parent
  fieldC: String
}
```

You can pick which fields you want to inherit from the parent using the `Pick` type function:

```fa
type Child = {
  ...Pick(Parent, { fieldA })
  fieldC = 13
}
```

You can also use the `Omit` type function to exclude fields from the parent object:

```fa
type Child = {
  ...Omit(Parent, { fieldA })
  fieldC = 13
}
```

See [type operations](/types/operations) for more details.

Fa does not allow ambiguous inheritance. If a field is defined in both the parent and child objects with a different type, you must explicitly specify which one to use.

```fa
type ParentA = {
	foo: String
}

type ParentB = {
	foo: Integer
}

type Child = {
	...ParentA
	...ParentB -- this will cause a compilation error because the type of 'foo' is ambiguous
}
```
