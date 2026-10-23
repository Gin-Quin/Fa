# Relations

A **relation** is a reference stored in a field. Relations let objects point at each other safely, without pointers, reference counting or manual bookkeeping. They replace the weak pointers of other languages: a relation never dangles.

## Copies and references

A field typed `Person` or `Person?` holds a **copy**: the person is stored inside the object that has the field.

```fa
type Person = {
	name: String
	mentor: Person? -- a copy of another person, owned by this one
}
```

To *point at* a person instead of holding one, the field names the collection the person lives in:

```fa
type Person = {
	name: String
	bestFriend: (Person in people)? -- a reference to a person stored in `people`
	friends: Bag(Person in people) -- references to persons stored in `people`
}
```

`Person in people` is the type of a reference: a `Person` stored in the collection `people`, as for [local references](/advanced/owned-references). A field with this type is always one side of a relation. There is no reference without a named collection.

## Where the collection lives

The collection must be visible where the type is declared. It is either a field of the containing type:

```fa
type Town = {
	people: Bag(Person) = []

	type Person = {
		name: String
		bestFriend: (Person in people)?
	}
}
```

Each town has its own people, and a person's best friend is always someone from the same town. The [linked list](#example-a-linked-list) below works the same way, with a private `#nodes` field.

Or it is a module-level value:

```fa
mutable people: Bag(Person) = []

type Person = {
	name: String
	bestFriend: (Person in people)?
}
```

The type is always written in full, `Person in people`, and the compiler checks that `Person` is the element type of `people`.

Objects are added to the collection, which returns a reference to each of them, and linked by assigning the field:

```fa
mutable town: Town = {}

use ana = town.people.add({ name = "Ana" })
use bao = town.people.add({ name = "Bao" })

ana.bestFriend = bao
```

## Reverse sides

We know who Ana's best friend is, but not whose best friend Bao is. In TypeScript, you would add a second field:

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

	// 2. attach the new one
	person.bestFriend = newBestFriend

	// 3. and update the reverse side
	newBestFriend.bestFriendOf.push(person)
}
```

In Fa, you declare the reverse side in terms of the forward one, with the arrow `->`:

```fa
type Town = {
	people: Bag(Person) = []

	type Person = {
		name: String
		bestFriend: (Person in people)?
		bestFriendOf: Bag(Self->bestFriend)
	}
}
```

`Self->bestFriend` reads as: *the persons whose `bestFriend` field points at me*. The compiler keeps both sides in sync:

```fa
ana.bestFriend = bao

console.log(bao.bestFriendOf) -- contains ana
```

Assigning `bestFriend` detaches the old reverse link and attaches the new one. The relation is the code.

There are three kinds of reverse sides. The examples below use a module-level collection, `mutable people: Bag(Person) = []`.

### One-to-one

```fa
type Person = {
	name: String
	bestFriend: (Person in people)?
	bestFriendOf: (Self->bestFriend)?
}
```

Each person can be the best friend of at most one other person. If Ana's best friend is Bao, then `bao.bestFriendOf` is Ana. Setting `cleo.bestFriend = bao` moves the reverse link: `bao.bestFriendOf` becomes Cleo, and `ana.bestFriend` becomes `none`.

### One-to-many

```fa
type Person = {
	name: String
	bestFriend: (Person in people)?
	bestFriendOf: Bag(Self->bestFriend)
}
```

Many people can have the same best friend, but each person has at most one best friend.

### Many-to-many

When the forward side is a bag, the reverse side points *into* it with `in`:

```fa
type Person = {
	name: String
	bestFriends: Bag(Person in people)
	bestFriendOf: Bag(in Self->bestFriends)
}
```

Adding to the forward side updates the reverse side:

```fa
ana.bestFriends.add(bao)

console.log(bao.bestFriendOf) -- contains ana
```

:::warning
A relation is always declared from the *one* side: the reverse field (`bestFriendOf`) is declared in terms of the forward field (`bestFriend`). You cannot declare `bestFriend` in terms of `bestFriendOf`.
:::

## Deleting a target

When an object is deleted, every field that pointed at it is updated, as part of the deletion: an optional field becomes `none`, and a bag no longer contains it.

```fa
town.people.delete(bao) -- invalidates the local reference `bao`, and updates every field that pointed at Bao

