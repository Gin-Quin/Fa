# Multiple inheritance

In Fa, multiple inheritance is allowed.

It is not as problematic as in C++ because the resulting object is *flat*. It means there is no prototype chain - not even a __proto__ object.

Every property and function of an object is inside the object.



Different types for the same variable name is not allowed. For example:

```coffee
type A =
  value: string
  getValue() => value

type B =
  value: integer

type C =
  ...A
  ...B

let c = C()
c.getValue() # it's supposed to be a string but `value` is overwritten by B
```

# Composition

Should it be possible to reuse some parts of a type?

Like this:

```coffee
type A =
  value: string
  getValue() => value

type B =
  ...A { value }
```

It is very cool, but quite complex with methods:

```coffee
type A =
  value: string
  getValue() => value

type B =
  # should it be allowed?
  # because getValue without value makes no sense
  ...A { getValue }
```

The question can also be asked for regular destructuration:

```coffee
let { getValue } = A
let A >> { getValue }
getValue() # will not work
```

Destructuring a method would work only when the function is bound. In that case, we expect `getValue()` to be bound with `A`.

In the previous case, we expect `getValue` to be bound with B... but it's not possible if B does not have the `value` field.

## Conclusion

- Destructuring an object into a variable binds the variable to the object it comes from.
- Destructuring a type into another type **is not allowed**. The use case is rare, and there can be situations where that does not make sense.
