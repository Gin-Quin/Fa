# Enumerations

Enumerations are a set of named integers that can be used to represent a value among this set.

```rs
type MyEnumeration = Enumeration { foo, bar, baz }

myEnumeration = MyEnumeration::foo
myEnumeration: MyEnumeration = 'foo' // single-quote notation
```

If you need to associate enumerations with strings, you should use **string unions** instead.

Enumerations are an expressive way to represent an **union of integers**.
