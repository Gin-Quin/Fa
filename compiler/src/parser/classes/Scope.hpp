#pragma once
/*
* A scope contains information about the variable/properties that exist at its level.
*/

#include "Property.hpp"

struct Scope {
	vector<Property> properties;

	bool has(string propertyName) {
		for (auto property : properties) {
			
		}
	}

	Property operator[](string propertyName) {
		a = b
		a == b
	}
};