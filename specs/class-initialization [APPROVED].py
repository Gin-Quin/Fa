
# let's suppose we have the following classes :
class Character
	name = "Unknown name"
	strength = 1
	health = 10
	codeError : CodeError

	from <String>
		name = <String>.toUpperCase

class Hero is Character
	attack <Character>
		<Character>.health -= strength

# this will create an empty pointer :
let foo : Hero

# this will create a brand new Hero :
let foo = Hero

# with a name :
let foo = Hero "Foo"
# or
let foo : Hero = "Foo"

# with a name and a strength :
let foo = Hero:
	name = "Foo"
	srength = 6
# or
let foo = Hero "Foo":
	srength = 6
# in that case, the constructor will be called first the the strength property will be set
