
# Fa is a data-driven language
# basic classes are defined with the 'class' keyword
class Character
	name: String
	strength: Number

	# we create constructors using the 'from' keyword
	from x:Number  # constructor
		strength = x

	# this is the typical use case for subsitute types :
	from @Number  # see type-as-variable.py
		strength = @Number

	from @CharacterId  # constructor
		self << find @CharacterId

	from @Number, @CharacterId
		self << find @CharacterId
		strength = @Number

	# we create a converter using the 'to' keyword
	# conversion can be implicit, or explicit (using the keyword 'to')
	# ex: character to String
	to @String -> "My name is {name}"

	# methods are defined as described in `definitions.py
	yell(msg:String)
		print msg.toUpperCase

	# aliases are used for inlining
	bigName -> name.toUpperCase

	# we define setters with the 'set' keyword
	set myName to @String
		name = @String

	set myName to @Integer
		name = @Integer.toString().toUpperCase()

	# every property or method beginning with _ is protected
	_x = 3213
	get x -> _x

	# every property or method beginning with __ is private
	__x = 3213
	get x -> __x

	# properties cannot begin with '___' as its reserved internally by Fa

	# setters can be defined
	set x to @Integer
		_x = @Integer

	# for the typical use case when we want the user to read but not modify a property,
	# we can use the 'readonly' keyword
	readonly x = 3213  # x is then privately writable, but is publicly only readable

	# which will be translated to :
	___x = 3213
	get x -> ___x
	

	# instead of a setter you can use a watcher
	x: Integer = 12
	watch x
		print "x is now {x}"

	# which will be translated to :
	___x: Integer = 12
	get x -> ___x
	set x to @Integer
		___x = @Integer
		print "x is now {x}"

	# you can watch every variable with `watch all`
	watch all(name: String, value: Any)
		print "The variable {name} has changed"
		print "Its new value is now {value}"




