# Sets

A set stores unordered, unique values. Sets can be mutable or immutable.

Uniqueness is decided by [equality](/operations/logical#equality) of the values: `==` compares objects and collections deeply, by their content.

Like every collection, a set is created from a list literal, typed by the context, or with `Set.from` when there is no context (see [creating collections](/collections/arrays#creating-collections)).

```fa
let setOfIntegers: Set(Integer) = [1, 2, 2, 3]

console.log(setOfIntegers.length) -- 3, because 2 is only stored once

let distinct = Set.from([1, 2, 2, 3]) -- no context: `Set.from` infers a `Set(Integer)`
```

```fa
mutable setOfHumans: Set(Human) = [
	{ name = "John", age = 20 }
	{ name = "Jane", age = 21 }
	{ name = "John", age = 22 }
]

let anotherJohn = Human { name = "John", age = 20 }
setOfHumans.add(anotherJohn) -- not added: an equal Human is already present
```

Note that the third human *is* stored: `{ name = "John", age = 22 }` is not equal to `{ name = "John", age = 20 }`.

## Custom comparison

Pass a `compare` function to `Set.from` when the identity of a value is carried by one field, such as an id:

```fa
type User = {
	id: Integer
	email: String
	name: String
}

let initialUsers = [
	User { id = 1, email = "john@doe.com", name = "John Doe" }
	User { id = 2, email = "jane@doe.com", name = "Jane Doe" }
]

mutable setOfUsers = Set.from(initialUsers, compare = (a, b) => a.id == b.id)

setOfUsers.add({ id = 1, email = "foo@bar.com", name = "Foo Bar" })
-- not added: a User with id 1 is already present
```

## Adding and removing

```fa
mutable tags: Set(String) = ["fa", "docs"]

tags.add("parser")
tags.delete("docs")
tags.clear()

tags.includes("fa") -- false: the set has been cleared
tags.length         -- 0
tags.isEmpty()      -- true
```

`add` returns whether the value was inserted:

```fa
if tags.add("fa") {
	console.log("this tag is new")
}
```

`add`, `delete` and `clear` require a `mutable` set, and declare the `invalidates self` effect (see [stable references](#stable-references)).

## Iteration

Order is not guaranteed and may change as the set is modified.

```fa
for tag in tags {
	console.log(tag)
}
```

Adding or removing values of a set while iterating over it is a compile error.

## Set operations

```fa
let a: Set(Integer) = [1, 2, 3]
let b: Set(Integer) = [2, 3, 4]

a.union(b)        -- [1, 2, 3, 4]
a.intersection(b) -- [2, 3]
a.difference(b)   -- [1]
a.isSubsetOf(b)   -- false
```

These operations return a new set and leave `a` and `b` unchanged.

Two sets are equal when they contain the same values, in any order:

```fa
let c: Set(Integer) = [3, 2, 1]
assert a == c
```

## Stable references

Adding to or removing from a set may rehash it, which moves its values and invalidates every reference into it. This is why `add`, `delete` and `clear` declare `invalidates self`, and the compiler rejects any use of a reference obtained before one of these calls until it is declared again (see [owned references](/advanced/owned-references#after-invalidation)):

```fa
mutable people: Set(Person) = [{ name = "Ana" }]
use ana = people.find((person) => person.name == "Ana") -- type: `(Person in people)?`

people.add({ name = "Bao" })

console.log(ana?.name) -- error: `ana` was invalidated by `people.add`: declare it again with `use ana = ...`

use ana = people.find((person) => person.name == "Ana") -- declared again: it finds Ana at her new place
console.log(ana?.name) -- "Ana"
```

When you need references that survive modification, use a [`Bag`](/collections/bags) or a [growing collection](/collections/growing-collections) instead.

:::tip
This is why the objects linked by [relations](/advanced/relations) live in a `Bag`, a `Columnar` or a `GrowingBag`, never in a `Set`.
:::

## Key sets

A set can contain a fixed number of values known at compile time:

```fa
let myKeySet: Set("hello" | "world" | "!") = ["hello", "!"]
```

The keys of an object type form a **key set**, written with braces: `{ foo, nested { bar } }`. It is the only use of braces that is not an object. Key sets are what the key-taking [type functions](/types/type-functions) expect:

```fa
type MyType = {
	foo: String
	bar: Integer
	nested: {
		baz: Integer
		qux: Integer
	}
}

type Picked = Pick(MyType, { foo, nested { baz } }) -- { foo: String, nested: { baz: Integer } }
type Omitted = Omit(MyType, { bar })                -- { foo: String, nested: { baz: Integer, qux: Integer } }
```

Because a key set is a compile-time value, a function can take one as a compile-time parameter and use it to generate code:

```fa
function logProperties(Item, keys: Set(DeepKeys(Item))) = (object: Item) => {
	@for key in keys {
		console.log("{key} = {object[key]}")
	}
}

let myObject = {
	foo = 12
	bar = 13
	baz = 14
	nested = {
		foo = 15
		bar = 16
	}
}

logProperties(Type(myObject), { foo, baz, nested { foo } })(myObject)
```

The call above compiles to three `console.log` lines with no runtime lookup at all. See [fields](/types/fields#key-sets) and [generics](/types/generics).

## Choosing a set

- Need uniqueness and fast membership tests? `Set`.
- Need to store many objects without uniqueness, with stable references? [`Bag`](/collections/bags).
- Need a fixed set of names known at compile time? A [key set](/types/fields#key-sets), written `{ ... }`.
