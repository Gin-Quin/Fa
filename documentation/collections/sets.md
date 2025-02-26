# Sets

Sets are collections that store unordered unique values. They can be mutable or immutable.

Unicity is checked by equality of the values.

```ts
setOfIntegers = Set(Integer) { 1, 2, 2, 3 }

console.log(setOfIntegers.size()) // 3 because 2 is only added once

setOfHumans = Set(Human) {
  Human(name = "John", age = 20)
  Human(name = "Jane", age = 21)
  Human(name = "John", age = 22)
}

anotherJohn = Human(name = "John", age = 20)
setOfHumans.add(anotherJohn) // will not be added because there is already a John
```

## Custom comparison function

You can provide a custom comparison function to the set. This is useful when you want to store objects that have unique fields (like an id):

```ts
type User = {
  id: Integer
  email: String
  name: String
}

setOfUsers = Set(User, compare = (a, b) => a.id == b.id) {
  User(id = 1, email = "john@doe.com", name = "John Doe")
  User(id = 2, email = "jane@doe.com", name = "Jane Doe")
}

setOfUsers.add(User(id = 1, email = "foo@bar.com", name = "Foo Bar")) // will not be added because there is already a User with id 1
```

## Key sets

You can create a set containing a fixed number of values:

```ts
myKeySet = Set(Union { hello, "world", "!" }) { hello, "!" }
```

This is useful when you want to get some keys from an object:

```ts
logProperties = (object: Object, @keys: Set(DeepKeys(object))) {
  @for @keys as @key {
    log("{@key} = {object[@key]}")
  }
}

myObject = {
  foo = 12
  bar = 13
  baz = 14
  nested = {
    foo = 15
    bar = 16
  }
}

logProperties(myObject, { foo, baz, nested { foo } })
logProperties(myObject, "foo" | "baz" | "nested.foo")
```
