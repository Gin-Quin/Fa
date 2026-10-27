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

A reference is **invalidated** when the memory it points to may have been freed, moved, or reused for a value of another type. The compiler knows when this happens:

```fa
mutable persons: Array(Person) = [Person {}, Person {}, Person {}]

use someone = persons[1]

-- fine: writing to an element doesn't move the others
someone.age = 30

-- fine: `pop` only removes the last element, and `someone` is the element at index 1
persons.pop()

-- `push` may reallocate the array, which moves every element
persons.push(Person {})

console.log(someone.age) -- error: `someone` was invalidated by `persons.push`: declare it again with `use someone = persons[...]`
```

The compiler doesn't guess which operations are dangerous: every function declares what it changes in its signature, with two [effects](/functions/defining-functions#mutating-parameters), and the compiler deduces from them which references are invalidated.

### Mutations

`mutates path` means that the value at `path` may change: it may be replaced, or modified in any way. The place itself stays valid: a reference to `path` still designates it, and sees the new value.

What a mutation invalidates depends on the **type** of the value, because replacing a value destroys what it owned:

- A **flat** value owns nothing: a primitive (number, boolean, character), a union of literals, a reference stored in a [relation](/advanced/relations), or an object whose fields are all flat. Mutating it invalidates nothing.
- Any other value owns memory: the elements of a collection, the characters of a string, the payload of a union (including an optional), the children of a [recursive type](/advanced/memory-management#recursive-types). Mutating it invalidates every reference into that memory: the elements, the [string slices](/values/strings#slices), the payload, and everything inside them.

```fa
type Person = {
	name: String
	age: Integer
	pets: Array(Pet)
}

mutable people: Array(Person) = [Person { name = "Ana", age = 31, pets = [Pet { name = "Rex" }] }]

use person = people[0]
use age = people[0].age
use initials = people[0].name[0..1]
use pet = people[0].pets[0]

people[0].age = 32 -- `age` is an `Integer`, a flat value: nothing is invalidated

people[0] = Person { name = "Bao", age = 12, pets = [] }
console.log(person.name) -- "Bao": `people[0]` is still a valid place, holding the new value
console.log(age) -- 12: a flat field of the replaced value is still a valid place too
console.log(pet.name) -- error: `pet` was invalidated: the old `pets` array was destroyed
console.log(initials) -- error: `initials` was invalidated: the old `name` was destroyed
```

A mutation is declared as precisely as needed, so that it invalidates as little as possible: `mutates player.score` invalidates nothing when `score` is an `Integer`, while `mutates player` invalidates every reference into the memory `player` owns.

Mutating a whole collection (`mutates self`) invalidates every reference into it. This is what `Array.push` declares: growing an array may reallocate its storage, which moves every element.

Adding an element to a collection that never moves its elements, like a [`Bag`](/collections/bags), only changes its length: `Bag.add` declares `mutates self.length`. The length is an `Integer`, so nothing is invalidated. Outside the implementation of a collection, its `length` is read-only.

### Invalidations

`invalidates path` means that the place at `path` itself may disappear, or no longer designate the same element: the references to it, and everything inside it, are invalidated. This effect is reserved for precise places, such as one element or a range of elements. An operation that affects a whole collection declares `mutates self` instead.

| Effect | Invalidates the references to |
| --- | --- |
| `invalidates self[key]`, `invalidates self[index]` | the element at this key or index |
| `invalidates self[index..]` | the elements from this index on |
| `invalidates element` | this element |

`Array.delete(index)` declares `invalidates self[index..]`: the elements after `index` are shifted down, so `persons[3]` would silently designate the next person. The compiler makes you declare these references again instead.

### Effects of the collections

| Collection | Operation | Effect | References that stay valid |
| --- | --- | --- | --- |
| [`Array`](/collections/arrays) | `array[index] = value` | `mutates self[index]` | all but the references into the replaced element |
| | `delete(index)`, `take(index)` | `invalidates self[index..]` | the elements before `index` |
| | `pop` | `invalidates self[self.length - 1]` | all but the last element |
| | `push`, `insert` | `mutates self`: growing may reallocate the array | none |
| | `sortInPlace`, `reverseInPlace`, `deleteWhere`, `clear` | `mutates self` | none |
| [`Map`](/collections/maps) | `map[key] = value` | `mutates self[key]`: values live in stable slots | all but the references into the replaced value |
| | `getOrInsert` | `mutates self.length`: it only adds a key | all |
| | `delete(key)`, `take(key)` | `invalidates self[key]` | the other keys |
| | `deleteWhere`, `clear` | `mutates self` | none |
| [`Set`](/collections/sets) | `add`, `delete`, `deleteWhere`, `clear` | `mutates self`: a set may rehash, which moves its values | none |
| [`Bag`](/collections/bags), [`Columnar`](/collections/columnars) | `add` | `mutates self.length`: adding never moves the other elements | all |
| | `delete(element)`, `take(element)` | `invalidates element` | the other elements |
| | `deleteWhere`, `clear` | `mutates self` | none |
| [`Growing*`](/collections/growing-collections) | `push`, `add` | `mutates self.length`: they never delete or move | all |

### Exact keys and indices

To invalidate as little as possible, the compiler remembers where each reference comes from, with the **exact** key or index whenever it is known at compile time, not just its type. Like the [range of an integer](/values/numbers#ranges-through-arithmetic), this knowledge propagates through the program: the type of `"ana"` is `String`, but the compiler knows it is exactly `"ana"`.

```fa
mutable scores: Map(String, Integer) = [("ana", 10), ("bao", 7)]

use score = scores.get("ana")

scores.delete("bao") -- invalidates `scores["bao"]`: `score` is `scores["ana"]`, it stays valid

let name = readName()
scores.delete(name) -- `name` may be "ana": `score` is invalidated
```

An invalidation targets every reference that **may** designate the invalidated element, and so does a mutation for the references into the mutated value. Two references are known to be distinct when the compiler can prove it: their keys or indices are known and different, or they were returned by two different calls to `add`:

```fa
mutable people: Bag(Person) = []

use ana = people.add({ name = "Ana" })
use bao = people.add({ name = "Bao" })

people.delete(bao) -- invalidates `bao`
console.log(ana.name) -- fine: `ana` and `bao` were returned by two different `add` calls

use someone = people.find((person) => person.name == "Cleo")
if someone is not none {
	people.delete(someone)
	console.log(ana.name) -- error: `ana` was invalidated by `people.delete`: `someone` may be `ana`
}
```

Stored references don't need any of this: they are [relations](/advanced/relations), cleaned up at runtime when their target is deleted.

## After invalidation

After a call that invalidates a reference, using it is a compile error until you **declare it again**, whatever the collection:

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

people.deleteWhere((person) => person.age < 18) -- `mutates people`: invalidates every reference into it

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

let loser = readName()
scores.delete(loser) -- `loser` may be "ana"

console.log(score) -- error: `score` was invalidated by `scores.delete`: declare it again with `use score = scores.get(...)`

use score = scores.get("ana")
```

There is no way to check whether an invalidated reference is still valid: Fa keeps no generation counter to compare at runtime. Declaring it again is the only form, and it says exactly which element you mean.

Declaring a name twice in the same scope is normally an error (see [declaring values](/values/declaring-values#shadowing)). Declaring again an invalidated reference is the only exception: redeclaring a reference that is still valid remains an error.

## Invalidation through functions

Your own functions declare the same effects, and the compiler checks them at every call site:

```fa
function addGuest = (guests: Array(Person), name: String) mutates guests => {
	guests.push(Person { name = name })
}

function welcome = (guest: Person) mutates guest.welcomed => {
	guest.welcomed = true
}

mutable guests: Array(Person) = [Person { name = "Ana" }, Person { name = "Bao" }]
use second = guests[1]

welcome(guests[1]) -- `second` is still valid: `welcomed` is a `Boolean`, a flat value
console.log(second.welcomed) -- true

addGuest(guests, "Chloé") -- invalidates `second`: the whole array is mutated
console.log(second.name) -- error: `second` was invalidated by `addGuest`: declare it again with `use second = guests[...]`
```

When the body of a function is a single expression, its effects are inferred. When it is a block, they must be declared: a function that calls `push` without declaring `mutates` on the array is a compile error.

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

`take` moves rather than copies, and it declares the same effect as `delete`: `take(1)` on an array invalidates the references from index 1 on.
