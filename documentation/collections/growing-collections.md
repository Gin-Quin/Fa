# Growing-only collections

Growing-only collections are specialized data structures in Fa that only permit the addition of elements, never their removal.

The main advantage of growing-only collections is that any **owned reference** to an item in the collection remains valid for the lifetime of the collection. There is no need to check if the item is still within the bounds of the collection, even after the collection is modified.

## When to use growing collections

Growing collections are ideal for scenarios where the data in a collection **only accumulates over time until the collection is disposed of all at once**.

Examples of situations that are best suited for growing collections include:

- A chat application that stores messages,
- A parser that accumulates tokens from an input stream,
- A logger that accumulates log messages.

## Available growing collections

Each standard collection has a growing-only variant:

- `Array` -> `GrowingArray`
- `Set` -> `GrowingSet`
- `Map` -> `GrowingMap`
- `Bag` -> `GrowingBag`
- `Columnar` -> `GrowingColumnar`

The methods available in a growing-only variant are identical to those in its standard collection counterpart, except that methods for removing elements are not available.

### Example: GrowingBag

`GrowingBag` is an unordered collection that only grows. It is optimized for fast insertion and stable references.

```fa
// Create a growing bag of humans
humans = GrowingBag(Human)

// Add elements
john = humans.add(Human("John", 30))
jane = humans.add(Human("Jane", 28))

// Later in your code, these references remain valid
john.age += 1
console.log(john.age) // 31

// Even after adding more elements
humans.add(Human("Alice", 25))
humans.add(Human("Bob", 32))

// The original references are still valid
console.log(jane.name) // "Jane"
```
