# The exclamation mark operator `!` can be used in two ways

# First, when declaring a variable or property, you can use it to make sure this
# property will never be nil :
class Human
	bestFriendForever! = Human  # everyone has always a bff
	otherFriend : Human


# The second use case is when you want to throw an error if a value is nil
let type = node.attributes.type!.name

