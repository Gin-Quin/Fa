# The colon is maybe the most important and subtle operator in Fa
# What exactly does the colon do?

# First, it's used to define types
let x: String
let x: String = 125
let x: [Number] = [5, 12, 8]


# Second, it's used to define unnamed structures
let options:
	a = false
	b = true
	c = 1321
	d:
		- 321
		- 3213
		- 55312

# Third, it is used to define mandatory functions arguments
let method arg1: String, arg2: Number, arg3 = 0
 
# Fourth, it is used to call a method with specific arguments
method arg2: 5321
# (same as in Swift)

# Fifth, same but with indentation
method:
	arg2 = 5321

method:
	...arguments

# Sixth, it can be used to initialize an instance of a class
let dog = Dog:
	name = "Doggie"
	speed = 1321

let dog: Dog
	name = "Doggie"
	speed = 1321

# this syntax may raise an ambiguity for Fa beginners : do we use the constructor or do we define fields?
# the answer is : we define fields,
# because Fa is a data-oriented language rather than constructor-oriented

# if one want to create an instance from a constructor, one can use the .from method :
let dog = Dog from:
	dad = zatu
	mom = zakar


#-----------------------------------------------------------------------------
# Let's start with this spec. With use, maybe I'll start to see improvements
#-----------------------------------------------------------------------------
