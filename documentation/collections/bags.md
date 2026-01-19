
# Bags

The `Bag` collection is optimized for storing unordered objects. It's similar
to a `Set`, but without the uniqueness constraint.

> It's similar to what's also called a "multiset" or an "arena".

Internally, a `Bag` uses an arena for fast insertion and stable references.

```fa
bag = Bag(Human)

bag.add(Human("John", 20))
bag.add(Human("Jane", 21))
bag.add(Human("John", 22))

for bag >> (human) {
  console.log(human.name) // will log "John", "Jane", "John" -- order is not guaranteed
}

bag.delete(1) // delete the element at index 1
```

## When to use a bag

Use a `Bag` when you want:

- fast insertion
- stable references to stored objects
- no uniqueness checks
