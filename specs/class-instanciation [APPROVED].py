
# let's suppose we have the following classes :
class Character
	name = "Unknown name"
	strength = 1
	health = 10
	codeError: CodeError

	from @String
		name = @String.toUpperCase()

class Hero is Character
	attack @Character
		@Character.health -= strength

# this will create an empty pointer :
let foo: Hero
let foo: Hero = null

# this will create a brand new Hero with default values :
let foo = Hero

# with a name and a strength :
let foo = Hero:
	name = "Foo"
	srength = 6

# or
let foo = Hero("Foo", 6)
let foo = Hero "Foo", 6  # parenthesis are recommanded but they are actually optional

# or
let foo: Hero = ("Foo", 6)

# or
let foo = Hero(name: "Foo", strength: 6)
