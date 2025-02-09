# Fas has built-in reactivity ✨
# This means you can declare properties that are derived from other properties
# This derived properties are called "reactive"
# There can also be reactive statements

export type Button =
	...Container
	...Focusable, Activable, Ripplable

export type Text =
	...Node
	default reactive value = ""

# so, what does the "reactive" keyword means?
# it means this:

var firstName = "John"
var lastName = "Doe"
let text = Text("Hello {firstName} {lastName} :)")

print: text.value # prints "Hello John Doe :)"

firstName = "Foo"
lastName = "Bar"

print: text.value # prints "Hello Foo Bar :)"


# awesome, right?


# How does it work? I don't know yet :p
# By reassigning every time a property change

# Possibility #1 - Every property change triggers a new assignment
var name = "John Doe"
const text = Text(`Hello ${firstName} ${lastName}`)

firstName = "Foo"
lastName = "Bar"
text.value = `Hello ${firstName} ${lastName}` # <-- automatically added by the transpiler

# Question: how to deal with multiple changes?
# Chaining changes must be detected


# -> it's in the module that uses a reactive value that reactivity must be set
# Example with two modules:

# ./Text.fa
use Flame/Node

export type Text is Node =
	reactive value = ""

# ./Button.fa
use Flame/Node
import Text

export type Button is Node =
	content: String
	- Text: "{content}!!!"

# ./Other.fa
import Button

let button = Button("foo")
button.content = "bar"

# This will produce the following JS:

# ./Button.js
export const Button = (content: string) => {
	const self = []
	self.content = content
	self.push(Text(`${self.content}!!!`))
	return self
}

# ./Other.js
const button = Button("foo")
button.content = "bar" # triggers an update of a reactive value
button[0].value = `${button.content}!!!` # <-- the update


## Declaring reactive values to DOM default nodes will automatically bring
## reactivity to the screen :D
# Example:
declare type Node =
	reactive children: [...Node]

declare type HTMLCheckboxElement is Node =
	reactive checked: boolean



# REACTIVITY CONCEPTS

# Reactivity can be thought in two ways:
# (a) - from the element that triggers changes
# (b) - from the element that pull changes

# (a) example
type Button =
	reactive value = ""

let button = Button(value = 12)
let buttonValue = button.value # now every reference will be reactive

# (b) example
type Button =
	value = ""

let button = Button(value = 12)
reactive buttonValue = button.value # we declare a value that "subscribes" to another value

# I think (b) is better than (a) because it brings more control over what is reactive or not
