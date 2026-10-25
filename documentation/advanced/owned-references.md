# Owned references

An **owned reference** is a reference to an element *inside a collection*. The collection owns the element; the reference points at it, and its type names the collection.

```fa
let persons: Array(Person) = [Person {}, Person {}, Person {}]

let copied = persons[1] -- a copy
use someone = persons[1] -- a reference

console.log(@typeToString(Type(copied))) -- "Person"
console.log(@typeToString(Type(someone))) -- "Person in persons"
```

The type of `someone` is not `Person` but `Person in persons`. The `in` part is what makes the reference safe: the type system knows exactly which collection the reference depends on. When you write the type yourself, in a parameter or a return type, you write it in full: `Person in persons`.

## Local and stored references

A reference is either:

- **local**: declared with `use`, received as a parameter, or returned by a function. The compiler tracks it statically, call after call. This page is about local references.
- **stored**: a field typed `(Person in people)?` or `Bag(Person in people)`. A stored reference is always one side of a [relation](/advanced/relations), with a declared reverse side: the compiler updates both sides on every write, and when the target is deleted.

A local reference can't be stored as it is: a field typed `Person` or `Person?` holds a copy. To store a reference, declare a relation.

## Invalidation

If the collection changes in a way that could move or remove its elements, the references into it become invalid, and the compiler knows:

```fa
mutable persons: Array(Person) = [Person {}, Person {}, Person {}]

use someone = persons[1]

-- fine: writing to an element doesn't move the others
someone.name = "John"

-- this may move or remove elements of the array, invalidating every reference into it
persons.pop()

console.log(someone.name) -- error: `someone` was invalidated by `persons.pop`: declare it again with `use someone = persons[...]`
```

The compiler doesn't guess which operations are dangerous: every method that can move or remove elements declares it in its signature with the `invalidates self` [effect](/functions/defining-functions#invalidating-references), which invalidates **every** local reference into the collection. A method that only writes an element in place declares `mutates self[index]`, and invalidates nothing.

| Collection | `invalidates self` | Never invalidates |
| --- | --- | --- |
| [`Array`](/collections/arrays) | `push`, `insert`, `delete`, `deleteWhere`, `pop`, `clear`, `sortInPlace`, `reverseInPlace` | writing an element (`array[index] = value`) |
| [`Set`](/collections/sets) | `add`, `delete`, `deleteWhere`, `clear` | |
| [`Map`](/collections/maps) | `delete`, `deleteWhere`, `clear` | writing a key (`map[key] = value`, `getOrInsert`): values live in stable slots, so adding a key never moves the others |
| [`Bag`](/collections/bags), [`Columnar`](/collections/columnars) | `delete`, `deleteWhere`, `clear` | `add`: adding never moves the other elements |
| [`Growing*`](/collections/growing-collections) | | everything: they never delete or move, so references live as long as the collection |

Deleting one element of a `Bag` or a `Columnar` doesn't move the others, but it still invalidates every local reference: the compiler can't tell which of them pointed at the deleted element. Stored references don't have this problem: they are [relations](/advanced/relations), cleaned up at runtime when their target is deleted.

## After invalidation

After an invalidating call, using the reference is a compile error until you **declare it again**, whatever the collection:

```fa
mutable persons: Array(Person) = [{ name = "Ana" }, { name = "Bao" }, { name = "Cleo" }]

use someone = persons[1] -- Bao

persons.delete(0) -- Ana is removed: Bao and Cleo move down by one

console.log(someone.name) -- error: `someone` was invalidated by `persons.delete`: declare it again with `use someone = persons[...]`

use someone = persons[0] -- fine: `someone` was invalidated, it can be declared again
console.log(someone.name) -- "Bao"
```

When the element may be gone, declare the reference again with a lookup that returns an [optional](/values/optionals), then check the optional:

```fa
mutable people: Bag(Person) = []

use ana = people.add({ name = "Ana", age = 31 })
people.add({ name = "Bao", age = 12 })

people.deleteWhere((person) => person.age < 18) -- invalidates every reference into `people`

console.log(ana.name) -- error: `ana` was invalidated by `people.deleteWhere`: declare it again with `use ana = ...`

use ana = people.find((person) => person.name == "Ana") -- type: `(Person in people)?`

if ana is not none {
	console.log(ana.name) -- "Ana"
}
```

The same goes for a map, with `get`:

```fa
mutable scores: Map(String, Integer) = [("ana", 10), ("bao", 7)]

use score = scores.get("ana") -- type: `(Integer in scores)?`

scores.delete("bao")

console.log(score) -- error: `score` was invalidated by `scores.delete`: declare it again with `use score = scores.get(...)`

use score = scores.get("ana")
```

There is no way to check whether an invalidated reference is still valid: Fa keeps no generation counter to compare at runtime. Declaring it again is the only form, and it says exactly which element you mean.

Declaring a name twice in the same scope is normally an error (see [declaring values](/values/declaring-values#shadowing)). Declaring again an invalidated reference is the only exception: redeclaring a reference that is still valid remains an error.

## Invalidation through functions

Your own functions declare the same effects, and the compiler checks them at every call site:

```fa
function addGuest = (guests: Array(Person), name: String) invalidates guests => {
	guests.push(Person { name = name })
}

function welcomeAll = (guests: Array(Person)) mutates guests => {
	for guest in guests {
		guest.welcomed = true
	}
}

mutable guests: Array(Person) = [Person { name = "Ana" }, Person { name = "Bao" }]
use second = guests[1]

welcomeAll(guests) -- `second` is still valid: elements were written, not moved
console.log(second.welcomed) -- true

addGuest(guests, "Chloé") -- invalidates `second`
console.log(second.name) -- error: `second` was invalidated by `addGuest`: declare it again with `use second = guests[...]`
```

When the body of a function is a single expression, its effects are inferred. When it is a block, they must be declared: a function that calls `push` without declaring `invalidates` is a compile error.

## Lifetime

The lifetime of an owned reference is the lifetime of the collection that owns it.

Because every value in Fa is destroyed at the end of its scope, a reference cannot outlive its collection by accident: the compiler rejects any code where it would.

## Returning a reference

Returning a reference into a collection that is local to the function is impossible, since the collection is destroyed on return. Fa copies the value out instead:

```fa
function createPerson = (): Person => {
	mutable persons = [Person {}]
	use person = persons[0] -- type: `Person in persons`
	person -- converted to `Person`: a copy is made
}
```

When the collection comes from the caller, the reference can be returned as it is. Say so in the return type, and make it optional if the collection may be empty:

```fa
function firstPerson = (persons: Array(Person)): (Person in persons)? => {
	if persons.length == 0 {
		return none
	}
	use person = persons[0]
	person -- no copy: the caller owns `persons`
}
```

The caller receives a reference whose validity is tied to its own collection:

```fa
mutable persons = [Person { name = "Ana" }]

use first = firstPerson(persons)

if first is not none {
	console.log(first.name) -- "Ana"

	persons.push(Person {}) -- invalidates `first`
	console.log(first.name) -- error: `first` was invalidated by `persons.push`: declare it again with `use first = ...`
}
```

## Moving an element out

Every collection has a `take` method, which removes an element and returns it as a standalone value:

```fa
mutable arrayOfPersons = [Person {}, Person {}, Person {}]
mutable anotherArrayOfPersons = [Person {}]

let someone = arrayOfPersons.take(1)

anotherArrayOfPersons.push(someone)
```

`take` moves rather than copies, and it invalidates references into the source collection just like any other removal.
