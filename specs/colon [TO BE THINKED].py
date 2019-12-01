# The colon is maybe the most important and subtle operator in Fa
# What exactly does the colon do?

# First, it's used to define types
x: String
x: String = 125
x: [Number] = [5, 12, 8]


call


# Second, it's used to define unnamed structures
options:
	a = false
	b = true
	c = 1321
	d:
		- 321
		- 3213
		- 55312

# Third, it is used to define mandatory functions arguments
method arg1:String, arg2:Number, arg3=0

# Fourth, it is used to call a method with specific arguments
method arg2: 5321
# (same as in Swift)

# Fifth, it can take an object as parameters for a function
method:
	arg2 = 5321	

method:
	...arguments

# Seventh, it can be used to define a class
dog: Dog
	name = "Doggie"
	speed = 1321
# notice that there, the dog is initialized
-----------------------------------------------------
# I would say NO to this syntax!
# Because `dog: Dog` should mean `uninitialized`...
-----------------------------------------------------

# or should we use an equal :
dog = Dog:
	name = "Doggie"
	speed = 1321
# this syntax raise an ambiguity : do we use the constructor or do we define fields?
# if we want to call a specific constructor, we use the .from function :
dog = Dog.from:
	dad = zatu
	mom = zakar


-----------------------------------------------------------------------------
# Let's start with this spec. With use, maybe I'll start to see improvements!
-----------------------------------------------------------------------------