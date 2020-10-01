
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
