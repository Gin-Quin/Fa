# In Fa there are three types of source files :

# -- 1. Modules ---------------------------------
# 	extension : .fa
#  naming : the first letter is lowercase
#	JS equivalent : a JS module file
#	C++ equivalent : namespace
#	Privacy : identifiers beginning by '_' are private to the module and cannot be imported by another module
#
# Modules are the most common fa source files. They can contain definitions (functions, classes, variables) and executable code. They are completely similar to JS modules.
#
# The conversion from a Fa module to a C++ namespace is done this way :
# - First, all variables and classes are declared inside the namespace.
# - Second, just after the namespace declaration, the executable code is added.

# Example :

#myModule.fa
let y = 12
class Zabu
	x = y
let zabu1 = Zabu
y = 9
let zabu2 = Zabu

#JS
export let y = 12
export class Zabu {
	constructor() {
		this.x = y
	}
}
export let zabu1 = new Zabu
y = 9
export let zabu2 = new Zabu

#C++
namespace myModule {
	int y = 12;
	class Zabu {
		int x;
		Zabu() {
			x = y;
		}
	};
	Zabu zabu1;
	Zabu zabu2;

	void ___init() {
		zabu1 = Box<Zabu>();
		y = 9;
		zabu2 = Box<Zabu>();
	}
}
myModule::___init();


# -- 2. Classes ---------------------------------
# 	extension : .fa
#	sub-extension : indicates the class file (in the same folder) to derive from
#  naming : the first letter is uppercase
#	JS equivalent : a JS module file containing a class as default export
#	C++ equivalent : class
#	Privacy : identifiers beginning by '_' are protected ; identifiers beginning by '__' are private

# Example :

#Dog.Animal.fa
legs = 4
speed = 125

yell
	print "Wouaf!"

#JS
export default class Dog extends Animal {
	constructor() {
		super(...arguments)
		this.legs = 4
		this.speed = 125
	}

	yell() {
		console.log("Wouaf!")
	}
}

#C++
struct Dog : Animal {
	Dog() : Animal() {
		legs = 4;
		speed = 125;
	}

	void yell() {
		cout << "Wouaf!" << endl;
	}
};


# -- 3. Data objects ---------------------------------
# 	extension : .do
#  naming : the first letter is lowercase (raise an error if uppercase)
#	JS equivalent : JSON / a JS module file containing an object as default export
#	C++ equivalent : a runtime-readable structure
#	Privacy : every property is public

# Example :

#config.do
fps = 60
screen:
	width = 1140
	height = 2460
	colors = 32
url = "https://config.do/data"
authors:
	- "Elzpulnag"
	- "Corocico"
	- "Bazu el demonio"

#JS
export default const {
	fps: 60,
	screen: {
		width = 1140,
		height = 2460,
		colors = 32,
	},
	url: "https://config.do/data",
	authors: [
		"Elzpulnag",
		"Corocico",
		"Bazu el demonio",
	]
}

#C++
struct {
	int fps {60};
	struct {
		int width {1140};
		int height {2460};
		int colors {32};
	} screen;
	string url {"https://config.do/data"};
	vector<string> authors {
		"Elzpulnag",
		"Corocico",
		"Bazu el demonio"
	};
} config;
