
# Fa is a data-driven language
# basic classes are defined with the 'class' keyword
class Character
	name: String
	strength: Number

	from Number  # constructor
		strength = number

	from CharacterId  # constructor
		self <<< find characterId

	from
		<- Number
		<- CharacterId
		self = find characterId
		strength = number

	to String
		return "My name is {name}"

	find id: CharacterId -> Character


newCharacter
	- name: String
	- strength: Number
	return Character:


character = newCharacter:
	name: "zabu"
	strength: 121
	

# classes are constructed this way
coco = Character with
	name = "Alberto"
	strength = 532112

coco = Character 'zabu'..
	name = "Alberto"
	strength = 532112

coco = Character from 12

coco: Character = 12

coco = Character from
	strength: 12

coco: Character
	strength = 12

