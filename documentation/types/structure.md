# Structure

A `structure` defines a very simple `type` with some fields and no methods. It shares similarities with `tuples` from other languages.

A `structure` cannot be `null`.

You should use a `structure` for small data types only, and a `class` for medium or big data types or whenever a method is needed.

We use brackets `{ }` to define a structure.

Creating a `structure` using type inference :
```coffee
let position = { x: 30, y: -20 }

print position.x  # 30
```

Defining a `structure` using the `type` keyword :
```coffee
type Point = { x: Number, y: Number }

let p1 = Point { x: 60, y: -80 }  # constructor-like syntax
let p2 = Point { -100, 0 }

print p2.y  # 0
```

Unlike objects, **assignment** creates a **copy** :

```coffee
type Point = { x: Number, y: Number }

let p1 = Point { 5, 20 }
let p2 = p1

p2.x = 5000
print p1.x  # 5
```



## Implementation

Whichever the target, `structures` are great to improve a program speed.


- **Fa**

   ```coffee
   let position = { x: 30, y: -20 }

   type Point = { x: Number, y: Number }
   let p = Point { x: 60, y: -80 }

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

   Because `structures` are stored in the **stack** instead of the **heap** (unlike instantiated objects), they are very fast for small objects.

   ```cpp
   struct {
      double x {30};
      double y {-20};
   } position;

   struct Point {
      double x {0};
      double y {0};
   };

   Point p = { 60, -80 };

   std::cout << p.x << std::endl;
   ```
