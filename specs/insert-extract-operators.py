# The insert and extract operators (<< and >>) are similar to the C++ ones
# They can be used with streams, generators, and any type with a notion of "inserting" or "extracting", like FIFO

# -- examples
eatingMonster << hercule << ajax

while generator >> value
	print value

stack << itemA << itemB
while stack >> item
	print item.name

# without the insert operator :
stack.add itemA, itemB
while item = stack.next
	print item.name

# so, the interest is not... so awesome
# it adds some direct comprehension though
# we need to define and use methods like 'add' and 'next' instead
# ie a insert and an extract methods


# -- Another use case of the insert and extract operators would be object assignment
# Object assignment is a very important part of the Fa language
objA << objB  # we insert objA with objB
hercule << health, mana from ajax
hercule << { health, mana }
hercule <<
	health
	mana

# -> of course, private properties cannot be changed this way

# then what would do the extract operator? Maybe it would do the same :
hercule >> ajax  # insert ajax with hercule
{ health, mana } >> ajax << hercule  # which have precedence?
{ health, mana } >> ajax >> hercule
# or maybe it is not defined by default and need to be overloaded -> simpler

# containers all overload this operator
let arrayA = [5, 12, 3]
let arrayB = [8, 9]

arrayA << arrayB
# would be the same as
arrayA.push ...arrayB
# or
arrayA += arrayB  # there is a direct rivalry between those two operators
stringA += stringB + stringC + stringD
stringA << stringB << stringC << stringD
stringA << stringB + stringC + stringD

setA << setB  # same, we don't expect the 'length' property to be changed, it has no sense