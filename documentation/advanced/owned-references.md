# Owned references

An **owned reference** is a reference to an element *inside a collection*. The collection owns the element, and therefore owns the reference.

```fa
let persons: Array(Person) = [Person {}, Person {}, Person {}]

let copied = persons[1] -- a copy
use owned = persons[1] -- an owned reference

console.log(@typeToString(Type(copied))) -- "Person"
console.log(@typeToString(Type(owned))) -- "Person in persons"
```

The type of `owned` is not `Person` but `Person in persons`. The `in` part is what makes the reference safe: the type system knows exactly which collection the reference depends on.

## Invalidation

If the owning collection changes in a way that could move or remove its elements, the references into it become invalid, and the compiler knows:

```fa
mutable persons: Array(Person) = [Person {}, Person {}, Person {}]

use someone = persons[1]

-- fine: writing to an element doesn't move the others
someone.name = "John"

-- this may move or remove elements of the array, invalidating every reference into it
persons.pop()

-- error: `someone` is no longer valid
console.log(someone.name)
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

### Invalidation through functions

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
console.log(second.name) -- error: `second` was invalidated by `addGuest`
```

When the body of a function is a single expression, its effects are inferred. When it is a block, they must be declared: a function that calls `push` without declaring `invalidates` is a compile error.

## Lifetime

The lifetime of an owned reference is the lifetime of the collection that owns it.

Because every value in Fa is destroyed at the end of its scope, a reference cannot outlive its collection by accident: the compiler rejects any code where it would.

## Returning a reference

Returning an owned reference to a collection that is local to the function is impossible, since the collection is destroyed on return. Fa copies the value out instead:

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
	console.log(first.name) -- error: `first` is no longer valid
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

## Owned references and relations

An owned reference points at *one* element, for short-lived work, and is invalidated by structural changes of its collection. A [relation](/advanced/relations) links two objects stored in stable-address collections, is kept up to date by the compiler on both sides, and survives the removal of its target: the field simply becomes `none`.

Use an owned reference for short-lived work on a stored value. Use a relation when two objects need to know about each other over time.
