@functor
import Zabu


# The bare properties proposal is abandonned, because it can create confusing syntaxes


# This proposal is about bare properties
# Bare properties means no equal sign to define properties
# It's inspired by Stylus
# Example :


class Animal
	name String
	race String
	size Integer
	owner Human:
		name "Coco"
	speed Integer
	hp Natural
	strength Natural

	yell() -> String
	attack(target Animal)

	add(
		z : String
		x = 0
		y = 0
		onBefore?() -> Integer
		onAfter() -> Integer
		obj:
			g = 12
			h = "zabu"
	) -> x+y

	aFunc(x:Integer, y:Integer, options:{

	})

	bigMethod (
		msg? String
		depth 0
		onFinish? function<msg String, afterFinished(status:Integer)>
	)

class Dog extends Animal
	size 6
	race "Dog"

Dog()

startTheGame()
	let h = Dog:
		name "Hercule"
		strength 14

	let myDogzz = <Dog>:
		-	strength 132
			will 321
		-	strength 45
			will 480

	let options:
		- 321
		- 5454
		- 5321


execute
	startTheGame()

# Autre test de méthode
createPopup():
	width 32132
	height 321321
->
	if width < 21321
		print "Hey"
