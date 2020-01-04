# In Fa, each class is associated with a magic number
# Each object has the method getMagicNumber() wich return a constant
# (at least when tranpiled to C++)

# Then we can do something like :
if myObject is Dog
	myObject.bark  # because now it's known myObject is a dog - and this it can bark

# The inconvenient is : for small objects, you way not want to add a magic number
# But for small objects, structures should be used instead. See structures