#pragma once

#include "Body.hpp"
#include "Statement.hpp"
#include "IsNumber.hpp"


struct Tokenizer : Body {
	const char* melody;
	Statement currentStatement {};
	vector<Body*> scope { this };
	int position { 0 };  // current position of the cursor
	int length { 0 };  // length of the current word/symbol
	IsNumber isNumber;

	// Configuration variables
	int indentUnit { 0 };  // number of spaces for one indentation unit
	char indentType { 0 };  // space or tab (if tabs, indentUnit must be 1)
	int indentLevel { 0 };  // current indent level
	string stringOpeners { "" };  // list of recursive string openers (' or ")
	int stringDepth { 0 };  // depth of string recursion (inside a template)
	int curlyBraceDepth { 0 };  // depth of curly braces (inside a template)
	bool startOfLine { true };  // true if we are at the beginning of a new line

	// locking
	enum LockType {
		Comment,
		Multiline
	};
	int indentLock { 0 };  // locked indentation (multiline strings or comments)
	LockType lockType;  // locked indentation (multiline strings or comments)

	// Constant messages
	static constexpr const char* forbiddenEolInString = "Missing end of string character before new line";
	static constexpr const char* forbiddenEolInRegex = "Missing end of regex or glob before new line";

	// Constructor
	Tokenizer(const char* _melody) {
		melody = _melody;
		tokenize();
	}

	// The main tokenizing function
	void tokenize();

	inline void push(Token::Type type) {
		currentStatement.push({ type, position, length });
		position += length;
		length = 0;
	}

	inline void pushStatement() {
		if (currentStatement.size())
			scope.back()->push(currentStatement);
	}

	inline void indent() {
		indentLevel++;
		scope.push_back( &(scope.back()->back().body) );
	}

	inline void unindent() {
		indentLevel--;
		scope.pop_back();
	}


	// Display a pretty error
	inline string error(string msg) {
		return prettyError(melody, msg, position, length);
	};


	// Parse the string starting at the given location and push the right tokens
	void parseString(char opener=0);
	void parseRegexOrGlob(Token::Type);


	// Skip all empty lines and return the number of spaces/tabs before the first non-empty line
	int getIndentLevel();

	inline void pushRawLine(Token::Type);
	inline void pushLockedLine();




	// Print the whole tokenizer
	inline void print() {
		for (auto& statement : *this)
			print(statement);
	}

	// Print a statement
	void print(Statement& statement, int depth=0);

	// Print a token list
	// void print(TokenList& list, int depth=0);

	// Return a colored token
	string coloredToken(const Token& token);


	// Extract a token's content from the melody
	inline string extract(const Token& token) {
		return string(melody + token.position, token.length);
	}
};