# Relations

A **relation** is a weak reference that declares how two fields relate to each other. It is written with the arrow `->`, the *relation operator*.

Relations are what let objects in Fa refer to each other safely, without pointers, reference counting, or manual bookkeeping.

## The problem

Take a person who has a best friend:

```fa
type Person = {
  name: String
  bestFriend: Person?
}
```

We know who a person's best friend is, but not whose best friend *they* are. In TypeScript you would add a second field:

```ts
type Person = {
	name: string
	bestFriend?: Person
	bestFriendOf: Person[]
}
```

A human reading those names sees the relation immediately: if `personA.bestFriend` is `personB`, then `personA` should be in `personB.bestFriendOf`. The compiler sees two unrelated fields, and it is entirely up to you to keep them consistent.

## Declaring a relation

In Fa you declare not only the type of the field, but the relation that binds it to another:

```fa
type Person = {
  name: String
  bestFriend: Person?
  bestFriendOf: Bag(Self->bestFriend)
}
```

`Self->bestFriend` reads as: *the `bestFriend` field, of persons pointing at me*. `bestFriendOf` is not a bag of arbitrary persons — it is exactly the set of persons whose `bestFriend` is this one.

That single declaration gives you two things: a safe weak reference, and automatic maintenance of both sides.

In TypeScript, adding a best friend means updating three places by hand:

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

In Fa, the relation is the code:

```fa
person1.bestFriend = person2

console.log(person2.bestFriendOf) -- contains person1
```

Assigning `bestFriend` detaches the old reverse link and attaches the new one. More safety, less code.

## Where related objects live

A relation is a reference, so the objects it points at must live somewhere with **stable addresses**. In practice that means a [`Bag`](../collections/bags), a `GrowingBag`, or a [`Columnar`](../collections/columnars):

```fa
mutable people = Bag(Person) {}

use ana = people.add(Person { name = "Ana" })
use bao = people.add(Person { name = "Bao" })

ana.bestFriend = bao
```

Arrays, sets and maps move their elements when they grow, so a relation cannot point into one. The compiler will tell you.

:::tip
Use a `Bag` rather than an `Array` for the *many* side of a relation:

- order does not matter,
- adding and removing does not move the other elements, so references stay valid,
- it holds each element once.
:::

## Removal

When a related object is deleted, the compiler clears the references to it. A one-to-one or optional field becomes `none`; a bag simply no longer contains it. A relation can never dangle.

```fa
people.delete(bao)

console.log(ana.bestFriend) -- none
```

This is the difference between a relation and an [owned reference](./owned-references): removing the target of an owned reference invalidates it, and the compiler stops you from using it. Removing the target of a relation is a normal, handled event.

## One-to-one

```fa
type Person = {
  name: String
  bestFriend: Person?
  bestFriendOf: (Self->bestFriend)?
}
```

Each person can be the best friend of at most one other person.

If Ana's best friend is Bao, then `bao.bestFriendOf` is Ana. Setting `cleo.bestFriend = bao` would move the reverse link, so `bao.bestFriendOf` becomes Cleo and Ana's link is cleared.

## One-to-many

Use a bag on the reverse side:

```fa
type Person = {
  name: String
  bestFriend: Person?
  bestFriendOf: Bag(Self->bestFriend)
}
```

Many people can have the same best friend, but each person has at most one.

:::warning
The relation is always declared from the *one* side. You cannot declare `bestFriend` in terms of `bestFriendOf`.
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

Adding to either side updates the other:

```fa
ana.bestFriends.add(bao)

console.log(bao.bestFriendOf) -- contains ana
```

## Example: a linked list

A linked list is a good illustration, because it is exactly the structure that goes wrong with raw pointers.

First, decide who owns the nodes. Fa objects cannot own themselves recursively, so the nodes have to live in a collection — a `Bag`, so that references into it stay valid.

```fa
type LinkedList = (Data: Type) => {
  -- every node lives here (private field)
  #nodes: Bag(Node)

  type Node = {
    -- the data held by the node
    data: Data

    -- an optional reference to another node in the bag
    next: (Node in #nodes)?

    -- the reverse of `next`, maintained by the compiler
    previous: (Self->next)?
  }

  firstChild: (Node in #nodes)?
  lastChild: (Node in #nodes)?

  -- append a node
  add(mutable self, data: Data): Node in self.#nodes => {
    use newNode = self.#nodes.add(Node { data })

    if self.lastChild is not none {
      self.lastChild.next = newNode
    } else {
      self.firstChild = newNode
    }

    self.lastChild = newNode
    return newNode
  }

  -- unlink and destroy a node
  remove(mutable self, node: Node in self.#nodes) => {
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

Notice what you did *not* have to write: `previous` is never assigned. Declaring it as `Self->next` is enough for the compiler to keep it correct, including when `remove` relinks the list.

Storing all nodes in one bag has two further benefits: destroying the list frees every node in one step, and iterating `#nodes` walks the whole list contiguously, which is far faster than following `next` pointers through scattered memory.
