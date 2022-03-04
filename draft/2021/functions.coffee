# -- Function calls

# Functions can be called inline with parenthesis '()' or multiline with a colon ':'
call(number = '0102030405')
call:
	number = '0102030405'

# Type constructors are functions and follow the exact same rules
Hero(name = "Zabu", strength = 112)
Hero:
	name = "Zabu"
	strength = 112

# Both syntaxes can be used simultaneously
Hero(name = "Zabu"):
	strength = 112
# Maybe not, because it conflicts with the syntax of a function that returns another function


# Naming parameters when calling a function is mandatory!
# The only exception is when a function has only one parameter
let call = (number: String) -> callThisGuy(number)
call('0102030405')
call: '0102030405'

let print = (values: ...any)
print: "Hello", "how are you?" # because print's default parameter is a spread parameter, it works

# You can pass a variable that have the same name of a parameter
attack(target = target, weapon = weapon)
attack(target, weapon) # same as before


# -- Function definitions

# The functions are defined like any variables, with a type and a value
let call = (number: String) ->
	callThisGuy(number)

let call(number: String) ->
	callThisGuy(number)

# The return type of the function MUST be indicated if it is not void
let call = (number: String): String ->
	callThisGuy(number)
	return number

# Should it be possible to define a return variable?
let call(number: String): let result = number ->
	callThisGuy(number)
# I don't like it when using the ':' syntax, it's kind of confusing
let call(number: String): let result: Number ->
	callThisGuy(number)

# Use big arrow to return directly a value (and return type is not mandatory)
let add(x: Number, y: Number) => x + y

# Indentation after big arrow means we return an object
let createPerson(name: String, strength: Number) =>
	name
	strength

let createPerson(name: String, strength: Number) => { name, strength }

# Functions with a lot of arguments can be multilined:
let createPerson:
	firstName: String
	lastName: String
	strength: Number
: String =>
	name = "{firstName} {lastName}"
	strength

# It is possible to inherits parameters from an object
type Hero =
	name: String
	strength: Number

let createHero(...Hero) -> # parameters inheritance
	print name, strength

let createHero(hero: Hero) -> # parameters composition
	print hero.name, hero.strength

let createHero(...Hero: { name, strength }) -> # explicit parameters inheritance
	print name, strength


# -- Method definitions
type Dog =
	name = "Albert"

	bark() ->
		print "Ouaf! I'm {name}!"

	bark(message: String) ->
		print "Ouaf! I'm {message}!"

	bark(message: String): String ->
		let barked = "Ouaf! I'm {message}!"
		print barked
		return barked


# -- Function type definitions
type Call = (number: String) -> void

# When a function has a lot of argument, it can be multilined
type Call = 
	number: String
-> void

