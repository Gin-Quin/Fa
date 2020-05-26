#pragma once
#include "Token.hpp"

using Validator = int (*) (int);


namespace validate {
	int number(int a) { return a+1; }
}

Validator validator[] = {
	NULL,
	NULL,
	validate::number
};
