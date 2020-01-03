
# Description of the different to initialize values / functions / objects
# Inside the main scope of a .fa file, all the 'let' are to be removed

# scalar
let x = 9

# typed with no value
let x : Type

# typed with value
let x : Type = 9

# method / functions
let method arg1, arg2, ... -> returnType
	..code..

# pointeurs de fonction (aka callbacks)
let *method arg1, arg2, ... -> returnType
	..code..  # optional (if not defined, the function is set to null (in C++))

# big method
let method
<- arg1
<- arg2
<- ...
-> returnType
	..code..

# lambda
let method = (..args..) -> return

# anonymous structures
let x :
	y = 12
	z = 11

# typed structures / classes
let x = Type :
	y = 12
	z = 11
# typed structures / classes with constructor call
let x = Type(..args..) :
	y = 12
	z = 11
