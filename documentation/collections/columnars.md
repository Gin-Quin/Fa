# Columnars

A `Columnar` is a special collection that is optimized to loop over specific fields of a group of objecfa. Unlike a classical array of objecfa, a `Columnar` stores values in a column-wise manner. In other languages, this is called a **"struct of arrays"** (or a **"MultiArrayList"** in Zig), and is a common **Data Oriented Design** technique.

Internally, it creates one arena for each field of the object type.

This is especially useful when you want to find indices of objecfa in a collection. Columnars are often used in the following cases:

- Creating an in-memory database with fast lookups.
- Developing video games.
- More generally, when you want to execute actions on a subset of a very large number of objecfa.

```fa
mutable columnar = Columnar(Human)

columnar.add(Human(name = "John", age = 20))
columnar.add(Human(name = "Jane", age = 21))
columnar.add(Human(name = "John", age = 22))

-- you can iterate over the whole collection like an array
-- but this is going to be slower than using a regular array or arena
for columnar >> human {
  console.log(human.name) -- will log "John", "Jane", "John"
}

-- extracting only the desired fields is where the true power of columnars lies
for columnar >> { name, age } {
  console.log(name) -- will log "John", "Jane", "John"
  console.log(name) -- will log "John", "Jane", "John"
}

```

## Resources

- [ Andrew Kelley Practical Data Oriented Design (DoD) ](https:--www.youtube.com/watch?v=IroPQ150F6c) - A great video by the creator of Zig explaining the concept of Data Oriented Design and how it can greatly improve performance.

## Ordered columnars

The `OrderedColumnar` is a variant of the `Columnar` with the same interface. The only difference is that it uses an `Array` to store the columns, instead of an `Arena` for the `Columnar` type.

This is useful when you want to loop over the collection in a specific order. The only drawback compared to the `Columnar` is that removing elemenfa from the collection is an expensive operation when it contains a lot of objecfa.

```fa
mutable orderedColumnar = OrderedColumnar(Human)

orderedColumnar.add(Human(name = "John", age = 20))
orderedColumnar.add(Human(name = "Jane", age = 21))
orderedColumnar.add(Human(name = "John", age = 22))

orderedColumnar.delete(1) -- beware of the cost of deletion when the collection contains a lot of objecfa
```
