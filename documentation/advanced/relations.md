# Relations

A **relation** links two fields that mirror each other, like a person's best friend and the people whose best friend they are. In Fa, a relation is declared in the type, and the compiler keeps both sides consistent: when you write one side, when you delete an object. Relations replace the weak pointers of other languages: a stored reference never dangles.

## The problem

In TypeScript, you would write:

```ts
type Person = {
	name: string
	bestFriend?: Person
	bestFriendOf: Person[]
}
```

A human reading those names sees the relation immediately: if `personA.bestFriend` is `personB`, then `personA` should be in `personB.bestFriendOf`. But the compiler sees two unrelated fields, and keeping them consistent is up to you. Changing a best friend means updating three places by hand:

```ts
const setBestFriend = (person: Person, newBestFriend: Person) => {
	// 1. detach from the previous best friend
	if (person.bestFriend) {
		person.bestFriend.bestFriendOf = person.bestFriend.bestFriendOf.filter(
			(friend) => friend !== person,
		)
	}

	// 2. write the new best friend
	person.bestFriend = newBestFriend

	// 3. attach to the new best friend
	newBestFriend.bestFriendOf.push(person)
}
```

Forget one step, or delete a person without cleaning up the fields that point at them, and the data is silently wrong. In Fa, you declare that the two fields are one relation, and the compiler writes these steps for you.

## Copies and references

