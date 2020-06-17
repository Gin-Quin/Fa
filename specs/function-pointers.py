
# Function pointers can be defined by :

#1: Using the Function type with a signature
class Button
	hover: Function
	swipe: Function[-> Integer]
	click: Function[MouseEvent ->]
	dblClick: Fuction[MouseEvent -> Integer]



#2: Using the shorthand parenthesis syntax
class Button
	hover: (->)
	swipe: (-> Integer)
	click: (MouseEvent ->)
	dblClick: (MouseEvent -> Integer)

class Button
	hover: ->
	swipe: -> Integer
	click: MouseEvent ->
	dblClick: MouseEvent -> Integer


class Coco
	increment = x:Integer -> x + 1 
	increment x:Integer -> x + 1

# Then we can use it this way :
let button: Button
	hover ->
		print "Hover"

	swipe -> 0
		print "Hover"

	click event ->
		print "Click!!"

	dblClick event ->
		print "DblClick!!"
		return 1


button.click = event ->
	if event is EnvironmentError
		print "WTF"
	else
		print "That's cool dude..."
	
