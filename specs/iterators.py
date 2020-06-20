
# One should not confound *iterators*, which are functions, and *iterable classes*


# An *iterator* is a function that return an instance of a *Yield* object
# Any iterator function can be used in a for .. in .. loop
# Exemple :
let myDogs : [String]  # very exhaustive list of all my dearest puppies's names
let dogs = -> String
	for dog in myDogs
		yield dog

for dog in dogs
	print "I have a beloved dog and its name is {dog}"



# *Iterable classes* are classes that implement the special *iterate* method:
class MyDogContainer
	myDogs : [String]  # very exhaustive list of all my dearest puppies's names

	#1: Using yield
	# because a `yield` is detected in the function, the compiler will
	# understand we want to return a Yield[String] object - not a String
	iterate -> String
		for dog in myDogs
			yield dog

	#2: Not using yield
	_cursor = 0
	iterate -> {value, done} = Yield[String]
		value = myDogs[_cursor++]
		done = _cursor is myDogs.length
		_cursor = 0 if done



# Most of the time, an iterable class just iterate through a property
# so that an iterator can alias to that property's iterator
class MyDogContainer
	myDogs : [String]
	iterate -> myDogs

for dog in myDogContainer
	print "I have a beloved dog and its name is {dog}"
