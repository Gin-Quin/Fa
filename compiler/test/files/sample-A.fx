
import FileSystem

> Multiline strings
let xml = XML ...
	<p>I'm XML...</p>
	   <b>That's awesome right???</b>
			I can indent how I want

> Template strings
let msg = 'I am some Fa code!'
print "Hi! " + msg

> For loop in file system
for file in ||*.js|fs|
	if file != "zabu.js"
		print file

> Regular expression
let match = //zabu|co\/co/gi/.match("zabu")

> Comprehension
add x for x in 0..5


let happiness = 1235  # this is a very huge value for happiness
print "My \`happiness\` is : {happiness.toUpperCase} and my will is {will * 10}"
print "And for my friend, his happiness is : {friend.happiness} and his will is {friend.will * 10}"

if happiness > 1000
	print `I'm very happy!`

#description
	This is a doc comment for a method named `myCommand`
	It can have multiple lines
myCommand
	<- leDepuisFile:    Integer  # File de la pièce à déplacer
	<- leDepuisRangée:  Integer  # Rangée de la pièce à déplacer
	<- leVersFile:      Integer  # File de la case de destination
	<- leVersRangée:    Integer  # Rangée de la case de destination
	->  Boolean                  # true si le mouvement d'échec est valide