A field typed `Person` or `Person?` owns its stored person. Initialization follows the [copy and move categories](/advanced/memory-management#copies-moves-unique-values-and-shared-handles); a relation instead links an existing element.

```fa
type Person = {
	name: String
	mentor: Person? -- a copy of another person, owned by this one
}
```

To *point at* a person instead of holding one, the field names the collection the person lives in: `Person in people`, as for [local references](/advanced/owned-references). There is no reference without a named collection.

The collection must be visible where the type is declared. It is either a field of the containing type:

```fa
type Town = {
	people: Pool<Person> = []

	type Person = {
		name: String
		bestFriend: (Person in people)?
		bestFriendOf: Pool<people->bestFriend>
	}
}
```

or a [module-level value](/values/declaring-values#module-level-values), here exported by its own file:

```fa
-- in "people.fa"
export mutable: Pool<Person> = []
```

```fa
-- in "Person.fa"
export type = {
	name: String
	bestFriend: (Person in people)?
	bestFriendOf: Pool<people->bestFriend>
}
```

In the first form, each town has its own people, and a person's best friend is always someone from the same town. In the second form, there is one collection for the whole program, and every file can add people to it. The type is always written in full, `Person in people`, and the compiler checks that `Person` is the element type of `people`.

Both sides of a relation are declared in the same file, with the element types; the collection can live in another file. Its declaration is checked against the relation's element type and stable-location requirement. Functions writing a collection from another file declare that write with `mutates`, just like other exported mutable values.

## Declaring a relation

A relation has two sides:

- the **direct side** is a reference field: `bestFriend: (Person in people)?`;
- the **reverse side** names the collection, then the field it mirrors: `bestFriendOf: Pool<people->bestFriend>`. It reads as *the elements of `people` whose `bestFriend` points at me*.

The reverse side is always declared. Without the explicit `link` modifier, a reference field without a reverse side is a compile error (the only exception is an [owner field](#example-a-linked-list), stored on the object that owns the collection).

```fa
type Person = {
	name: String
	bestFriend: (Person in people)? -- error: `bestFriend` has no reverse side: declare one, for example `bestFriendOf: Pool<people->bestFriend>`
}
```

The reverse side is what makes the relation cheap and safe: when an object changes or disappears, the compiler finds every object linked to it by reading its fields, without scanning the collection.

A direct side is always optional or a `Pool`, so that it has something to become when its target is deleted: `bestFriend: Person in people` is an error.

### The four kinds of relations

```fa
type Town = {
	people: Pool<Person> = []

	type Person = {
		name: String

		-- one-to-many: many people can have the same best friend
		bestFriend: (Person in people)?
		bestFriendOf: Pool<people->bestFriend>

		-- one-to-one: a person has at most one mentor, and mentors at most one person
		mentor: (Person in people)?
		mentee: (people->mentor)?

		-- many-to-many: a person has many friends, and is the friend of many people
		friends: Pool<Person in people>
		friendOf: Pool<in people->friends>

		-- symmetric: if Ana is Bao's spouse, Bao is Ana's spouse
		spouse: (people->spouse)?
	}
}
```

| Kind | Direct side | Reverse side |
| --- | --- | --- |
| one-to-many | `bestFriend: (Person in people)?` | `bestFriendOf: Pool<people->bestFriend>` |
| one-to-one | `mentor: (Person in people)?` | `mentee: (people->mentor)?` |
| many-to-many | `friends: Pool<Person in people>` | `friendOf: Pool<in people->friends>` |
| symmetric | `spouse: (people->spouse)?` | itself |

In a many-to-many relation, the direct side is a pool of references, and the reverse side points *into* those pools with `in`: *the elements of `people` whose `friends` contain me*. A symmetric relation is a field that is its own reverse side.

These are the only shapes: a direct side is `(T in c)?` or `Pool<T in c>`, and it has exactly one reverse side, of the shape given by the table (a second reverse side for the same direct side is an error). Ordered relations, like a list of children in a given order, are built with a linked list (see [the example below](#example-a-linked-list)).

Neither side is initialized: an optional side starts as `none`, a pool side starts empty, and the reverse side only changes through its direct side. An object can be linked to itself: after `ana.bestFriend = ana`, Ana is in her own `bestFriendOf`.

### Between two types

The two sides can belong to different types, each living in its own collection:

```fa
type Blog = {
	users: Pool<User> = []
	posts: Pool<Post> = []

	type User = {
		name: String
		articles: Pool<posts->author> -- the posts whose `author` is this user
	}

	type Post = {
		title: String
		author: (User in users)?
	}
}
```

```fa
mutable blog: Blog = {}

use ana = blog.users.add({ name = "Ana" })
use post = blog.posts.add({ title = "Hello" })

post.author = ana

console.log(ana.articles.size) -- 1
```

## Writing a relation

The examples below use a town:

```fa
mutable town: Town = {}

use ana = town.people.add({ name = "Ana" })
use bao = town.people.add({ name = "Bao" })
use cleo = town.people.add({ name = "Cleo" })
```

You only ever write the direct side. Every write is compiled into an update of both sides, in constant time:

```fa
ana.bestFriend = bao
ana.bestFriend = cleo

-- the second write is compiled into:
-- if ana.bestFriend is not none {
-- 	ana.bestFriend.bestFriendOf.delete(ana) -- 1. detach from the previous best friend
-- }
-- ana.bestFriend = cleo                     -- 2. write
-- cleo.bestFriendOf.add(ana)                -- 3. attach to the new best friend

console.log(bao.bestFriendOf.size)  -- 0
console.log(cleo.bestFriendOf.size) -- 1
```

These are the three steps of the TypeScript version, generated from the declaration. You can't write them yourself: the reverse side is read-only.

```fa
cleo.bestFriendOf.add(ana) -- error: `bestFriendOf` is the reverse side of `bestFriend` and is read-only: write `ana.bestFriend = cleo`
```

A many-to-many relation is written through the pool of the direct side:

```fa
ana.friends.add(bao)

console.log(bao.friendOf.size) -- 1

ana.friends.delete(bao)

console.log(bao.friendOf.size) -- 0
```

The pools of a relation hold links to their targets, not values of their own:

- `add(target)` links a target, and returns `false` when it was already linked: a target is in a pool at most once;
- `delete(target)` unlinks a target, and returns `false` when it wasn't linked; `clear()` unlinks every target;
- `size`, `includes(target)`, `find`, and iteration work as on any pool, in no guaranteed order, and give the targets: iterating `ana.friends` yields references typed `Person in people`;
- a reverse side has the same reading methods, and is read-only.

Two kinds of relations have one more rule.

**One-to-one**: the target can only have one reverse link. If it already had one, the previous holder's direct side becomes `none`:

```fa
ana.mentor = bao
cleo.mentor = bao -- `bao.mentee` is now Cleo, and `ana.mentor` becomes `none`
```

**Symmetric**: writing one side writes the other, and detaches the previous spouses of both:

```fa
ana.spouse = bao  -- `bao.spouse` is now Ana
cleo.spouse = bao -- `bao.spouse` is now Cleo, and `ana.spouse` becomes `none`
```

### Which objects can be linked

Only the elements of the collections named by a relation are linked:

- A direct side is written through a reference into the collection that the reverse side names, here `town.people`, and its new target is a reference into the target collection of the same owner: `ana.bestFriend = bao` needs `ana` and `bao` to be references into the same `town.people`. Linking the people of two different towns is an error; with a module-level collection, there is only one.
- A copy of an element, or an element stored in another collection, has its relation fields `none` (or empty), and they are read-only: writing them is an error.
- An object literal given to `add` may set the direct sides, which are linked once the element is stored: `town.people.add({ name = "Dan", bestFriend = ana })`. A many-to-many side takes an array literal of references, linked in order: `friends = [ana, bao]`. A reverse side can't be set.
- Assigning a whole value to a linked element (`ana = Person { name = "Ana" }`) replaces its data fields and keeps its links: relation fields are not data, as for [equality](/operations/logical#equality). The assigned literal can't set a relation field: write the link separately, with `ana.bestFriend = bao`.
- Printing an object, or interpolating it in a string, omits its relation fields too, so that printing a linked object never walks the whole graph.

### Effects

A function that writes a direct side declares it with `mutates`, like any other field. The reverse sides it changes are implied: they can only change through the direct side. The compiler counts these implied effects like declared ones, and so do deletions, which write every direct side that pointed at the deleted object: [reference checking](/advanced/reference-checking#relations) lists them.

```fa
mutable people: Pool<Person> = []

type Person = {
	name: String
	bestFriend: (Person in people)?
	bestFriendOf: Pool<people->bestFriend>
}

function befriend = (person: Person in people, friend: Person in people) mutates person.bestFriend => {
	person.bestFriend = friend -- also updates `bestFriendOf` of the previous best friend and of `friend`
}
```

Implied effects also drive reactivity. A relation side of an element of a reactive collection is observed like a reactive field: a `derived` value or `watch` reading `bao.bestFriendOf` reruns when a link adds Bao or removes Bao, including deletion, and only when that side changes. Relation sides are not data, so this does not change a dependency on `people.size`. A watch that writes a direct side and reads its reverse side writes what it reads and is rejected.

## Deleting an object

Deleting an object cleans up every relation it was part of, at once, through the reverse sides:

- every direct side that pointed at it is set to `none`, or loses it if it is a pool;
- it is removed from the reverse sides of its own targets.

```fa
ana.bestFriend = bao
cleo.friends.add(bao)

town.people.delete(bao) -- `ana.bestFriend` becomes `none`, and `cleo.friends` no longer contains Bao
```

`delete` also invalidates the local references that may designate the deleted object, which must be declared again before being used (see [owned references](/advanced/owned-references#invalidation)). The other local references stay valid, and see the cleanup:

```fa
console.log(bao.name) -- error: `bao` was invalidated by `town.people.delete`
console.log(ana.bestFriend) -- none: `ana` is another element, it is still valid
```

Because the cleanup writes other objects, deleting can't happen while one of the changed relations is iterated:

```fa
for friend in ana.friends {
	town.people.delete(friend) -- error: deleting `friend` removes it from `ana.friends`, which is being iterated
}
```

The cleanup costs O(k) for an object linked to k others, and is paid at deletion only: reading a relation never checks anything. There are no generation counters and no hidden indexes: no stale reference ever exists.

## Copying linked objects

Copying an object that owns linked collections copies the links too. In the copy, they point at the copied objects, never at the original ones:

```fa
ana.bestFriend = bao

let copiedTown = copy town -- copies the people and their relations
-- in `copiedTown`, the copy of Ana has the copy of Bao as best friend
```

Copying a single linked object out of its collection copies its data, not its links. A copy is not an element of the collection, so nothing can point at it: its direct sides are `none` (or empty, for a pool), and its reverse sides are empty.

```fa
let anaCopy = ana -- a copy of Ana, outside of `town.people`
console.log(anaCopy.bestFriend) -- none
console.log(bao.bestFriendOf.size) -- 1: Ana only, the copy is not linked
```

Adding this copy to another collection, or taking an object out of its collection with `take`, gives the same result: an object without links.

The rule is the same for every copy: links are kept only between collections copied together, as fields of one copied owner. Copying a collection on its own, like `let snapshot = copy town.people` or a copy of a module-level `people`, gives elements without links, since the copy is not the collection the relation names.

## Where linked objects live

Both sides of a relation are elements of a collection with **stable addresses**: a [`Pool`](/collections/pools), a [`Columnar`](/collections/columnars), or their growing variants [`GrowingPool` and `GrowingColumnar`](/collections/growing-collections). Their elements live in slots that never move, so a stored reference stays valid until its target is deleted, and deletion cleans it up.

An `Array` or a `Set` can't hold linked objects, because they move their elements:

- an `Array` reallocates when it grows, and shifts its elements on `insert` and `delete`;
- a `Set` rehashes when an element is added or deleted.

A [`Map`](/collections/maps) doesn't move its values, but a relation links elements of a collection, not values found by key: store linked objects in a `Pool`.

A [local reference](/advanced/owned-references) into these collections is fine: the compiler sees the call that invalidates it, and makes you declare it again. A stored reference lives in data, and can be read anywhere, long after the collection changed: there is no place where it could be declared again. So its target must never move.

```fa
mutable people: Array<Person> = []

type Person = {
	name: String
	bestFriend: (Person in people)? -- error: `people` is an `Array`, whose elements move: a relation needs a `Pool`, a `Columnar`, a `GrowingPool` or a `GrowingColumnar`
	bestFriendOf: Pool<people->bestFriend>
}
```

:::tip
Use a `Pool` for the objects that point at each other: order does not matter, and adding elements never moves the others.
:::

## Relations and local references

A module-level value is neither a field nor a local reference, so it can't hold a reference: `mutable first: (Person in people)? = none` at the top level of a module is an error. Keep the reference in a field of an owner type, or keep an index.

A [local reference](/advanced/owned-references) (`use`, a parameter, a return value) is short-lived: the compiler tracks it statically through the `mutates` and `invalidates` effects of the calls, and after a call that invalidates it, you must declare it again before using it.

A relation is stored: it lives as long as the object that holds it, and the compiler keeps it up to date at runtime, on every write and every deletion.

Use a local reference for short-lived work on a stored value. Use a relation when two objects need to know about each other over time.

`use` on a relation field gives a local reference to its **target**, not to the field: after `use friend = ana.bestFriend`, `friend` designates Bao, and writing `ana.bestFriend = cleo` later doesn't change what `friend` designates. A local reference never points into a relation field, which is why a relation field is a flat value (see [mutations](/advanced/owned-references#mutations)).

## Relations and recursion

A type can't contain itself directly, because its size would be infinite. Recursion through a union, including an optional (`next: Node?`), is allowed: the compiler stores the child on the heap, owned by its parent. Recursive values are structures: the child transfers at the owner's last use, or needs explicit `copy` to keep both owners (see [recursive types](/advanced/memory-management#recursive-types)). There is no `Box` type.

Owned children make trees: a node can't have two parents, and a child can't point back to its parent. As soon as objects need to point at each other (a doubly linked list, a graph, a parent and its children), store them in a stable-address collection and link them with relations.

## Example: a linked list

A doubly linked list is exactly the structure that goes wrong with raw pointers: nothing guarantees that `previous` and `next` stay consistent. In Fa, `previous` is the reverse side of `next`, and every node lives in a `Pool` owned by the list:

```fa
type LinkedList<Data> = {
	-- every node lives here (private field)
	#nodes: Pool<Node> = []

	type Node = {
		data: Data

		-- one-to-one: a node is the `next` of at most one node
		next: (Node in #nodes)?
		previous: (#nodes->next)?
	}

	-- owner fields: the ends of the list
	firstChild: (Node in #nodes)?
	lastChild: (Node in #nodes)?

	-- append a node: adding to a pool never moves the other nodes
	function add = (self, data: Data): Node in self.#nodes mutates self.#nodes.size, self.#nodes[..].next, self.firstChild, self.lastChild => {
		use newNode = self.#nodes.add(Node { data = data })
		use last = self.lastChild -- the last node, not the field: see below

		if last is not none {
			last.next = newNode -- `newNode.previous` becomes `last`
		} else {
			self.firstChild = newNode
		}

		self.lastChild = newNode
		newNode
	}

	-- unlink and destroy a node
	function remove = (self, node: Node in self.#nodes) mutates self.#nodes[..].next, self.firstChild, self.lastChild invalidates node => {
		use previous = node.previous
		use next = node.next

		if previous is not none {
			previous.next = next -- one-to-one: `node.next` becomes `none`, `next.previous` becomes `previous`
		} else {
			self.firstChild = next
		}

		if next is none {
			self.lastChild = previous
		}

		self.#nodes.delete(node) -- cleans up the links that remain
	}
}
```

`firstChild` and `lastChild` are not stored in a node, but in the list that owns `#nodes`. They are **owner fields**: fields of the type that owns a collection, pointing into that collection. An owner field has exactly one holder, the owner itself, so it has no reverse side to declare: when a node is deleted from `#nodes`, the owner fields that pointed at it become `none`. This is the only relation field without a reverse side; explicit `link` fields follow the separate containment contract below.

- An owner field is optional. Several owner fields may point at the same element: here, `firstChild` and `lastChild` both point at the only node of a one-node list.
- A collection that owner fields point into is only modified through its owner: an operation that adds, deletes or mutates its elements names it from the owner (`self.#nodes.delete(node)`, `list.#nodes`), never through a parameter or an alias that designates the collection alone: such a collection can't be aliased with `use` or `mutable use`, nor passed to a parameter that a function mutates or invalidates, while the built-in operations called on the full path are allowed. This is how a deletion finds the owner fields to clear.

`use` on a relation field or an owner field designates the target node, not the field. In `remove`, the write `previous.next = next` sets `node.next` to `none`, but `next` still designates the node after `node`: the test `next is none` is true only when `node` was the last node.

The effects in the signatures tell the caller exactly what happens to its references:

- `add` declares `mutates self.#nodes.size`, but not `mutates self`: adding to a `Pool` never moves the existing nodes, so every reference to a node stays valid. The other effects are flat: a relation field (`next`, and `previous` through it) and the owner fields. The returned node comes from `self.#nodes.add`, so it is fresh: the compiler knows it is distinct from every node the caller already holds.
- `remove` declares `invalidates node`, like `Pool.delete`: the references that may designate the removed node must be declared again, and the others stay valid. It also declares the fields it writes.

```fa
mutable list: LinkedList<String> = {}

use first = list.add("first")
use second = list.add("second")
list.add("third")

console.log(first.next?.data) -- "second"

list.remove(second)

console.log(second.data) -- error: `second` was invalidated by `list.remove`: declare it again with `use second = ...`
console.log(first.next?.data) -- "third": `first` is still valid, and was relinked
console.log(first.next?.previous?.data) -- "first"
```

Notice what you did *not* have to write: `previous` is never assigned. Declaring it as `#nodes->next` is enough for the compiler to keep it correct, including when `remove` relinks the list.

Storing all the nodes in one pool has two further benefits: destroying the list frees every node in one step, and iterating over `#nodes` walks through every node contiguously, which is much faster than following `next` references through scattered memory.

## Links without a reverse side

A **link** is a non-owning stored reference whose validity rests on a growing target collection and containment inside a complete declared owner. Write `link` before the field name, including in a union member record. It is contextual: `link` remains an ordinary identifier elsewhere, and a field can itself be named `link`.

```fa
-- Acceptance: stored links (implemented in subsequent link slices).
type Model = {
	nodes: GrowingPool<Node> = []
	labels: GrowingPool<Label> = []
	link root: (Node in nodes)? = none
	type Label = { text: String }
	type Node = {
		value: Int32
		link next: (Node in nodes)? = none
		link label: (Label in labels)? = none
	}
}
mutable first: Model = {}
mutable second: Model = {}
use label = first.labels.add({ text = "one" })
use node = first.nodes.add({ value = 1, label = label })
first.root = node
-- second.root = node -- error: expected Node in second.nodes, got Node in first.nodes
```

Allowed shapes are `T in path`, `(T in path)?`, `Array<T in path>` and `InlineArray<T in path, capacity>`. T is the target's complete element type. Required links must be supplied; optional/list links start empty only with an explicit `= none`/`= []`. Cycles, self-links, repeated targets and sharing are allowed. Seed cycles through initialized optional/list fields or a link-free union member, then patch or replace the element. No uninitialized element or constructor-self reference can seed an always-required cycle.

A link is initialized like an ordinary field: supply a required link at construction, or write `= none` to give an optional link a default. A relation side is maintained by its collection and starts empty, so it does not take an explicit `= none` initializer.

| Promise or cost | Relation | Explicit link |
| --- | --- | --- |
| Target | Stable relation collection, including deletable Pool | Whole element of GrowingPool/GrowingArray |
| Validity | Maintained inverse finds and clears incoming sides on deletion | All surviving holders stay within the target's complete owner |
| Write | Updates direct and old/new reverse sides; may allocate reverse state | Stores the target, no reverse state or implied reverse writes |
| Read | No liveness check | No liveness check |
| Deletion | Cleanup O(k) for k memberships | Individual deletion absent |
| Ordered children | A linked list/relation structure | Array or InlineArray, preserving order and duplicates |
| Detached element copy | Erases relation sides | Refused; borrow or project ordinary data |
| Whole-element assignment | Keeps relation sides | Replaces outgoing link fields; incoming links still designate the place |
| Complete-owner copy | Remaps jointly copied relation sides | Preserves sharing/cycles within the independent copy |

Choose relations for objects that disappear individually, reverse queries or automatic partner maintenance. Choose links for graphs built and discarded together, ordered adjacency lists and low storage/write costs. A link does not promise a tree or acyclicity. A bare reference field keeps relation semantics even in GrowingPool, and absence of a reverse side remains an error. Ordinary fields cannot hide a reference in an optional, tuple, collection, alias or generic argument (`REF010`); local reference collections remain forbidden.

These operations would leave a link without its target collection and are rejected:

```fa
-- Acceptance: link containment is implemented in subsequent link slices.
use { Json } = std.json
type Graph = {
	nodes: GrowingPool<Node, UInt16> = []
	link root: (Node in nodes)? = none
	type Node = { value: Integer, link next: (Node in nodes)? = none }
}
mutable graph: Graph = {}
use a = graph.nodes.add({ value = 1 })
use b = graph.nodes.add({ value = 2, next = a })
graph.root = b
let detached = copy b -- error: this node has links into graph.nodes; borrow it or copy graph
graph.nodes = [] -- error: graph.root may outlive graph.nodes; replace the complete graph
let encoded = Json.stringify(graph) -- error: project the data and node identifiers into a separate type
```

A deletable pool needs a relation so that deleting a target can clear its incoming sides:

```fa
-- Acceptance: link target validation is implemented in subsequent link slices.
type Wrong = {
	nodes: Pool<Node> = []
	type Node = {
		link next: (Node in nodes)? = none -- error: nodes can remove elements; use GrowingPool or a relation
	}
}
```

Reading a scalar link with `use` binds its target; a later write does not reseat that alias. Link-list iteration and indexing bind target references, never addresses of encoded fields. Following a link restarts an effect path at its target collection. Printing, interpolation, equality and hashing omit link fields, as they omit relations: data equality is not graph-isomorphism equality. A reactive read of the list observes list writes; a read of target data observes those data writes, without a reverse-side notification.

### Complete tree

The complete owner below builds left, then right, then parent, and computes eight pre-order sums. It needs no reverse sides. Links are inert on destruction; this flat payload requires only page cleanup.

```fa
-- Acceptance: complete linked tree.
function xorshift = (x: UInt64): UInt64 => {
	mutable v = x
	v = v.bitXor(v.shiftLeft(13))
	v = v.bitXor(v.shiftRight(7))
	v.bitXor(v.shiftLeft(17))
}
type Tree = {
	nodes: GrowingPool<Node> = []
	type Node = {
		value: Int32
		link left: (Node in nodes)? = none
		link right: (Node in nodes)? = none
	}
	function build = (self, depth: Integer<min = 0>, seed: UInt64): Node in self.nodes
		mutates self.nodes.size => {
		if depth == 0 {
			return self.nodes.add({ value = Int32(seed %% 1000) })
		}
		use left = self.build(depth - 1, xorshift(seed))
		use right = self.build(depth - 1, xorshift(seed.wrappingAdd(1)))
		self.nodes.add({ value = Int32(seed %% 1000), left = left, right = right })
	}
	function sum = (self, node: Node in self.nodes, pass: Integer): Integer => {
		mutable total = Integer(node.value).bitXor(pass)
		use left = node.left
		if left is not none { total += self.sum(left, pass) }
		use right = node.right
		if right is not none { total += self.sum(right, pass) }
		total
	}
}
mutable tree: Tree = {}
use root = tree.build(21, 12345)
mutable total = 0
for pass in 0..8 { total += tree.sum(root, pass) }
console.log(tree.nodes.size)
console.log(total)
```

### Complete scene

This scene allocates each parent first, animates by insertion-order scans, recursively updates world transforms, then checksums by another scan. Children use [InlineArray](/collections/inline-arrays); `try` remains on pushes whose remaining capacity is not proven. The Transform snapshot before recursion is essential: a child may designate its parent, so borrowing `node.world` across a call mutating every world would overlap.

```fa
-- Acceptance: complete linked scene.
function xorshift = (x: UInt64): UInt64 => {
	mutable v = x
	v = v.bitXor(v.shiftLeft(13))
	v = v.bitXor(v.shiftRight(7))
	v.bitXor(v.shiftLeft(17))
}
function randomBelow = (x: UInt64, bound: Integer<min = 1>): Integer =>
	Integer(x %% UInt64(bound))

type Vector = { x: Float32, y: Float32, z: Float32 }
type Transform = { matrix: CArray<Float32, 9>, translation: Vector }
let identity = Transform {
	matrix = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0]
	translation = { x = 0.0, y = 0.0, z = 0.0 }
}
function rotation = (seed: UInt64): CArray<Float32, 9> => {
	let axis = randomBelow(seed, 3)
	let quarter = randomBelow(seed.shiftRight(8), 4)
	mutable c: Float32 = 1.0
	mutable s: Float32 = 0.0
	if quarter == 1 {
		c = 0.0
		s = 1.0
	} else if quarter == 2 {
		c = -1.0
		s = 0.0
	} else if quarter == 3 {
		c = 0.0
		s = -1.0
	}
	if axis == 0 {
		return [1.0, 0.0, 0.0, 0.0, c, 0.0 - s, 0.0, s, c]
	}
	if axis == 1 {
		return [c, 0.0, s, 0.0, 1.0, 0.0, 0.0 - s, 0.0, c]
	}
	[c, 0.0 - s, 0.0, s, c, 0.0, 0.0, 0.0, 1.0]
}
function compose = (parent: Transform, local: Transform): Transform throws IndexError => {
	mutable matrix: CArray<Float32, 9> = [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]
	for i in 0..3 {
		for j in 0..3 {
			mutable sum: Float32 = 0.0
			for k in 0..3 {
				sum += (try parent.matrix[i * 3 + k]) * (try local.matrix[k * 3 + j])
			}
			matrix[i * 3 + j] = sum
		}
	}
	let p = parent.matrix
	let t = local.translation
	Transform {
		matrix = matrix
		translation = {
			x = p[0] * t.x + p[1] * t.y + p[2] * t.z + parent.translation.x
			y = p[3] * t.x + p[4] * t.y + p[5] * t.z + parent.translation.y
			z = p[6] * t.x + p[7] * t.y + p[8] * t.z + parent.translation.z
		}
	}
}

type Scene = {
	nodes: GrowingPool<Node> = []
	type Node = {
		id: Int32
		local: Transform
		world: Transform
		link children: InlineArray<Node in nodes, 4> = []
	}
	function build = (self, depth: Integer<min = 0>, seed: UInt64): Node in self.nodes
		throws CapacityError | ArithmeticError
		mutates self.nodes.size, self.nodes[..].children => {
		let state = xorshift(seed)
		use node = self.nodes.add({
			id = try Int32(self.nodes.size)
			local = {
				matrix = rotation(state)
				translation = {
					x = Float32(randomBelow(state.shiftRight(16), 9))
					y = Float32(randomBelow(state.shiftRight(24), 9))
					z = Float32(randomBelow(state.shiftRight(32), 9))
				}
			}
			world = identity
			children = []
		})
		if depth == 0 { return node }
		let count = 1 + randomBelow(state.shiftRight(40), 4)
		for index in 0..count {
			use child = try self.build(depth - 1, xorshift(state.wrappingAdd(UInt64(index))))
			try node.children.push(child)
		}
		node
	}
	function update = (self, node: Node in self.nodes, parent: Transform)
		throws IndexError mutates self.nodes[..].world => {
		node.world = try compose(parent, node.local)
		let world = node.world -- ordinary inline value snapshot; no heap copy
		for child in node.children {
			try self.update(child, world)
		}
	}
}

mutable scene: Scene = {}
use root = try scene.build(12, 12345)
mutable total = 0
for frame in 0..16 {
	let step = Float32(frame %% 5)
	for node in scene.nodes {
		if node.id %% 7 == frame %% 7 {
			node.local.translation.x = step
			node.local.translation.z = 8.0 - step
		}
	}
	try scene.update(root, identity)
	for node in scene.nodes {
		let t = node.world.translation
		total += try Integer.truncate(t.x + t.y + t.z)
	}
}
console.log(scene.nodes.size)
console.log(total)
```

:::implementation
## Implementation details

### Native

- Each relation side has a source created when an observer reads it. Updates notify the sources of the sides they change; native relation helpers identify these sources by their offsets.

### JavaScript

- Runtime relation helpers notify observers through callbacks. An element carries a symbol-keyed description of its sides, used by generic copies, printing, equality and hashing in erased generic code.


### Link storage and observation

Link fields are classified separately from relation sides and carry their exact symbolic collection path, shape and Index. They never call inverse-maintenance helpers. Generic and imported operations retain that classification. Native following resolves the target ordinal through its pages/segments; JavaScript stores the object itself (primitive targets use slot indices), including in erased callbacks. Whole-element assignment preserves target identity but initializes outgoing links from the written value. Reactive sources belong to the modified link field/list or target data field, with no implied reverse source. Copy/print/equality/hash metadata skips links instead of traversing cycles. Unsupported link storage must produce an actionable not-implemented diagnostic until all engines implement its contract.

:::
