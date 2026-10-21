# Extract and insert

Fa has two operators for moving values between a source and a set of names:

- `>>` **extracts** one or more values out of a namespace, collection, object, tuple, iterator or generator.
- `<<` **inserts** one or more values into a collection, object or tuple.

## Extracting

```
<source> >> [<keyword>] <bindings>
```

The bindings are written with the brackets of the source: `{ }` for objects and namespaces, `[ ]` for arrays, `( )` for tuples.

The optional keyword says what to do with the extracted values, and is one of `let`, `mutable` or `use`. Without a keyword, the values are assigned to names that already exist.

```fa
let object = { a = 1, b = 2 }

object >> let { a, b }      -- declare `a` and `b` as immutable copies
object >> mutable { a, b }  -- declare them as mutable copies
object >> use { a, b }      -- declare them as references into `object`
```

Assigning to existing values needs no keyword:

```fa
mutable a = 1
mutable b = 2

object >> { a, b } -- reassigns `a` and `b`
```

The keyword can also be applied per binding:

```fa
mutable a = 1

object >> { a, use b }  -- reassign `a`, and declare `b` as a reference
object >> { a, let b }  -- reassign `a`, and copy `object.b` into a new `b`
```

### Comparison with TypeScript

Destructuring in TypeScript:

```ts
const object = { a: 1, b: 2 }
const { a, b } = object
```

The equivalent in Fa:

```fa
let object = { a = 1, b = 2 }
object >> let { a, b }
```

Extracting is more flexible than destructuring, because the source and the bindings are separate: you choose whether each name is declared or reassigned, and whether it is a copy or a reference.

### Defaults and renaming

```fa
object >> let { a, b = 4 }        -- default value for `b`
object >> let { a, b: renamed }   -- bind `object.b` to `renamed`
```

### Nested extraction

```fa
let config = {
  server = { host = "localhost", port = 8080 }
}

config >> let { server { host, port } }

console.log(host) -- "localhost"
```

## Sources

### Objects

```fa
object >> let { a, b = 4 }
```

### Namespaces

```fa
utils >> use { formatDate, validateEmail }

formatDate(now())
```

See [modules](../architecture/modules).

### Arrays

```fa
array >> let [first, second = 4]
```

### Tuples

```fa
tuple >> let (a, b = 4)
```

### Function parameters

```fa
function someFunction = (SomeObject >> { foo, bar }) => {
  console.log(foo, bar)
}
```

See [destructuring parameters](../functions/defining-functions#destructuring-parameters).

### Pattern matches

```fa
when shape is {
  Rect >> { width, height } => width * height
}
```

### Loops

```fa
for usersById >> (id, name) { }
for people >> { name, age } { }
```

### Types

At the type level, `>>` picks fields, using a [key set](../types/fields):

```fa
type TypeB = {
  ...TypeA >> @{ foo }
  someNumber: Int
}
```

## Inserting

`<<` writes values **into** an existing container, at the positions or keys named.

```fa
array << [a, b]   -- replaces the first two items
object << { a, b }
tuple << (a, b)
```

Inserting is not appending:

```fa
mutable numbers = [1, 2, 3]

numbers << [10, 20]      -- [10, 20, 3]
numbers.push(10, 20)     -- [10, 20, 3, 10, 20]
```

Inserting into an object writes the named fields, leaving the others untouched:

```fa
mutable settings = { host = "localhost", port = 8080, debug = false }

let port = 9090
settings << { port } -- { host = "localhost", port = 9090, debug = false }
```

The target must be mutable, and every name being inserted must already exist in it — `<<` never adds a new field.

## Summary

| Written | Meaning |
| --- | --- |
| `src >> { a }` | reassign the existing `a` |
| `src >> let { a }` | declare `a` as an immutable copy |
| `src >> mutable { a }` | declare `a` as a mutable copy |
| `src >> use { a }` | declare `a` as a reference into `src` |
| `src >> let { a = 1 }` | with a default value |
| `src >> let { a: b }` | copy `src.a` into `b` |
| `dst << { a }` | write `a` into `dst.a` |
