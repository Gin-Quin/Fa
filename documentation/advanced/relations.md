# Relations

A **relation** is a weak reference that declares a relation between two fields of two objects. It is declared using the arrow `->` operator, also called the "relation operator".

Let's take this simple example where a person has a best friend:

```fa
type Person = {
  name: String
  bestFriend: Person?
}
```

We can easily know who's the best friend of a person, but what if we also want to know who this person is a best friend of?

In TypeScript, you would naively implement it like this:

```js
type Person = {
  name: String
  bestFriend?: Person
  bestFriendOf: Array<Person>
}
```

Here, because of the naming of the fields, any human will instinctively see the relation between the fields `bestFriend` and `bestFriendOf`. If we have `personA.bestFriend = personB`, then we also should have `personA in personB.bestFriendOf`.

For humans it seems obvious, but the compiler does not know that. There is no information indicating that `bestFriend` and `bestFriendOf` are related to each other.

Fa brings a new way to solve this. You would not only declare the **type** but also the **relation** that binds the two fields.

Like this:

```fa
type Person = {
  name: String
  bestFriend: Person?
  bestFriendOf: Set(Self->bestFriend)
}
```

:::tip
When defining one-to-many relations, we must use the `Set` type instead of `Array` because:

- order does not matter,
- adding or removing elements in the set will not reallocate the underlying memory of the collection, which means references will remain safe,
- we want uniqueness.
  :::

Here, we just indicated `bestFriendOf` is not just a set of `Person`, but a set of `Person` that are related to the `bestFriend` field in the `Person` type.

Now, not only have we created a **safe weak reference**, but we also don't have to manually update the `bestFriendOf` field when the `bestFriend` field changes.

In the TypeScript world, when adding a new best friend, you would have to do something like this:

```ts
const addBestFriend = (person: Person, newBestFriend: Person) => {
	// 1. Remove the previous best friend
	if (person.bestFriend) {
		person.bestFriend.bestFriendOf = person.bestFriend.bestFriendOf.filter(
			(friend) => friend != person,
		);
	}

	// 2. Add the new best friend
	person.bestFriend = newBestFriend;

	// 3. Add the person to the new best friend's best friends
	newBestFriend.bestFriendOf.push(person);
};

addBestFriend(person1, person2);

console.log(person2.bestFriendOf); // will print [person1]
```

This is imperative programming, and it leaves a lot of room for error.

In Fa, since we declared the relation, you would just do this instead:

```fa
person1.bestFriend = person2 -- this will automatically trigger the update of person2.bestFriendOf

console.log(person2.bestFriendOf) -- will print [person1]
```

Because we declared the relation `Person.bestFriend`, the compiler will automatically update `person2.bestFriendOf` when `person1.bestFriend` changes.

More safety, less code.

We just created a **one-to-many** relation, but you can also declare **one-to-one** and **many-to-many** relations.

## One-to-one relations

```fa
type Person = {
  name: String
  bestFriend: Person?
  bestFriendOf: Self->bestFriend
}
```

Here, each person can only be the best friend of one unique other person.

This means that if person A is the best friend of person B, when we change A's best friend to person C, B will no longer be A's best friend.

## One-to-many relations

To declare a many-to-one relation, you need to use a **bag of references**.

```fa
type Person = {
  name: String
  bestFriend: Person?
  bestFriendOf: Set(Self->bestFriend)
}
```

:::warning
It's not possible to declare a 1-N relation the other way around.
:::

## Many-to-many relations

In a many-to-many relation, we have a collection of references to store all the elements of the relation.

```fa
type Person = {
  name: String
  bestFriends: Set(Person)
  bestFriendOf: Set(in Self->bestFriends)
}
```

## Concrete example: Linked lists

A good example of where to use relations in Fa is linked lists.

When you create a new collection, we should first wonder: how are the objects stored? In other words, what collection owns the objects?

In the case of a linked list, we have two possibilities:

1. There is no overall collection, objects are stored as the "next" object of the previous node, inside a `Box`.
2. We use a `Set` as the underlying collection, and objects are "references" of values in this bag.

Method 1 is the simplest and will be better for small linked lists, but method 2 is the most powerful: since all objects are stored contiguously, it's very efficient to clean the whole list. Also, it brings the advantage of allowing us to iterate through the whole list very quickly.

### Method 1: Box

This would be the TypeScript implementation:

```ts
type ListNode<Data> = {
	data: Data;
	next?: ListNode;
	previous?: ListNode;
};
```

Notice how `previous` here is unsafe: if the developer failed to connect the right `next` and `previous` fields, we can end up in inconsistent state.

```fa
type ListNode(Data) = {
	data: Data
	next: Set(Self)?
	previous: Self->next
}
```

We have to use a `Box` here because we cannot have an object recursively owning itself.

### Method 2: Set

With this method, we put all our objects inside a `Set` and link references.

```fa
type LinkedList(Data) = {
	type Node = {
		-- the data stored in the node
	  data: Data

		-- an optional reference to another node in the set
	  next: in #set?

		-- the related previous node in the set
	  previous: Self->next
	}

	-- the set containing all nodes (private field)
	#set: Set(Node)

	-- the first node in the list
	firstChild: in #set?

	-- the last node in the list
	lastChild: in #set?

	-- add a new node to the end of the list
	add(mutable self, data: Data): in self.#set => {
		let newNode = self.#set.add(Node(data))

		if self.lastChild {
			self.lastChild.next = newNode
		} else {
			self.firstChild = newNode
		}

		self.lastChild = newNode
		newNode
	}

	-- remove a node from the list
	remove(mutable self, node: in #set) => {
		node.previous.next = node.next
		self.#set.remove(node)
	}
}
```
