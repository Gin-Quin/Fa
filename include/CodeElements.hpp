#pragma once


#include <string>
#include <vector>

#include "Operators.hpp"


struct CodeElement {
	std::string value;
	const Operator *op;
};

using CodeElements = std::vector<CodeElement>;


