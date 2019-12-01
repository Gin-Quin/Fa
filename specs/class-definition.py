
# Fa is a data-driven language
# basic classes are defined with the 'class' keyword
class Character
	name: String
	strength: Number

	# we create constructors using the from' keyword
	from <Number>  # constructor
		strength = <Number>

	from <CharacterId>  # constructor
		self << find(<CharacterId>)

	from <Number>, <CharacterId>
		self << find(<CharacterId>)
		strength = <Number>

	# to define a constructor with no argument, use 'new'
	# the `new` constructor is called before every other constructor
	new
		print "I'm created!"

	# we create a converter using the 'to' keyword
	# should it automatically generate the 'toString' function?
	# conversion can be implicit, or explicit (using 'to' or 'toString')
	to String -> "My name is {name}"

	# methods are defined as described in `definitions.py
	yell msg: String -> void
		print msg.toUpperCase

	# getters and methods are the same
	bigName -> name.toUpperCase

	# we define setters with the 'set' keyword
	set myName <String>
		name = <String>

	set myName <Integer>
		name = <Integer>.toString.toUpperCase

	# every property or method beginning with _ is private
	_x = 3213
	x -> _x
	set x = <Integer> -> _x = <Integer>

	# for the typical use case when we want the user to read but not modify a property,
	# we can use the 'readonly' keyword
	readonly x = 3213
	# which spares us to do :
	# _x = 3213
	# x -> _x
	# clean, right?

