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
- **stored**: a field typed `(Person in people)?` or `Bag(Person in people)`. A stored reference is always one side of a [relation](/advanced/relations): the compiler updates it when its target is deleted.

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

The compiler doesn't guess which operations are dangerous: every method that can move or remove elements declares it in its signature, with an [effect](/functions/defining-functions#invalidating-references):

- `invalidates self`: the operation can move or remove any element, so **every** reference into the collection becomes invalid,
- `invalidates item`: the operation only removes one element, so only the reference to this element becomes invalid,
- `mutates self[index]`: the operation writes one element in place, and no reference becomes invalid.

Which operations invalidate references depends on the collection:

| Collection | `invalidates self` | `invalidates item` | Never invalidates |
| --- | --- | --- | --- |
| [`Array`](/collections/arrays) | `push`, `insert`, `delete`, `deleteWhere`, `pop`, `clear`, `sortInPlace`, `reverseInPlace` | | writing an element (`array[index] = value`) |
| [`Set`](/collections/sets), [`Map`](/collections/maps) | `add`, `delete`, `deleteWhere`, `clear` (rehashing) | | |
| [`Bag`](/collections/bags), [`Columnar`](/collections/columnars) | `clear` | `delete(item)`, and `deleteWhere` for each deleted element | `add`: growth is safe |
| [`Growing*`](/collections/growing-collections) | | | everything: there is no delete, references live as long as the collection |

This is why [bags](/collections/bags) and [growing collections](/collections/growing-collections) are the right home for objects that reference each other, and why [relations](/advanced/relations) require them.

## After invalidation

After an invalidating call, the compiler forbids using the reference until you **check it** or **set it again**. Which one depends on how the collection identifies its elements.

### Collections with a stable identity

In a `Bag`, a `Columnar` or a growing collection, an element is identified by its slot, which never moves. In a `Set` or a `Map`, it is identified by its key. An invalidated reference into one of these collections behaves like an [optional](/values/optionals): the element may be gone. Checking it with `is not none` finds the element again, and narrows the reference to a valid one:

```fa
mutable people: Bag(Person) = []

use ana = people.add({ name = "Ana", age = 31 })
use bao = people.add({ name = "Bao", age = 12 })

people.deleteWhere((person) => person.age < 18) -- may invalidate any reference into `people`

console.log(ana.name) -- error: `ana` may have been invalidated by `people.deleteWhere`: check it with `if ana is not none`

if ana is not none {
	console.log(ana.name) -- "Ana": `ana` is valid in this block
}

if bao is not none {
	console.log(bao.name) -- not reached: Bao was deleted
}
```

For a `Set` or a `Map`, the check looks the element up by its key: after a rehash, the reference points at the element's new place.

### Arrays

In an `Array`, an element is identified by its index. After an `insert` or a `delete`, the same index can hold another element, so checking the reference would not tell you whether it still points at the element you meant. Instead, set it again with a new `use` declaration:

```fa
mutable persons: Array(Person) = [{ name = "Ana" }, { name = "Bao" }, { name = "Cleo" }]

use someone = persons[1] -- Bao

persons.delete(0) -- Ana is removed: Bao and Cleo move down by one

console.log(someone.name) -- error: `someone` was invalidated by `persons.delete`: declare it again with `use someone = persons[...]`

if someone is not none { } -- error: a reference into an `Array` can't be checked, its index may now hold another element: declare it again with `use someone = persons[...]`

use someone = persons[0] -- fine: `someone` was invalidated, it can be declared again
console.log(someone.name) -- "Bao"
```

Declaring a name twice in the same scope is normally an error (see [declaring values](/values/declaring-values)). Declaring again an invalidated reference is the only exception: redeclaring a reference that is still valid remains an error.

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
