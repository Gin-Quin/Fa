
# Traits or Mixins is Fa's way to have multiple inheritance
# Fa's mixins work the same way as in Dart : https://alligator.io/dart/mixins/
# Mixins can easily be used in JS : https://justinfagnani.com/2015/12/21/real-mixins-with-javascript-classes/
# For the C++ I still need to think it through. Maybe using multiple inheritance or virtual inheritance is a good idea
# Traits are like classes but :
# - they can't inherite classes (but they can extend other traits)

trait Legs
	legs: Integer


class Dog is Animal with Legs, Head
	speed = 12
	position = 0

	run()
		position += speed * legs


let zabu = Animal with Legs(speed: 125)
print "My zabu has {zabu.legs} legs"
