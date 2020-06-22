#pragma once

#include "Symbols.hpp"
#include "Keywords.hpp"
#include "Parser.hpp"


Parser::Parser(const char* _melody) {
	melody = _melody;
}

Parser::~Parser() {
	delete scope[0];
	delete tree;
}

#include "Parser/tokenize.cpp"
#include "Parser/grow-tree.cpp"
#include "Parser/print.cpp"
