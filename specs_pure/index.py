# The idea of the "pure" Fa is :
# 	1. no ':' or '=' to define properties
# 	2. no '(' and ')' to define methods
# These two ideas do not necessarily need to be implemented both together
# -> I think there is a need to use a colon or an equal to define properties

# Here is an example (this reaaaally looks like YAML :D) :

class Hero
	name = "Hercule"
	strength = 12
	health = 30
	mana = 40
	friends: 
		type = "sticky"

	dog = Dog:
		name = "My beloved"

	monture = dog

	friends = [Person]:
		-	strength = 3213
			name = "Unknown"
		-	strength = 900
			name = "Coco"

	heal health: Integer, mana: Integer -> self
		self.health += health
		self.mana += mana

	# commands/methods with a lot of arguments :
	heal -> self
		<- health: Integer  @ I am a doc comment!
		<- mana: Integer  @ This is crazy shit!
		self.health += health
		self.mana += mana

	attack enemy:Person
		enemy.health -= strength

	attackAll enemies:[Person]
		for enemy in enemies
			attack enemy



# pull and push
myCommand
	let a, b << obj
	a += b
	a >> obj


# pull inside a for loop
myCommand
	for author, name << books

# tuples tuples tuples !
	let a, b = obj  # what does this command do?

# anonymous structures
	let x:
		a = 32132
		b = 513213