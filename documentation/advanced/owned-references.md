# Owned references

An **owned reference** is a reference to an element *inside a collection*. The collection owns the element; the reference points at it, and its type names the collection.

```fa
let persons: Array<Person> = [Person {}, Person {}, Person {}]

let copied = persons[1] -- a copy
use someone = persons[1] -- a reference

console.log(@typeToString(Type<copied>)) -- "Person"
console.log(@typeToString(Type<someone>)) -- "Person in persons"
```

The type of `someone` is not `Person` but `Person in persons`. The `in` part is what makes the reference safe: the type system knows exactly which collection the reference depends on. The type of a `use` is inferred; when you write it yourself, in a parameter or a return type, you write it in full: `Person in persons`.

This page explains the model with examples. [Reference checking](/advanced/reference-checking) gives the exact rules the compiler applies.

## Local and stored references

A reference is either:

- **local**: declared with `use` or `mutable use`, received as a parameter, or returned by a function. The compiler tracks it statically, call after call. This page is about local references.
- **stored**: a field typed `(Person in people)?` or `Pool<Person in people>`. A stored reference is always one side of a [relation](/advanced/relations), with a declared reverse side: the compiler updates both sides on every write, and when the target is deleted.

A local reference can't be stored as it is: a field typed `Person` or `Person?` holds a copy. To store a reference, declare a relation. A local collection of references, like `Array<Person in people>`, is not allowed either: keep indices, or declare a relation `Pool`.

## Invalidation

A reference is **invalidated** when the memory it points to may have been freed, moved, or reused for a value of another type. The compiler knows when this happens:

```fa
mutable persons: Array<Person> = [Person {}, Person {}, Person {}]

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

A union that is not a union of literals is never flat, even when its members hold no heap memory, like `Integer?`: a new value may change its member, and so what its payload is. A relation reference is the exception, because a local reference never points into it: `use friend = ana.bestFriend` designates the target, not the field.

```fa
type Person = {
	name: String
	age: Integer
	pets: Array<Pet>
}

mutable people: Array<Person> = [Person { name = "Ana", age = 31, pets = [Pet { name = "Rex" }] }]

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

A mutation is declared as precisely as needed, so that it invalidates as little as possible: `mutates player.score` invalidates nothing when `score` is an `Integer`, while `mutates player` invalidates every reference into the memory `player` owns. `[..]` designates every element: `mutates players[..].score` writes the score of every player, and invalidates nothing either.

Mutating a whole collection (`mutates self`) invalidates every reference into it. This is what `Array.push` declares: growing an array may reallocate its storage, which moves every element.

Adding an element to a collection that never moves its elements, like a [`Pool`](/collections/pools), only changes its size: `Pool.add` declares `mutates self.size`. The size is an `Integer`, so nothing is invalidated. Outside the implementation of a collection, its `length` or `size` is read-only.

### Invalidations

`invalidates path` means that the place at `path` itself may disappear, or no longer designate the same element: the references to it, and everything inside it, are invalidated. This effect is reserved for precise places, such as one element or a range of elements: an operation that affects a whole collection declares `mutates self` instead, and `invalidates self` is an error.

| Effect | Invalidates the references to |
| --- | --- |
| `invalidates self[key]`, `invalidates self[index]` | the element at this key or index |
| `invalidates self[index..]` | the elements from this index on |
| `invalidates element` | this element |

`Array.delete(index)` declares `invalidates self[index..]`: the elements after `index` are shifted down, so `persons[3]` would silently designate the next person. The compiler makes you declare these references again instead.

`invalidates` implies `mutates` on the same path, and an element that disappears changes the size of its collection: `Pool.delete(element)`, with `element: T in self`, also mutates `self.size`. Conversely, `mutates path` covers the invalidations of the places inside it: a function that declares `mutates self` may delete elements of `self`, since its callers already treat every reference into `self` as invalidated.

### Effects of the collections

