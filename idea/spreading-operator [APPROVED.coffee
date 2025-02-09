
# I want Fa's spreading operator to act in the most *exact* way possible
# Of course, there will be special cases where Fa and Js are too different

#   A) First use-case : spread arguments
# -----------------------------------------
method name: String, args: ...Number
	# here, args is nothing more than an array of numbers
	print args.length
	print args[0]
	method 'Coucou', 5, 3, 12, 521

# En JS :
method(name, args) {
	console.log(args.length)
	console.log(args[0])
	method('Coucou', [5, 3, 12, 521])
}

# En C++ :
void method(string name, vector<double> args) {
	cout << args.length << endl;
	cout << args[0] << endl;
	method('Coucou', {5, 3, 12, 521})
}


#   B) Second use-case : spread properties
# ------------------------------------------
# inside objects and array-like declarations
let objA:
	x = 3213
	y = 65321
	z = 65321

let objB:
	...objA
	y = 321312

let objC = { ...objA, y = 321321 }

let listA:
	- 321
	- 3213

let listB:
	- 12
	- ...listA

# En JS, la conversion est assez évidente
# En C++, les choses peuvent être plus compliquées
# une technique serait de recopier à la main toutes les propriétés de objA :
struct {
	int x {3213};
	int y {65321};
	int z {65321};
} objA;  # I really should name those unnamed structures :D Life will be easier

struct {
	int x {objA.x};
	int z {objA.z};
	int y {321312};
} objB;



#   C) Third use-case : multi-line strings
# ------------------------------------------
# if the triple comma is followed by and an end-of-line, it expects the
# next block to be a multiline string
let zabu = ...
	Hery everyone! I am a multiline string
	Thats cool right

let query = GraphQL...
	heroes {
		name
		id
		friends {
			name
		}
	}

let xml = XML...
	<p>Hola here.<p>
	<p>I am some <b>sweeet<b> XML<p>

# That's awesome, right ? :D

class Hour is Component
	hour = 11
	minute = 12

	render -> HTML...
		<p>Time is {hour}:{minute}</p>
