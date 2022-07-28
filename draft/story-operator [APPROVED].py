
# the story operator uses the double dot '..' syntax
# it is similar to the dot '.' operator excepts that it returns the left expression
let otherObject = (myObject.. x = 12)

myObject.x = 12
let otherObject = myObject

# it can be used to chain operations
myObject.. x = 12 .. call() .. x = 121

# the story operator can be followed by a block
myObject..
	x = 32132
	y = 2131
	objectCommand()

hercule..
	attack = ajax
	attackAll = hercule.enemies

hercule.attack(ajax)

# it returns the object itself so you can chain operations
# even when functions do not return the object itself
hercule..attack(ajax)..save(padmé)
hercule..
	attack(ajax)
	save(padmé)

# with classing chaining, if "attack" returns the attacked target for example,
# it would not be possible
hercule.attack(ajax).save(padmé) # this would not work

# inside brackets '[]', the double dot is a shortcut to create a new slice
let slice = [4..12]