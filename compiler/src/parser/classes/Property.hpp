#pragma once
/*
* A property is an element of a scope.
* It can be defined as a field 
*/
#include "Unit.hpp"


struct Property : Unit {
	Expression value;
	
	Property(string _name, Expression _value) {
		name = _name;
		value = _value;
	}
};