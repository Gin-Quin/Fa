
# Relations / Weak References

A **relation** is a weak reference that declares a relation between two types. It is declared using the arrow `->` operator, also called the "relation operator."

Let's take this simple example where a person has a best friend:

```fa
type Person = {
  name: String
  bestFriend: Person?
}
```

We can easily know who's the best friend of a person, but what if we also want to know who this person is a best friend of?

In Typescript, you would naively implement it like this:

```js
type Person = {
  name: String
  bestFriend: Person?
  bestFriendOf: Array<Person>
}
```

Here, any human will instinctively see the relation between the fields `bestFriend` and `bestFriendOf`. But the compiler? It's blind.

This is also quite bad code because you can easily create cyclic relationships, for example, if two persons are best friends of each other. Checking cyclic references is why garbage collectors use so much memory.

In Fa, you would not only declare the type but also the **relation** between the fields `bestFriend` and `bestFriendOf`:

```fa
type Person = {
  name: String
  bestFriend: Person?
  bestFriendOf: Array(Self->bestFriend)
}
```

A **weak reference** (aka **relation**) is defined using the arrow operator 
`->`.

Now, not only have we created a **safe weak reference**, but we also don't have to manually update the `bestFriendOf` field when the `bestFriend` field changes.

In the Typecript world, when adding a new best friend, you would have to do something like this:

```js
const addBestFriend = (person: Person, newBestFriend: Person) => {
  // 1. Remove the previous best friend
  if (person.bestFriend) {
    person.bestFriend.bestFriendOf = person.bestFriend.bestFriendOf.filter((friend) => friend != person)
  }

  // 2. Add the new best friend
  person.bestFriend = newBestFriend

  // 3. Add the person to the new best friend's best friends
  newBestFriend.bestFriendOf.push(person)
}

addBestFriend(person1, person2)

console.log(person2.bestFriendOf) // will print [person1]
```

In Fa, you would do this instead:

```fa
person1.bestFriend = person2

console.log(person2.bestFriendOf) // will print [person1]
```

Because we declared the relation `Person.bestFriend`, the compiler will automatically update `person2.bestFriendOf` when `person1.bestFriend` changes.

More safety, less code.

We just created a many-to-one relation, but you can also declare:

- one-to-one relations
- many-to-one relations
- one-to-many relations
- and many-to-many relations.

## One-to-one relations

```fa
type Person = {
  name: String
  bestFriend: Person?
  bestFriendOf: Self->bestFriend
}
```

Here, each person can only be the best friend of one unique other person.

This means that if person A is the best friend of person B, and then we change A's best friend to person C, B will no longer be A's best friend.

## Many-to-one relations

To declare a many-to-one relation, you need to use a **collection of references**, like an Array, a Set, a Bag, etc.

```fa
type Person = {
  name: String
  bestFriend: Person?
  bestFriendOf: Array(Self->bestFriend)
}
```

## One-to-many relations

In a one-to-many relation, we have a single reference to an element in a collection.

```fa
type Person = {
  name: String
  bestFriends: Set(Person)
  bestFriendOf: Person in Self->bestFriends
}
```

In this example, `Person.bestFriends` must be a collection for this to work.

## Many-to-many relations

In a many-to-many relation, we have a collection of references to store all the elements of the relation.

```fa
type Person = {
  name: String
  bestFriends: Set(Person)
  bestFriendOf: Set(Person in Self->bestFriends)
}
```

> In the real world, this is what we'd use to describe the best friends of a person. One human can have many best friends and can be the best friend of many other humans.

## Concrete example: Linked lists

A good example of where to use relations in Fa is linked lists.

When you create a new collection, we should first wonder: how are the objects stored? In other words, what collection owns the objects?

In the case of a linked list, we have two possibilities:

1. There is no overall collection, objects are stored as the "next" object of the previous node, inside a `Box`.
2. We use a `Bag` as the underlying collection, and objects are "references" of values in this bag.

Method 1 is the simplest and will better for small linked lists, but method 2 is the most powerful: since all objects are stored contiguously, it's very efficient to clean the whole list. Also, it brings the advantage to allow to iterate throught the whole list very quickly. 


### Method 1: Box

This would be the Typescript implementation:

```ts
type ListNode<Data> = {
	data: Data
	next?: ListNode
	previous?: ListNode
}
```

Notice how `previous` here is unsafe: if the developer failed to connect the right `next` and `previous` fields, we can end up in inconsistent state.

```fa
type ListNode(Data) = {
	data: Data
	next: Box(Self)?
	previous: Self->next
}
```

We have to use a `Box` here because we cannot have an object recursively owning itself. In Typescript, every object is by default somewhat a \*Box\*.


### Method 2: Bag

With this method, we put all our objects inside a `Bag` and link references.

```ts
type ListNode<Data> = {
	data: Data
	next?: ListNode
	previous?: ListNode
}

type LinkedList<Data> {
	container: Set<Data>
	firstChild: ListNode<Data>
}
```


```fa
type ListNode(bag: Bag, Data) = {
	data: Data
	next: Self in bag
	previous: Self->next
} in bag
```

Let's define a basic linked list:

```fa
type Node(Pool: Collection) = {
  value: Int
  next: Node in Pool?
}
```

Now, what if we also want to know the previous node?

This is actually a one-to-one relation, as one node can only be the previous node of one other node.

Let's declare the relation:

```fa
type LinkedNode(Pool: Collection) = {
  value: Int
  next: Self in Pool?
  previous: Self->next
}
```

And that's it!

## Concurrent modifications in relationships

TODO: explain how to have safe multi-threading with relationships.
