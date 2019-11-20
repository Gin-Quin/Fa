// #include <string>

#ifndef VAR_H
#define VAR_H


union varValue {
	int integer;
	double number;
	bool boolean;
	
	// pointers (values to free)
	string *str;

	// constructors / destrutors (unused but mandatory :( [C++ is shit])
	varValue();
	~varValue();
};



class var {
public:
	string type;  // indique le type de la propriété. Sa valeur dépendra de celle-ci
	varValue value;
	bool isWritable;  // indique si la propriété peut être écrasé ou non
	bool isPrivate;  // indique si la propriété est privée ou publique
	bool isEnumerable;  // indique si la propriété apparaîtra lorsqu'on la parcourt au sein d'une boucle "for"
	bool isNull;

	// construction
	var(const var&);
	~var();
	var(bool);
	var(int i);  // nouvel entier
	var(float);
	var(double);
	var(string);  // nouvelle string
	var(const char *);  // nouvelle string

	// obtention de valeur
	int getInt();
	double getNumber();
	string getString();
	bool getBool();

	var& operator=(int i);
	var& operator=(bool b);
	var& operator=(float f);
	var& operator=(double d);
	var& operator=(string);
	var& operator=(var&);

};




// Cas de la classe NULL
class Null {};

bool operator==(var const&, Null const&);
bool operator!=(var const&, Null const&);
bool operator==(Null const&, var const&);
bool operator!=(Null const&, var const&);





#endif