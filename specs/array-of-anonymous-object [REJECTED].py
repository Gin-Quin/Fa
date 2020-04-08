# let's say we have the class Hero :
class Hero
	name: String
	strength: Integer

	from <String>, <Number>
		name = <String>
		strength = <Number>

let ajax = Hero "Ajax", 7

# we can create arrays of objects this way :
myHeroes = [Hero]:
	-	name = "Zabu"
		strength = 12
	-	name = "Hercule"
		strength = 9

# or this way, if we don't want to repeat the informations all the time :
myHeroes = [Hero]:
	- "Zabu", 12  # we use the constructor
	- "Hercule", 9

# but what if the class Hero was not defined? Then we could do it this way :
myHeroes:
	| name : String, strength : Integer
	- "Zabu", 12
	- "Hercule", 9
# but pretty sure it's a bad idea... things would get clearer if the structure was previously defined