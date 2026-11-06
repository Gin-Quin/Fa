# Reference checking

This page is the specification of the reference checker: the part of the compiler that decides, for every local reference, whether it is still valid where it is used. [Owned references](/advanced/owned-references) and [relations](/advanced/relations) explain the model with examples; this page states the rules precisely. Every compiler version accepts exactly the programs these rules accept: a smarter analysis that would accept more programs is not allowed, so that a program that compiles today compiles everywhere.

The checker runs on every target, before any code is generated. It has no runtime counterpart: there is no generation counter and no liveness check.

## Vocabulary

- A **place** is a memory location that holds a value: a local value, a parameter, a module-level value, a field of a place, an element of a collection, the characters of a string, the payload of a union.
- A **local reference** is a name that designates a place without owning it: a `use` or `mutable use` alias, a parameter, a loop variable, an iterator. "Reference" means local reference on this page; a reference stored in a field is one side of a [relation](/advanced/relations).
- A reference is **valid** or **invalidated**. Using an invalidated reference (reading it, writing through it, passing it, moving another reference from it) is a compile error that names the operation that invalidated it.
- A **fact** is what the checker knows about a value at a point of the program: an exact literal, an integer range, a narrowed union member (see [range analysis](/values/numbers#range-analysis) and [narrowing](/control-flow/conditionals#narrowing)). The facts used here are listed in [may designate](#may-designate).

## Places and paths

Effects name places with **paths**. The same paths describe the place each reference designates.

```
path   = root { "." field | ".#" field | "[" index "]" | "[" range "]" | "[..]" }
root   = parameter | "self" | module-level value | requirement name
index  = term [ ("+" | "-") integer literal ]
term   = literal | parameter | root ".length" | root ".size"
range  = [ index ] ( ".." | "..=" ) [ index ]
```

- `[index]` designates the element at an index (`Array`, `GrowingArray`, `OrderedColumnar`, `String`) or at a key (`Map`, `GrowingMap`): `self[key]`.
- `[range]` designates the elements whose index is in the range: `self[index..]`.
- `[..]` designates every element: `mutates players[..].score` mutates the score of every player, and nothing else.
- `length` and `size` are read-only fields of a collection: `mutates people.size` says that elements may be added, without moving any.
- An effect lists several paths separated by commas: `mutates self.firstChild, self.lastChild`.

```fa
type Player = { name: String, score: Integer }

function resetScores = (players: Array<Player>) mutates players[..].score => {
	for player in players {
		player.score = 0
	}
}
```

Inside a body, the checker names every place from these roots, and from the function's own local values. A local reference stands for the place it designates; an element whose index or key isn't known is `[..]`; a place reached through a relation field starts again at the collection of the target: in `node.next?.data`, where `next: (Node in self.#nodes)?`, the place is `self.#nodes[..].data`.

The paths of an effect are evaluated with the values before the call: `Array.pop` declares `invalidates self[self.length - 1]`, the last element before it is removed. A path that designates no element (`pop` on an empty array) invalidates nothing. A call applies its effects on every exit, including when it throws.

### Owning edges

Each step of a path is an **edge**. An edge **owns memory** when the value it leads to may be destroyed or replaced by a mutation of the value it starts from:

| Edge | Owns memory |
| --- | --- |
| an element of a collection: `[index]`, `[key]`, `[range]`, `[..]` | yes |
| the characters of a string (a slice `text[1..3]`) | yes |
| the payload of a union or of an optional, including after narrowing, and the children of a recursive type | yes |
| a field of an object: `.field`, `.#field` | no: the field is stored inline |
| `.length`, `.size` | no |

A type is **flat** when no path below it goes through an owning edge: a primitive, a union of literals, a relation reference, an object whose fields are all flat. Any other union is not flat, even when it holds no heap memory (`Integer?`, `Union { Circle = { radius: Float }, Square = { side: Float } }`): changing the member changes what its payload is. A relation reference is flat because `use` never points into it: it binds its target (see [`use` and `mutable use`](#use-and-mutable-use)).

### Mutations

`mutates P` gives a new value to `P`, which stays a valid place. It invalidates a reference `R` if and only if `R` is strictly below `P` and the path from `P` to `R` goes through at least one owning edge.

```fa
type Person = { name: String, age: Integer, pets: Array<Pet> }

mutable people: Array<Person> = [Person { name = "Ana", age = 31, pets = [Pet { name = "Rex" }] }]

use person = people[0]         -- the place `people[0]`
use age = people[0].age        -- `.age`: no owning edge below `people[0]`
use initials = people[0].name[0..1] -- `.name`, then the characters: an owning edge
use pet = people[0].pets[0]    -- `.pets`, then `[0]`: an owning edge

people[0] = Person { name = "Bao", age = 12, pets = [] } -- `mutates people[0]`
-- valid: `person`, `age`
-- invalidated: `initials`, `pet`
```

A reference above `P` stays valid too: after `mutates people[0]`, a reference to `people` itself is still the same array. Mutating a flat place (`mutates people.size`, `mutates player.score`) invalidates nothing.

The type that decides the owning edges is the type of the path as the callee declares it, with the type arguments of the call substituted. A function `(person: { age: Integer }) mutates person` can only write `age`, so it invalidates nothing in the caller, whatever the other fields of the argument. Inside a generic body, a value of a type parameter is never flat.

### Invalidations

`invalidates P` means that the place `P` may disappear, or designate another element. It invalidates a reference `R` if and only if `R` may designate `P`, or a place below `P` (see [may designate](#may-designate)). `invalidates self[index..]` covers every index from `index` on.

`invalidates P` implies `mutates P`. When `P` is a parameter typed `T in c` (`invalidates element` in `Pool.delete`), it also implies `mutates c.size`: the collection loses an element.

### What a body may do

With a block body, the declared effects must cover every effect of the body; with a single-expression body, they are inferred. A declared path covers the paths below it; `[..]` covers every index and key; a declared effect covers its implied effects (see [relations](#relations)). Effects on the function's own local values are not effects of the function.

Every declared `mutates` path must also cover an effect that the body actually performs. Declaring an unused mutation is an error, including on a single-expression body or a function with an explicit function type. The covering rules still allow `mutates player` for a body that writes `player.score`.

## May designate

The checker keeps, for each reference, its **owner** (the collection or string it points into, see [reference types](#reference-types)), its path below the owner, and an **identity** for each index or key on that path. An identity is one of:

- an **exact value**: a literal, or a value whose exact literal is a fact (`people[0]`, `scores.get("ana")`);
- an **integer range**, from the type or the facts of the index (see [range analysis](/values/numbers#range-analysis));
- a **symbol plus an offset**: an immutable binding (`let`, a parameter) or the `length` of a collection that hasn't changed since, plus a constant (`people[index + 1]`, `people[people.length - 1]`);
- **fresh**: the reference was returned by a call that creates a new element (below);
- **unknown**.

The identity is taken when the reference is created: a later change of a `mutable` index doesn't change it. A fact about a collection's `length` is dropped when the length changes.

Two places **may designate** the same place unless the checker proves them distinct with one of these rules, applied at the first step where their paths differ:

1. Different fields, or different roots: two local values, two parameters, a parameter and a module-level value. Parameters are distinct because of the rule on [arguments that alias](#arguments-that-alias), except a parameter typed `T in p`, which is inside `p`.
2. Two exact values that differ, two integer ranges that don't intersect, or the same symbol with different offsets. Against a range path (`[a..]`, `[a..b]`), an index is distinct when the same comparisons prove it below the start or above the end.
3. One of them is fresh, and the other existed before the call that created it: a fresh reference is distinct from every reference live before its call, and from every later fresh reference.
4. Their owners are distinct by rules 1 to 3 (`towns[0].people` and `towns[1].people`).

Nothing else is proven: no relation between two different symbols is kept (a guard `if j < i` only narrows the ranges of `i` and `j`), and there is no other reasoning about arithmetic.

**Fresh references** are returned by the built-in `add` of `Pool`, `GrowingPool`, `Columnar` and `GrowingColumnar`, and by a function whose every returned reference provably comes from such an `add` in its body, or from another function that returns fresh references. Freshness is inferred, and is part of the function's interface, like its inferred effects. A call through a function value never returns a fresh reference.

Other identity facts don't cross a user function: a reference returned by `pick(people, 2)` has an unknown index. Built-in operations keep them: `getOrInsert(key)` returns a reference whose key is the identity of `key`.

```fa
function example = (index: Integer<min = 0, max = 2>) => {
	mutable people: Array<Person> = [Person { name = "Ana" }, Person { name = "Bao" }, Person { name = "Cleo" }, Person { name = "Dan" }]

	use current = people[index]
	use last = people[3]

	people.delete(index + 1) -- `invalidates people[index + 1..]`

	console.log(current.name) -- valid: `index` is below `index + 1`
	console.log(last.name) -- error: `last` was invalidated by `people.delete`: 3 may be `index + 1` or above
}
```

## Control flow

The checker walks the structured code once, applying the effects of each operation in evaluation order. Its state holds, for each reference, whether it is valid, its identity, and for a `mutable use`, the set of places it may designate.

- **Branches** (`if`, `when`, `catch` handlers, the right side of `and`, `or` and `??`): after the branches, a reference is invalidated if it is invalidated in any branch. A reference that is only maybe invalidated counts as invalidated. A `mutable use` may designate any place it designates in one of the branches.
- A branch that ends with `return`, `throw`, `break` or `continue` doesn't reach the code after the branches.
- **Loops** (`for`, `while`, `loop`) are analysed to a fixpoint: the state at the start of the body joins the state before the loop and the state at the end of every iteration (and every `continue`); the state after the loop joins the exits and every `break`. The state is finite (identities follow the widening of [range analysis](/values/numbers#range-analysis)), so the fixpoint is reached.
- A **non-escaping closure** runs during the call it is passed to: its effects are effects of that call, and the references it captures are used during that call (see [arguments that alias](#arguments-that-alias)). An escaping closure captures copies, and holds no reference (see [closures](/functions/defining-functions#closures)).

### Declaring again

- A `use` reference that is invalidated, or maybe invalidated, may be declared again with `use` in the same scope. Declaring again a valid reference is an error.
- A declaration in an inner block (a branch, a loop body) is a new name that ends with that block: after the block, the outer reference is still invalidated. To repair a reference from inside a branch or a loop, declare it with `mutable use` and move it with `reuse`.
- A declaration again may change the type: `Person in people` can become `(Person in people)?`.
- A `mutable use` is never declared again: it is moved with `reuse`, which makes it valid.

## Reference types

The type of a reference is `T in path`: `T` is the type of the designated value, and `path` names its owner, the nearest collection or string that contains the place.

| Declaration | Type |
| --- | --- |
| `use someone = persons[1]` | `Person in persons` |
| `use age = people[0].age` | `Integer in people` |
| `use pet = people[0].pets[0]` | `Pet in people[0].pets` |
| `use middle = text[1..3]` | `String in text` |
| `use score = scores.get("ana")` | `(Integer in scores)?` |
| `use address = user.address` (not inside a collection) | `Address`: an alias, tracked like any reference, that can't be returned |

- The type of a `use` is inferred, and may be written: `use someone: Person in persons = persons[1]`. It is written in full in parameters, return types and relation fields.
- The path after `in` follows the grammar of [paths](#places-and-paths). At a call, the parameter names in the callee's reference types are replaced by the argument paths: `firstPerson(persons: Array<Person>): (Person in persons)?` called as `firstPerson(town.people)` returns a `(Person in town.people)?`. When that argument isn't a place (a temporary), the result can only be copied with `let`.
- A parameter can be typed by another parameter: in `(self, node: Node in self.#nodes)`, the argument of `node` must be a reference into the argument of `self`. Private paths like `self.#nodes` may appear in a public signature: the caller holds a `Node in list.#nodes`, but can't read `list.#nodes`.
- Owners are identified by their path, with the identities of [may designate](#may-designate): `town1.people` and `town2.people` are two owners; `towns[i].people` and `towns[j].people` may be the same one.
- Inside a type, a field type names a collection of the enclosing value: in `type Town = { people: Pool<Person> = [] ... }`, the field `bestFriend: (Person in people)?` of an element reached as `Person in town.people` has the type `(Person in town.people)?`.
- A function returns a reference only when its declared return type says `in`, and the path after `in` starts at a parameter or a module-level value. With an inferred return type, it returns a copy. A returned reference is valid after the effects of the call.

## `use` and `mutable use`

- `use x = place` makes `x` an alias of the place. When the place holds a relation reference (a direct side, a one-to-one reverse side, an owner field), `x` designates the **target**, not the field: after `use next = node.next`, a write to `node.next` doesn't change `next`.
- `use x = call(...)` binds the reference the call returns: `pool.add(...)`, `scores.get(key)` (an optional reference), `people.find(...)`, or a function whose return type says `in`. A `try` before the call or the place doesn't change this.
- Any other expression after `use` is an error: `use x = 5`, `use x = makePerson()`, `use x = if ready { a } else { b }`. Use `let` to copy a value, or `mutable use` and `reuse` to choose between places.
- `let x = reference` copies the designated value.
- `use { name, age } = person` declares one alias per field.
- A `mutable use` can be declared on an immutable collection: it designates read-only places, and can be moved with `reuse`. `reuse x = ...` takes the same forms as `use`, and must give the declared type, with the same owner: `none` if the type is optional.
- `use y = x`, where `x` is a `mutable use`, designates the place `x` designates at that moment: a later `reuse x` doesn't move `y`.
- References to the elements of a `Set` or `GrowingSet` and to the keys of a `Map` or `GrowingMap` are read-only, including loop variables over a `mutable` collection: changing them would change their place in the table. These element types are invariant under [structural views](/advanced/structural-typing): a `Set<Hero>` is not a `Set<Human>`.
- A local collection of references (`Array<Person in people>`) is not allowed in this version: keep indices, or declare a relation `Pool`. Besides `use` aliases, parameters and loop variables, an iterator (`values()`) is the only local value that holds references.

## Iteration

While a `for` loop iterates over a collection place `L`, its body can't:

- mutate `L`, or a place above `L`: `mutates P` where `L` may designate `P` or a place below `P`;
- mutate `L.length` or `L.size`;
- invalidate a place that may designate an element of `L`.

Writing the elements, or places inside them, is allowed: `player.score += 1`, `scores[index] = 0`. Implied effects count (see [relations](#relations)). The same rules hold while an iterator into `L` is used.

## Relations

Writing or deleting linked objects changes other objects too. These **implied effects** are effects of the operation, like its declared ones: they count for the [iteration](#iteration) rule, for "calling a mutating function requires a `mutable` value", for [arguments that alias](#arguments-that-alias), and for [task conflicts](/advanced/concurrency#tasks-and-references). A signature declares the direct effects only; the implied ones follow from them.

In the table, a relation has a direct side `f` on the elements of a holder collection `H`, pointing into a target collection `C`, and a reverse side `r` on the elements of `C`. `H` and `C` are paths with the same owner as the operated collection (`town.people`). An owner field `g` is a field of the owner `o` of `C`.

| Operation | Declared effect | Implied effects |
| --- | --- | --- |
| `x.f = t`, one-to-many (`r: Pool<C->f>`) | `mutates x.f` | `mutates C[..].r`: the old and the new target |
| `x.f = t`, one-to-one (`r: (C->f)?`) | `mutates x.f` | `mutates C[..].r`, `mutates H[..].f`: the previous holder of `t` becomes `none` |
| `x.f = t`, symmetric | `mutates x.f` | `mutates C[..].f`: `t`, and the previous partners of `x` and `t` |
| `x.f.add(t)`, `x.f.delete(t)`, `x.f.clear()`, many-to-many (`r: Pool<in C->f>`) | `mutates x.f` | `mutates C[..].r` |
| `o.g = t`, owner field | `mutates o.g` | none |
| `c.add(value)` | `mutates c.size` | when the value sets direct sides: the implied effects of writing them |
| `c.delete(e)`, `c.take(e)` | `invalidates e` | `mutates c.size`; for each relation whose target collection is `c`: `mutates H[..].f` (the direct sides that pointed at `e` become `none`, or lose `e`); for each relation held by the elements of `c`: `mutates C[..].r` (`e` leaves the reverse sides of its targets); `mutates o.g` for each owner field into `c` |
| `c.deleteWhere(...)`, `c.clear()`, `c = ...`, any `mutates c` | `mutates c` | the implied effects of `delete`, for every element |
| destroying the owner of both collections | | none: both sides are destroyed together |

```fa
for friend in ana.friends {
	town.people.delete(friend) -- error: deleting `friend` removes it from `ana.friends` (implied `mutates town.people[..].friends`), which is being iterated
}
```

Deleting a user of `blog.users` writes `blog.posts[..].author`: `blog` must be `mutable`, which it is to delete from `blog.users`. With module-level collections, `posts` must be declared `mutable` too. A function that receives the collection alone (`users: Pool<Blog.User>`) and deletes from it declares its effect on `users` only: the implied effect on the posts of the same owner is added at each call, where the owner of the argument is known.

### Writing relation fields

- A direct side is written only through a reference typed `T in H`, where `H` is the holder collection of the relation (the collection its reverse side names), and its new target must be a reference typed `U in C`, with the same owner as `H`. Linking the elements of two different owners (two `Town`s) is therefore an error, unless the relation's collections are module-level values.
- In a copy, or in an element of another collection, relation fields are `none` (or empty) and read-only: writing them is an error.
- A reverse side is never written.

## Arguments that alias

A place that a call mutates or invalidates must not overlap any other argument of the same call, any `requires` value the callee uses, any module-level value the callee reads or mutates, or any reference captured by a callback argument. Two places overlap when one may designate the other or a place below it. "May overlap" is an error. The places a call mutates are the paths of its declared effects, with the argument paths substituted, and their implied effects.

```fa
mutable numbers = [1, 2, 3]

numbers.push(numbers[0]) -- error: `push` mutates `numbers`, and its argument `numbers[0]` is inside `numbers`

let first = numbers[0]
numbers.push(first) -- valid: `first` is a copy
```

Two overlaps are allowed, because the callee's signature declares them, so the callee checks its body knowing them:

- the argument of a parameter typed `T in p` is inside the argument of `p` (`people.delete(ana)`, `list.remove(node)`);
- an implied effect on `C[..].f` overlaps the argument of a parameter typed `T in C`: [`befriend(ana, bao)`](/advanced/relations#effects), with two parameters typed `Person in people`, writes the reverse sides of `people[..]`.

The declared effects are still checked: `befriend` declares `mutates person.bestFriend`, so `befriend(ana, ana)`, or `befriend(ana, someone)` where `someone` may be `ana`, is an error.

`use` aliases can alias each other freely: only the arguments of a call are constrained. Inside the callee, two parameters are distinct roots, except for the declared overlaps above.

## Tasks

`start f(...)` applies the effects of `f` to the caller's references at the `start`: the references they invalidate are invalidated there. Until the task ends (it is awaited, cancelled, or its scope ends), the places it reads can't be modified and the places it modifies can't be used, as [tasks and references](/advanced/concurrency#tasks-and-references) explains.

## Worked examples

### A reference kept across a loop that pushes

```fa
mutable people: Array<Person> = [Person { name = "Ana" }, Person { name = "Bao" }]

use first = people[0]

for index in 0..10 {
	console.log(first.name) -- error: `first` may have been invalidated by `people.push` in a previous iteration
	if index == 3 {
		people.push(Person { name = "Cleo" })
		use first = people[0] -- a new `first`, which ends with the `if` block
	}
}
```

The fixpoint joins the end of each iteration into the start of the next one, so `first` is maybe invalidated at the start of the body. The declaration inside the `if` doesn't repair the outer `first`. With `mutable use`, it does:

```fa
mutable use first = people[0]

for index in 0..10 {
	console.log(first.name) -- valid
	if index == 3 {
		people.push(Person { name = "Cleo" })
		reuse first = people[0] -- `first` is valid again at the end of both branches
	}
}
```

### Two indices, then `delete`

```fa
function example = (i: Integer<min = 1, max = 2>, j: Integer<min = 0, max = 3>) => {
	mutable people: Array<Person> = [Person { name = "Ana" }, Person { name = "Bao" }, Person { name = "Cleo" }, Person { name = "Dan" }]

	use a = people[0]
	use b = people[i]
	use c = people[j]

	if j < i {
		people.delete(i) -- `invalidates people[i..]`
		console.log(a.name) -- valid: 0 is below every value of `i`
		console.log(b.name) -- error: `b` is `people[i]`
		console.log(c.name) -- error: `j` (0 to 1 here) may reach `i` (1 to 2): the relation `j < i` is not kept
	}
}
```

### A reference found before an `add`

```fa
mutable pool: Pool<Person> = []
pool.add(Person { name = "Ana" })

use found = pool.find((person) => person.name == "Ana") -- `(Person in pool)?`
use added = pool.add(Person { name = "Bao" })          -- fresh

if found is not none {
	pool.delete(found)
	console.log(added.name) -- valid: `added` is distinct from every reference that existed before its `add`
}
```

A reference found after the `add` may designate the added element: deleting it invalidates `added`.

### A function that returns a reference into its parameter

```fa
function pick = (persons: Array<Person>, index: Integer): Person in persons => persons[index]
function pickCopy = (persons: Array<Person>, index: Integer) => persons[index] -- inferred return type: a copy

mutable people: Array<Person> = [Person { name = "Ana" }, Person { name = "Bao" }, Person { name = "Cleo" }]

use chosen = try pick(people, 2) -- `Person in people`, at an unknown index
people.delete(1) -- `invalidates people[1..]`
console.log(chosen.name) -- error: the index of `chosen` is unknown: identities don't cross a user function
```

### A callback that reads a reference into the mutated collection

```fa
mutable people: Pool<Person> = []
use ana = people.add(Person { name = "Ana", age = 31 })

people.deleteWhere((person) => person.age < ana.age) -- error: the callback captures `ana`, a reference into `people`, which `deleteWhere` mutates

let limit = ana.age
people.deleteWhere((person) => person.age < limit) -- valid: `limit` is a copy
```

### Deleting while iterating a relation

`for friend in ana.friends { town.people.delete(friend) }` is an error, shown in [relations](#relations). Mark the elements, then delete them:

```fa
for friend in ana.friends {
	friend.leaving = true -- valid: writes a field of an element of `town.people`, not `ana.friends`
}

town.people.deleteWhere((person) => person.leaving)
```

### Unlinking a node of a linked list

In `LinkedList.remove` (see [relations](/advanced/relations#example-a-linked-list)), `use next = node.next` designates the next node. The one-to-one write `previous.next = next` sets `node.next` to `none`, but `next` still designates the next node, so `if next is none` tests the end of the list, as intended.

### Invalidated in one branch only

```fa
mutable people: Pool<Person> = []
use ana = people.add(Person { name = "Ana" })

if leaving {
	people.delete(ana)
}

console.log(ana.name) -- error: `ana` may have been invalidated by `people.delete`
use ana = people.find((person) => person.name == "Ana") -- valid: `ana` may be invalidated, so it can be declared again
```

:::implementation
## Implementation details

The checker is shared by every target. It runs on the typed tree, after name resolution and type checking, once per function body, and needs no control-flow graph: the code is structured.

- **State.** For each live reference: valid or invalidated (with the operation, for the message), the owner path, the path below it, and an identity per index or key. For a `mutable use`, a small set of such places. Identities are a tagged value: exact literal, interval, `(symbol, offset)`, fresh token (a counter per function), unknown.
- **Effects.** Each call substitutes the arguments into the callee's effect paths and adds the implied effects of relations, which are computed once per relation declaration. The paths are interned; the overlap test walks two paths step by step with the rules of [may designate](#may-designate).
- **Joins.** A join is a pointwise union: invalidated wins, `mutable use` sets are united, identities are widened to their convex hull. Loops iterate until the state doesn't change; intervals are widened as for [range analysis](/values/numbers#range-analysis), so a loop needs at most a few passes.
- **Interfaces.** The inferred freshness of a returned reference and the escape bit of each function-typed parameter are stored with the function's inferred effects, in the same cache entry.
- **Messages.** An error names the reference, the operation that invalidated it and its line, and proposes the fix: `declare it again with use someone = persons[...]`, or `reuse` for a `mutable use`, or a copy with `let` for an overlapping argument.
:::
