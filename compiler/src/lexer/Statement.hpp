#pragma once
/**
* A Token statement is basically the abstract retpresentation of one line of code.
* It is a token list (the statement) plus a possible body.
*/

#include "Token.hpp"

struct Statement : public TokenList {
	Body body;
	Token::Type lastType;

	inline void push(Token token) {
		push_back(token);
		lastType = token.type;
	}

	inline void push(Token& token) {
		push_back(token);
		lastType = token.type;
	}

	inline Token& last() {
		return back();
	}

	// consume the next token
	// Token* consume() {
	// 	return cursor < list.size() ? &list[cursor++] : NULL;
	// }

	// consume the next token if it has the right type
	// Token* consume(Token::Type type) {
	// 	return cursor < list.size() && list[cursor].type == type ? &list[cursor++] : NULL;
	// }

};
