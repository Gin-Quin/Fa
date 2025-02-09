# Sometimes you retrieve POJOs from a library or a network
# How to make it possible to call methods on POJOs?

# Let's suppose we use a 3rd party library that returns a type and a function to get this type
use MyLibrary: fetch, FetchedType

# Then we need to create a wrapper type on FetchedType


# Structure:
# /MyLibraryWrapper/fetch.fa
# /MyLibraryWrapper/FetchedType.fa


# /MyLibraryWrapper/FetchedType.fa:
use MyLibrary: FetchedType
export default type extends FetchedType =
	toString() => "Heyyy I'm a method on a POJO"

# /MyLibraryWrapper/fetch.fa:
use MyLibrary: fetch
import FetchedType
export default fetch as (...Parameters<fetch>) -> FetchedType

# One huge disadvantage is that forces to wrap not only the type but every function that returns this type... which is a no-go

# Another solution would be to only apply methods on the given type:

# /MyLibraryWrapper.fa:
use and export MyLibrary # we export everything from the library we wrap

extends MyLibrary.FetchedType: # we extend the POJO type with some methods
	toString() => "Heyyy I'm a method on a POJO"

# maybe the "extends" statement should act like a "use and export?"
extends MyLibrary/FetchedType:
	toString() => "Heyyy I'm a method on a POJO"
extends import MyLibrary/FetchedType:
	toString() => "Heyyy I'm a method on a POJO"

# this will define a method "toString(self: FetchedType) -> String" that will be automatically imported with this file
