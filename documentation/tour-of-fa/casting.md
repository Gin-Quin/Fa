# Casting

There are two ways to cast the the type of a variable : by downcasting or upcasting.

Let's take our good old example of Charlie. Charlie is a Dog and therefore a Pet.

```coffee
let charlie = Dog()
```

And then we have our type `Human` who expects a field `pet` :

```coffee
type Human
  pet: Pet
```

But what if we want Charlie to be the default pet of all new humans? Charlie is a Dog, not a Pet, so we need to downcast him (poor Charlie).

```coffee
type Human
  pet = charlie as Pet
  pet: Pet = charlie
```

Upcasting - casting a Pet as a Dog - is not possible.
