# big methods are a way to write functions with a lot of arguments
# when a function has too much arguments, you can't have them described in line
# so I proposed this syntax :

# big method
method
<- arg1 : Integer
<- arg2 = "321321"
<- ...
-> returnType
	..code..

# But I decided to remove this syntax for several reasons :
# - First, I was not 100% convinced by the syntax. Having to write '<-' at the beginning of every line is boring
# - And second, functions are meant to have few arguments. A function with many arguments is bad practice.
# The present function syntax is made for small and fast elements, and it should be this way.