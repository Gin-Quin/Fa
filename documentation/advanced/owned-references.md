# Owned references

An **owned reference** is a reference to an element *inside a collection*. The collection owns the element, and therefore owns the reference.

```fa
let persons: Array(Person) = [Person {}, Person {}, Person {}]

let copied = persons[1]     -- a copy
use owned = persons[1]      -- an owned reference

console.log(@typeToString(TypeOf(copied))) -- "Person"
console.log(@typeToString(TypeOf(owned)))  -- "Person in persons"
```

The type of `owned` is not `Person` but `Person in persons`. The `in` part is what makes the reference safe: the type system knows exactly which collection the reference depends on.

## Invalidation

If the owning collection changes in a way that could move its elements, every reference into it becomes invalid — and the compiler knows:

```fa
mutable persons: Array(Person) = [Person {}, Person {}, Person {}]

use someone = persons[1]

-- fine
someone.name = "John"

-- this may move the array's storage, invalidating every reference into it
persons.pop()

-- error: `someone` is no longer valid
console.log(someone.name)
```

Which operations invalidate references depends on the collection:

| Collection | Invalidated by |
| --- | --- |
| `Array` | `push`, `insert`, `delete`, `pop`, `clear`, sorting |
| `Set`, `Map` | `add`, `delete`, `clear` (rehashing) |
| `Bag`, `Columnar` | `delete`, `clear` only — growth is safe |
| `Growing*` | nothing; references live as long as the collection |

This is why [bags](../collections/bags) and [growing collections](../collections/growing-collections) are the right home for objects that reference each other.

## Lifetime

The lifetime of an owned reference is the lifetime of the collection that owns it.

Because every value in Fa is destroyed at the end of its scope, a reference cannot outlive its collection by accident — the compiler rejects any code where it would.

## Returning a reference

Returning an owned reference to a collection that is local to the function is impossible: the collection is destroyed on return. Fa copies the value out instead:

```fa
function createPerson = (): Person => {
  mutable persons = [Person {}]
  use person = persons[0] -- type: `Person in persons`
  return person           -- implicitly converted to `Person`: a copy is made
}
```

When the collection comes from the caller, the reference can be returned as it is. Say so in the return type:

```fa
function firstPerson = (persons: Array(Person)): Person in persons => {
  use person = persons[0]
  return person -- no copy: the caller owns `persons`
}
```

And make it optional if the collection may be empty:

```fa
function firstPerson = (persons: Array(Person)): (Person in persons)? => {
  if persons.length == 0 { return none }
  use person = persons[0]
  return person
}
```

The caller receives a reference whose validity is tied to its own collection:

```fa
mutable persons = [Person { name = "Ana" }]

use first = firstPerson(persons)
first.name = "Bao"       -- writes into `persons`

persons.push(Person {})  -- invalidates `first`
console.log(first.name)  -- error: `first` is no longer valid
```

## Moving an element out

Every collection has a `take` method, which removes an element and returns it as a standalone value:

```fa
mutable arrayOfPersons = [Person {}, Person {}, Person {}]
mutable anotherArrayOfPersons = [Person {}]

let someone = arrayOfPersons.take(1)

anotherArrayOfPersons.push(someone)
```

`take` moves rather than copies, and it invalidates references into the source collection just like any other removal.

## Owned references and relations

An owned reference points at *one* element and is invalidated by structural change. A [relation](./relations) is a weak reference that the compiler keeps up to date on both sides, and which survives the removal of its target — the field simply becomes `none`.

Use an owned reference for short-lived work on a stored value. Use a relation when two objects need to know about each other over time.
