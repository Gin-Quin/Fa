
# Sets

Sets are containers that store unique values. They can be mutable or immutable.

```ts
mySet = Set(Integer) { 1, 2, 3 }
```

## Key sets

You can create a set containing a fixed number of values:

```ts
myKeySet = Set("hello" | "world" | "!") { hello, "!" }
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
