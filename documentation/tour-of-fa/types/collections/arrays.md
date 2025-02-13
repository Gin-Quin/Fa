
# Arrays

Arrays are the most common container to store a list of values. They can be mutable or immutable.

```ts
myArray = [1, 2, 3] // immutable array, you cannot modify, add or remove elements
myArray: Array(Integer) = [1, 2, 3]
myArray = Array(Integer) [1, 2, 3]

myMutableArray = mutable [1, 2, 3] // mutable array, you can modify, add and remove elements
myMutableArray: mutable Array(Integer) = [1, 2, 3]
myMutableArray = mutable Array(Integer) [1, 2, 3]
```

## Vectors

Vectors are arrays with a fixed length.

```ts
myFixedArray = Vector(4, Integer) [1, 2, 3, 4]

type Matrix4x4 = Vector(4, Vector(4, Number))

myMatrix = mutable Matrix4x4 [
  [1, 2, 3, 4],
  [1, 2, 3, 4],
  [1, 2, 3, 4],
  [1, 2, 3, 4],
]
```

## Tuples

Unlike other languages, Fa does not have tuples. Instead, you should use objects with the constructor notation.

For example, in Rust:

```rs
struct MyTuple(f32, String, i32);

let myTuple = MyTuple(1.0, "Hello", 3);
```

In Fa, you should declare a type object and use the constructor notation:

```ts
type MyTuple = {
  x: Number
  y: String
  z: Integer
}

myTuple = MyTuple(1.0, "Hello", 3)
```
