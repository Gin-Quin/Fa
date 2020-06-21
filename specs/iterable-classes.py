

# Iterable classes are classes that implement the special `iterate` method:
class MyDogContainer
	myDogs : [String]  # very exhaustive list of all my dearest puppies's names

	# because a `yield` is detected in the function, the compiler will
	# understand we want to return a Yield[String] object - not a String
	iterate over String
		for dog in myDogs
			yield dog


	to @String = "Hello there"
		if myDogs.length == 12
			return "I have 12 dogs"
	from @String
	print "Hello there"


# Most of the time, an iterable class just iterate through a property
# so that an iterator can alias to that property's iterator
class MyDogContainer
	myDogs : [String]
	iterate -> myDogs

for dog in myDogContainer
	print "I have a beloved dog and its name is {dog}"
