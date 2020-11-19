# Function

```coffee
var startCoffee()
   print "Coffee is starting..."

var add(x: Integer, y: Integer) -> Integer
   return x + y

var add(x: Integer, y: Integer) -> x + y

var add = (x: Integer, y: Integer) ->
   return x + y

# function types
let startCoffee: () -> None
let add: (Integer, Integer) -> Integer

```



We start to create the syntax from assignment :
```coffee
# no arguments and no return value
let startCoffee()
   print "Coffee is starting"

let startCoffee = () ->
   print "Coffee is starting"

startCoffee = () -> print "Coffee is starting"

# arguments but no return value
let sayName(name: String) ->
   print name

let sayName = (name: String) ->
   print name

sayName = (name) -> print name  # reassignment

# arguments and return value
let add(x: Integer, y: Integer) -> Integer
   return x + y

let add = (x: Integer, y: Integer) ->
   return x + y

# no arguments but a return value
let getPort() -> String
   return "8080"

let getPort = () -> String
   return "8080"

getPort = () -> "8080"
```

Then we port the syntax to type definition :
```coffee
var startCoffee: () -> = () ->
   print "Coffee is starting"

var add: (Integer, Integer) -> Integer = (x, y) ->
   return x + y

var sayName: (String) -> = name ->
   print name

var getPort: () -> String = -> String
   return "8080"

achille.attack()
achille.attack!

charlie..
   speak()
   rotate(90°)

zabu = Dog
friends = [Human]

```
