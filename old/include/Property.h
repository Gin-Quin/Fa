// #include <string>

#ifndef DEF_PERSONNAGE
#define DEF_PERSONNAGE



class Property {
public:
	string type;  // indique le type de la propriété. Sa valeur dépendra de celle-ci
	bool writable;  // indique si la propriété peut être écrasé ou non
	bool isPrivate;  // indique si la propriété est privée ou publique
	bool enumerable;  // indique si la propriété apparaîtra lorsqu'on la parcourt au sein d'une boucle "for"
	bool isNull;

	Property(string type, bool isNull, bool writable, bool isPrivate, bool enumerable);

	// string toString();
};


class Null {};

bool operator==(Property const&, Null const&);
bool operator!=(Property const&, Null const&);
bool operator==(Null const&, Property const&);
bool operator!=(Null const&, Property const&);



class Int : public Property {
public:
	int value;

	Int();
	Int(int i);
	Int(const Int& i);
	
	// Int& operator=(int i);

	bool operator==(Int const&);
	bool operator!=(Int const&);

	bool operator+(Int const&);
	bool operator*(Int const&);
	bool operator+=(Int const&);
	bool operator-=(Int const&);

	string toString();
};




#endif