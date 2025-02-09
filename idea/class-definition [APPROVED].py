
# Fa is a data-driven language
# basic classes are defined with the 'class' keyword
type Character =
	name: String
	strength: Number

	# we create constructors using the 'from' keyword
	from(x: Number)  # constructor
		strength = x

	from(id: Id)  # constructor
		self << find(id)

	from(strength: Number, id: CharacterId)
		self << find(id)
		strength = strength

	# we create a converter using the 'to' keyword
	# conversion can be implicit, or explicit (using the keyword 'to')
	# ex: character to String
	toString() = "My name is {name}"

	# methods are defined as described in `definitions.py
	yell(msg:String) ->
		print msg.toUpperCase

	# redirections are used for inlining aliases
	bigName => name.toUpperCase

	# for the typical use case when we want the user to read but not modify a property,
	# we can use the 'readonly' keyword
	readonly x = 3213  # x is then privately writable, but is publicly only readable
