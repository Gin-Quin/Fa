
# ---- Proposal 0 ----
# Tuples are the result of the comma `,` operator
# The comma takes one or more arguments and return a tuple

# Function arguments is either :
# 1. A single element
print @String

# 2. A tuple
add a:Number, b:Number

# Examples of use :
let a, b = 11, 15
let a = "Hey", 12  # inferred type : String, Integer
print a[0]  # "Hey"
print a[1]  # 12

type Point = Integer, Integer
let position = 5, 11

draw @Point
	...

draw 5, 12, 11


# In C++, tuples are converted into simple structures

# Fa
type Point = { x:0, y:0, depth:0 }
p:Point = { x:5, y:12 }
let { x, y } << p

# C++
struct {
	int x { 0 };
	int y { 0 };
} Point;




# ---- Test ----
# In Fa, Tuples are simple structures
let myTuple:String, Integer = "Hello", 9
print myTuple[0]  # "Hello"
print myTuple[1]  # 9

# They can have named fields
let myTuple: [say:String, importance:Integer] = ["Hello", 9]
print myTuple.say  # "Hello"
print myTuple.importance  # 9

# A tuple is always inlined. You can't do things like :
let myTuple = [say:String, importance:Integer]:
	- "Hello"
	- 9

type Point = Integer, Integer


# Let's question about tuples in Fa. Here are some proposals :

# ---- Proposal 1 ---- [REFUSED]
# Tuples are generated with the ',' operator

let x = 4, 5  # Here x is a Tuple[Integer, Integer]
let { a, b } << x

using Point = (Integer, Integer)

# => This proposal is impossible because it brings ambiguity
# Here is an example :

let drawPoint = Point ->
	...

drawPoint 5, 12      # <-- is `5, 12` a tuple or the functions arguments ?
drawPoint(5, 12)     # <-- same question here
drawPoint((5, 12))   # Here we would be finally sure to have a tuple as argument ; but the syntax is ugly



# ---- Proposal 2 ---- [ACCEPTED]
# Tuples are generated with the '{...}' group operator
let x = { 4, 5 }  # x is of type `Tuple[Integer, Integer]`, alias `{Integer, Integer}`

let { a, b } << x  # this syntax now makes a lot of sense

type Point is { Integer, Integer }

let drawPoint = Point ->
	...

# Now we can do :
drawPoint { 5, 12 }
