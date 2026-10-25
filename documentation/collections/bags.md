# Bags

A `Bag` stores unordered values with no uniqueness constraint. It is the collection to reach for when you need many objects to live somewhere and stay referenceable.

> Other languages call this a *multiset* or an *arena*.

Internally a bag is an arena: insertion is fast, and stored objects keep a **stable address** even as the bag grows.

```fa
mutable bag: Bag(Human) = []

use john = bag.add({ name = "John", age = 20 })
use jane = bag.add({ name = "Jane", age = 21 })
bag.add({ name = "John", age = 22 })

for human in bag {
	console.log(human.name) -- "John", "Jane", "John", in no guaranteed order
}
```

`add` returns a **reference** to the stored value. Bind it with `use` to reach the object later: `let` would copy the value, and modifying the copy would leave the bag untouched.

```fa
john.age += 1 -- modifies the Human stored in the bag
```

## Removing

Remove a value with `delete`, passing a reference to it:

```fa
bag.delete(john)

console.log(john.name) -- error: `john` was invalidated by `bag.delete`: declare it again with `use john = ...`
console.log(jane.name) -- fine: `jane` is another element
```

Deleting a value doesn't move the others: `delete(element)` declares `invalidates element`, which invalidates the references that may designate the deleted value, and only them. The compiler knows that `john` and `jane` are different elements, because they were returned by two different `add` calls.

When the compiler can't prove that a reference designates another element, it is invalidated too, and must be declared again before it is used:

```fa
use someone = bag.find((human) => human.age > 30) -- type: `(Human in bag)?`

if someone is not none {
	bag.delete(someone)
	console.log(jane.name) -- error: `jane` was invalidated by `bag.delete`: `someone` may be `jane`
}

use jane = bag.find((human) => human.name == "Jane") -- declared again
```

References stored in fields are not affected: they are [relations](/advanced/relations), cleaned up when their target is deleted.

Removing values during a loop over the bag is a compile error, because it would invalidate the iteration. Use `deleteWhere` instead:

```fa
bag.deleteWhere((human) => human.name == "John")
```

## Stable references

This is what distinguishes a bag from a set or an array:

```fa
mutable bag: Bag(Human) = []

use john = bag.add({ name = "John", age = 20 })

-- adding more elements does not move the existing ones
bag.add({ name = "Alice", age = 25 })
bag.add({ name = "Bob", age = 32 })

john.age += 1
console.log(john.age) -- 21: `john` is still valid
```

Compare with an array, where `push` may reallocate the storage and invalidate every reference into it.

| Operation | Invalidates |
| --- | --- |
| `add` | nothing |
| `delete(element)`, `take(element)` | the references that may designate `element` (`invalidates element`) |
| `deleteWhere`, `clear` | every local reference into the bag (`invalidates self`) |

For references that stay valid for the whole lifetime of the collection, use a [`GrowingBag`](/collections/growing-collections).

## When to use a bag

Use a `Bag` when you want:

- fast insertion,
- stable references to stored objects,
- no uniqueness checks,
- no meaningful order.

Bags are the natural home for the objects of a graph, a scene, or an entity system: anything where objects refer to each other. Because its elements never move, a bag is also a collection that can hold the objects linked by [relations](/advanced/relations).

## Choosing a collection

| Need | Collection |
| --- | --- |
| Ordered list, indexed access | [`Array`](/collections/arrays) |
| Unique values, fast membership | [`Set`](/collections/sets) |
| Key to value | [`Map`](/collections/maps) |
| Many objects, stable references | `Bag` |
| Iterate one field over many objects | [`Columnar`](/collections/columnars) |
| Any of the above, never removing | [growing collections](/collections/growing-collections) |
