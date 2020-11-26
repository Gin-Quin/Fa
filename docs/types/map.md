# Map

A `Map` is a dictionary.

`Maps` are defined this way :

```coffee
# with explicit type (full notation)
let integers = [String: Number] { a: 12, b: 12 }
integers = { x: 125 }
integers:
   x = 124
let numbers[String: Number]:
   a = 12
   b = 123
let fruits = [String: Number]:
   - "banana"
   - "strawberry"
   - "apple"

# with explicit type (shorthand notation)
let integers[Integer] = [5, 12, 13]
let numbers[Number] = [5, 3.14]
let fruits = [String]:
   - "banana"
   - "strawberry"
   - "apple"
let fruits[String]:
   - "banana"
   - "strawberry"
   - "apple"
```
