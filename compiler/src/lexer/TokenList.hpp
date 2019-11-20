#pragma once

#include "Token.hpp"

struct TokenList {
	vector<Token> list;
	int cursor {0};
	const char* source;

	TokenList(const char* _source) {
		source = _source;
	}

	string extract(const Token& token) {
		return string(source + token.position, token.length);
	}

	void push(Token token) {
		list.push_back(token);
	}

	// consume the next token
	Token* consume() {
		return cursor < list.size() ? &list[cursor++] : NULL;
	}

	// consume the next token if it has the right type
	Token* consume(Token::Type type) {
		return cursor < list.size() && list[cursor].type == type ? &list[cursor++] : NULL;
	}

	void print() {
		for (auto token : list) {
			       /**/ cout /**/
			<< Ink::brightBlue		<< "Token  "
			<< Ink::yellow				<< "`"
			<< Ink::brightYellow		<< extract(token)
			<< Ink::yellow				<< "`"
			<< Ink::cyan				<< "  \ttype  "
			<< Ink::brightCyan		<< token.type
			<< Ink::green				<< "  \tposition  "
			<< Ink::brightGreen		<< token.position
			<< Ink::magenta			<< "  \tlength  "
			<< Ink::brightMagenta	<< token.length
			<< Font::reset				<< endl;
		}
	}
};