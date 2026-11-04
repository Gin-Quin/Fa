# Columnars

A `Columnar` stores objects **by column** rather than by row. Where an array of objects lays each object's fields out next to each other, a columnar keeps one contiguous run per field.

In other languages this is called a **struct of arrays**, or a `MultiArrayList` in Zig, and it is a standard **data-oriented design** technique.

Internally, a columnar keeps one column per field of the object type, split into pages like a [`Pool`](/collections/pools) (see [how it works](#how-it-works)).

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

Like a [`Pool`](/collections/pools), a `Columnar` gives stable references to its elements: `add` returns a reference, to bind with `use` (`let` would copy the element).

```fa
use john = columnar.add({ name = "John", age = 30 })

columnar.add({ name = "Jane", age = 28 }) -- does not move `john`

john.age += 1
```

Unlike a pool, a reference into a columnar points at fields spread across several columns. This is invisible in your code, but means copying an element out of a columnar costs a gather.

`delete` takes a reference, and declares `invalidates element`: like in a [pool](/collections/pools#removing), it invalidates the references that may designate the deleted element, and only them. `deleteWhere` and `clear` declare `mutates self`: every local reference into the columnar must be declared again before it is used.

| Operation | Effect |
| --- | --- |
| `add` | `mutates self.size`: returns a reference, distinct from every existing one |
| `delete(element)`, `take(element)` | `invalidates element` |
| `deleteWhere`, `clear` | `mutates self` |
| `reserve(additional)` | `mutates self.capacity`: it moves no element |

Like every collection, a columnar takes a `capacity` in `from`, and has a read-only `capacity` property and a `reserve(additional)` method (see [capacity](/collections/arrays#capacity)).

```fa
use jane = columnar.add({ name = "Jane", age = 28 })

columnar.delete(john)
console.log(john.name) -- error: `john` was invalidated by `columnar.delete`: declare it again with `use john = ...`
console.log(jane.name) -- fine: `jane` is another element
```

Because its elements never move, a `Columnar` is one of the collections that can hold the objects linked by [relations](/advanced/relations).

`columnar.size` is the number of elements. Like a pool, a columnar has a `size` rather than a `length`, because its elements have no positions.

## Ordered columnars

`OrderedColumnar` stores each column in an `Array` rather than an arena, and keeps its elements in insertion order. Use it when iteration order matters. Its elements have positions, so its interface is the one of an [`Array`](/collections/arrays), with `add` to append: a `length`, indices, `delete(index)` and `take(index)` instead of a reference, `pop`, and the index in loops (`for human, index in orderedColumnar`) and callbacks.

```fa
mutable orderedColumnar: OrderedColumnar(Human) = []

orderedColumnar.add({ name = "John", age = 20 })
orderedColumnar.add({ name = "Jane", age = 21 })
orderedColumnar.add({ name = "John", age = 22 })

orderedColumnar.delete(1) -- deletes the element at index 1
```

The trade-off is deletion: removing an element has to shift the rest of every column, which is expensive on a large collection. A plain `Columnar` removes in constant time but does not preserve order.

Because its columns are arrays, an `OrderedColumnar` behaves like an `Array` for references: `add` may reallocate the columns, so it declares `mutates self`, and `delete(index)` shifts the elements after `index`, so it declares `invalidates self[index..]`. For the same reason, it can't hold the objects linked by relations, and it has no growing counterpart.

| Operation | Effect |
| --- | --- |
| `add` | `mutates self`: returns a reference to the new element |
| `columnar[index] = value` | `mutates self[index]` |
| `delete(index)`, `take(index)` | `invalidates self[index..]` |
| `pop` | `invalidates self[self.length - 1]` |
| `deleteWhere`, `clear`, `reserve` | `mutates self` |

## How it works

A columnar is a [pool](/collections/pools#how-it-works) whose pages are split by field. A page holds a fixed number of slots, and one contiguous run per field: the `age` of every element of the page, then the `name` of every element, and so on. An element has the same slot number in every column, and a reference to an element is that slot number.

- `add` takes a free slot, from the first page that has one, or appends after the last slot. It writes each field into its column.
- `delete` marks the slot as free in the page's **occupancy bitmap**, shared by all the columns. Nothing moves.
- `for { age } in columnar` reads only the `age` runs, page after page, and skips the free slots with the bitmap. This is the loop the compiler vectorizes.
- `for human in columnar` reads every column at the same slot to rebuild each element: this is the *gather* that makes reading whole objects slower than with an array.

An `OrderedColumnar` keeps one [`Array`](/collections/arrays#how-it-works) per field instead: dense columns with no holes, in insertion order, but adding may reallocate them and deleting shifts them.

A [`GrowingColumnar`](/collections/growing-collections) has no free slots and no bitmap: its columns are always dense.

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

:::implementation
## Implementation details

### Native

- **Columns.** There is one column per field of the element type, with the field's own layout: a `Columnar(Human)` with `name: String` and `age: Integer` has a column of string headers and a column of `i64`. Nested objects are not split further in the first version: a field holding an object is one column of that object type.
- **Chunks.** All columns share the same chunk capacity `2^k` slots, chosen per type so that the widest column fills about 64 KiB. Slot `i` is in chunk `i >> k` at offset `i & (2^k - 1)` in every column. A chunk is one allocation holding the runs of every column one after the other, each run aligned to 64 bytes, preceded by a header with the occupancy bitmap, the live count, and the head of the chunk's free list. The columnar keeps an array of chunk pointers: a slot index is a shift, a mask and one load away from its fields.
- **Free slots, deletion and iteration** follow the pool: a bitset of the chunks with a free slot, lowest chunk first; a per-chunk free list stored in the dead slot of the first column (so the first column is at least 4 bytes wide, padded if needed); `tzcnt` iteration over the occupancy words; empty chunks freed except one spare; no compaction.
- **References.** A reference is the slot index (`u32`), not a pointer: the type of the reference names the columnar, so the base is known. Reading `john.age` is `chunks[i >> k].age[i & mask]`.
- **Single-field loops.** `for { age } in columnar` lowers to a loop over the chunks, and inside a chunk to a loop over the `age` run with the occupancy word as a mask: LLVM vectorizes it, and a full word needs no mask at all.
- **OrderedColumnar.** One `Array` per field, all with the same length and capacity, grown together. A reference is the index.

### JavaScript

- One array per field, all with the same capacity. Numeric fields use typed arrays: `Float64Array` for `Float` and `Integer` (JavaScript numbers), `Int32Array` or `Uint8Array` when the integer's range fits, `Uint8Array` for `Boolean`. Other fields use a plain `Array`.
- Typed arrays have a fixed length, so growing reallocates every column to twice the capacity and copies them with `set`. This is invisible to the program: references are slot indices, not objects.
- The occupancy is a `Uint32Array` bitmap, and the free slots are an `Int32Array` used as a stack. `add` pops a free slot or appends, `delete` clears the bit and pushes the slot.
- `for { age } in columnar` compiles to a loop over the slot indices that tests the bitmap and reads `ages[i]`. `for human in columnar` builds a new object per element.
- `OrderedColumnar` has the same columns, without the bitmap and the free list; `delete(index)` shifts each column with `copyWithin`.
:::
