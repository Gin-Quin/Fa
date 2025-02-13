Unions are extremely powerful and I can't imagine Fa without unions.

Unions are created with the syntax '|':

```coffee
# inline syntax
type Animal = Dog | Cat
type Animal = { name: "cat" } | { name: "dog" }

# multiline syntax
type Animal =
  | Dog
  | Cat

type Animal =
  | { name: "cat", speed: Number }
  | { name: "dog", speed: Number }

type Animal =
  | name: "cat"
    speed: Number
  | name: "dog"
    speed: Number
# <-- for this we use the same syntax as "-"

type Animal =
  ...Dog | Cat

type Animal =
  strength: Number
  ...
    | Dog
    | Cat

# Ts Equivalent would be
type Animal = 
  & {
    strength: number
  }
  & (
    | Dog
    | Cat
  )
```
