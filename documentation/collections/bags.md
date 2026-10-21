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

console.log(john.name) -- error: `john` has been invalidated by `bag.delete`
```

`delete` declares the `invalidates item` effect: it only invalidates the reference to the deleted value. Every other reference into the bag (`jane` here) stays valid.

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
| `delete(item)` | the reference to `item` only (`invalidates item`) |
| `deleteWhere` | the references to the deleted values only |
| `clear` | every reference into the bag (`invalidates self`) |

For references that stay valid for the whole lifetime of the collection, use a [`GrowingBag`](/collections/growing-collections).

## When to use a bag

Use a `Bag` when you want:

- fast insertion,
- stable references to stored objects,
- no uniqueness checks,
- no meaningful order.

Bags are the natural home for the objects of a graph, a scene, or an entity system: anything where objects refer to each other. Because references into a bag are stable, a bag is also a collection that [relations](/advanced/relations) can target.

## Choosing a collection

| Need | Collection |
| --- | --- |
| Ordered list, indexed access | [`Array`](/collections/arrays) |
| Unique values, fast membership | [`Set`](/collections/sets) |
| Key to value | [`Map`](/collections/maps) |
| Many objects, stable references | `Bag` |
| Iterate one field over many objects | [`Columnar`](/collections/columnars) |
| Any of the above, never removing | [growing collections](/collections/growing-collections) |
