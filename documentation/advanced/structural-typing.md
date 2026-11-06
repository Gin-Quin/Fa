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

## What is compared

Two object types are compared by their **public data fields**: their names and their types. Everything else belongs to the type that is written where the value is used:

- **Methods** are not compared. A method call is resolved from the type written at the use site: after `let human: Human = animal`, `human.describe()` calls the `describe` of `Human`. In [generic code](/types/generics#checking-generic-code), it is resolved on the actual type.
- **Default values** are not compared either: they are used when an object literal of that type leaves the field out.
- **`readonly`** is part of the field: a `readonly` field can't be passed where a writable field is mutated (see [mutating through a narrower type](#mutating-through-a-narrower-type)).
- **`reactive`** is part of the field too: a value with [reactive](/advanced/reactivity) fields is still accepted through a narrower type, and writes through it notify the watchers.
- A type with **`#private` fields** is not structural: it is distinct from every other type, since only its own methods can use its private fields.

```fa
type Human = {
	name: String

	function describe = (self): String => "a human named {self.name}"
}

type Animal = {
	name: String

	function describe = (self): String => "an animal named {self.name}"
}

let animal = Animal { name = "Zeus" }
let human: Human = animal -- valid: same fields

console.log(human.describe()) -- "a human named Zeus"
console.log(animal.describe()) -- "an animal named Zeus"
```

A field and a method can't have the same name in a type.

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

Narrowing is always a copy. A value is only moved (when it is not used afterwards) into a place of the same shape; moving it into a narrower place copies it and drops its extra fields, exactly like `let`.

## Object literals have exactly the expected fields

An object literal written where a type is expected must have exactly the fields of that type. An extra field could never be read, so it is an error rather than a silent drop:

```fa
type Point = { x: Integer, y: Integer }

let point: Point = { x = 12, y = 13, z = 14 } -- error: `z` is not a field of Point
```

Only an existing value with more fields is narrowed:

```fa
type Point3d = { x: Integer, y: Integer, z: Integer }

let point3d = Point3d { x = 12, y = 13, z = 14 }
let point: Point = point3d -- valid: a copy, `z` is dropped
```

The same rule applies to an object literal passed to a function or added to a collection: `points.add({ x = 1, y = 2, z = 3 })` is an error when `points` holds `Point` values.

A literal can leave out a field that has a default value, or whose type accepts `none` (see [objects](/values/objects)). An existing value can't: a value without a field is never accepted where the field is expected, even an optional one.

```fa
type Contact = { name: String, email: String? }

let contact: Contact = { name = "Ana" } -- valid: `email` is none
let human = Human { name = "Bao" }
let other: Contact = human -- error: a `Human` has no `email` field
```

Where the expected type is a union, a literal must have exactly the fields of one of its members. Where it is a generic parameter, the literal gives the parameter its type, with all of its fields: `findById([{ id = 1, name = "Ana" }], 1)` infers `Entity` as `{ id: Integer, name: String }`.

## Collections of supersets

A collection stores its elements with the layout of its element type, so a value added to it is narrowed, like any copy:

```fa
mutable humans: Array<Human> = []

humans.push(superHuman) -- a copy: the stored Human has no `powers`

let copiedHumans: Array<Human> = superHumans -- copies every element, without `powers`
```

A collection of supersets can be passed to a function that only reads it:

```fa
function logNames = (humans: Array<Human>) => {
	for human in humans {
		console.log(human.name)
	}
}

logNames(superHumans) -- valid: no copy, no truncation
```

The elements of a `Set` and the keys of a `Map` are the exception: they are placed by their hash and compared with `==`, which depend on all of their fields. A `Set<Hero>` is never a `Set<Human>`, and a `Map<Hero, Integer>` is never a `Map<Human, Integer>`, even when they are only read. For the same reason, a reference to a `Set` element or a `Map` key is read-only (see [reference checking](/advanced/reference-checking#use-and-mutable-use)). The values of a `Map` follow the general rule.

Comparing two values with `==` compares the fields of their declared type: two `Human` parameters are equal when their `name` is, whatever the arguments hold beyond it.

A function that mutates the collection can't receive a collection of supersets: it could add a `Human`, which has no `powers`, to an `Array<SuperHuman>`. Make the function generic instead. Its element type is then the full type of the argument, and only values of that type can be added:

```fa
function addJohn = (humans: Array<Human>) mutates humans => {
	humans.push({ name = "John" })
}

addJohn(superHumans) -- error: `addJohn` mutates `humans`, so it needs an `Array<Human>`: make it generic

function keepNamed = <Item: Human>(items: Array<Item>) mutates items => {
	items.deleteWhere((item) => item.name == "")
}

keepNamed(superHumans) -- valid: `Item` is SuperHuman
```

## Mutating through a narrower type

A value whose fields are more precise satisfies a type when it is read: an `Integer<min = 0, max = 10>` is an `Integer`. Writing goes the other way, so a function that mutates a parameter can only receive a value whose written fields have **exactly** the type it writes:

```fa
type Scored = { score: Integer }
type Player = { name: String, score: Integer<min = 0, max = 10> }

function penalize = (scored: Scored) mutates scored => {
	scored.score = -5 -- valid for a Scored
}

mutable player = Player { name = "Ana", score = 3 }
penalize(player) -- error: `penalize` writes `score: Integer` into `player.score`, an `Integer<min = 0, max = 10>`
```

Otherwise, `player.score` would hold `-5`, outside of its range. The same applies to a field that is a union (writing a `String` into a `"ready" | "done"` field) or an object (writing a `Human` into a `SuperHuman` field, which has more fields).

Only the places named by the `mutates` effect count, down to the fields actually written: a function declared with `mutates scored.name` can receive a `Player`, since it never writes `score`, and a function that writes `contact.address.city` only needs `city` to have exactly its type, not the whole `address`.

A `readonly` field can't be written from outside its type, so it is never accepted where a function writes that field.

## Structural constraints

Because types are structural, a [generic constraint](/types/generics#constraints) is just a shape. Any type with the required fields satisfies it, with no declaration of intent (no `implements`, no trait):

```fa
function readId = <Item: { id: Integer }>(item: Item): Integer => item.id

readId(User { id = 1, name = "Ana" }) -- fine
readId(Product { id = 7, price = 12 }) -- also fine
readId(Comment { text = "Hello" }) -- error: `Comment` has no field `id: Integer`
```

The generic parameter `Item` is inferred from the argument, and names its full type: a function returning `Item` returns a `User` when given a `User`. A plain parameter gives the same result with `Type<parameter>`, see [keeping the type of the argument](#keeping-the-type-of-the-argument).

## Keeping the type of the argument

A parameter is a reference to the argument, which can have more fields than the declared type. Inside the function, only the fields of the declared type can be used, but the value keeps its own type, [`Type<parameter>`](/types/type-functions#typevalue). A copy without annotation keeps it too, with every field of the argument:

```fa
function renamed = (human: Human, name: String): Type<human> => {
	mutable copy = human -- a copy of the argument, of type Type<human>
	copy.name = name
	copy
}

let clark = renamed(superHuman, "Clark") -- a SuperHuman: `powers` is kept
let john = renamed(Human { name = "Ana" }, "John") -- a Human
```

A `use` alias and the loop variable of a `for` keep the type of the value they designate in the same way, whatever their annotation.

`copy.name = name` writes a `String` into a value whose actual `name` field may be narrower, so each call is checked as in [mutating through a narrower type](#mutating-through-a-narrower-type): `renamed` can't be given a value whose `name` is `"Ana" | "Bao"`.

This check also applies when `renamed` is imported, called through an import alias, or called by a wrapper (`EF004`). Its interface records the written paths and types, so the caller can check them without the function's source tree.

A function whose single-expression body returns a parameter returns its full type: `function same = (human: Human) => human` returns `Type<human>`.

Only an annotation narrows the copy, as everywhere else:

```fa
function nameOnly = (human: Human): Human => {
	let copy: Human = human -- `powers` is dropped
	copy
}
```

`Type<human>` is a shorthand for a generic parameter: `renamed` above is the same as `function renamed = <Item: Human>(human: Item, name: String): Item`. Use a generic parameter when the type must appear inside another type, as in `Array<Item>`.

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

Membership in an anonymous union is structural too. A value belongs to `String | Integer` when it is a string or an integer, and nothing has to be declared. The members of a union must be [distinguishable at runtime](/types/unions#members-must-be-distinguishable):

```fa
function describe = (value: String | Integer): String => when value is {
	String => "a string"
	Integer => "an integer"
}

describe("hello") -- "a string"
describe(42) -- "an integer"
```

Named unions behave differently on purpose: their members are distinguished by name, so `MyUnion.Car` and `MyUnion.House` stay distinct even if their fields match. See [unions](/types/unions).

:::implementation
## Implementation details

### Native

The strategy is detailed in `design/compilation-with-llvm.md`.

- **Identity and layout.** Types are interned by shape, the public data fields with their types (and `readonly`): `Human` and `Animal` are one type, with one layout. Methods and defaults are attached to the name written at the use site, and resolved statically. A type with `#private` fields, a named union, an `Errors` type and an enumeration are interned by declaration. Recursive types are compared coinductively (two shapes are equal if no difference is found while unfolding them), and interned as a cycle of nodes. The declared name is kept beside it for diagnostics and `@typeToString`. Fields are sorted by alignment, then by name, so two equal shapes always have the same layout; a narrower type is in general **not** a prefix of a wider one.
- **Views.** A function taking a `Human` only touches the fields of `Human`, so its machine code depends on one thing about the argument: where each of these fields sits in it. This is the *view* of `Human` inside the argument: an offset per field, recursively for nested objects stored inline, or the column of each field for a row of a `Columnar`.
- **Specialization.** A direct call whose argument has a different view gets an instance of the callee compiled for that view, with constant offsets: an implicit generic over the layout, invisible to the type checker. The instance key is the function plus the view of each structural parameter, so two argument types with the same view share an instance, and identical generated code is merged by hashing. An argument whose fields sit at the same offsets reuses the instance of the declared type.
- **`Type<parameter>`.** A function that uses the type of an argument (to copy it without annotation, return it or reflect on it) is specialized on the argument's full type, exactly like an explicit generic, since it needs its size and its copy and drop functions.
- **Collections of supersets.** A read-only `Array<SuperHuman>` passed as an `Array<Human>` specializes the callee on the element stride and the view. A `Columnar` view ignores the extra columns.
- **Copies.** Narrowing a copy calls the copy function generated for the pair (source, destination), which copies the destination's fields only. It is never a `memcpy` of the start of the source.
- **Function values.** A function stored in a value is compiled once, for no particular argument. It gets a second entry point that takes, with the pointer, a static table describing the actual argument: the offset of each field of the declared type, the stride for a collection, and the size, alignment, copy and drop functions of the actual type. The table is generated once per pair (actual type, declared type); a caller passing the exact type passes the identity table. This costs one argument and one load per field, on indirect calls only, and keeps `mutates` and returned references exact.

### JavaScript

- Objects are dynamic: a view is a plain property read, and nothing is specialized.
- A narrowing copy calls the copy function generated for the destination type, which reads only its fields; a spread (`{ ...value }`) would keep the extra ones.
- An unannotated copy of an erased parameter copies the complete concrete shape with the structural copy helper. Mutable destinations, returned borrowed values and stored elements use the ordinary copy rules; primitive values pass through unchanged. An array of generic elements needs element copies too. Type-dependent reflection (`@typeToString(Type<human>)`) still needs the actual argument type.
:::
