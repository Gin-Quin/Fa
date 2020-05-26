# The question mark operator have three use cases :

#1: Along with 'then' and 'else' keywords it can be used as the ternary conditional operator
# JS;
x = y > 0? 12 : 5312
# FA:
x = y > 0? then 12 else 5312

# Fa:
type = node?.attributes.type?.name else nil
# JS:
type = node && node.attributes.type? node.attributes.type.name : nil

#2: Without the 'else'
x = y > 0? then 12
# which is exactly the same as :
x = 12 if y > 0
# which would be in JS :
if (y > 0) x = 12

#3: It can indicate when a variable or property is nilable
class Human
	friend? : Human



let zabu = coco?.bestFriend? else Hero
let zabu = coco && coco.bestFriend? coco.bestFriend : Hero

let zabu = coco?.bestFriend? then Human else Hero
let zabu = coco && coco.bestFriend? 42 : Hero

let zabu = coco?.bestFriend else Hero
let zabu = coco? coco.boyFriend : Hero

