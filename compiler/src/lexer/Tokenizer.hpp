#pragma once
/**
* The worktop for tokenizing.
* Define semi-global variables.
*/

#include "TokenList.hpp"
#include "IsNumber.hpp"


struct Tokenizer {
	const char* melody;
	TokenList tokens;
	int position;  // current position of the cursor
	int length;  // length of the current word/symbol
	CodeError codeError;
	IsNumber isNumber;

	// Configuration variables
	int indentUnit;  // number of spaces for one indentation unit
	char indentType;  // space or tab (if tabs, indentUnit must be 1)
	int indentLevel;  // current indent level
	string stringOpeners;  // list of recursive string openers (' or ")
	int stringDepth;  // depth of string recursion (inside a template)
	int curlyBraceDepth;  // depth of curly braces (inside a template)
	bool startOfLine;  // true if we are at the beginning of a new line
	int indentLock;  // locked indentation (multiline strings or comments)

	// Constant messages
	static constexpr const char* forbiddenEolInString = "Missing end of string character before new line";

	// Constructor
	Tokenizer(const char* _melody) {
		melody = _melody;
		codeError.source = _melody;
		tokens.source = _melody;
	}

	// Reset to default values
	void reset() {
		isNumber.reset();
		position = 0;
		length = 0;
		indentUnit = 0;
		indentType = 0;
		indentLevel = 0;
		stringOpeners = "";
		stringDepth = 0;
		curlyBraceDepth = 0;
		startOfLine = true;
		indentLock = 0;
	}

	// The main tokenizing function
	TokenList tokenize();


	// Display a pretty error
	inline string error(string msg) {
		return codeError(msg, position, length);
	};


	// Parse the string starting at the given location and push the right tokens
	void parseString(char opener=0);


	// Skip all empty lines and return the number of spaces/tabs before the first non-empty line
	int getIndentLevel();


	/**
	* Extract a string from the melody
	*/
	// inline string extract(int position, int length) {
	// 	return string(melody + position, length);
	// }

};