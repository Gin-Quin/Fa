# Sets

A set stores unordered, unique values. Sets can be mutable or immutable.

Uniqueness is decided by equality of the values.

```fa
let setOfIntegers = Set(Int) { 1, 2, 2, 3 }

console.log(setOfIntegers.length) -- 3, because 2 is only stored once
```

```fa
mutable setOfHumans = Set(Human) {
  Human { name = "John", age = 20 }
  Human { name = "Jane", age = 21 }
  Human { name = "John", age = 22 }
}

let anotherJohn = Human { name = "John", age = 20 }
setOfHumans.add(anotherJohn) -- not added: an equal Human is already present
```

Note that the third human *is* stored: `{ name = "John", age = 22 }` is not equal to `{ name = "John", age = 20 }`.

## Custom comparison

Supply a comparison function when identity is carried by one field, such as an id:

```fa
type User = {
  id: Int
  email: String
  name: String
}

mutable setOfUsers = Set(User, compare = (a: User, b: User) => a.id == b.id) {
  User { id = 1, email = "john@doe.com", name = "John Doe" }
  User { id = 2, email = "jane@doe.com", name = "Jane Doe" }
}

setOfUsers.add(User { id = 1, email = "foo@bar.com", name = "Foo Bar" })
-- not added: a User with id 1 is already present
```

## Adding and removing

```fa
mutable tags = Set(String) { "fa", "docs" }

tags.add("parser")
tags.delete("docs")
tags.clear()

tags.includes("fa") -- true
tags.length
tags.isEmpty()
```

`add` returns whether the value was inserted:

```fa
if tags.add("fa") {
  console.log("this tag is new")
}
```

## Iteration

Order is not guaranteed and may change as the set is modified.

```fa
for tags >> tag {
  console.log(tag)
}
```

## Set operations

```fa
let a = Set(Int) { 1, 2, 3 }
let b = Set(Int) { 2, 3, 4 }

a.union(b)        -- { 1, 2, 3, 4 }
a.intersection(b) -- { 2, 3 }
a.difference(b)   -- { 1 }
a.isSubsetOf(b)   -- false
```

## Stable references

Adding to or removing from a set may rehash it, which moves its items and invalidates references into it:

```fa
mutable people = Set(Person) { }
use someone = people.add(Person { name = "Ana" })

people.add(Person { name = "Bao" })

console.log(someone.name) -- error: `someone` may have been invalidated
```

When you need references that survive modification, use a [`Bag`](./bags) or a [growing collection](./growing-collections) instead.

:::tip
This is why [relations](../advanced/relations) are declared over a `Bag`, not a `Set`.
:::

## Choosing a set

- Need uniqueness and fast membership tests? `Set`.
- Need to store many objects without uniqueness, with stable references? [`Bag`](./bags).
- Need a fixed set of names known at compile time? A [key set](../types/fields#key-sets), written `@{ ... }`.
