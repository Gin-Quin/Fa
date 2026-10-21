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

They are created like any other collection, from a list literal typed by the context, or with `from` (see [creating collections](/collections/arrays#creating-collections)).

The methods are identical, minus the ones that remove or move elements: no `delete`, no `deleteWhere`, no `pop`, no `clear`, and for `GrowingArray` no `insert`, `sortInPlace` or `reverseInPlace`. None of the remaining methods declares the `invalidates` effect.

## Example

```fa
mutable humans: GrowingBag(Human) = []

use john = humans.add({ name = "John", age = 30 })
use jane = humans.add({ name = "Jane", age = 28 })

-- adding more elements never invalidates existing references
humans.add({ name = "Alice", age = 25 })
humans.add({ name = "Bob", age = 32 })

john.age += 1
console.log(john.age)  -- 31
console.log(jane.name) -- "Jane"
```

Note the `use`: `add` returns a reference into the collection, and `let` would copy it, leaving the stored value untouched.

## Comparison with the standard collections

```fa
mutable array: Array(Human) = [{ name = "Ana", age = 30 }]
use first = array[0]
array.push({ name = "Bao", age = 25 })
console.log(first.name) -- error: `first` may have been invalidated by `array.push`

mutable growing: GrowingArray(Human) = [{ name = "Ana", age = 30 }]
use firstGrowing = growing[0]
growing.push({ name = "Bao", age = 25 })
console.log(firstGrowing.name) -- "Ana", guaranteed valid
```

`Array.push` declares `invalidates self`; `GrowingArray.push` does not. The compiler enforces this difference: it is part of the type, not a convention.

## Freeing the memory

A growing collection is destroyed like any other value: at the end of its scope, or when the value that owns it is destroyed. Everything it holds is released in one step, which is what makes it fast.

```fa
function parse = (source: String): SyntaxTree => {
	mutable tokens: GrowingArray(Token) = []
	-- ... fill `tokens`, hold references into it freely ...
	return buildTree(tokens)
} -- every token is released here, in one deallocation
```

To keep the data beyond the scope, return the collection itself rather than references into it.
