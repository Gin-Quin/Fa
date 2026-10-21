# Relations

A **relation** is a safe reference that declares how two fields relate to each other. It is written with the arrow `->`, the *relation operator*.

Relations are what let objects in Fa refer to each other safely, without pointers, reference counting, or manual bookkeeping. They replace the weak pointers of other languages: a relation never dangles.

## The problem

Take a person who has a best friend:

```fa
type Person = {
	name: String
	bestFriend: Person?
}
```

We know who a person's best friend is, but not whose best friend *they* are. In TypeScript, you would add a second field:

```ts
type Person = {
	name: string
	bestFriend?: Person
	bestFriendOf: Person[]
}
```

A human reading those names sees the relation immediately: if `personA.bestFriend` is `personB`, then `personA` should be in `personB.bestFriendOf`. But the compiler sees two unrelated fields, and it is entirely up to you to keep them consistent.

## Declaring a relation

In Fa, you declare not only the type of the field, but the relation that binds it to another field:

```fa
type Person = {
	name: String
	bestFriend: Person?
	bestFriendOf: Bag(Self->bestFriend)
}
```

`Self->bestFriend` reads as: *the persons whose `bestFriend` field points at me*. `bestFriendOf` is not a bag of arbitrary persons: it is exactly the persons whose `bestFriend` is this one.

That single declaration gives you two things: a safe reference, and the automatic maintenance of both sides.

In TypeScript, changing a best friend means updating three places by hand:

```ts
const addBestFriend = (person: Person, newBestFriend: Person) => {
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

This is imperative code, and it leaves a lot of room for errors. In Fa, the relation is the code:

```fa
ana.bestFriend = bao

console.log(bao.bestFriendOf) -- contains ana
```

Assigning `bestFriend` detaches the old reverse link and attaches the new one. More safety, less code.

## Where related objects live

The fields of a relation are references to other objects, not copies. So the objects they point at must live somewhere with **stable addresses**: a [`Bag`](/collections/bags), a [`GrowingBag`](/collections/growing-collections) or a [`Columnar`](/collections/columnars).

```fa
mutable people: Bag(Person) = []

use ana = people.add(Person { name = "Ana" })
use bao = people.add(Person { name = "Bao" })

ana.bestFriend = bao
```

Arrays, sets and maps move their elements when they grow (their `add` and `push` methods declare `invalidates self`, see [owned references](/advanced/owned-references#invalidation)), so a relation cannot point into one. Using an object stored in such a collection as the target of a relation is a compile error.

:::tip
Use a `Bag` rather than an `Array` for the *many* side of a relation:

- order does not matter,
- adding and removing elements does not move the other elements, so references stay valid,
- it holds each element once.
:::

## Removal

When a related object is deleted, the compiler clears the references to it: an optional field becomes `none`, and a bag simply no longer contains it. A relation never dangles.

```fa
people.delete(bao) -- invalidates `bao`, and clears every relation to it

console.log(ana.bestFriend) -- none
```

This is the difference between a relation and an [owned reference](/advanced/owned-references): removing the target of an owned reference invalidates it, and the compiler stops you from using it. Removing the target of a relation is a normal, handled event.

## One-to-one

```fa
type Person = {
	name: String
	bestFriend: Person?
	bestFriendOf: (Self->bestFriend)?
}
```

Each person can be the best friend of at most one other person.

If Ana's best friend is Bao, then `bao.bestFriendOf` is Ana. Setting `cleo.bestFriend = bao` moves the reverse link: `bao.bestFriendOf` becomes Cleo, and `ana.bestFriend` becomes `none`.

## One-to-many

Use a bag on the reverse side:

```fa
type Person = {
	name: String
	bestFriend: Person?
	bestFriendOf: Bag(Self->bestFriend)
}
```

Many people can have the same best friend, but each person has at most one best friend.

:::warning
A relation is always declared from the *one* side: the reverse field (`bestFriendOf`) is declared in terms of the forward field (`bestFriend`). You cannot declare `bestFriend` in terms of `bestFriendOf`.
:::

## Many-to-many

When both sides hold collections, the reverse side points *into* the forward collection with `in`:

```fa
type Person = {
	name: String
	bestFriends: Bag(Person)
	bestFriendOf: Bag(in Self->bestFriends)
}
```

Adding to the forward side updates the reverse side:

```fa
ana.bestFriends.add(bao)

console.log(bao.bestFriendOf) -- contains ana
```

## Relations and recursion

A type can't contain itself directly, because its size would be infinite. Recursion through a union or an optional (`next: Node?`) is allowed: the compiler stores the child on the heap automatically, and each value owns its children (see [recursive types](/advanced/memory-management#recursive-types)). There is no `Box` type.

Owned children make trees: a node can't have two parents, and a child can't point back to its parent. As soon as objects need to point at each other (a doubly linked list, a graph, a parent and its children), store them in a stable-address collection and link them with relations.

## Example: a linked list

A doubly linked list is a good illustration, because it is exactly the structure that goes wrong with raw pointers: nothing guarantees that `previous` and `next` stay consistent.

First, decide who owns the nodes. Here, every node lives in a `Bag`, so that references to the nodes stay valid when nodes are added:

```fa
type LinkedList(Data) = {
	-- every node lives here (private field)
	#nodes: Bag(Node) = []

	type Node = {
		-- the data held by the node
		data: Data

		-- an optional reference to another node of the bag
		next: (Node in #nodes)?

		-- the reverse of `next`, maintained by the compiler
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
console.log(second.data) -- error: `second` was invalidated by `list.remove`
```

Notice what you did *not* have to write: `previous` is never assigned. Declaring it as `Self->next` is enough for the compiler to keep it correct, including when `remove` relinks the list.

Storing all the nodes in one bag has two further benefits: destroying the list frees every node in one step, and iterating over `#nodes` walks through every node contiguously, which is much faster than following `next` references through scattered memory.
