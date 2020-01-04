
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

# I used to doubt about the usefulness of this operator, but...
# Sometimes we do want to call chained functions which are not part of an object
# In those cases, the pipe operator is huge. It really makes things cleaner.

# And there is another huge benefit : chaining big function calls
makePopup :
	width = 513
	height = 640
	borderless = false
	closable = true
	title = "Yo I'm a popup"
|> show hideOtherPopups: true
|> shake
