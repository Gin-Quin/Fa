extends View<Template>


View circled
	borderRadius = 100%
	onClick -> print "Circle clicked!"


View bigger
	width = 84
	height = width

View black
	color = BLACK

let friends[String]:
	- "Coco"
	- "Tutu"
	- "Tata"
	-	name = "zut"
		friends = "none"
		strength = 187687


let fruits:
	count = 11
	-	"Banana"
	-	"Coco"
	-	11
	-	name = "coco"
		length = "12"
		friends:
			- "Zabu"
			- "Coco"


let friends[[Being]]:
	-	-	-	name = "coco"
				friends = 2
			-	name = "zabu"
				friends = 4
		-	-	name = "coco"
				friends = 9


x = [4, 5] # tuple
x[Int] = [4, 5]  # tuple to array
x:CustomType<String>
x <CustomType<String>>
x String = 12
x = String 12

x[Int] = 12
x = [Int] 12


let x: Integer = 12
let z: String = 121
let coco: [String] = ["Hey", "You"]
let zabu: Map<String>

let zabu<String> = [hey: "You", what: "The hell"]

friends = MyArray<String> []
zabu = String ""
coco = Number 12

type Point = [x: Integer, y: Integer]



zabu(x: Integer, y: Integer) -> (x: Integer, y: Integer)

Point [5, 5]

come -> 12
	print "Wouah"
	add 4, 5 |> 
	call Zabu


add x:Integer, y:Integer => Zabu:
	x: 125
	z: 114
	options:
		x: 124

x, y => x + y
x, y -> print x + y




add(x: Integer, y: Integer) -> 
let x = 125..coco()
print(add(5, 4))
add 5, 4 |> print
print <| add 5, 4 <| 

add(5, add(7, add(2, 1)))
add((5, 4), 12)
drawPoint [x: 5, y: 4]
drawPoint(x: 5, y: 4)
drawPoint()
x : String, String -> String, String
	return x, y
y = 12

friends << {name: "Coco", strength: 12}

- "Hey"
- "You"
if x is strong
	- "How are you"

weak coco:Animal
let *coco = x

let zabu:
	if x == 9
		z = 11
	else
		z = 121

let zabu = {}
if (x == 9)
	zabu.z = 11
else
	zabu.z = 12


friends <<
	if x == 9
		-	name = "coco"
			strength = 12
		-	name = "Hercule"
			strength = 120
	-	name = "zabu"
		strength = 11
	

# comment compiler ça en JS ??
friends.push(
	...(x == 9 ? [{name: "coco", strength: 12}, {name: "Hercule", strength: 120}] : []),
)

if (x == 9)
	friends.push({name: "coco", strength: 12}, {name: "Hercule", strength: 120})
friends.push({name: "zabu", strength: 11})

friends.push({name: "Coco", strength: 12})


square x:Integer -> x + 121


square x:Integer -> x + 1
	x + 121

call @Hero ->


options =>
	x = 12

render =>
	-	Table
		cursor = default
		
		-	Header
			-	Module "Module souscrit"

			-	List for key in columns
				onHover -> width = 500
				onClick -> width = 800

				if key == 'selected'
					-	Checkbox ...circled, ...bigger, ...black
						...circled  if key == 'names'
						...bigger  if key != 'zototo'
						...black
						value = selectAll
				else
					- Text key
