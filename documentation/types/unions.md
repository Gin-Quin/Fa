# Unions

When you want to declare a type that is a union of multiple subtypes, you should use the `union` keyword:

```fa
-- a union of either `Hello`, `World`, `Car` or `House`
union MyUnion = {
  Hello = "hello"
  
  World = "world"

  Car = {
    brand: String
    model: String
    year: Integer
  }

  House = {
    address: String
    price: Integer
  }
}

-- you can now use MyUnion as a type:

let myValue1 = MyUnion.Car {
  brand = "Toyota"
  model = "Corolla"
  year = 2020
}

let myValue2: MyUnion = "hello"

when myValue1 is {
  Car => log("It's a car")
  House => log("It's a house")
  Hello => log("It's hello")
  World => log("It's world")
}
```
