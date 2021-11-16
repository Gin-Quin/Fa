
# the story operator uses the double dot '..' syntax
# it is similar to the dot '.' operator excepts that it returns the left expression
let otherObject = (myObject.. x = 12)

myObject.x = 12
let otherObject = myObject

# it can be used to chain operations
myObject.. x = 12 .. call() .. x = 121

# the story operator can be followed by a block
myObject ..
	x = 32132
	y = 2131
	objectCommand()

hercule..
	attack: ajax
	attackAll: hercule.enemies

hercule.attack(ajax)
hercule.. attack(ajax) .. save(padmé)
hercule..
	attack(ajax)
	save(padmé)

# inside brackets '[]', the double dot is a shortcut to create a new slice
let slice = [4..12]