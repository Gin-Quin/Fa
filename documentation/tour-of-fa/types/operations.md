# Type operations

## Union

```ts
type MyType = { foo: String, bar: Integer } | { foo: Integer, baz: String }
// MyType is either { foo: String, bar: Integer } or { foo: Integer, baz: String }

type MyType = "hello" | "world"
// MyType is "hello" or "world"
```

You can check which type a value is with the `is` keyword:

```ts
type MyType = "hello" | "world"

value: MyType = "hello"

if value is "hello" {
  log("hello")
} else if value is "world" {
  log("world")
}
```

#### The `union` declaration

When you want to declare a type that is a union of multiple complicated, you can use the `union` keyword:

```ts
type MyUnion = Union {
  Hello = "hello"
  World = "world"

  Car = {
    brand: String
    model: String
    year: Integer
  }

  House = {
    address: String
    price: Integer
  }
}

// you can now use MyUnion as a type:

myValue1 = MyUnion::Car {
  brand = "Toyota"
  model = "Corolla"
  year = 2020
}

myValue2: MyUnion = "hello"

when myValue1 is {
  Car => log("It's a car")
  House => log("It's a house")
  Hello => log("It's hello")
  World => log("It's world")
}
```

## Intersection

```ts
type TypeA = { foo: String, bar: Integer }
type TypeB = { foo: Integer, baz: String }

type MyType = TypeA & TypeB
// MyType is { foo: Integer }
```

## Difference

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
