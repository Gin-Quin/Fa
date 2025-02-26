# Columnars

A `Columnar` is a special collection that is optimized to loop over specific fields of a group of objects. Unlike a classical array of objects, a `Columnar` store values in a column-wise manner. In other languages, this is called a **"struct of arrays"** (or a **"MultiArrayList"** in Zig), and is a common **Data Oriented Design** technique.

Internally, it creates one arena for each field of the object type.

This is especially useful when you want to find indexes of objects in a collection. Columnars are often used in the following cases:

- Creating an in-memory database with fast lookups.
- Developping video games.
- More generally, when you want to execute actions on a subset of a very large number of objects.

```ts
columnar = Columnar(Human)

columnar.add(Human(name = "John", age = 20))
columnar.add(Human(name = "Jane", age = 21))
columnar.add(Human(name = "John", age = 22))

// you can iterate over the whole collection like an array
// but this is going to be slower than using a regular array or arena
for columnar >> (human) {
  console.log(human.name) // will log "John", "Jane", "John" -- order is not guaranteed
}

// if you can, try to capture only the fields you need
for columnar >> ({ name }) {
  console.log(name) // will log "John", "Jane", "John" -- order is not guaranteed
}

// the real power of columnars comes when you need to loop over one specific field
// then it's going to be much faster than using a regular array or arena
for columnar.name >> (name) {
  console.log(name) // will log "John", "Jane", "John" -- order is not guaranteed
}

// you can also capture other fields of the object using the second parameter
for columnar.name >> (name, { age }, index) {
  console.log(index) // will log 0, 1, 2
  console.log(name) // will log "John", "Jane", "John" -- order is not guaranteed
  console.log(age) // will log 20, 21, 22
}

// you can capture the whole object when not destructuring them, but it's going to be slower
for columnar >> (name, human) {
  console.log(index) // will log 0, 1, 2
  console.log(name) // will log "John", "Jane", "John" -- order is not guaranteed
  console.log(human)
}
```

## Resources

- [ Andrew Kelley Practical Data Oriented Design (DoD) ](https://www.youtube.com/watch?v=IroPQ150F6c) - A great video by the creator of Zig explaining the concept of Data Oriented Design and how it can greatly improve performance.

## Ordered columnars

The `OrderedColumnar` is a variant of the `Columnar` with the same interface. The only difference is that it uses an `Array` to store the columns, instead of an `Arena` for the `Columnar` type.

This is useful when you want to loop over the collection in a specific order. The only drawback compared to the `Columnar` is that removing elements from the collection is an expansive operation when it contains a lot of objects.

```ts
orderedColumnar = OrderedColumnar(Human)

orderedColumnar.add(Human(name = "John", age = 20))
orderedColumnar.add(Human(name = "Jane", age = 21))
orderedColumnar.add(Human(name = "John", age = 22))

orderedColumnar.delete(1) // beware of the cost of deletion when the collection contains a lot of objects
```
