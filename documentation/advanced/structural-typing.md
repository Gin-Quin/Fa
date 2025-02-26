# Structural typing

Like Typescript, Fa has a **structural type system**. This is unlike languages like Rust or Haskell, which have a **nominal type system**.

In a **structural type system**, the type of a value is determined by the fields it has, rather than by the name of the type.

In a **nominal type system**, the type of a value is determined by the name of the type, rather than by the fields it has.

This means that in Fa, two types are considered the same if they have the same fields, even if they have different names.

This is valid in Fa (as well as in Typescript), but would be invalid in Rust, :

```ts
type Human = {
  name: String
}

type Animal = {
  name: String
}

// conversion from Animal to Human is valid
animal = Animal { name = "Zeus" }
human = Human from animal // conversion from Animal to Human is valid
human = Human::from(animal) // conversion from Animal to Human is valid
human = animal as Human // conversion from Animal to Human is valid

logAnimalName = (animal: Animal) {
  console.log(animal.name)
}

logAnimalName(human) // valid as well, because Human and Animal have the same fields
```

This also means that a **superset** of a type can be used as the type itself:

```ts
type Human = {
  name: String
}

type SuperHuman = {
  ...Human
  powers = "unlimited"
}

human = Human::from(SuperHuman {
  name = "John"
  powers = "unlimited"
})

console.log(human) // will log { name = "John" }, "powers" will be dropped
``` 