console.log(ana.bestFriend) -- none
```

To find those fields, the compiler uses the reverse side when the type declares one. When it doesn't, the compiler maintains a hidden reverse index instead: a relation is kept correct whether or not you declared its reverse side. Declaring it only lets you read it.

Two rules make this possible.

**A stored reference is always optional or held in a `Bag`**, so that it has something to become when its target is deleted:

```fa
type Person = {
	name: String
	bestFriend: Person in people -- error: a stored reference must be optional or held in a `Bag`: write `(Person in people)?`
}
```

**Its collection has stable addresses**: a [`Bag`](/collections/bags), a [`GrowingBag`](/collections/growing-collections) or a [`Columnar`](/collections/columnars), never an `Array`, a `Set` or a `Map`. These three move their elements:

- an `Array` reallocates when it grows, and shifts its elements on `insert` and `delete`: a reference is an index, which can silently point at another element;
- a `Set` or a `Map` rehashes when an element is added or deleted.

For a [local reference](/advanced/owned-references#after-invalidation), this is fine: the compiler sees the call that invalidates it, and makes you check it or set it again. A stored reference lives in data, and can be read anywhere, long after the collection changed: there is no place where the compiler could ask you to fix it. So its target must never move.

```fa
mutable people: Array(Person) = []

type Person = {
	name: String
	bestFriend: (Person in people)? -- error: `people` is an `Array`, whose elements move: a stored reference needs a `Bag`, a `GrowingBag` or a `Columnar`
}
```

:::tip
Use a `Bag` for the objects that point at each other:

- order does not matter,
- adding and removing elements does not move the other elements.
:::

## Relations and local references

A [local reference](/advanced/owned-references) (`use`, a parameter, a return value) is short-lived: the compiler tracks it through the `invalidates` effects of the calls, and after an invalidating call, you must check it or set it again before using it.

A relation is stored: it lives as long as the object that holds it, and the compiler keeps it up to date at runtime. Deleting its target is a normal, handled event.

Use a local reference for short-lived work on a stored value. Use a relation when two objects need to know about each other over time.

## Relations and recursion

A type can't contain itself directly, because its size would be infinite. Recursion through a union, including an optional (`next: Node?`), is allowed: the compiler stores the child on the heap, but it is still a copy, owned by its parent (see [recursive types](/advanced/memory-management#recursive-types)). There is no `Box` type.

Owned children make trees: a node can't have two parents, and a child can't point back to its parent. As soon as objects need to point at each other (a doubly linked list, a graph, a parent and its children), store them in a stable-address collection and link them with relations.

## Example: a linked list

A doubly linked list is a good illustration, because it is exactly the structure that goes wrong with raw pointers: nothing guarantees that `previous` and `next` stay consistent.

First, decide where the nodes live. Here, every node lives in a `Bag` owned by the list, so that references to the nodes stay valid when nodes are added:

```fa
type LinkedList(Data) = {
	-- every node lives here (private field)
	#nodes: Bag(Node) = []

	type Node = {
		-- the data held by the node
		data: Data

		-- a relation to another node of the list
		next: (Node in #nodes)?

		-- the reverse side of `next`, maintained by the compiler
		previous: (Self->next)?
	}

	firstChild: (Node in #nodes)?
	lastChild: (Node in #nodes)?

	-- append a node: adding to a bag never moves the other nodes
	function add = (self, data: Data): Node in self.#nodes mutates self => {
		use newNode = self.#nodes.add(Node { data = data })

		if self.lastChild is not none {
			self.lastChild.next = newNode
		} else {
			self.firstChild = newNode
		}

		self.lastChild = newNode
		newNode
	}

	-- unlink and destroy a node: only the reference to this node is invalidated
	function remove = (self, node: Node in self.#nodes) mutates self invalidates node => {
		if node.previous is not none {
			node.previous.next = node.next
		} else {
			self.firstChild = node.next
		}

		if node.next is none {
			self.lastChild = node.previous
		}

		self.#nodes.delete(node)
	}
}
```

The effects in the signatures tell the caller exactly what happens to its references:

- `add` declares `mutates self`, but not `invalidates`: adding to a `Bag` never moves the existing nodes, so every reference to a node stays valid.
- `remove` declares `invalidates node`, like `Bag.delete`: only the reference to the removed node becomes invalid.

```fa
mutable list: LinkedList(String) = {}

use first = list.add("first")
use second = list.add("second")
use third = list.add("third")

list.remove(second)

console.log(first.next?.data) -- "third"
console.log(third.previous?.data) -- "first"
console.log(second.data) -- error: `second` was invalidated by `list.remove`: check it with `if second is not none`
```

Notice what you did *not* have to write: `previous` is never assigned. Declaring it as `Self->next` is enough for the compiler to keep it correct, including when `remove` relinks the list.

Storing all the nodes in one bag has two further benefits: destroying the list frees every node in one step, and iterating over `#nodes` walks through every node contiguously, which is much faster than following `next` references through scattered memory.
