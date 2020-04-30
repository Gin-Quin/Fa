
# Fa is a data-driven language
# basic classes are defined with the 'class' keyword
class Character
	name : String
	strength : Number

	# we create constructors using the 'from' keyword
	from x:Number  # constructor
		strength = x

	# this is the typical use case for subsitute types :
	from @Number  # see type-as-variable.py
		strength = @Number

	from @CharacterId  # constructor
		self << find @CharacterId

	from <Number>, <CharacterId>
		self << find <CharacterId>
		strength = <Number>

	from @Number, @CharacterId
		self << find @CharacterId
		strength = @Number

	# we create a converter using the 'to' keyword
	# conversion can be implicit, or explicit (using the keyword 'to')
	# ex: character to String
	to String -> "My name is {name}"

	# methods are defined as described in `definitions.py
	yell msg:String
		print msg.toUpperCase

	# getters are methods with no arguments
	bigName -> String
		return name.toUpperCase

	# aliases are inline getters
	bigName -> name.toUpperCase

	# we define setters with the 'set' keyword
	set myName $String
		name = $String

	set myName $Integer
		name = $Integer.toString.toUpperCase

	# every property or method beginning with _ is private
	_x = 3213
	x -> _x

	# setters can be defined
	set x $Integer -> _x = $Integer

	# as well as watchers (they trigger only when the value changes)
	watch x -> print "x is now {x}"
	watch x (oldValue) -> print "x was {oldValue} and is now {x}"

	# for the typical use case when we want the user to read but not modify a property,
	# we can use the 'readonly' keyword
	readonly x = 3213
	# x is then privately writable, but is publicly only readable
