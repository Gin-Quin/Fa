
# Instead of using simple constructors, I should find a nice way to instantiate objects
# using the default constructor (object copy) or the given constructor

class MyClass zabu=12 zinzin=11
	zabu
	zinzin = zabu + zinzin


# peut remplacer :

class MyClass
	zabu
	zinzin

	MyClass _zabu=12, _zinzin=11
		zabu = _zabu
		zinzin = _zabu + _zinzin


let cl = MyClass zabu: 212, zinzin: 545

# Question : is this really useful?
# It can create ambiguities about class field values and constructors

# In C++, we have to do this kind of very annoying task :

class Foo() {
	int a;
	int b;
	
	Foo(int _a, int _b) {
		a = _a
		b = _b
	}
};

auto foo = new Foo(5, 12);

# But this kind of field-initialization can be done very simply in Fa:
class Foo
	a: Integer
	b: Integer
	
	# (useless)
	from a:Integer, b:Integer
		self.a = a
		self.b = b

let foo = Foo(5, 12)
let foo = Foo:
	a = 5
	b = 12