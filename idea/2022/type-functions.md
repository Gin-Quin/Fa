Type functions are compile-time functions that transform types into others.

They follow the same syntax as regular functions, except that they manipulate types instead of values.

Unlike regular functions, they start with an uppercase letter.

Like this:

```coffee
functype Maybe(Type: Any) -> return Type | Null
functype Maybe(Type: Any) => Type | Null

let value1: Maybe(Number) = 351
let value2: Maybe(Number) = null
```


This is a huge improvement over Typescript that only allows type expressions, not truly functions.
