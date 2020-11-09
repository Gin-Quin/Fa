# The access operator enables fast-access of a sub-object

# Typical example, let's say we have a Location class who stores a point :
class Point
	x = 0
	y = 0

class Location
	position : Point

location = Location

# Then instead of having to write :
print location.position.x
print location.position.y

# You can add this line to the class definition
class Location
	position : Point
	@ -> position  # '@' then means 'access position'

# And use it this way :
print location.@x
print location.@y
print location.@Point.x
print location.@Point.y


