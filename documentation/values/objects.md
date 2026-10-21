# Objects

Objects associate named fields with values. They can be mutable or immutable.

Objects always live on the stack, and assigning one copies it. See [memory management](../advanced/memory-management).

## Anonymous objects

```fa
let myObject = {
  foo = 1
  bar = 2
}

-- immutability is deep, so this is rejected
myObject.foo += 1 -- error: `myObject` is immutable
```

A larger example, showing every kind of member:

```fa
let myComplexObject = {
  -- a field with an inferred type
  helloString = "hello world"

  -- a field with an explicit type
  withTypeInformation: String | Int = "Hello"
  optionalString: String? = "Hello"

  -- a private field, only visible from within the object
  #somePrivateValue = "private"

  -- a readonly field: writable only by the object's own mutable methods
  readonly someReadonlyValue = "readonly"

  -- a nested object
  nestedObject = {
    someNumber = 3
  }

  -- a method: shared by all instances, not reassignable
  sayHello(self): String => self.helloString

  -- a mutable method: callable only on mutable instances
  helloYou(mutable self) => {
    self.helloString = "hello you"
  }

  changeSomeReadonlyValue(mutable self) => {
    self.someReadonlyValue = "new value"
  }

  -- a lambda field: stored per instance, reassignable
  someFunction = (a: Int, b: Int): Int => a + b
}
```

## Accessing fields

Use the `.` operator:

```fa
myObject.helloString
```

Private fields (`#`) are reachable only from inside the object that declares them:

```fa
myObject.#somePrivateValue -- error: `#somePrivateValue` is private
```

## Declaring an object type

Most of the time you will name the type of an object, with the `type` keyword:

```fa
type MyObject = {
  someString: String
  someStringWithDefaultValue = "Hello"

  optionalString: String?
  optionalStringWithDefaultValue: String? = "Hello"

  point: Point
  pointWithDefaultValue = Point(3, 4)

  nestedObject: {
    someNumber: Int
  }

  someLambda: (Int, Int) => Int

  someMethod(self, a: Int, b: Int): Int => a + b
}
```

A field must be given a value when you create the object, unless it has a default value or is optional.

## Creating an object

There are two ways to create an object.

```fa
type MyType = {
  someString: String
  someStringWithDefaultValue = "Hello"
}

-- 1. the brace syntax
let myObject = MyType {
  someString = "hello"
}

-- 2. the constructor syntax
let myObject = MyType("hello", "Hello")

-- the constructor syntax also accepts labels
let myObject = MyType("hello", someStringWithDefaultValue = "Hello")
let myObject = MyType(someString = "hello", someStringWithDefaultValue = "Hello")
```

Prefer the brace syntax; keep the constructor syntax for small objects such as `Point(3, 4)`.

:::tip
Calling a **type** creates a value. Calling a **type function** produces a type. So `Point(3, 4)` builds a point, while `Array(Int)` builds a type — because `Point` is a type and `Array` is a type function. See [generics](../types/generics).
:::

## Methods and lambdas

Both are functions attached to an object; the only syntactic difference is the `=`.

```fa
type MyType = {
  -- a lambda field: has `=`, so it is a reassignable field holding a function
  myLambda = () => {
    console.log("something")
  }

  -- a method: no `=`, so it is shared and fixed
  myMethod() => {
    console.log("something")
  }
}
```

They follow different rules:

1. Methods are not re-created for each instance, so they cost no memory per instance.
2. Methods cannot be overridden when creating an object.
3. Methods must be declared in the object's type.

```fa
let myObject = MyType {
  myLambda = () => {      -- OK: a lambda can be replaced per instance
    console.log("something else")
  }

  myMethod() => {         -- error: a method cannot be overridden
    console.log("something else")
  }
}
```

Prefer methods, and use lambdas only when instances genuinely need different behaviour.

> In C these are called *function pointers*.

### `self`

A function inside an object is a method when its first parameter is `self`:

```fa
type Counter = {
  count: Int = 0

  -- an associated function: no `self`, called on the type
  zero(): Counter => Counter { count = 0 }

  -- a method: reads `self`
  isEmpty(self): Boolean => self.count == 0

  -- a mutable method: writes through `self`
  increment(mutable self) => {
    self.count += 1
  }
}

let empty = Counter.zero()

mutable counter = Counter { count = 5 }
counter.increment()
```

`self` never carries a type annotation — its type is the object being declared. A mutable method can only be called on a mutable value.

## Inheritance

Inheritance uses the spread operator `...`:

```fa
type Parent = {
  fieldA: Int
  fieldB = 13
}

type Child = {
  ...Parent
  fieldC: String
}
```

Pick which fields to inherit with the extract operator `>>` and a [key set](../types/fields):

```fa
type Child = {
  ...Parent >> @{ fieldA }
  fieldC = 13
}
```

Exclude fields with the difference operator `-`:

```fa
type Child = {
  ...Parent - @{ fieldA }
  fieldC = 13
}
```

Fa rejects ambiguous inheritance. If two parents declare the same field with different types, you must resolve it yourself:

```fa
type ParentA = { foo: String }
type ParentB = { foo: Int }

type Child = {
  ...ParentA
  ...ParentB -- error: the type of `foo` is ambiguous
}

type Child = {
  ...ParentA
  ...ParentB - @{ foo }  -- resolved: `foo` comes from ParentA
}
```

## Comparing objects

`==` compares objects **structurally**: two objects are equal when all their fields are equal. Object identity is not observable in Fa, because objects have no address you can take.

```fa
assert(Point(1, 2) == Point(1, 2))
```

See [type operations](../types/operations) for combining object types, and [structural typing](../advanced/structural-typing) for how object types relate to each other.
