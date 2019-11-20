#pragma once

#include <iostream>
#include <fstream>
#include <string>
#include <cstring>

#include "CodeElements.hpp"
#include "Operators.hpp"


// namespace FaTools {

	std::string fragment(std::string, int offset=0);
	std::string trim(std::string);
	std::string getLine(std::string);
	std::string getWord(std::string);
	std::string evalFaCode(std::string, std::string);

	bool startsWith(std::string, std::string);
	bool startsWithWord(std::string, std::string);
	std::string getFaLineType(std::string s);
	bool isPunctuation(char c);

	CodeElements splitCodeString(std::string str);
	const Operator *matchOperator(std::string str);
	CodeElement nextCodeElement(std::string str, unsigned int &offset);
	
// }