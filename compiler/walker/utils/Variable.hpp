#pragma once

#include "Type.hpp"

/**
 * A Variable.
 */
struct Variable {
	Type type;  // the type of the variable
	struct {
		bool constant { false };
		bool shared { false };  // in Fa, a shared variable is a static variable
	} is;
	string value;  // used by the interpreter to store the variable value
};
