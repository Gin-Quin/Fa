

use MyLibrary
import MyModule
export Animal, Hero

### Description
	Ceci est un commentaire sur plusieurs lignes
	Il se termine quand j en ai envie

callZabu x:String -> Zabu
	@description  # this is a decorator, can be used to generate doc
		This file is a .. dskjdfqslkdjfh lsqkjfdl
	@param zabu
		zereazeaze
	@authors
		Matthieu Gindre
		Quentin Franchette

	if zabu = myZabus.next
		return zabu


"coucou" + XML ~ <p>

@description
	This file is used to be a sample
@authors
	Matthieu Gindre
	Quentin Franchette

file <<< zabu  # opérateur dinsertion
file >>> json  # opérateur d'extraction

stream1 <<< stream2
stream2 >>> stream1

321 >>> cout
cout <<< 321

myFile
>>> name : String
>>> strength : Integer

string = "Hey there :)"
xml = XML "<p>Je suis du xml!!</p>"

longString = ...
	Je suis une longue chaîne de caractères
	Les premières tabulations ne comptent pas.

bigXML = GraphQL {}

bigXML = XML ...
	C est plutôt cool non ?
	<p>Tout ceci est plutôt cool en effet lol</p>

uris:
	- '/long/way/home'
	- 'node_modules/'

- Row  for zabu in coco
	width = 50%
	tiny = zabu.x < 12

	:hover
		backgroundColor = blue

	:tiny
		width = 33%
		color = red

	if tiny
		- "Coucou"
		- Label zabu.name

	- Text 'Yo {name}!'
		color = red

	- Bold-Italic "Comment ça va?"
	- Row-Cell x  for x in 0..12

	- "Salut toi :)"
	- "x vaut : {x}."
	- x + 15221
	- "Chaque saut à la ligne est *important*"
	- "Voir la [documentation]()"
	- Row
	- List
		- "Hey there!"
		- Bold "How are you?"
		- Link "See my website"
			href = 'lepzulnag.tech'


zabu : Map[Integer]
zabu : Map[Int, Int]
zabu : [[Integer]]

x = { zabu = 12, caca = 421, sympa : String, lolo }

y : String
x :
	zabu = 12
	lolo
	sympa : String
