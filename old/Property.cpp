
#include <string>
using namespace std;

#include "Property.h"




//-- PROPERTY --//
Property::Property(string ty="null", bool isNull=true, bool wr=true, bool pr=false, bool en=true)
	: type(ty), isNull(isNull), writable(wr), isPrivate(pr), enumerable(en)
{}

// string Property::toString() {
// 	if (isNull) return "null";
// 	return "Undefined property";
// }



//-- NULL --//
// bool Null::operator==(Property const&p) {
// 	return p.isNull;
// }

// bool Null::operator!=(Property const&p) {
// 	return !p.isNull;
// }


bool operator==(Property const& p, Null const& n) {
	return p.isNull;
}
bool operator!=(Property const& p, Null const& n) {
	return !p.isNull;
}
bool operator==(Null const& n, Property const& p) {
	return p.isNull;
}
bool operator!=(Null const& n, Property const& p) {
	return !p.isNull;
}




//-- INT --//
Int::Int()
	: Property("int"), value(0)
{}

Int::Int(const Int& i)
	: Property("int", i.isNull), value(i.value)
{}

Int::Int(int i)
	: Property("int", false), value(i)
{}

// Int& Int::operator=(int i) {
// 	value = i;
// 	return this;
// }

string Int::toString() {
	if (isNull) return "null";
	return std::to_string(value);
}


bool Int::operator==(Int const& i) {
	return value == i.value;
}

bool Int::operator!=(Int const& i) {
	return value != i.value;
}
