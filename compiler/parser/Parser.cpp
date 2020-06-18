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

#include "Parser/tokenizer.cpp"
#include "Parser/ast-builder.cpp"
#include "Parser/printers.cpp"
