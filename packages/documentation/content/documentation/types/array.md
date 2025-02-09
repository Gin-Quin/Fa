# Array

An `Array` is list of elements that all have the same `type`.

`Arrays` are defined this way :

```ts
// inline notation with subtype inference
integers = [5, 12, 13]  // inferred type : Array<Integer>
numbers = [5, 3.14]  // inferred type : Array<Number>

// multiline notation
fruits = [  // inferred type : Array<String>
   "banana" // no comma needed
   "strawberry"
   "apple"
]

// with explicit type (full notation)
integers: Array(Integer) = [5, 12, 13]
numbers = Array(Number) [5, 3.14]
fruits = Array(String) [
   "banana"
   "strawberry"
   "apple"
]
```
