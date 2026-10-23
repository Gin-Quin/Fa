# Objects

Objects are containers that associate named fields with values. They can be mutable or immutable.

An object is stored inline: on the stack, or inside the value that contains it (another object, a collection). Assigning an object copies it:

```fa
let original = { count = 1 }
mutable copy = original

copy.count = 2
console.log(original.count) -- 1: `copy` is a separate object
```

To refer to an existing object without copying it, use [`use`](/values/declaring-values#references-with-use). See [memory management](/advanced/memory-management).

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
myComplexObject.nestedAnonymousObject.someNumber
```

## Updating several fields

The `.=` operator assigns several fields of a `mutable` object at once:

```fa
mutable settings = Settings { host = "example.com", port = 80, secure = false }

settings .= { port = 8080, secure = true }
```

It is exactly a list of plain assignments, and it is compiled as such:

```fa
settings.port = 8080
settings.secure = true
```

- The object is updated in place: it is not recreated, so the references to it stay valid. This is the difference with `settings = { ...settings, port = 8080 }`, which creates a new object and assigns it.
- Only existing fields can be assigned, with values of their type.
- The update is shallow: to update the fields of a nested object, use `.=` on it: `settings.build .= { minify = false }`.

## Declaring an object type

Most of the time, you will want to declare the type of an object. This is done with the `type` keyword.

```fa
type MyObject = {
	someString: String
	someStringWithDefaultValue = "Hello"

	optionalString: String?
	optionalStringWithDefaultValue: String? = "Hello"

	point: Point
	pointWithDefaultValue = Point { x = 3, y = 4 }

	nestedAnonymousObject: {
		someNumber: Integer
	}

	-- function pointer
	someFunction: (a: Integer, b: Integer) => Integer

	-- method
	function someMethod = (self, a: Integer, b: Integer) => a + b
}
```

When creating an object, you must give a value to all the fields that do not have a default value and are not optional.

## Creating an object

An object is always created with braces. When the expected type is known from the context, write the literal directly:

```fa
type Point = {
	x: Integer
	y: Integer
}

let origin: Point = { x = 0, y = 0 }

function distance = (from: Point, to: Point): Float => ...

distance({ x = 0, y = 0 }, { x = 3, y = 4 }) -- the parameters give the type
```

When there is no context, write the type before the literal:

```fa
let origin = Point { x = 0, y = 0 }

let myObject = MyObject {
	someString = "hello"
	point = { x = 3, y = 4 } -- the field type gives the type
	someFunction = (a, b) => a * b
}
```

For a generic type, the type arguments are written before the literal: `Point(Float) { x = 1, y = 2 }`. See [generics](/types/generics).

There is no constructor call: `Point(3, 4)` and `Point(x = 3, y = 4)` do not create objects. Always write `Point { x = 3, y = 4 }`.

Writing the type twice is a warning:

```fa
let origin: Point = Point { x = 0, y = 0 } -- warning: the type is already known, write `let origin: Point = { x = 0, y = 0 }`
```

## Methods and function pointers

A **method** is a function that belongs to a type. It is declared with the `function` keyword inside the type, and its first parameter is `self`.

A **function pointer** is a field that holds a function. It is declared like any other field, with a name and a function type.

```fa
type MyType = {
	-- function pointer
	onClick: () => Void

	-- method
	function describe = (self): String => "I am a MyType"

	-- mutating method (only callable on mutable instances)
	function reset = (self) mutates self => {
		self.onClick = () => console.log("clicked")
	}
}
```

`self` is an immutable reference, like every parameter. A method that changes its object declares it with the `mutates self` effect, and can only be called on a mutable value. The effect can be narrowed to a single field, like `mutates self.balance`. See [defining functions](/functions/defining-functions).

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

## Readonly fields

A `readonly` field cannot be modified from outside the type, even on a mutable instance. It can only be modified by the mutating methods of its type.

```fa
type Account = {
	readonly balance: Integer = 0

	function deposit = (self, amount: Integer(min = 0)) mutates self.balance => {
		self.balance += amount
	}
}

mutable account = Account {}

account.deposit(100) -- OK
account.balance = 1000 -- error: `balance` is readonly
```

## Private fields

A field whose name starts with `#` is private: it can only be used by the methods of its type. Accessing it from outside is an error.

```fa
type Counter = {
	#count: Integer = 0

	function increment = (self) mutates self => {
		self.#count += 1
	}

	function current = (self): Integer => self.#count
}

mutable counter = Counter {}

counter.increment()
counter.current() -- 1
counter.#count -- error: `#count` is private
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

Fa does not allow ambiguous inheritance. If the same field is defined with different types in two spread types, it is a compile error:

```fa
type ParentA = {
	foo: String
}

type ParentB = {
	foo: Integer
}

type Child = {
	...ParentA
	...ParentB -- error: the type of `foo` is ambiguous
}
```

Resolve the conflict explicitly, by omitting the field from one of the parents:

```fa
type Child = {
	...ParentA
	...Omit(ParentB, { foo }) -- `foo` comes from ParentA
}
```

## Comparing objects

`==` and `!=` compare objects **structurally**: two objects are equal when all their fields are equal, deeply. Identity is not observable in Fa: there is no way to ask whether two values are "the same object".

```fa
let a = Point { x = 1, y = 2 }
let b = Point { x = 1, y = 2 }

assert a == b
assert a != Point { x = 2, y = 1 }
```

See [structural typing](/advanced/structural-typing) for how object types relate to each other.
