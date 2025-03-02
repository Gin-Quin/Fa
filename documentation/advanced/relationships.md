
# Relationships

A **relation** is a weak reference that declares a relation between two types. It is declared using the arrow `->` operator, also called the "relation operator."

Let's take this simple example where a person has a best friend:

```fa
type Person = {
  name: String
  bestFriend: Person?
}
```

We can easily know who's the best friend of a person, but what if we also want to know who this person is a best friend of?

In Javascript, you would naively implement it like this:

```js
type Person = {
  name: String
  bestFriend: Person?
  bestFriendOf: Array<Person>
}
```

Here, any human will instinctively see the relation between the fields `bestFriend` and `bestFriendOf`. But the compiler? It's blind.

This is also quite bad code because you can easily create cyclic relationships, for example, if two persons are best friends of each other. Cyclic references are why garbage collectors use so much memory.

In Fa, you would not only declare the type but also the **relation** between the fields `bestFriend` and `bestFriendOf`:

```fa
type Person = {
  name: String
  bestFriend: Person?
  bestFriendOf: many Person -> one bestFriend
}
```

For a relation to work, you have to have one of the fields being the owner of the relation, the other being a weak reference.

The weak reference can be either a single reference (One) or multiple references (Many) pointing back to the owner field, and is distinguished by using the `->` reference operator.

Now, not only have we created a **safe weak reference**, but we also don't have to manually update the `bestFriendOf` field when the `bestFriend` field changes.

In the Javascript world, when adding a new best friend, you would have to do something like this:

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
  bestFriendOf: OneToOne(Person.bestFriend)
}
```

Here, each person can only be the best friend of one unique other person.

This means that if person A is the best friend of person B, and then we change A's best friend to person C, B will no longer be A's best friend.

## Many-to-one relations

In a many-to-one relation, a collection of references is implicitly created to store all the elements of the relation.

```fa
type Person = {
  name: String
  bestFriend: Person?
  bestFriendOf: ManyToOne(Person.bestFriend)
}
```

## One-to-many relations

In a one-to-many relation, we have a single reference to an element in a collection.

```fa
type Person = {
  name: String
  bestFriends: Set(Person)
  bestFriendOf: OneToMany(Person.bestFriends)
}
```

In this example, `Person.bestFriends` must be a collection for this to work.

## Many-to-many relations

In a many-to-many relation, we have a collection of references to store all the elements of the relation.

```fa
type Person = {
  name: String
  bestFriends: Set(Person)
  bestFriendOf: ManyToMany(Person.bestFriends)
}
```

> In the real world, this is what we'd use to describe the best friends of a person. One human can have many best friends and can be the best friend of many other humans.

## Linked lists

A good example of where to use relations in Fa is linked lists.

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
type Node(Pool: Collection) = {
  value: Int
  next: Node in Pool?
  previous: OneToOne(Node.next)
}
```

And that's it!

## Concurrent modifications in relationships

TODO: explain how to have safe multi-threading with relationships.
