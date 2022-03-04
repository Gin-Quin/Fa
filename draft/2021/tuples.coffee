# Tuples types are constructed with the [] operator
# In opposition to objects, tuples are not keyed

let tuple: [Integer, String] = [12, "Hello"]
print: tuple[1] # print "Hello"
print: tuple[2] # compile time error

# The size of a tuple can be accessed as a static compile-time value
print: tuple.length # will be tranpiled into "console.log(2)"

# It is possible to iterate through a tuple
for value, index in tuple
	print: value # value will be of type "Integer | String"

# It is also possible to define a tuple with many times the same type
let twelveIntegers: [Integer * 12] = [1, 2, 3, 4, 5, 7, 8, 9, 10, 11, 12]

# Do not mix up with this syntax that is used to define an array:
let imAnArrayNotATuple: [...Integer] = []
