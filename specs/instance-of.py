
# Fa have the JS equivalent of instanceof :
if myObject is Dog
	myObject.bark  # because now it's known myObject is a dog - and this it can bark

# Every variable in Fa must implement the method `toString()`
# For structures, this variable is static - but it's virtual for regular objects
# Since this method is virtual, it can be used at runtime to identify precisely a type

# Two problems :
# - it breaks the basic principles of OOP
# - it's an additional work for the compiler to deal with these 'if .. is ..' and so to know what an object is or is not
