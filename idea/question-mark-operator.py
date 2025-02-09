
# The question mark operator can be used for conditional chaining :
# Fa:
name = node?.attributes.type?.name or ''
# JS:
name = node?.attributes.type?.name || ''


# If one wants to use the ternary operator, in Fa he should uses ... if ... else ...
# JS:
x = y > 0 ? 12 : 15
# Fa:
x = y > 0 ? 12 else 15 #if z == 12
x = 12 if y > 0 else 15

# Fa:
x = 12 if y > 0
# JS :
if (y > 0) x = 12



#-- some more examples :

x = 12 if y > 0     #Fa
if (y > 0) x = 12   #JS

let zabu = coco?.bestFriend? else Hero  #Fa
let zabu = coco && coco.bestFriend? coco.bestFriend : Hero #JS

let zabu = Human if coco?.bestFriend else Hero  #Fa
let zabu = coco && coco.bestFriend? Human : Hero   #JS

let zabu = Human if coco?.bestFriend           #Fa
let zabu = coco && coco.bestFriend? Human : null  #JS
