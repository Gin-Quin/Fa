
# In one word, the copy operator is the JS equivalent of 'Object.assign'
# In JS you would do :
Object.assign(objectA, objectB)
# In Fa :
objectA << objectB




# The copy operator is a very useful one, but a bit complex to explain
# And still more complex to implement in JS and C++, as these languages do not
# natively implement that kind of behaviour

# Basically, the copy operator is used for assigning the 'content', of an object
# to another
let ajax = Hero(name = "Ajax", strength = 7)
let hercule = Hero(name = "Hercule", strength = 13)


ajax << hercule
print ajax.strength  # 13
print ajax.name      # 'Hercule'

# the copy operator can work from left to right or from right to left :
hercule >> ajax  # same as before

# You can also use the copy operator with objects made 'à la volée' :
ajax << { name = "Ajax II", strength = 14 }

ajax <<
	...hercule
	name = "Ajax III"


# And you can also use it for destructuring objects :
let { name, strength } << hercule
{ name } << ajax
name from ajax
let name, strength from hercule
import name, strength from Zabu
import Zabu
import Coco

{ title, author } << book

for book in books
for {title, author} in books
for title, author from books
for [title, author] from books

print name       # 'Hercule'
print strength   # 13

# This is a bit different of this one :
let { name, strength } = hercule  # here we create aliases 'name' -> 'hercule.name' and 'strength' -> 'hercule.strength'

# or do some awesome stuff like this :
ajax >> { mana, health } >> hercule  # the transitivity operation

# which is the equivalent of :
hercule.mana = ajax.mana
hercule.health = ajax.health


# The copy operator has special behaviour when used with containers (like arrays) :
let fruits = ['apple', 'banana', 'strawberry']
let [fruitA, fruitB] from fruits  # unstoring
print fruitA  # 'apple'
print fruitB  # 'banana'

[fruitA, fruitB] >> fruits  # storing
fruits.add fruitA, fruitB  # same - and cleaner :/

let [fruitA, fruitB, fruitC] = fruits[2..4]

# The behaviour of the copy operator depends on the container
# There are a lot of rather complicated cases so this operator should be thought
# very carefully when time will come for the implementation
