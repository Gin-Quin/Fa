
# The following case happens very often :
myMethod num:Number, str:String, dog:Dog
	print num
	print str
	print dog

# -> That is when the variable name is more or less the same as the type

# In Fa, you can use the *type as variable* form, which makes it cleaner :
myMethod <Number>, <String>, <Dog>
	print <Number>
	print <String>
	print <Dog>


# It makes things crystal clear when working with constructors :
class Hero
	name = ''
	strength = 10

	from <String>
		name = <String>

	from <Number>
		strength = <Number>