#pragma once

#include <string>
#include <unordered_map>
#include <vector>

#include "Type.hpp"
using Property = unsigned char;

// #include "Array.hpp"
// #include "Object.hpp"



class Atom;  // définie en-dessous



// innerTypes
using faInt 	=	long long;
using faNumber 	=	double;
using faString 	=	std::string;
using faBool 	= 	bool;
using faArray 	= 	std::vector<Atom>;
using faObject 	= 	std::unordered_map<faString, Atom>;





/**
* Définit la classe Atom qui est la base de toutes les données en Fa.
* Un atom prend 16 octets de mémoire.
* Les strings et les conteneurs utilisent de la mémoire allouée en plus.
*/
class Atom {

private:
	Type type = VAR;  // type de la variable
	Property property = NIL;  // liste des propriétés
	
	union {
		faInt integer;  // entier
		faNumber number;  // nombre décimal
		faBool boolean;  // booléen
		faString *str;  // chaîne de caractères
		// faFunction *function;
		// faGetset *getset;
		faArray *array;  // tableau
		faObject *object;  // objet
	} value;


	static const Property TYPED = 1;
	static const Property WRITABLE = 2;
	static const Property NIL = 4;
	static const Property SEALED = 8;
	static const Property ENUMERABLE = 16;
	static const Property STATIC = 32;


public:
	// création / destruction
	Atom() {
		init();
	}
	~Atom() {
		clear();
	}

