
### The `Bag` collection

The `Bag` collection is a special collection optimized to store unordered objects. It's very similar to a `Set`, but without the unicity constraint.

> It's similar to what is also called a "multiset" or an "arena".

```ts
bag = Bag(Human)

bag.add(Human("John", 20))
bag.add(Human("Jane", 21))
bag.add(Human("John", 22))

for bag >> (human) {
  console.log(human.name) // will log "John", "Jane", "John" -- order is not guaranteed
}

bag.delete(1) // delete the element at index 1
```
