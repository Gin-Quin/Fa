# let's say we have the class Hero :
class Hero
	constant pi = 3.145321543121
	name: String
	strength: Integer

let ajax:Hero = "Ajax", 7
let ajax = Hero "Ajax", 7  # here we use the 'default constructor'
 
# we can create arrays of objects this way :
myHeroes: [Hero]
	-	name: "Zabu"
		strength: 12
	-	name: "Hercule"
		strength: 9

# or this way, if we don't want to repeat the informations all the time :
myHeroes = [Hero]
	- "Zabu", 12  # we use the default constructor
	- "Hercule", 9

# but what if the class Hero was not defined? Then we could do it this way :
myHeroes:
	| name : String, strength : Integer
	- "Zabu", 12
	- "Hercule", 9
# but not sure it's such a great idea... things would get clearer if the structure was previously defined

hero = Hero:
	zabu: 12
	coco: 321
	options:
		x = 32132