
# In Fa the `new` operator is very different from other languages : you
# don't need to call `new` to create a new instance of an object
# Instead, using `new` recreate an instance of a given variable

# Exemple :
let zabu = Hero:
	name = "Zabu"
	strength = 985

new zabu:
	name = "Zabu II"
	strength = 986

# It actually is similar to :
zabu = Hero:
	name = "Zabu II"
	strength = 986

# it's especially useful for structures with anonymous types :
let options:
	shouldUpdate = false
	autoRender = true

new options:
	shouldUpdate = true


# I'm not sure yet to use this operator...
# Because we could just use the simple dot syntax :
let fruits:
	- "Banana"
	- "Coco"

fruits:
	- "Banana"
	- "Coco"



# I don't know which is best
# Using `new` may add some understanding about what is done
