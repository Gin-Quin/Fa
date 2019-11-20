
#include <string>
using namespace std;

#include "var.h"






varValue::varValue() {
	integer = 0;
}


varValue::~varValue() {}


var::~var() {
	if (type == "string")
		delete value.str;
}



//-- PROPERTY : ASSIGNMENT --//

// -- Integer
var::var(int i) {
	isNull = false;
	isWritable = true;
	isPrivate = false;
	isEnumerable = true;
	type = "integer";
	value.integer = i;
}

// -- String (from string)
var::var(string s) {
	isNull = false;
	isWritable = true;
	isPrivate = false;
	isEnumerable = true;
	type = "string";
	value.str = new string(s);
}

// -- String (from char*)
var::var(const char *s) {
	isNull = false;
	isWritable = true;
	isPrivate = false;
	isEnumerable = true;
	type = "string";
	value.str = new string(s);
}


// -- Number (from float)
var::var(float f) {
	isNull = false;
	isWritable = true;
	isPrivate = false;
	isEnumerable = true;
	type = "number";
	value.number = static_cast<double>(f);
}

// -- Number (from double)
var::var(double d)
{
	isNull = false;
	isWritable = true;
	isPrivate = false;
	isEnumerable = true;
	type = "number";
	value.number = d;
}


// -- Boolean
var::var(bool b) {
	isNull = false;
	isWritable = true;
	isPrivate = false;
	isEnumerable = true;
	type = "boolean";
	value.boolean = b;
}


// -- Var
var::var(const var& p) {
	isNull = false;
	isWritable = true;
	isPrivate = false;
	isEnumerable = true;
	type = p.type;

	if (type == "integer")
		value.integer = p.value.integer;
	else if (type == "string")
		value.str = p.value.str;
	else if (type == "number")
		value.number = p.value.number;
	else if (type == "boolean")
		value.boolean = p.value.boolean;
}














//-- PROPERTY : EQUAL ASSIGNMENT --//
var& var::operator=(int i) {
	type = "integer";
	value.integer = i;
	return *this;
}


var& var::operator=(string s) {
	type = "string";
	value.str = new string(s);
	return *this;
}


var& var::operator=(float f) {
	type = "number";
	value.number = static_cast<double>(f);
	return *this;

}

var& var::operator=(double d) {
	type = "number";
	value.number = d;
	return *this;
}


var& var::operator=(bool b) {
	type = "boolean";
	value.boolean = b;
	return *this;
}

var& var::operator=(var& p) {
	type = p.type;
	if (type == "integer")
		value.integer = p.value.integer;
	else if (type == "string")
		value.str = p.value.str;
	else if (type == "number")
		value.number = p.value.number;
	else if (type == "boolean")
		value.boolean = p.value.boolean;
	return *this;
}










//-- PROPERTY : GET VALUE --//
int var::getInt() {
	if (isNull)
		return 0;
	if (type == "integer")
		return value.integer;
	if (type == "string")
		return std::stoi(value.str);
	if (type == "number")
		return static_cast<int>(value.number);
	if (type == "boolean")
		return value.boolean ? 1 : 0;

	// sinon, c'est une erreur
	return 0;
}

double var::getNumber() {
	if (isNull)
		return 0;
	if (type == "integer")
		return static_cast<double>(value.integer);
	if (type == "string")
		return std::stod(value.str);
	if (type == "number")
		return value.number;
	if (type == "boolean")
		return value.boolean ? 1 : 0;

	// sinon, c'est une erreur
	return 0;
}


string var::getString() {
	if (isNull)
		return "";
	if (type == "integer")
		return std::to_string(value.integer);
	if (type == "string")
		return value.str;
	if (type == "number")
		return std::to_string(value.number);
	if (type == "boolean")
		return value.boolean ? "true" : "false";

	// sinon, c'est une erreur
	return 0;
}


bool var::getBool() {
	if (isNull)
		return false;
	if (type == "boolean")
		return value.boolean;
	if (type == "integer")
		return (value.integer ? true : false);
	if (type == "string") {
		return (value.str.length() ? true : false);
	}
	if (type == "number")
		return (value.number ? true : false);

	// sinon, c'est une erreur ??
	return true;
}



