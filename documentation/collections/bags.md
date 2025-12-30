
### The `Bag` collection

The `Bag` collection is a special collection optimized for storing unordered objects. It's very similar to a `Set`, but without the uniqueness constraint.

> It's similar to what's also called a "multiset" or an "arena".

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
