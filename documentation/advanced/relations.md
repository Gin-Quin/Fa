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

A field typed `Person` or `Person?` holds a **copy**: the person is stored inside the object that has the field.

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
	people: Bag(Person) = []

	type Person = {
		name: String
		bestFriend: (Person in people)?
		bestFriendOf: Bag(people->bestFriend)
	}
}
```

or a module-level value:

```fa
mutable people: Bag(Person) = []

type Person = {
	name: String
	bestFriend: (Person in people)?
	bestFriendOf: Bag(people->bestFriend)
}
```

In the first form, each town has its own people, and a person's best friend is always someone from the same town. The type is always written in full, `Person in people`, and the compiler checks that `Person` is the element type of `people`.

## Declaring a relation

A relation has two sides:

- the **direct side** is a reference field: `bestFriend: (Person in people)?`;
- the **reverse side** names the collection, then the field it mirrors: `bestFriendOf: Bag(people->bestFriend)`. It reads as *the elements of `people` whose `bestFriend` points at me*.

The reverse side is always declared. There is no one-way reference: a reference field without a reverse side is a compile error (the only exception is an [owner field](#example-a-linked-list), stored on the object that owns the collection).

```fa
type Person = {
	name: String
	bestFriend: (Person in people)? -- error: `bestFriend` has no reverse side: declare one, for example `bestFriendOf: Bag(people->bestFriend)`
}
```

The reverse side is what makes the relation cheap and safe: when an object changes or disappears, the compiler finds every object linked to it by reading its fields, without scanning the collection.

A direct side is always optional or a `Bag`, so that it has something to become when its target is deleted: `bestFriend: Person in people` is an error.

### The four kinds of relations

```fa
type Town = {
	people: Bag(Person) = []

	type Person = {
		name: String

		-- one-to-many: many people can have the same best friend
		bestFriend: (Person in people)?
		bestFriendOf: Bag(people->bestFriend)

		-- one-to-one: a person has at most one mentor, and mentors at most one person
		mentor: (Person in people)?
		mentee: (people->mentor)?

		-- many-to-many: a person has many friends, and is the friend of many people
		friends: Bag(Person in people) = []
		friendOf: Bag(in people->friends)

		-- symmetric: if Ana is Bao's spouse, Bao is Ana's spouse
		spouse: (people->spouse)?
	}
}
```

| Kind | Direct side | Reverse side |
| --- | --- | --- |
| one-to-many | `bestFriend: (Person in people)?` | `bestFriendOf: Bag(people->bestFriend)` |
| one-to-one | `mentor: (Person in people)?` | `mentee: (people->mentor)?` |
| many-to-many | `friends: Bag(Person in people)` | `friendOf: Bag(in people->friends)` |
| symmetric | `spouse: (people->spouse)?` | itself |

In a many-to-many relation, the direct side is a bag of references, and the reverse side points *into* those bags with `in`: *the elements of `people` whose `friends` contain me*. A symmetric relation is a field that is its own reverse side.

A reverse side is never initialized: it starts empty, and only changes through its direct side.

### Between two types

The two sides can belong to different types, each living in its own collection:

```fa
type Blog = {
	users: Bag(User) = []
	posts: Bag(Post) = []

	type User = {
		name: String
		articles: Bag(posts->author) -- the posts whose `author` is this user
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

console.log(ana.articles.length) -- 1
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

console.log(bao.bestFriendOf.length)  -- 0
console.log(cleo.bestFriendOf.length) -- 1
```

These are the three steps of the TypeScript version, generated from the declaration. You can't write them yourself: the reverse side is read-only.

```fa
cleo.bestFriendOf.add(ana) -- error: `bestFriendOf` is the reverse side of `bestFriend` and is read-only: write `ana.bestFriend = cleo`
```

A many-to-many relation is written through the bag of the direct side:

```fa
ana.friends.add(bao)

console.log(bao.friendOf.length) -- 1
```

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

### Effects

A function that writes a direct side declares it with `mutates`, like any other field. The reverse sides it changes are implied: they can only change through the direct side.

```fa
mutable people: Bag(Person) = []

type Person = {
	name: String
	bestFriend: (Person in people)?
	bestFriendOf: Bag(people->bestFriend)
}

function befriend = (person: Person in people, friend: Person in people) mutates person.bestFriend => {
	person.bestFriend = friend -- also updates `bestFriendOf` of the previous best friend and of `friend`
}
```

## Deleting an object

Deleting an object cleans up every relation it was part of, at once, through the reverse sides:

- every direct side that pointed at it is set to `none`, or loses it if it is a bag;
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

The cleanup costs O(k) for an object linked to k others, and is paid at deletion only: reading a relation never checks anything. There are no generation counters and no hidden indexes: no stale reference ever exists.

## Where linked objects live

Both sides of a relation are elements of a collection with **stable addresses**: a [`Bag`](/collections/bags), a [`Columnar`](/collections/columnars) or a [`GrowingBag`](/collections/growing-collections). Their elements live in slots that never move, so a stored reference stays valid until its target is deleted, and deletion cleans it up.

An `Array` or a `Set` can't hold linked objects, because they move their elements:

- an `Array` reallocates when it grows, and shifts its elements on `insert` and `delete`;
- a `Set` rehashes when an element is added or deleted.

A [`Map`](/collections/maps) doesn't move its values, but a relation links elements of a collection, not values found by key: store linked objects in a `Bag`.

A [local reference](/advanced/owned-references) into these collections is fine: the compiler sees the call that invalidates it, and makes you declare it again. A stored reference lives in data, and can be read anywhere, long after the collection changed: there is no place where it could be declared again. So its target must never move.

```fa
mutable people: Array(Person) = []

type Person = {
	name: String
	bestFriend: (Person in people)? -- error: `people` is an `Array`, whose elements move: a relation needs a `Bag`, a `Columnar` or a `GrowingBag`
	bestFriendOf: Bag(people->bestFriend)
}
```

:::tip
Use a `Bag` for the objects that point at each other: order does not matter, and adding elements never moves the others.
:::

## Relations and local references

A [local reference](/advanced/owned-references) (`use`, a parameter, a return value) is short-lived: the compiler tracks it statically through the `invalidates` effects of the calls, and after an invalidating call, you must declare it again before using it.

A relation is stored: it lives as long as the object that holds it, and the compiler keeps it up to date at runtime, on every write and every deletion.

Use a local reference for short-lived work on a stored value. Use a relation when two objects need to know about each other over time.

## Relations and recursion

A type can't contain itself directly, because its size would be infinite. Recursion through a union, including an optional (`next: Node?`), is allowed: the compiler stores the child on the heap, but it is still a copy, owned by its parent (see [recursive types](/advanced/memory-management#recursive-types)). There is no `Box` type.

Owned children make trees: a node can't have two parents, and a child can't point back to its parent. As soon as objects need to point at each other (a doubly linked list, a graph, a parent and its children), store them in a stable-address collection and link them with relations.

## Example: a linked list

A doubly linked list is exactly the structure that goes wrong with raw pointers: nothing guarantees that `previous` and `next` stay consistent. In Fa, `previous` is the reverse side of `next`, and every node lives in a `Bag` owned by the list:

```fa
type LinkedList(Data) = {
	-- every node lives here (private field)
	#nodes: Bag(Node) = []

	type Node = {
		data: Data

		-- one-to-one: a node is the `next` of at most one node
		next: (Node in #nodes)?
		previous: (#nodes->next)?
	}

	-- owner fields: the ends of the list
	firstChild: (Node in #nodes)?
	lastChild: (Node in #nodes)?

	-- append a node: adding to a bag never moves the other nodes
	function add = (self, data: Data): Node in self.#nodes mutates self => {
		use newNode = self.#nodes.add(Node { data = data })
		use last = self.lastChild

		if last is not none {
			last.next = newNode -- `newNode.previous` becomes `last`
		} else {
			self.firstChild = newNode
		}

		self.lastChild = newNode
		newNode
	}

	-- unlink and destroy a node
	function remove = (self, node: Node in self.#nodes) invalidates node => {
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

`firstChild` and `lastChild` are not stored in a node, but in the list that owns `#nodes`. They are **owner fields**: fields of the type that owns a collection, pointing into that collection. An owner field has exactly one holder, the owner itself, so it has no reverse side to declare: when a node is deleted from `#nodes`, the owner fields that pointed at it become `none`. This is the only kind of reference field without a reverse side.

The effects in the signatures tell the caller what happens to its references:

- `add` declares `mutates self`, but not `invalidates`: adding to a `Bag` never moves the existing nodes, so every reference to a node stays valid.
- `remove` declares `invalidates node`, like `Bag.delete`: the references that may designate the removed node must be declared again, and the others stay valid.

```fa
mutable list: LinkedList(String) = {}

use first = list.add("first")
use second = list.add("second")
list.add("third")

console.log(first.next?.data) -- "second"

list.remove(second)

console.log(second.data) -- error: `second` was invalidated by `list.remove`: declare it again with `use second = ...`

use first = list.firstChild -- `first` is declared again
console.log(first?.next?.data) -- "third"
console.log(first?.next?.previous?.data) -- "first"
```

Notice what you did *not* have to write: `previous` is never assigned. Declaring it as `#nodes->next` is enough for the compiler to keep it correct, including when `remove` relinks the list.

Storing all the nodes in one bag has two further benefits: destroying the list frees every node in one step, and iterating over `#nodes` walks through every node contiguously, which is much faster than following `next` references through scattered memory.
