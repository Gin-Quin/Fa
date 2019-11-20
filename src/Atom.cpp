

#include "Atom.hpp"
#include <iostream>  // for debugging only
#include <algorithm>  // for copy


using namespace std;



// -- Création d'un nouvel atome
Atom::Atom() {
	if (type != VAR) property |= TYPED;
	if (type != LET) property |= WRITABLE;
}

// (depuis un type)
Atom::Atom(Type t) {
	type = t;
	Atom();
}

// (depuis un type sous forme de chaîne de caractères)
Atom::Atom(string s) {
	//on traitera le moment venu le cas des initialisations particulières (tableaux, objets, instances, ...)
}





// -- Destruction d'un atome
Atom::~Atom() {
	clear();
}

void Atom::clear() {
	isNull(true);

	if (type == STRING && value.str)
		delete value.str;
	else if (type == OBJECT && value.object)
		delete value.object;
	else if (type == ARRAY && value.array)
		delete value.array;

	// gérer le cas des instances et des getsets

}




// -- Assignations
Atom& Atom::operator= (Atom& v) {
	cout << "Assign from another Atom" << endl;

	// si la variable-cible est nulle, on retourne (on a déjà free la variable)
	if (v.isNull()) {
		clear();
		return *this;
	}

	// sinon, on agit en fonction de son type
	switch (v.type) {
		case INT:
			*this = v.value.integer;
			break;
		case NUMBER:
			*this = v.value.number;
			break;
		case BOOL:
			*this = v.value.boolean;
			break;
		case STRING:
			*this = v.value.str;
			break;
		case ARRAY:
			*this = v.value.array;
			break;
		case OBJECT:
			*this = v.value.object;
			break;
	}

	return *this;
}



// vérifie 
void Atom::checkLock() {
	if (isLocked())  // si la variable est bloquée, erreur
		throw "Can't assign a value to a constant.";
	if (!isWritable())  // sinon, si elle était constante, elle est dorénavant bloquée
		isLocked(true);
}




// = int
Atom& Atom::operator= (int v) {
	cout << "Assign from int" << endl;
	*this = static_cast<faInt>(v);
}
Atom& Atom::operator= (faInt v) {
	cout << "Assign from faInt" << endl;
	checkLock();
	clear();  // on libère la mémoire anciennement allouée par la variable
	isNull(false);

	// si le type est fixé, on transforme en le type donné
	if (isTyped()) {
		if (type == INT)
			value.integer = v;
		else if (type == NUMBER)
			value.number = static_cast<faNumber>(v);
		else if (type == STRING)
			value.str = new faString(to_string(v));
		else if (type == BOOL)
			value.boolean = !!v;
		else throw "Trying to assign an integer to a non-primitive variable.";
	}
	else {
		type = INT;
		value.integer = v;
	}

	return *this;
}






// = number
Atom& Atom::operator= (float v) {
	cout << "Assign from float" << endl;
	*this = static_cast<faNumber>(v);
}
Atom& Atom::operator= (faNumber v) {
	cout << "Assign from faNumber" << endl;
	checkLock();
	clear();
	isNull(false);

	// si le type est fixé, on transforme en le type donné
	if (isTyped()) {
		if (type == INT)
			value.integer = static_cast<faInt>(v);
		else if (type == NUMBER)
			value.number = v;
		else if (type == STRING)
			value.str = new faString(to_string(v));
		else if (type == BOOL)
			value.boolean = !!v;
		else throw "Trying to assign a number to a non-primitive variable.";
	}
	else {
		type = NUMBER;
		value.number = v;
	}

	return *this;
}






// = bool
Atom& Atom::operator= (faBool v) {
	cout << "Assign from bool" << endl;
	checkLock();
	clear();
	isNull(false);

	// si le type est fixé, on transforme en le type donné
	if (isTyped()) {
		if (type == INT)
			value.integer = 1;
		else if (type == NUMBER)
			value.number = 1.0;
		else if (type == STRING)
			value.str = new faString(v ? "true" : "false");
		else if (type == BOOL)
			value.boolean = v;
		else throw "Trying to assign a boolean to a non-primitive variable.";
	}
	else {
		type = BOOL;
		value.boolean = v;
	}

	return *this;
}







// = string
Atom& Atom::operator= (const char *v) {
	cout << "Assign from const char *" << endl;
	faString s(v);
	*this = s;
}
Atom& Atom::operator= (faString* v) {
	cout << "Assign from faString *" << endl;
	*this = *v;
}
Atom& Atom::operator= (faString& v) {
	cout << "Assign from faString" << endl;
	checkLock();
	clear();
	isNull(false);

	// si le type est fixé, on transforme en le type donné
	if (isTyped()) {
		if (type == INT)
			value.integer = stoi(v);
		else if (type == NUMBER)
			value.number = stod(v);
		else if (type == STRING)
			value.str = new faString(v);
		else if (type == BOOL)
			value.boolean = !!(v.length());
		else throw "Trying to assign a string to a non-primitive variable.";
	}
	else {
		type = STRING;
		value.str = new faString(v);
	}

	return *this;
}








