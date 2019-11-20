#pragma once
/*
* A Unit is an abstract element of a Scope.
* It can be defined as a field 
*/

struct Unit {
	enum Decorator {
		CONSTANT = 1,
		PRIVATE = 2,
	};

	string name;
	unsigned int flags;
	
	inline bool isPrivate() { return flags & PRIVATE; }
	inline bool isConstant() { return flags & CONSTANT; }
};