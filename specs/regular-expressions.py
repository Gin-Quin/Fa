
# regular expressions are created with (/.../)
let myregex = ( /zabu|coco/gi )
# note that the parenthesis are mandatory!

# In order to create this Fa <-> Javascript API compatibility,
# regular expressions are exactly the same as in Javascript (same methods)


# Fa also add globs :

for file in |*.js|
	print file  # file is an object which have a `to String` method

if "Hello you" match (|Hello*|fs)
	print "He is saying hello!"

for file in (|/dev/**/*.js|fs)
	print file

# Question : should I use the |...| operator, or the (|...|) with parenthesis?
for file in ( |/dev/**/*.js| )
	print file
