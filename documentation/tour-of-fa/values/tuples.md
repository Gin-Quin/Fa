# Tuple

A `tuple` defines a very simple `type` with some fields and no methods. It shares similarities with `tuples` from other languages.

A `tuple` cannot be `null`.

You should use a `tuple` for small data types only, and a `class` for medium or big data types or whenever a method is needed.

We use parenthesis `()` to define a tuple.

Creating a `tuple` using type inference :
```coffee
let position = (x: 30, y: -20)

print position.x  # 30
```

Defining a `tuple` using the `type` keyword :
```coffee
type Point = (x: Number, y: Number)

let p1: Point = (x = 60, y = -80)
let p2: Point = (-100, 0)  # field names is encouraged but optional

let p3 = Point(x = 60, y = -80)  # constructor-like syntax
let p4 = Point(-100, 0)

print p2.y  # 0
```

Unlike objects, **assignment** creates a **copy** :

```coffee
type Point = (x: Number, y: Number)

let p1 = Point(5, 20)
let p2 = p1

p2.x = 5000
print p1.x  # 5
```

**Destructuring assignment** is possible :

```coffee
p = (x: 30, y: -20)
use p >> (a, b)  # the variable names can change

log(a)  # 30
log(b)  # -20
```


## Implementation

Whichever the target, `tuples` are great to improve a program speed.


- **Fa**

   ```coffee
   position = (x = 30, y = -20)

   type Point = (x: Number, y: Number)
   p: Point = (x = 60, y = -80)

   print p.x
   ```

- **Javascript**

   Using arrays instead of objects greatly improves speed.

   ```js
   let position = [30, -20];
   let point = [60, -80];

   console.log(point[0])
   ```

- **C++**

   Because `tuples` are stored in the **stack** instead of the **heap** (unlike `instantiated objects`), they are very fast.

   ```c++
   std::tuple<Integer, Integer> position = {30, -20};

   using Point = std::tuple<Integer, Integer>;
   Point p = {60, -80};

   std::cout << std::get<0>(p) << std::endl;
   ```