// -- Gestion du type
const Type Atom::getType() {
	// gérer le cas des getset
	return type;
}


const string Atom::getTypeName() {
	return getNameFromType(type);
}


const Type Atom::getTypeFromName(faString t) {
	if (t == "int")		return INT;
	if (t == "number")	return NUMBER;
	if (t == "boolean")	return BOOL;
	if (t == "string")	return STRING;
	if (t == "function")return FUNCTION;
	if (t == "array")	return ARRAY;
	if (t == "object")	return OBJECT;
	if (t == "class")	return CLASS;
	if (t == "instance")return INSTANCE;
	return UNKNOWN;
}


const string Atom::getNameFromType(Type t) {
	if (t == INT)		return "int";
	if (t == NUMBER)	return "number";
	if (t == BOOL)		return "bool";
	if (t == STRING)	return "string";
	if (t == FUNCTION)	return "function";
	if (t == ARRAY)		return "array";
	if (t == OBJECT)	return "object";
	if (t == CLASS)		return "object";
	if (t == INSTANCE)	return "object";
	return "unknown";
}












// -- Gestion des propriétés
const bool Atom::isTyped()		{ return property & TYPED; }
const bool Atom::isWritable()	{ return property & WRITABLE; }
const bool Atom::isNull()		{ return property & NIL; }
const bool Atom::isSealed()		{ return property & SEALED; }
const bool Atom::isEnumerable()	{ return property & ENUMERABLE; }
const bool Atom::isStatic()		{ return property & STATIC; }
const bool Atom::isLocked()		{ return property & LOCKED; }



bool Atom::isTyped(bool b) {
	if (b) property |= TYPED;
	else property &= ~(TYPED);
	return b;
}
bool Atom::isWritable(bool b) {
	if (b) property |= WRITABLE;
	else property &= ~(WRITABLE);
	return b;
}
bool Atom::isNull(bool b) {
	if (b) property |= NIL;
	else property &= ~(NIL);
	return b;
}
bool Atom::isSealed(bool b) {
	if (b) property |= SEALED;
	else property &= ~(SEALED);
	return b;
}
bool Atom::isEnumerable(bool b) {
	if (b) property |= ENUMERABLE;
	else property &= ~(ENUMERABLE);
	return b;
}
bool Atom::isStatic(bool b) {
	if (b) property |= STATIC;
	else property &= ~(STATIC);
	return b;
}
bool Atom::isLocked(bool b) {
	if (b) property |= LOCKED;
	else property &= ~(LOCKED);
	return b;
}











// -- convertisseurs
const faInt Atom::toInt() {
	if (isNull())
		return 0;
	if (type == INT)
		return value.integer;
	if (type == STRING)
		return stoi(*value.str);
	if (type == NUMBER)
		return static_cast<faInt>(value.number);
	if (type == BOOL)
		return value.boolean ? 1 : 0;

	throw "Can't convert a non-primitive to an integer.";
}
Atom::operator int() {
	std::cout << "Converting to int : " << toString() << std::endl;
	return static_cast<int>(toInt());
}
Atom::operator faInt() {
	std::cout << "Converting to faint : " << toString() << std::endl;
	return toInt();
}

const faNumber Atom::toNumber() {
	if (isNull())
		return 0.0;
	if (type == INT)
		return static_cast<faNumber>(value.integer);
	if (type == STRING)
		return stod(*value.str);
	if (type == NUMBER)
		return value.number;
	if (type == BOOL)
		return value.boolean ? 1.0 : 0.0;

	throw "Can't convert a non-primitive to a nummber.";
}
Atom::operator float() {
	std::cout << "Converting to float : " << toString() << std::endl;
	return static_cast<float>(toNumber());
}
Atom::operator faNumber() {
	std::cout << "Converting to fanumber : " << toString() << std::endl;
	return toNumber();
}


const faBool Atom::toBool() {
	if (isNull())
		return false;
	if (type == INT)
		return value.integer ? true : false;
	if (type == STRING)
		return value.str->length() ? true : false;
	if (type == NUMBER)
		return value.number ? true : false;
	if (type == BOOL)
		return value.boolean;

	throw "Can't convert a non-primitive to a boolean.";
}
// operator bool() const;  //!\\ ATTENTION : bien checker les effets secondaires



const faString Atom::toString() {
	if (isNull())
		return "null";
	if (type == INT)
		return std::to_string(value.integer);
	if (type == STRING)
		return *value.str;
	if (type == NUMBER)
		return numberToString(value.number);
	if (type == BOOL)
		return value.boolean ? "true" : "false";
	if (type == FUNCTION) {
		return "Function";
	}
	if (type == ARRAY) {
		return "faArray";
	}

	return "faObject";
}
// const operator String();


// fonctions-outils (à déplacer dans Tools)
const std::string Atom::numberToString(faNumber d) {
	std::string s = std::to_string(d);

	// on supprime les '0' inutiles à la fin
	if (s.find_first_of('.')) {
		s.erase(s.find_last_not_of('0') + 1);

		// si ça termine par un '.', on l'enlève
		if (s.back() == '.')
			s.pop_back();
	}

	return s;
}

