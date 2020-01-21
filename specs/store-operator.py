
# The store operator is a very useful one, but a bit complex to explain
# And still more complex to implement in JS and C++, as these languages do not
# natively implement that kind of behaviour

# Basically, the store operator is used for assigning the 'content', of an object
# to another
let ajax = Hero name: "Ajax", strength: 7
let hercule = Hero name: "Hercule", strength: 13

ajax << hercule
print ajax.strength  # 13
print ajax.name      # 'Hercule'

# the store operator can work from left to right or from right to left :
hercule >> ajax  # same as before

# Quite useful, right?
# But here things begin to be interesting..
# You can also use the store operator with objects made 'à la volée' :
ajax << { name: "Ajax II", strength: 14 }

ajax <<
	...hercule
	name: "Ajax III"

# And you can also use it for destructuring objects :
let { name, strength } << hercule
print name       # 'Hercule'
print strength   # 13

# or do some awesome stuff like this :
ajax >> { mana, health } >> hercule

# which is the equivalent of :
hercule.mana = ajax.mana
hercule.health = ajax.health


# The store operator has special behaviour when used with containers (like arays) :
let fruits = ['apple', 'banana', 'strawberry']
let [fruitA, fruitB] << fruits
print fruitA  # 'apple'
print fruitB  # 'banana'

# The behaviour of the store operator depends on the container
# There are a lot of rather complicated cases so this operator should be thought
# very carefully when time will come for the implementation
