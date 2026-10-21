# Bags

A `Bag` stores unordered values with no uniqueness constraint. It is the collection to reach for when you need many objects to live somewhere and stay referenceable.

> Other languages call this a *multiset* or an *arena*.

Internally a bag is an arena: insertion is fast, and stored objects keep a **stable address** even as the bag grows.

```fa
mutable bag = Bag(Human) {}

let john = bag.add(Human { name = "John", age = 20 })
let jane = bag.add(Human { name = "Jane", age = 21 })
bag.add(Human { name = "John", age = 22 })

for bag >> human {
  console.log(human.name) -- "John", "Jane", "John" — in no guaranteed order
}
```

`add` returns a **handle** to the stored value. Use it to reach the object later:

```fa
use someone = bag[john]
someone.age += 1
```

## Removing

Remove by handle:

```fa
bag.delete(john)
```

Removing during iteration is not allowed, because it would invalidate the iteration. Collect the handles first, or use `retain`:

```fa
bag.retain((human: Human) => human.name is not "John")
```

## Stable references

This is what distinguishes a bag from a set or an array:

```fa
mutable bag = Bag(Human) {}

use john = bag.add(Human { name = "John", age = 20 })

-- adding more elements does not move the existing ones
bag.add(Human { name = "Alice", age = 25 })
bag.add(Human { name = "Bob", age = 32 })

john.age += 1
console.log(john.age) -- 21, still valid
```

Compare with an array, where `push` may reallocate and invalidate every reference into it.

References into a bag are still invalidated by `delete` and `clear`. For references that stay valid for the whole lifetime of the collection, use a [`GrowingBag`](./growing-collections).

## When to use a bag

Use a `Bag` when you want:

- fast insertion,
- stable references to stored objects,
- no uniqueness checks,
- no meaningful order.

Bags are the natural home for the objects of a graph, a scene, or an entity system — anything where objects refer to each other. Because references into a bag are stable, a bag is also the collection that [relations](../advanced/relations) are declared over.

## Choosing a collection

| Need | Collection |
| --- | --- |
| Ordered list, indexed access | [`Array`](./arrays) |
| Unique values, fast membership | [`Set`](./sets) |
| Key to value | [`Map`](./maps) |
| Many objects, stable references | `Bag` |
| Iterate one field over many objects | [`Columnar`](./columnars) |
| Any of the above, never removing | [growing collections](./growing-collections) |
