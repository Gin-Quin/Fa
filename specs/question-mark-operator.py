
# The question mark operator can be used for conditional chaining :
# Fa:
name = node?.attributes.type?.name else ''
# JS:
name = node && node.attributes.type? node.attributes.type.name : ''


# If one wants to use the ternary operator, in Fa he should uses ... if ... else ...
# JS:
x = y > 0 ? 12 : 15
# Fa:
x = 12 if y > 0 else 15



#-- some more examples :

x = 12 if y > 0     #Fa
if (y > 0) x = 12   #JS

let zabu = coco?.bestFriend? else Hero  #Fa
let zabu = coco && coco.bestFriend? coco.bestFriend : Hero #JS

let zabu = Human if coco?.bestFriend else Hero  #Fa
let zabu = coco && coco.bestFriend? 42 : Hero   #JS

let zabu = Human if coco?.bestFriend           #Fa
let zabu = coco && coco.bestFriend? 42 : null  #JS
