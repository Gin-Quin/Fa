# Structural typing

Like TypeScript, Fa has a **structural** type system. This is unlike Rust or Haskell, whose type systems are **nominal**.

In a structural type system, the type of a value is determined by the fields it has. In a nominal one, it is determined by the name the type was declared with.

So in Fa, two types with the same fields are interchangeable, whatever they are called. This is valid in Fa (as well as in TypeScript), but would be invalid in Rust:

```fa
type Human = {
	name: String
}

type Animal = {
	name: String
}

let animal = Animal { name = "Zeus" }

let human: Human = animal -- valid: an Animal has every field of a Human

function logAnimalName = (animal: Animal) => {
	console.log(animal.name)
}

logAnimalName(human) -- valid: Human and Animal have the same fields
```

## Types are constraints

In Fa, a type annotation is a **constraint**: writing `value: Type` means that `value` **satisfies** `Type`, that is, it has at least the fields of `Type`, with compatible types. See [generics](/types/generics#a-type-is-a-constraint).

No conversion happens when a value satisfies a type. And the `as` keyword never converts a value to another type: it only gives a name (when [destructuring](/operations/destructuring), or in the branches of a [`when`](/control-flow/conditionals)). Conversions between different types are always explicit calls, like `Float(count)`. See [type conversion](/types/type-conversion).

## Supersets

A type with *more* fields can be used wherever a type with fewer fields is expected:

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

logName(superHuman) -- valid: every field of Human is present in SuperHuman, with the same type
```

## Copying narrows, referencing does not

This is the one place where structural typing meets Fa's [memory model](/advanced/memory-management), and it is worth being precise about it.

**Assigning** to a narrower type copies the value, and the extra fields are dropped:

```fa
let human: Human = superHuman
console.log(human) -- { name = "John" }: `powers` is gone
```

**Aliasing** it with `use` does not copy, and nothing is dropped: the value is only *viewed* through the narrower type:

```fa
use human: Human = superHuman
console.log(human.name) -- "John"
console.log(human.powers) -- error: `powers` is not part of Human
```

Function parameters are references, so passing a `SuperHuman` to a function taking a `Human` costs nothing and truncates nothing:

```fa
logName(superHuman) -- no copy, no truncation
```

The narrowed view is a compile-time restriction, not a runtime transformation.

## Structural constraints

Because types are structural, a [generic constraint](/types/generics#constraints) is just a shape. Any type with the required fields satisfies it, with no declaration of intent (no `implements`, no trait):

```fa
function readId(Item: { id: Integer }) = (item: Item): Integer => item.id

readId(User { id = 1, name = "Ana" }) -- fine
readId(Product { id = 7, price = 12 }) -- also fine
readId(Comment { text = "Hello" }) -- error: `Comment` has no field `id: Integer`
```

The generic parameter `Item` is inferred from the argument. Unlike a plain parameter typed `{ id: Integer }`, it keeps the full type of the argument: a function returning `Item` returns a `User` when given a `User`.

This is what makes [function composition](/functions/function-composition#middlewares) able to express middlewares and plugins: each step declares the shape it needs, and the compiler checks the order of the steps.

## When you want types to be distinct

Sometimes two types have the same shape but must not be interchangeable: a `UserId` and a `ProductId`, both integers.

Structural typing will happily mix them:

```fa
type UserId = Integer
type ProductId = Integer

let userId: UserId = 12
let productId: ProductId = userId -- valid: they are the same type
```

To keep them apart, give them different shapes. A single-field object is the usual way:

```fa
type UserId = { user: Integer }
type ProductId = { product: Integer }

let userId = UserId { user = 12 }
let productId: ProductId = userId -- error: different fields
```

Or use a single-member union, which is distinct by name:

```fa
type UserId = Union { User = Integer }
type ProductId = Union { Product = Integer }
```

Both cost nothing at runtime: a single-field object has exactly the layout of the field it holds.

## Structural typing and unions

Membership in an anonymous union is structural too. A value belongs to `String | Integer` when it is a string or an integer, and nothing has to be declared:

```fa
function describe = (value: String | Integer): String => when value is {
	String => "a string"
	Integer => "an integer"
}

describe("hello") -- "a string"
describe(42) -- "an integer"
```

Named unions behave differently on purpose: their members are distinguished by name, so `MyUnion.Car` and `MyUnion.House` stay distinct even if their fields match. See [unions](/types/unions).
