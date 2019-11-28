

# Defining static elements can be done with the 'static' or 'const' keywords
class Math
	const PI = 3.14957
	static count = 0
	squared x:Number -> x * x

# Accessing static elements is done like in Javascript (and like dynamic elements) with the dot
const PI = Math.PI
const PI from Math

# The syntaxic coloration and the fact we use a capital letter help to understand we access a static element (and not a dynamic one)

# There is no obligation but a constant should be fully uppercased