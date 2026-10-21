# Columnars

A `Columnar` stores objects **by column** rather than by row. Where an array of objects lays each object's fields out next to each other, a columnar keeps one contiguous run per field.

In other languages this is called a **struct of arrays**, or a `MultiArrayList` in Zig, and it is a standard **data-oriented design** technique.

Internally, a columnar creates one arena per field of the object type.

## When to use one

A columnar pays off when you repeatedly read a *few* fields across *many* objects, because those fields are contiguous in memory and nothing else is loaded alongside them. Typical uses:

- an in-memory database with fast lookups,
- video games and entity systems,
- analysis over large datasets,
- any operation applied to a subset of a very large number of objects.

```fa
mutable columnar: Columnar(Human) = []

columnar.add({ name = "John", age = 20 })
columnar.add({ name = "Jane", age = 21 })
columnar.add({ name = "John", age = 22 })

-- iterating whole objects works, but is slower than an array,
-- because each object has to be gathered from its columns
for human in columnar {
	console.log(human.name)
}

-- extracting only the fields you need is where a columnar wins
for { name, age } in columnar {
	console.log("{name} is {age}")
}
```

Extracting a single field reads one contiguous run of memory:

```fa
mutable total = 0

for { age } in columnar {
	total += age
}
```

## References

Like a [`Bag`](/collections/bags), a `Columnar` gives stable references to its elements: `add` returns a reference, to bind with `use` (`let` would copy the element).

```fa
use john = columnar.add({ name = "John", age = 30 })

columnar.add({ name = "Jane", age = 28 }) -- does not move `john`

john.age += 1
```

Unlike a bag, a reference into a columnar points at fields spread across several arenas. This is invisible in your code, but means copying an element out of a columnar costs a gather.

`delete` takes a reference and declares the `invalidates item` effect: it only invalidates the reference to the deleted element. `clear` invalidates every reference (`invalidates self`).

```fa
columnar.delete(john)
console.log(john.name) -- error: `john` has been invalidated by `columnar.delete`
```

Because its references are stable, a `Columnar` is one of the collections that [relations](/advanced/relations) can target.

## Ordered columnars

`OrderedColumnar` has the same interface, but stores each column in an `Array` rather than an arena. Use it when iteration order matters.

```fa
mutable orderedColumnar: OrderedColumnar(Human) = []

orderedColumnar.add({ name = "John", age = 20 })
orderedColumnar.add({ name = "Jane", age = 21 })
orderedColumnar.add({ name = "John", age = 22 })

orderedColumnar.delete(1) -- deletes the element at index 1
```

The trade-off is deletion: removing an element has to shift the rest of every column, which is expensive on a large collection. A plain `Columnar` removes in constant time but does not preserve order.

Because its columns are arrays, an `OrderedColumnar` behaves like an [`Array`](/collections/arrays) for references: adding or deleting an element may move the others, so these operations declare `invalidates self`.

## Choosing between an array and a columnar

| | `Array(Human)` | `Columnar(Human)` |
| --- | --- | --- |
| Read whole objects | fast | slower |
| Read one field over many objects | slower | fast |
| Append | fast | fast |
| Remove | shifts the array | constant time |
| References survive growth | no | yes |

Start with an array. Move to a columnar when you have measured a loop that touches a small part of a large number of objects.

## Resources

- [Andrew Kelley: Practical Data Oriented Design](https://www.youtube.com/watch?v=IroPQ150F6c), by the creator of Zig, explaining the idea and its performance impact.