	// création/assignement/conversion depuis un autre atome
	// Atom(Atom& v) {
	// 	init();
	// 	*this = v;
	// }
	Atom& operator=(Atom& v) {
		std::cout << "Assign from another Atom" << std::endl;
		checkConstance();
		clear();

		// si la variable-cible est nulle, on retourne (on a déjà clear la variable)
		if (v.isNull())
			return *this;

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

	// création
	// template <typename T>
	// Atom(T v) {
	// 	init();
	// 	*this = v;
	// }
	Atom(int v) {
		init();
		*this = v;
	}
	Atom(float v) {
		init();
		*this = v;
	}
	Atom(faInt v) {
		init();
		*this = v;
	}
	Atom(faNumber v) {
		init();
		*this = v;
	}
	Atom(Atom v) {
		init();
		*this = v;
	}

	
	// assignement depuis un int
	// Atom(int v) {
	// 	init();
	// 	*this = v;
	// }
	// Atom(faInt v) {
	// 	init();
	// 	*this = v;
	// }
	Atom& operator=(int val) {
		std::cout << "Assign from int" << std::endl;
		*this = static_cast<faInt>(val);
	}
	Atom& operator=(faInt val) {
		std::cout << "Assign from faInt" << std::endl;
		checkConstance();
		clear();  // on libère la mémoire anciennement allouée par la variable
		isNull(false);

		// si le type est fixé, on transforme en le type donné
		if (isTyped()) {
			if (type == INT)
				value.integer = val;
			else if (type == NUMBER)
				value.number = static_cast<faNumber>(val);
			else if (type == STRING)
				value.str = new faString(std::to_string(val));
			else if (type == BOOL)
				value.boolean = !!val;
			else throw "Trying to assign an integer to a non-primitive variable.";
		}
		else {
			type = INT;
			value.integer = val;
		}

		return *this;
	}


	// assignement depuis un nombre
	Atom& operator=(float val) {
		std::cout << "Assign from float" << std::endl;
		*this = static_cast<faNumber>(val);
	}
	Atom& operator=(faNumber val) {
		std::cout << "Assign from faNumber" << std::endl;
		checkConstance();
		clear();
		isNull(false);

		// si le type est fixé, on transforme en le type donné
		if (isTyped()) {
			if (type == INT)
				value.integer = static_cast<faInt>(val);
			else if (type == NUMBER)
				value.number = val;
			else if (type == STRING)
				value.str = new faString(std::to_string(val));
			else if (type == BOOL)
				value.boolean = !!val;
			else throw "Trying to assign a number to a non-primitive variable.";
		}
		else {
			type = NUMBER;
			value.number = val;
		}

		return *this;
	}


	// assignement depuis un booléen
	Atom& operator=(bool val) {
		std::cout << "Assign from bool" << std::endl;
		checkConstance();
		clear();
		isNull(false);

		// si le type est fixé, on transforme en le type donné
		if (isTyped()) {
			if (type == INT)
				value.integer = 1;
			else if (type == NUMBER)
				value.number = 1.0;
			else if (type == STRING)
				value.str = new faString(val ? "true" : "false");
			else if (type == BOOL)
				value.boolean = val;
			else throw "Trying to assign a boolean to a non-primitive variable.";
		}
		else {
			type = BOOL;
			value.boolean = val;
		}

		return *this;
	}
	

	// assignement depuis une chaîne de caractères
	Atom& operator=(const char *val) {
		std::cout << "Assign from const char *" << std::endl;
		faString s(val);
		*this = s;
	}
	Atom& operator=(faString *val) {
		std::cout << "Assign from faString *" << std::endl;
		*this = *val;
	}
	Atom& operator=(faString &val) {
		std::cout << "Assign from faString" << std::endl;
		checkConstance();
		clear();
		isNull(false);

		// si le type est fixé, on transforme en le type donné
		if (isTyped()) {
			if (type == INT)
				value.integer = stoi(val);
			else if (type == NUMBER)
				value.number = stod(val);
			else if (type == STRING)
				value.str = new faString(val);
			else if (type == BOOL)
				value.boolean = !!(val.length());
			else throw "Trying to assign a string to a non-primitive variable.";
		}
		else {
			type = STRING;
			value.str = new faString(val);
		}

		return *this;
	}
	


	// assignement depuis un tableau
	// Atom& operator=(faArray *array);
	// Atom& operator=(faArray &array);


	// getters de type
	static const Type getTypeFromName(faString t) {
		if (t == "int")		return INT;
		if (t == "number")	return NUMBER;
		if (t == "boolean")	return BOOL;
		if (t == "string")	return STRING;
		if (t == "function")return FUNCTION;
		if (t == "array")	return ARRAY;
		if (t == "object")	return OBJECT;
		return VAR;
	}
	
	static const std::string getNameFromType(Type t) {
		if (t == INT)		return "int";
		if (t == NUMBER)	return "number";
		if (t == BOOL)		return "bool";
		if (t == STRING)	return "string";
		if (t == FUNCTION)	return "function";
		if (t == ARRAY)		return "array";
		if (t == OBJECT)	return "object";
		return "var";
	}

	const Type getType() {
		return type;
	}

	const std::string getTypeName() {
		return getNameFromType(type);
	}

	const bool isTyped()		{ return property & TYPED; }
	const bool isWritable()		{ return property & WRITABLE; }
	const bool isNull()			{ return property & NIL; }
	const bool isSealed()		{ return property & SEALED; }
	const bool isEnumerable()	{ return property & ENUMERABLE; }
	const bool isStatic()		{ return property & STATIC; }



	// setters de type
	bool isTyped(bool b) {
		if (b) property |= TYPED;
		else property &= ~(TYPED);
		return b;
	}
	bool isWritable(bool b) {
		if (b) property |= WRITABLE;
		else property &= ~(WRITABLE);
		return b;
	}
	bool isNull(bool b) {
		if (b) property |= NIL;
		else property &= ~(NIL);
		return b;
	}
	bool isSealed(bool b) {
		if (b) property |= SEALED;
		else property &= ~(SEALED);
		return b;
	}
	bool isEnumerable(bool b) {
		if (b) property |= ENUMERABLE;
		else property &= ~(ENUMERABLE);
		return b;
	}
	bool isStatic(bool b) {
		if (b) property |= STATIC;
		else property &= ~(STATIC);
		return b;
	}



	// convertisseurs
	const faInt toInt() {
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

		throw "Can't convert an object to an integer.";
	}
	operator int() {
		std::cout << "Converting to int : " << toString() << std::endl;
		return static_cast<int>(toInt());
	}
	operator faInt() {
		std::cout << "Converting to faint : " << toString() << std::endl;
		return toInt();
	}

	const faNumber toNumber() {
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

		throw "Can't convert an object to a nummber.";
	}
	operator float() {
		std::cout << "Converting to float : " << toString() << std::endl;
		return static_cast<float>(toNumber());
	}
	operator faNumber() {
		std::cout << "Converting to fanumber : " << toString() << std::endl;
		return toNumber();
	}


	const faBool toBool() {
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

		throw "Can't convert an object to a boolean.";
	}
	// operator bool() const;  //!\\ ATTENTION : bien checker les effets secondaires



	const faString toString() {
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
	static const std::string numberToString(faNumber d) {
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


	// décorators
	// static std::vector<void (*) (Atom &)> decorators;


	// others
	void checkConstance() {
		if (!isWritable())
			throw "Can't assign a value to a constant.";
	}


private:
	// initialise une variable (utilisé par les constructeurs)
	void init() {
		if (type != VAR) property |= TYPED;
		if (type != LET) property |= WRITABLE;
	}

	// libère la mémoire occupée par la variable (et met isNull(true))
	void clear() {
		if (isNull()) return;
		isNull(true);

		if (type == STRING && value.str)
			delete value.str;
		else if (type == OBJECT && value.object)
			delete value.object;
		else if (type == ARRAY && value.array)
			delete value.array;
	}
};

