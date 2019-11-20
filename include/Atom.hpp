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
using faObject 	= 	std::unordered_map<std::string, Atom>;





class Animal {
	extends Mammal
	extends Zabu

	string x = 12
}


/**
* Définit la classe Atom qui est la base de toutes les données en Fa.
* Un atom prend 16 octets de mémoire.
* Les strings et les conteneurs utilisent de la mémoire allouée en plus.
*/
class Atom {

protected:
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
	static const Property LOCKED = 64;

public:
	// constructeur
	// (prend automatiquement en paramètre le type de la variable)
	Atom();
	Atom(Type t = VAR);
	Atom(std::string t);

	// destructeur
	// les deux fonctions suivantes libèrent la mémoire et indiquent que l'élément est null
	~Atom();
	void clear();

	// assignations
	Atom& operator= (Atom& v);
	Atom& operator= (int v);
	Atom& operator= (faInt v);
	Atom& operator= (float v);
	Atom& operator= (faNumber v);
	Atom& operator= (faBool v);
	Atom& operator= (const char *v);
	Atom& operator= (faString* v);
	Atom& operator= (faString& v);


	// gestion du type
	const Type getType();
	const std::string getTypeName();
	static const Type getTypeFromName(faString t);
	static const std::string getNameFromType(Type t);


	// gestion des propriétés
	const bool isTyped();
	const bool isWritable();
	const bool isLocked();
	const bool isNull();
	const bool isSealed();
	const bool isEnumerable();
	const bool isStatic();

	bool isTyped(bool b);
	bool isWritable(bool b);
	bool isLocked(bool b);
	bool isNull(bool b);
	bool isSealed(bool b);
	bool isEnumerable(bool b);
	bool isStatic(bool b);


	// convertisseurs explicites
	const faInt toInt();
	const faNumber toNumber();
	const faBool toBool();
	const faString toString();

	// convertisseurs implcites
	operator int();
	operator faInt();
	operator float();
	operator faNumber();
	

	



protected:
	// méthodes privées
	void checkLock();
	static const std::string numberToString(faNumber d);

};

