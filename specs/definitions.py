# the old system had two definition types : in classes and in methods (or control structures)
# definitions in control structures needed the use of the 'let' keyword
# I didn't like this need. Now both use the same syntax without the 'let'


# scalar
x = 9

# typed with no value
x:Type

# typed with value
x:Type = 9

# method (only in classes)
method arg1, arg2, ... -> returnType
	..code..

# pointeur de fonction (on ajoute un `:`)
method : arg1, arg2, ... -> returnType
	..code..  # optional (if not defined, the function is set to null (in C++))


# big method (only in classes)
method
<- arg 1
<- arg 2
<- ...
-> returnType
	..code..


# lambda
let method arg1, arg2, ... -> returnType
	..code..

#lambda (type-only)
let method -> returnType
	..code..

# small lambda
method = (..args..) -> return

# small lambda (type-only)
method : (..argsTypes..) -> void

# big lambdas
method
<- arg 1
<- arg 2
...
-> returnType / ..code..
	..code..

# anonymous structures
x:
	y = 12
	z = 11

# typed structures / classes
x = Type
	y = 12
	z = 11

x = Type:  # call a constructor with the arguments y and z
	y: 12
	z: 11

# typed structures 
# parenthsesis and colon are optionals
x = Type(..args..)
	y = 12
	z = 11

# this syntax is similar to a BigFunction call :
x = func:
	y: 12
	z: 11

# to avoid confusion : forbid non-classes to start with a capital letter? Maybe!
# -> I don't like that kind of obligations.. I want people to feel free
# -> this would lead to sweet and easy syntax highlighting
# -> what about uniques?