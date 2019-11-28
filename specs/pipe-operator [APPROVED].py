
# The pipe operator enable you to chain function calls
# It is similar to the pipe in shell '|'

func1 |> func2  # func1 is called and its arguments are passed to func2
# is the same as :
func2 func1

func1 |> func2 |> func3
func3(func2(func1))

# This operator has two great benefits :
# - readability
# - you can split the call in multiple lines (instead of having one huge line)



# I'm not sure about the interest of this operator, since Fa is an object-oriented language
# And this type of behaviour is used mostly with functions (not methods)

# Example : we want to apply transformations to an object
myObject
|> rotate 90°
|> translate 20
|> skew 50%
|> draw

# this would rather be done with the story operator :
myObject..
	rotate 90°
	translate 20
	skew 50%
	draw

# which have a cleaner syntax

# Buuuuuut...
# Sometimes we do want to call chained functions which are not part of an object
# In those cases, the pipe operator is huge. It really makes things cleaner.