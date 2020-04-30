# enums in Fa are declared using the keyword `type`
# a type is more powerful than just an enum
# however, like an enum, a Type is used at compile-time only

# simple enum exemples :
type NodeType is Integer
	comment
	text
	element

class Node
	static type Type is Integer
		comment = 0
		text = 1
		element = 2
	
	type:Type = comment


myNode.type = element

# a more advanced example with units :
type Degree is Number  # the default unit is Celsius
	AbsoluteZero = -273.15
	unit kelvin k:Integer -> k - AbsoluteZero
	unit fahreneit f:Integer -> (f - 32) / 1.8

temperature:Degree = 45 kelvin
# will be replaced at compile time by :
# temperature:Number = (45 - (-273.15))

# this can be used to create CSS-like files
