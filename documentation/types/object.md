# Object

In **Fa**, an `object` is either :

- an instance of a `class`,
- or a `singleton`.

## Singleton

A `singleton` is a single-instance object. It can have methods and properties, but cannot be instantiated.

It is created like any variable :

```coffee
let charlie:
   name = "Charlie"
   specy = "dog"
   speed = 125

   speak()
      print "Woof! My name is #{name}!"
```

A `singleton` is automatically a `constant`. You cannot reassign it, since its type is unique.

But you can `reset` it :

```coffee
charlie.name = "Coco"
charlie.speak()  # print "Woof! My name is Coco!"

reset charlie
charlie.speak()  # print "Woof! My name is Charlie!"
```

Like **classes**, singletons can `inherit` :

```coffee
class Dog
   specy = "dog"
   speed = 120

let charlie is Dog:
   name = "Charlie"
   speed = 125

   speak()
      print "Woof! My name is #{name}!"

print charlie is Dog  # true - charlie inherits Dog
print typeof charlie is Dog  # false - charlie is its own type
```

## Instance

An `instance` is created from a class :

```coffee
class Dog
   specy = "dog"
   speed = 120

   from @Integer
      speed = @Integer

# default dog
let dog1 = Dog()

# default copy constructor
let dog2 = Dog("superdog", 240)

# default copy constructor (with parameter name)
let dog3 = Dog(speed: 12)

# from integer constructor
let dog4 = Dog(130)

# with field assignment
let dog5 = Dog:
   specy = "cool dog"
   speed = 140
```
