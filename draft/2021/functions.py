# -- Function calls

# Functions can be called with parenthesis '()' or with a colon ':'
call(number: '0102030405')
call: number: '0102030405' # should it be authorized? Add rule "no named parameters for inline-colon-call"? I think I should

call('0102030405')
call: '0102030405'

# The syntax is ugly with inline named parameters, but it's better in multiline:
call:
	number: '0102030405'

# Type constructors are functions and follow the exact same rules
Hero(name: "Zabu")
Hero: name: "Zabu" # this one is still ugly
Hero("Zabu")
Hero: "Zabu"
Hero:
	name: "Zabu"


# -- Function definitions

# The functions are defined like any variables, with a type and a value
let call = (number: String) ->
	callThisGuy(number)

let call(number: String) = number ->
	callThisGuy(number)

let call(number: String)
	callThisGuy(number)

let call(number: String)
	callThisGuy(number)

# The return type of the function MUST be indicated
let call = (number: String): String ->
	callThisGuy(number)
	return number

let call(number: String): String = number ->
	callThisGuy(number)
	return number

let call(number: String): String
	callThisGuy(number)
	return number


# -- Method definitions

type Dog
	name = "Albert"

	bark()
		print "Ouaf! I'm {name}!"

	bark(message: String)
		print "Ouaf! I'm {message}!"

	bark(message: String): String
		let barked = "Ouaf! I'm {message}!"
		print barked
		return barked

