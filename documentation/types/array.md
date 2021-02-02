# Array

An `Array` is list of elements that all have the same `type`.

`Arrays` are defined this way :

```coffee
# inline notation with subtype inference
let integers = [5, 12, 13]  # inferred type : Array<Integer>
let numbers = [5, 3.14]  # inferred type : Array<Number>

# multiline notation
let fruits:  # inferred type : Array<String>
   - "banana"
   - "strawberry"
   - "apple"

# with explicit type (full notation)
let integers: Array<Integer> = [5, 12, 13]
let numbers = Array<Number>(5, 3.14)
let fruits = Array<String>:
   - "banana"
   - "strawberry"
   - "apple"
```

`[Type]` is a shorthand for `Array<Type>` :

```coffee
# with explicit type (shorthand notation)
let integers: [Integer] = [5, 12, 13]
let numbers = [Number](5, 3.14)
let fruits = [String]:
   - "banana"
   - "strawberry"
   - "apple"
```

Instead of :
```coffee
let integers: [Integer] = [5, 12, 13]
```

you can also write :
```coffee
let integers[Integer] = [5, 12, 13]
```
 which is the preferred way to declare arrays in `Fa`.

You can gracefully use this notation with a multiline declaration :

```coffee
let fruits[String]:
   - "banana"
   - "strawberry"
   - "apple"
```
