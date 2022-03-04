# Types and functions share a lot of common stuff
# Since types have a default constructors, they act like a type AND a function
# What if a user wants to create a new constructor for a given type (let's say from a 3rd party library)?
# And what if he wants to add methods that apply on this type?
# -> He should extend it with its own type, like this:

use myLibrary/Hero

export type = 
	...Hero

	from (name: String) =>
		name
		strength = 10

	from (hero: Hero) => { ...hero }

# We just defined a function named Hero
type Hero =
	name: String
	strength: Number

create Hero() =>
	name = "Zabu"
	strength = 1213