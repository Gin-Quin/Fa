# It is possible to define a type that extends and array
type ArrayWithName extends Array<number> =
	name: String

# Note: what exactly is different from this syntax?
type ArrayWithName =
	...Array<number>
	name: String
# => this syntax only extends properties, not the "global shape" of the type

# This unlocks some special syntaxes to define instances of this type
let myArray: ArrayWithName = { name = "Coco", [51, 654, 846] }

let myArray: ArrayWithName = 
	name = "Coco"
	- 51
	- 654
	- 846

# (the items must always come after the properties)

# Function parameters can inherit such an array
let createArrayWithName(...ArrayWithName) -> ...

createArrayWithName(name = "Coco", [51, 654, 846])

createArrayWithName:
	name = "Coco"
	- 51
	- 654
	- 846
