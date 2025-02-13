# The access operator enables fast-access of a sub-object

# Typical example, let's say we have a Location class who stores a point :
type Point =
	x = 0
	y = 0

type Location =
	position: Point

location = Location

# Then instead of having to write:
print location.position.x
print location.position.y

# You can add this line to the class definition
type Location =
	@ => position  # '@' then means 'access position'
	position: Point

# And use it this way:
print location.@x
print location.@y


# Other use case: the type is the name
# Note: it is not compatible with named parameters
let attack(@Hero) ->
	@Hero.health -= strength
