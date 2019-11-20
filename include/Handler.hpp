#pragma once

#include <string>


template <class T>
class Handler
{
public:
	T operatorLeft(std::string op, T exprLeft);
	T operatorRight(std::string op, T exprRight);
	T operatorLeftAndRight(std::string op, T exprLeft, T exprRight);

	T functionCall(std::string funcName, T exprRight);
};
