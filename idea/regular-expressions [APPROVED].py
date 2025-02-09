
# regular expressions are created with //...//
let myregex = //zabu|coco/gi/

# In order to create this Fa <-> Javascript API compatibility,
# regular expressions are exactly the same as in Javascript (same methods)

# Fa also add globs :
for file in ||/dev/**/*.js||
	print file

if "Hello you" match ||Hello*||
	print "He is saying hello!"
