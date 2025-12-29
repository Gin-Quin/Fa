# Extract and Insert operations

Fa introduces two operators to easily transfer values from and to one namespace, array, object or tuple.

The `>>` extract operator is used to extract one or more values from a namespace, collection, object, tuple, iterator or generator.

The `<<` insert operator is used to insert one or more values into a collection, object or tuple.

## Comparison vs Typescript

The `extract` operation is quite similar to Typescript's destructuring assignment.

Destructuring in Typescript:

```ts
const object = { a: 1, b: 2 }
const { a, b } = object;
```

Equivalent in Fa:

```fa
let object = { a = 1, b = 2 }
object >> use { a, b }
```

"Extracting" in Fa is more flexible than Typescript's destructuring assignment. For example, you can assign any extracted value to already existing variables:

```fa
mutable a = 1
mutable b = 2
object >> { a, b } -- this will reassign a and b
```

```fa
mutable a = 1
object >> { a, use b } -- this will reassign a and define `b` as new alias of `object.b`
```

```fa
mutable a = 1
object >> { a, let b } -- this will reassign a and copy the value of `object.b` into `b`
```

## Namespace

Extract values from a namespace:

```fa
namespace >> use { a, b }
```

## Collections

Extract values from an array:

```fa
array >> use [a, b = 4] -- set a default value for b
```


Insert values into an array:

```fa
array << [a, b] -- this replace the first two values in the array

-- this is not the same as pushing new values!

array.push(a, b) -- this pushes two values at the end of the array
```

## Object

Extract values from an object:

```fa
use object >> { a, b = 4 }
```

Insert values into an object:

```fa
object << { a, b }
```

## Tuple

Extract values from a tuple:

```fa
use tuple >> (a, b = 4)
```

Insert values into a tuple:

```fa
tuple << (a, b)
```
