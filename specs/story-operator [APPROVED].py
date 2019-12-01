
# the story operator is .. followed by a newline and block
# if not followed by a newline, it's a *range operator*
myObject ..
	x = 32132
	y = 2131
	objectCommand
# EN JS -> (myObject.x = 32123, myObject.y = 2131, myObject.objectCommand(), myObject)

hercule..
	attack ajax
	attackAll hercule.enemies

hercule.attack ajax  # -> hercule.attack
hercule..
	attack ajax  # -> hercule

# The story operator is awesome!