# Enumerations

Enumerations are a set of named integers that can be used to represent a value among this set.

```rs
enum MyEnum = { foo, bar, baz }

let myEnum = MyEnum.foo
let myEnum: MyEnum = .foo
```

If you need to associate enumerations with strings, you should use **string unions** instead.

Enumerations are an expressive way to represent an **union of integers**.
