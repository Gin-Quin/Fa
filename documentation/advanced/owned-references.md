# Owned references

An **owned reference** is a reference to an element *within a collection*. We say that the collection *owns* the reference.

Consider this simple example:

```fa
arrayOfPersons = [Person(), Person(), Person()]

person1Copied = arrayOfPersons.get(1) // this creates a copy
person1Owned = arrayOfPersons.at(1) // this creates an owned reference

console.log(@typeToString(person1Copied)) // will print "Person"
console.log(@typeToString(person1Owned)) // will print "Person in arrayOfPersons"
```

One would expect that `someone` is a `Person` object. But this is not the case, even though it can implicitly be converted to a `Person`. `someone` is actually a `Person in arrayOfPersons`.

This means that `someone` is not a standalone object, but only a reference to an object *within* `arrayOfPersons`.

This also means that `someone` cannot be used outside of `arrayOfPersons` unless you create a copy of it.

## Invalid references

Since a collection can evolve, it's possible that a reference to an element of a collection is invalidated.

For example, if we remove the element at index 1 from `arrayOfPersons`, the reference `someone` will be invalidated.

```fa
arrayOfPersons = [Person(), Person(), Person()]

someone = arrayOfPersons.last()

arrayOfPersons.pop() // mutating the collection

// now, 'someone' is invalid

console.log(someone) // this is dangerous!
```

Hopefully, Fa enforces the type of the reference to be potentially invalid, so the last line of our example will not compile.

Instead, you have to check if the reference is still valid:

```fa
arrayOfPersons.pop() // mutating the collection

// because a mutation happened, the compiler widened the type of 'someone'
// from `Person in arrayOfPersons` to `Optional(Person in arrayOfPersons)`

// so now, we have to handle the case where 'someone' is invalid
if someone => console.log(someone)
else => console.log("someone is now invalid")
```


## Lifetime of a owned reference

The lifetime of an owned reference is the same as the lifetime of the collection that owns it.

Since, in Fa, variables are destroyed at the end of their scope, it's actually impossible to have a reference that outlives its collection.

Unless you return it in a function:

```fa
createPerson = (): Person => {
  arrayOfPersons = [Person()]
  personOwned = arrayOfPersons.at(0)
  return personOwned
}
```

But even in this case, it still works because the compiler will automatically **move** the reference out of the function as a standalone object.

## Explicit type of an owned reference

A common use case is for a function to return an owned reference to an element of a collection that is passed as an argument.

Then you can indicate in the return type the owner of the reference:

```fa
getFirstPerson = (arrayOfPersons: Array<Person>): Person in arrayOfPersons => {
  return arrayOfPersons.at(0)
}
```

This is useful when you need to create a linked list:

```fa
type LinkedList(Type: Object) = {
  bag: Bag(Node(Type))

  type Node(Type) = {
    value: Type
    next: Node(Type) in bag
    previous: OneToOne(Node(Type).next)
  }
}
```

## Moving an element out of a collection

In Fa, all collections implement the method `take`, which takes an element from the collection and returns it as a standalone object.

```fa
arrayOfPersons = [Person(), Person(), Person()]
anotherArrayOfPersons = [Person()]

someone = arrayOfPersons.take(1)

anotherArrayOfPersons.push(someone)
```
