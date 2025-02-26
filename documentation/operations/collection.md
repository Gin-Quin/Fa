# Insert and extract

Fa introduces two operators to easily insert and extract values from namespaces, arrays, objects and tuples.

The `>>` extract operator is used to extract one or more values from a namespace, array, object or tuple.

The `<<` insert operator is used to insert one or more values into an array, object or tuple.

## Namespace

Extract values from a namespace:

```fa
use namespace >> { a, b }
```

## Array

Extract values from an array:

```fa
use array >> [a, b = 4]
```

Insert values into an array:

```fa
use array << [a, b] // this replace the first two values in the array
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
