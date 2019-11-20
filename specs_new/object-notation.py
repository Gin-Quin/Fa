
# Object notation

# La définition du langage Fa part de son système de description d'objets - qui doit être la plus simple possible.

# Pour cela, je m'inspire du YAML et du TOML. Particulièrement du YAML car Fa est un langage indenté.

# Un langage de notation d'objet doit pouvoir définir :

# - des scalaires (entiers, )
# - des objets lol,
# 	- ces objets peuvent avoir un type (instantier des classes)
# - des tableaux (qui peuvent aussi avoir des propriétés comme des objets)
# 	- ces tableaux peuvent aussi avoir un type


## Proposition avec signe égal


name = "Hercule"
strength = 12
friends: 
	type = "sticky"

monture = Dog:
	name = "Hercule"

friends = [Person]:
	-	Hero:  # inherits from Person
		strength = 900
	-	SuperHero:    # inherits from Person
		strength = 3213

	dog = Dog:
		name = "My beloved"





#  TEMPLATE
- Row = "Coucou tout le monde !"  if ...
	@strength = 125
	width = parent.width - 120
	height = 13320%
	
	hover -> width + 51321
	active -> hover && click
	fullName -> "{firstname} {lastname}"

	:hover
		width = 12
	:active

	~ Coucou toi !!

	- Stack
	~ Hey toi !
	~ Comment ça va ?
	~ Hey toi !
	~ Hey toi !
	- List

	- Stack = x
		width = 120
		height = 11
	~ Hey toi !
	~ Comment ça va ?
	- List
		color blue

	- Stack
		width = 120
		height = 11
	~ Hey toi !
	~ Comment ça va ?
	= x
	- List
		color blue
