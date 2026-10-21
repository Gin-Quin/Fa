# Growing collections

A growing collection only ever accepts new elements. Nothing is removed until the whole collection is destroyed.

Giving up removal buys a strong guarantee: **any reference to an item stays valid for the entire lifetime of the collection**. There is no need to check that an item is still there, even after the collection has grown.

## When to use one

Growing collections suit data that accumulates until it is all discarded at once:

- a chat application storing messages,
- a parser accumulating tokens from an input stream,
- a logger accumulating log lines,
- a compiler's arena of syntax nodes.

## Available variants

Every standard collection has a growing counterpart:

- `Array` → `GrowingArray`
- `Set` → `GrowingSet`
- `Map` → `GrowingMap`
- `Bag` → `GrowingBag`
- `Columnar` → `GrowingColumnar`

The methods are identical, minus the ones that remove: no `delete`, no `pop`, no `clear`, no `retain`.

## Example

```fa
mutable humans = GrowingBag(Human) {}

use john = humans.add(Human { name = "John", age = 30 })
use jane = humans.add(Human { name = "Jane", age = 28 })

-- adding more elements never invalidates existing references
humans.add(Human { name = "Alice", age = 25 })
humans.add(Human { name = "Bob", age = 32 })

john.age += 1
console.log(john.age)  -- 31
console.log(jane.name) -- "Jane"
```

Note the `use`: `add` returns a reference into the collection, and `let` would copy it, leaving the stored value untouched.

## Comparison with the standard collections

```fa
mutable array = Array(Human) {}
use first = array.add(Human { name = "Ana" })
array.add(Human { name = "Bao" })
console.log(first.name) -- error: `first` may have been invalidated by the growth

mutable growing = GrowingArray(Human) {}
use first = growing.add(Human { name = "Ana" })
growing.add(Human { name = "Bao" })
console.log(first.name) -- "Ana", guaranteed valid
```

The compiler enforces this difference; it is part of the type, not a convention.

## Freeing the memory

A growing collection is destroyed like any other value: at the end of its scope, or when the collection that owns it is destroyed. Everything it holds is released in one step, which is what makes it fast.

```fa
function parse = (source: String): SyntaxTree => {
  mutable tokens = GrowingArray(Token) {}
  -- ... fill `tokens`, hold references into it freely ...
  return buildTree(tokens)
} -- every token is released here, in one deallocation
```

To keep the data beyond the scope, return the collection itself rather than references into it.