| Collection | Operation | Effect | References that stay valid |
| --- | --- | --- | --- |
| [`Array`](/collections/arrays) | `array[index] = value` | `mutates self[index]` | all but the references into the replaced element |
| | `delete(index)`, `take(index)` | `invalidates self[index..]` | the elements before `index` |
| | `pop` | `invalidates self[self.length - 1]` | all but the last element |
| | `push`, `insert` | `mutates self`: growing may reallocate the array | none |
| | `sortInPlace`, `reverseInPlace`, `deleteWhere`, `clear` | `mutates self` | none |
| [`Map`](/collections/maps) | `map[key] = value` | `mutates self[key], self.size`: it may add the key, and values live in stable slots | all but the references into the replaced value |
| | `getOrInsert` | `mutates self.size`: it only adds a key | all |
| | `delete(key)`, `take(key)` | `invalidates self[key]` | the other keys |
| | `deleteWhere`, `clear` | `mutates self` | none |
| [`Set`](/collections/sets) | `add`, `delete`, `take`, `deleteWhere`, `clear` | `mutates self`: a set may rehash, which moves its values | none |
| [`Pool`](/collections/pools), [`Columnar`](/collections/columnars) | `add` | `mutates self.size`: adding never moves the other elements | all |
| | `delete(element)`, `take(element)` | `invalidates element` | the other elements |
| | `deleteWhere`, `clear` | `mutates self` | none |
| [`OrderedColumnar`](/collections/columnars#ordered-columnars) | `columnar[index] = value` | `mutates self[index]` | all but the references into the replaced element |
| | `add` | `mutates self`: adding may reallocate the columns | none |
| | `delete(index)`, `take(index)` | `invalidates self[index..]` | the elements before `index` |
| | `pop` | `invalidates self[self.length - 1]` | all but the last element |
| | `deleteWhere`, `clear` | `mutates self` | none |
| [`GrowingArray`](/collections/growing-collections) | `push` | `mutates self.length`: a `GrowingArray` never deletes or moves | all |
| | `array[index] = value` | `mutates self[index]` | all but the references into the replaced element |
| [`GrowingMap`](/collections/growing-collections) | `map[key] = value` | `mutates self[key], self.size` | all but the references into the replaced value |
| | `getOrInsert` | `mutates self.size` | all |
| `GrowingSet`, `GrowingPool`, `GrowingColumnar` | `add` | `mutates self.size`: growing collections never delete or move | all |

Relations add implied effects to these operations: deleting a linked object also writes the fields that pointed at it. See [reference checking](/advanced/reference-checking#relations).

### Exact keys and indices

To invalidate as little as possible, the compiler remembers where each reference comes from, with the **exact** key or index whenever it is known at compile time, not just its type. Like the [range of an integer](/values/numbers#ranges-through-arithmetic), this knowledge propagates through the program: the type of `"ana"` is `String`, but the compiler knows it is exactly `"ana"`.

```fa
mutable scores: Map<String, Integer> = [("ana", 10), ("bao", 7)]

use score = scores.get("ana")

scores.delete("bao") -- invalidates `scores["bao"]`: `score` is `scores["ana"]`, it stays valid

let name = readName()
scores.delete(name) -- `name` may be "ana": `score` is invalidated
```

An invalidation targets every reference that **may** designate the invalidated element, and so does a mutation for the references into the mutated value. Two references are known to be distinct when the compiler can prove it, with a fixed set of facts: their keys or indices are known and different (exact values, integer ranges that don't intersect, or the same immutable value with different offsets, like `index` and `index + 1`), or one of them was returned by a call to `add` made after the other one existed. See [may designate](/advanced/reference-checking#may-designate) for the exact rules.

```fa
mutable people: Pool<Person> = []

use ana = people.add({ name = "Ana" })
use bao = people.add({ name = "Bao" })

people.delete(bao) -- invalidates `bao`
console.log(ana.name) -- fine: `bao` was returned by an `add` made after `ana` existed

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
mutable persons: Array<Person> = [{ name = "Ana" }, { name = "Bao" }, { name = "Cleo" }]

use someone = persons[1] -- Bao

persons.delete(0) -- Ana is removed: Bao and Cleo move down by one

console.log(someone.name) -- error: `someone` was invalidated by `persons.delete`: declare it again with `use someone = persons[...]`

use someone = persons[0] -- fine: `someone` was invalidated, it can be declared again
console.log(someone.name) -- "Bao"
```

When the element may be gone, declare the reference again with a lookup that returns an [optional](/values/optionals), then check the optional:

```fa
mutable people: Pool<Person> = []

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
mutable scores: Map<String, Integer> = [("ana", 10), ("bao", 7)]

use score = scores.get("ana") -- type: `(Integer in scores)?`

let loser = readName()
scores.delete(loser) -- `loser` may be "ana"

console.log(score) -- error: `score` was invalidated by `scores.delete`: declare it again with `use score = scores.get(...)`

use score = scores.get("ana")
```

A reference declared with [`mutable use`](/values/declaring-values#changing-what-a-reference-designates) is not declared again: it is moved with `reuse`, which makes it valid again.

```fa
mutable people: Pool<Person> = []

mutable use current = people.add({ name = "Ana" })

people.delete(current) -- invalidates `current`

reuse current = people.add({ name = "Bao" }) -- `current` is valid again
```

After a condition or a loop that may move it, the compiler considers that a `mutable use` reference may designate any of the places it was given: an operation that invalidates one of them invalidates the reference.

A reference invalidated in one branch only is invalidated after the branches, and can be declared again there. A declaration again inside a branch or a loop body only lasts until the end of that block: to repair a reference from inside a branch or a loop, declare it with `mutable use` and move it with `reuse`.

```fa
mutable people: Pool<Person> = []

use ana = people.add({ name = "Ana" })

if leaving {
	people.delete(ana)
}

console.log(ana.name) -- error: `ana` may have been invalidated by `people.delete`
use ana = people.find((person) => person.name == "Ana") -- fine: declared again
```

See [control flow](/advanced/reference-checking#control-flow) for the exact rules, including loops.

There is no way to check whether an invalidated reference is still valid: Fa keeps no generation counter to compare at runtime. Declaring it again is the only form, and it says exactly which element you mean.

Declaring a name twice in the same scope is normally an error (see [declaring values](/values/declaring-values#shadowing)). Declaring again an invalidated reference is the only exception: redeclaring a reference that is still valid remains an error.

## Invalidation through functions

Your own functions declare the same effects, and the compiler checks them at every call site:

```fa
function addGuest = (guests: Array<Person>, name: String) mutates guests => {
	guests.push(Person { name = name })
}

function welcome = (guest: Person) mutates guest.welcomed => {
	guest.welcomed = true
}

mutable guests: Array<Person> = [Person { name = "Ana" }, Person { name = "Bao" }]
use second = guests[1]

welcome(guests[1]) -- `second` is still valid: `welcomed` is a `Boolean`, a flat value
console.log(second.welcomed) -- true

addGuest(guests, "Chloé") -- invalidates `second`: the whole array is mutated
console.log(second.name) -- error: `second` was invalidated by `addGuest`: declare it again with `use second = guests[...]`
```

When the body of a function is a single expression, its effects are inferred. When it is a block, they must be declared: a function that calls `push` without declaring `mutates` on the array is a compile error.

A place that a call mutates can't be passed as another argument of the same call, or read by a callback given to it: `guests.push(guests[0])` is an error, since `push` may move `guests[0]` while reading it. Copy the value first, with `let`. See [arguments that alias](/advanced/reference-checking#arguments-that-alias).

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

When the collection comes from the caller, the reference can be returned as it is. Say so in the return type, and make it optional if the collection may be empty. A function returns a reference only when its return type says `in`: with an inferred return type, it returns a copy.

```fa
function firstPerson = (persons: Array<Person>): (Person in persons)? => {
	if persons.length == 0 {
		return none
	}
	use person = persons[0]
	person -- no copy: the caller owns `persons`
}
```

The caller receives a reference whose validity is tied to its own collection: the parameter name in the return type is replaced by the argument, so `firstPerson(persons)` returns a `(Person in persons)?`. The index of the element is not known to the caller.

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
