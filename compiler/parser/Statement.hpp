#pragma once
/**
* A statement is the abstract retpresentation of one line of code.
* It is a token list plus a possible body.
*/

#include "Token.hpp"

struct Statement;
using Body = vector<Statement*>;

struct Statement : public vector<Token> {
	Body body;
	Token::Type lastType;

	~Statement() {
		for (Statement* child : body)
			delete child;
	}

	inline void push(Token token) {
		push_back(token);
		lastType = token.type;
	}

	inline void push(Token& token) {
		push_back(token);
		lastType = token.type;
	}
};
