
# Object assignemnt is done using the insert operator

class Hero
	strength = 10
	name = ""

let ajax = Hero(strength: 13, name: "Ajax")
let heracles = Hero(strength: 21, name: "Heracles")

# Initialization

let {strength} = heracles
print strength  # 21	

ajax >> strength
print strength  # 13

heracles << strength

# full object assignment
ajax << hercule



# note : this operator may be overloaded with a custom behaviour
# containers like arrays, maps, set, all overload this operator

let array = [1, 2, 3]
array << 4, 5
print array  # 1, 2, 3, 4, 5

hercule << health, mana from ajax
hercule << health, mana

# Old unaccepted Proposal :
# myObject.:
# 	x = 32132
# 	y = 2131