# The access operator enables fast-access of a sub-object

# Typical example, let's say we have a Location class who stores a point :
class Point
	x = 0
	y = 0

class Location
	position: Point

location = Location

# Then instead of having to write:
print location.position.x
print location.position.y

# You can add this line to the class definition
class Location
	position : Point
	@ -> position  # '@' then means 'access position'

# And use it this way:
print location.@x
print location.@y
# Instead of:
print location.position.x
print location.position.y


# Other use case: the type is the name
# Note: it is not compatible with named parameters
# I actually dislike this syntax!
attack(@Hero) ->
	@Hero.health -= strength

get x ->
	call zabu
	call coco

set x value: Integer -> 


let zabu x: Integer = x + 12

let coco => caca.x
	What

let zabu() =
	
let add(x: Integer, y: Integer) = x + y
let add = (x: Integer, y: Integer) => x + y


let add(x: Integer, y: Integer)
	return x + y

let add = (x: Integer, y: Integer) => {
	return x + y
}



children =
	-	Text "Hey"

render()



> Children
if x == 12
	- Text "Coucou"


render =>
	- 
