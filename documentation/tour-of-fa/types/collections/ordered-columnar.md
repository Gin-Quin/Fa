# Ordered columnars

The `OrderedColumnar` is a variant of the `Columnar` with the same interface. The only difference is that it uses an `Array` to store the columns, instead of an `Arena` for the `Columnar` type.

This is useful when you want to loop over the collection in a specific order. The only drawback compared to the `Columnar` is that removing elements from the collection is an expansive operation when it contains a lot of objects.

```ts
orderedColumnar = OrderedColumnar(Human)

orderedColumnar.add(Human(name = "John", age = 20))
orderedColumnar.add(Human(name = "Jane", age = 21))
orderedColumnar.add(Human(name = "John", age = 22))

orderedColumnar.delete(1) // beware of the cost of deletion when the collection contains a lot of objects
```
