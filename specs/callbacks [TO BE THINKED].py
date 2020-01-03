
# Let's say we have a class Button with a method `click` that is intended
# to be used as a callback :

button = Button:
	click(event)
		print "Click!!"

	dblClick(event) -> 1
		print "DblClick!!"


# The question is : what syntax should have the class?

#1: Same as a normal method but with an asterisk * before the method name
class Button
	*click MouseEvent
	*dblClick MouseEvent -> Integer

#2: Using a colon and parenthesis around the type
class Button
	click: (MouseEvent)
	dblClick: (MouseEvent) -> Integer


# I think I like more the first syntax because of its visual grep
# A first glance makes it clear click and dblClik are overridable functions
