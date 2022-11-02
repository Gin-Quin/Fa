What if a type can also be described by a function?

Like this:

```coffee
type Hero =
  name: string
  health = 100
  strength = 10
  
  attack(target: Hero) ->
    target.health -= strength

let heracles = Hero(name = "Heracles", strength = 12)
```


```coffee
let Hero() =>
  name: string
  health = 100
  strength = 10
  
  attack(target: Hero) ->
    target.health -= strength

let heracles = Hero(name = "Heracles", strength = 12)
```

What I dislike with the function as a type is that you have to create your own constructor.

It means possibly you have to repeat all properties in the constructor, like in TS... which is very bad.

But the advantage of function is that you can have an init field. You can call stuff whenever a new instance is created, things like that. You cannot do that with a regular type.

A proposal to deal with that issue is to use `let` declarations:

```coffee
type Hero =
  name: string
  health = initialHealth
  strength = 10

  let initialHealth = 120
```

But that's still not as powerful as to have a true initialisation function. What I would really like is something like this:

```coffee
# This is called a "constructor", it is a mix between a type and a constructor function.
type Hero ->
  let initialHealth = 120

  return
    name: string
    health = initialHealth
    strength = 10
```

I should think if creates a conflict with the **function types**.

Maybe I should differentiate with different names a raw type declaration (with fields) and a function type, that transforms types into others.
