# Owned references

An **owned reference** is a reference to an element *within a collection*. We say that the collection *owns* the reference.

Consider this simple example:

```fa
let persons: Array(Person) = [Person(), Person(), Person()]

let copied = persons[1] -- this creates a copy
use owned = persons[1] -- this creates an owned reference

console.log(@type(copied)) -- will print "Person"
console.log(@type(owned)) -- will print "Person in arrayOfPersons"
```

Here, we can see that the type of `owned` is not `Person`, but `Person in arrayOfPersons`.

This means that the type system knows that `owned` depends directly on `arrayOfPersons`. If `arrayOfPersons` is mutated, `owned` might become invalid:

```fa
-- a mutable array of persons
mutable persons: Array(Person) = [Person(), Person(), Person()]

-- the owned reference is now mutable as well
use someone = persons[1]

-- this works fine
somone.name = "John"

-- now, if we mutate the array, all references are invalidated
persons.pop()

-- error: 'someone' is invalid
console.log(someone.name)
```

## Lifetime of an owned reference

The lifetime of an owned reference is the same as the lifetime of the collection that owns it.

Since, in Fa, variables are destroyed at the end of their scope, it's actually impossible to have a reference that outlives its collection.

Unless you return it from a function; in that case it's automatically copied into a new object:

```fa
function createPerson = (): Person => {
  let persons = [Person()]
  use person = persons[0] -- type: `Person in persons`
  return person -- implicit conversion from type `Person in persons` to type `Person`: a copy is made
}
```

But you can also return a reference to an element of a collection that is passed as an argument:

```fa
function createPerson = (persons: Array(Person)): Person in persons? => {
  use person = persons[0] -- type: `Person in persons?`
  return person -- okay, we return the reference itself
}
```

## Moving an element out of a collection

In Fa, all collections implement the method `take`, which takes an element from the collection and returns it as a standalone object.

```fa
let arrayOfPersons = [Person(), Person(), Person()]
let anotherArrayOfPersons = [Person()]

let someone = arrayOfPersons.take(index = 1)

anotherArrayOfPersons.push(someone)
```
