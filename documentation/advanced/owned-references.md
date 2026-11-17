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
- **stored**: a field typed `(Person in people)?` or `Pool<Person in people>`. A stored reference is either a [relation](/advanced/relations), maintained on writes/deletion, or an explicit [link](/advanced/relations#links-without-a-reverse-side), contained by an owner of growing collections.

A local reference can't be stored as it is: a field typed `Person` or `Person?` owns a value under the [storage rules](/advanced/memory-management#copies-moves-unique-values-and-shared-handles). To store a reference, declare a relation or an explicit link field. A local collection of references, like `Array<Person in people>`, is not allowed either: keep indices, or declare a relation `Pool`. This also applies inside aliases, tuples, generic arguments and function types, including result types (`REF010`). Only the whole parameter or result may declare a local reference; nesting it inside an owned type would lose its collection.

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
- Any other value owns memory: the elements of a collection, the storage of a string, the payload of a union (including an optional), the children of a [recursive type](/advanced/memory-management#recursive-types). Mutating it invalidates every reference into that memory: the elements, the [string slices](/values/strings#slices), the payload, and everything inside them.

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

`reserve` declares `mutates self` on `Array`, `Set` and `OrderedColumnar`, and `mutates self.capacity` on every other collection, where it moves no item. An effect on `length` or `size` implies the same effect on `capacity`. See [capacity and reservation](/collections/arrays#capacity-and-reservation).

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

Stored references use a different proof: relations clean up on deletion, while explicit links require growing targets and [complete-owner containment](/advanced/reference-checking#link-containment).

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

A place that a call mutates can't be passed as another argument of the same call, or read by a callback given to it: `guests.push(guests[0])` is an error, since `push` may move `guests[0]` while reading it. Obtain an independent value first, with `let snapshot = copy value` for a collection owner. See [arguments that alias](/advanced/reference-checking#arguments-that-alias).

## Lifetime

The lifetime of an owned reference is the lifetime of the collection that owns it.

Because every value in Fa is destroyed at the end of its scope, a reference cannot outlive its collection by accident: the compiler rejects any code where it would.

## Returning a reference

Returning a reference into a collection that is local to the function is impossible, since the collection is destroyed on return. An ordinary result copies out; a structure result requires explicit `copy`, and a unique element must be removed with an owning operation such as `take`:

```fa
function createPerson = (): Person => {
	mutable persons = [Person {}]
	use person = persons[0] -- type: `Person in persons`
	person -- converted to `Person`: a copy is made
}
```

When the collection comes from the caller, the reference can be returned as it is. Say so in the return type, and make it optional if the collection may be empty. A function returns a reference only when its return type says `in`: with an inferred return type, it returns an owned value under those same rules.

```fa
function firstPerson = (persons: Array<Person>): (Person in persons)? => {
	if persons.length == 0 {
		return none
	}
	use person = persons[0]
	person -- no copy: the caller owns `persons`
}
```

A reference result can be an `if`, `when`, `catch`, block or `loop` expression. A loop returns the reference supplied by its reachable `break` values. Every reachable result must designate an element of the declared collection; returning a local collection's element in just one branch is still an error. A reseatable reference must satisfy that rule for every place it may designate at the return. An `async` function cannot declare a reference result: return an index or an owned copy instead.

The caller receives a reference whose validity is tied to its own collection: the parameter name in the return type is replaced by the argument, so `firstPerson(persons)` returns a `(Person in persons)?`. The caller uses the evaluated result provenance for validity: known functions retain their field and nested collection paths, including across imports; an unknown function value conservatively borrows anywhere below its declared owner. Collection metadata such as `length` is not an element and cannot satisfy an `in` result.

```fa
mutable persons = [Person { name = "Ana" }]

use first = firstPerson(persons)

if first is not none {
	console.log(first.name) -- "Ana"

	persons.push(Person {}) -- invalidates `first`
	console.log(first.name) -- error: `first` was invalidated by `persons.push`: declare it again with `use first = ...`
}
```

## Owner paths through methods

An enclosing type binds its nested element types to each owner's collection instance. Methods may return `Node in self.nodes`; dependent parameters may write `(model: Model, node: Model.Node in model.nodes)`. Calls replace self/parameter paths with the caller's full path, including private and nested paths, through generic functions, imports and function values. Results stored with `use` borrow the caller's collection; an inferred ordinary result is still owned data. Only a direct call whose every reachable reference result comes from a fresh add exports freshness; calls through function values have unknown identity.

A function following stored links needs that named owner path (or a module owner), because the target binding comes from the signature. A structural Node view may read ordinary fields but cannot invent a missing owner binding. Return a complete locally built owner to keep it alive; returning a reference into a local pool is `REF008`/`REF009`. A reference result into a temporary cannot be bound with `use` (`REF017`, alias of `REF004`): bind the owner first, or request an independent data value with `let` where ownership permits it.

## Moving an element out

Every deletable collection has a `take` method, which removes an element and returns it as a standalone value:

```fa
mutable arrayOfPersons = [Person {}, Person {}, Person {}]
mutable anotherArrayOfPersons = [Person {}]

let someone = arrayOfPersons.take(1)

anotherArrayOfPersons.push(someone)
```

`take` moves rather than copies, and it declares the same effect as `delete`: `take(1)` on an array invalidates the references from index 1 on.

:::implementation
## Reference function ABI

Native and wasm functions with an explicit borrowed result return its address
(nullable for an optional), using the ordinary error-result ABI when they throw.
Dependent parameters receive that same borrowed address, including primitive
subfields; no owned temporary may replace the designated place. Local `use`
keeps it, while an ordinary owning store follows the data's copy/move rules.
Relation-field borrows read the target address once, rather than aliasing the
field that can later be reassigned. Columnar references keep their row locator.
Collection lookups retain the element's slot address through optional narrowing;
an absent lookup and `reuse reference = none` both use a null address. Parentheses
and `try` preserve this designation without evaluating a lookup again.

JavaScript passes and returns object references as the objects themselves, with
`undefined` for an absent reference. A local primitive reference is a direct
place: evaluate its innermost container and key once, then rewrite each read or
write to that member access. `use score = player.score; score += 1` emits
`player.score += 1`. A mutable member reference retains two locals reassigned by
`reuse`. When a mutable reference can also designate scalar locals, a selector
chooses direct reads and writes of those locals; its other alternative uses the
container and key. Copying that reference with `use` snapshots the selector and
location, without copying the referent or allocating a cell. A map slot uses its existing `get`/`set` operations; a primitive pool
uses its slot array and stable slot index, with direct member accesses. There are no accessor cells or reference caches.

At a function boundary a primitive or mixed-union reference has two parts:
**(innermost container, key)**. For `values[i]` these are `(values, i)`;
for `values[i].left`, `(values[i], "left")`; for a nested collection slot,
the inner collection and its index. A dependent primitive parameter takes two
JavaScript arguments. Other parameters retain their ordinary argument list.
A general result returns the container and publishes the key in the shared
runtime module's live `faReferenceKey` binding. The caller captures both
immediately after the call, before evaluating anything else. Publishing happens
after the callee's cleanup. A throwing call publishes only on success; optional
absence uses an absent container. Reference results cannot be async, so no
`await` can intervene.

A known function additionally gets a compressed direct-call entry when all its
reachable result places prove the same container or key. A fixed key can be a field name, a nonnegative array index, or a literal map key; a negative array index depends on the container length and keeps the general convention. The analyzed layout
summary is exported and hashed with its interface, so separate compilation uses
the same convention. For example, the following JavaScript sketches omit bounds
checks and error propagation to show only the reference convention:

```js
// Every result is an element of the owner: return only the key.
function element$key(values, i) { return i; }
const i = element$key(values, nextIndex());
values[i] += 1;

// Every result has the static key "left": return only the container.
function left$field(values, i) { return values[i]; }
const node = left$field(nodes, 0);
node.left += 1;

// A fixed nested index also returns only its innermost container.
function nested$index(values, i) { return values[i].children; }
const children = nested$index(nodes, 0);
children[0] += 1;

// Dynamic projections: return the container and publish the key.
function choose(values, right) {
    return faReferenceResult(values[0], right ? "right" : "left");
}
const container = choose(nodes, condition);
const key = faReferenceKey;
container[key] += 1;

// Function values use the general convention of their function type.
const pick = choose;
const selectedContainer = pick(nodes, condition);
const selectedKey = faReferenceKey;
selectedContainer[selectedKey] += 1;
```

The general entry remains available even when a compressed entry exists, so
converting a known function to a function value needs no binding wrapper.
Generic bodies specialize only representation-sensitive parameters, selecting
bare objects or located slots from the instantiated type. Generic callbacks
select the corresponding emitted body through static function properties.
Ordinary generic calls and callbacks acquire neither representation flags nor
reference binding wrappers. Mixed primitive/object unions retain the slot form
regardless of the current member. Whole-object writes preserve the existing
object's identity. This is a call convention, not the storage representation of
a link.

The compile-time interpreter retains evaluated places as a frame identity and
generation, a local slot, and field/index keys evaluated once. Borrowed arguments,
local aliases and results retain that identity; ordinary reads materialize data.
Caller-owned argument and default temporaries remain alive until the caller
finishes; immutable module owners remain in the evaluation's root frame. Pools
keep stable slots through removal. Internal place carriers cannot be embedded
as constants: only the resulting data leaves evaluation.

Owner paths, effects and direct-call freshness stay compile-time facts; they
survive callable interface merging, hashing and import substitution. Structured
results merge addresses on native/wasm and evaluated container/key parts on JavaScript, with freshness
only when every reachable result is fresh.
:::
