# Structural typing

Fa has a **structural** type system, like TypeScript, and unlike Rust or Haskell which are **nominal**.

In a structural type system, a type is determined by the fields a value has. In a nominal one, it is determined by the name the type was declared with.

So in Fa, two types with the same fields are the same type, whatever they are called:

```fa
type Human = {
  name: String
}

type Animal = {
  name: String
}

let animal = Animal { name = "Zeus" }

let human: Human = animal        -- valid
let human = animal as Human      -- also valid, and more explicit

function logAnimalName = (animal: Animal) => {
  console.log(animal.name)
}

logAnimalName(human) -- valid: Human and Animal have the same fields
```

## Supersets

A type with *more* fields can be used wherever a type with fewer is expected:

```fa
type Human = {
  name: String
}

type SuperHuman = {
  ...Human
  powers: String
}

let superHuman = SuperHuman {
  name = "John"
  powers = "unlimited"
}

function logName = (human: Human) => console.log(human.name)

logName(superHuman) -- valid: every field of Human is present
```

## Copying narrows, referencing does not

This is the one place where structural typing meets Fa's memory model, and it is worth being precise about.

**Assigning** to a narrower type copies the value, and the extra fields are dropped:

```fa
let human: Human = superHuman
console.log(human) -- { name = "John" }: `powers` is gone
```

**Referencing** it does not copy, and nothing is dropped — the value is only *viewed* through the narrower type:

```fa
use human: Human = superHuman
console.log(human.name)   -- "John"
console.log(human.powers) -- error: `powers` is not part of Human
```

Function parameters are references, so passing a `SuperHuman` to a function taking a `Human` costs nothing and truncates nothing:

```fa
logName(superHuman) -- no copy, no truncation
```

The narrowed view is a compile-time restriction, not a runtime transformation.

## Structural constraints

Because types are structural, a [generic constraint](../types/generics#constraints) is just a shape. Any type with the required fields satisfies it, with no declaration of intent:

```fa
function readId = (Item: Type({ id: Int }), item: Item): Int => item.id

readId(User { id = 1, name = "Ana" })    -- fine
readId(Product { id = 7, price = 12 })   -- also fine
```

This is what makes [function composition](../functions/function-composition#middleware) able to express middleware and plugins: each step declares the shape it needs, and the compiler checks the ordering.

## When you want types to be distinct

Sometimes two types have the same shape but must not be interchangeable — a `UserId` and a `ProductId`, both integers.

Structural typing will happily mix them:

```fa
type UserId = Int
type ProductId = Int

let userId: UserId = 12
let productId: ProductId = userId -- valid: they are the same type
```

To keep them apart, give them different shapes. A single-field object is the usual way:

```fa
type UserId = { user: Int }
type ProductId = { product: Int }

let userId = UserId { user = 12 }
let productId: ProductId = userId -- error: different fields
```

Or use a single-member union, which is distinct by name:

```fa
type UserId = Union { User = Int }
type ProductId = Union { Product = Int }
```

Both cost nothing at runtime: a single-field object has exactly the layout of the field it holds.

## Structural typing and unions

Union membership is structural too. A value belongs to `String | Int` when it is a string or an integer — nothing has to be declared:

```fa
function describe = (value: String | Int) => ...

describe("hello")
describe(42)
```

Named unions behave differently on purpose: their members are distinguished by name, so `MyUnion.Car` and `MyUnion.House` stay distinct even if their fields match. See [unions](../types/unions).
