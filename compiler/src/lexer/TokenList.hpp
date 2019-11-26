#pragma once

#include "Token.hpp"

static string padding(int padding) {
	string str = "";
	while ((padding--) > 0)
		str += ' ';
	return str;
}


struct TokenList {
	vector<Token> list;
	int cursor {0};
	const char* source;

	TokenList(const char* _source) {
		source = _source;
	}

	inline int size() {
		return list.size();
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
			cout
			<< Ink::cyan            << "type  "
			<< Ink::brightCyan      << token.type
			<< Ink::green           << padding(8 - to_string(token.type).length())
			                        << " position  "
			<< Ink::brightGreen     << token.position
			<< Ink::magenta         << padding(8 - to_string(token.position).length())
			                        << " length  "
			<< Ink::brightMagenta   << token.length

			<< Ink::brightBlue      << padding(8 - to_string(token.length).length())
			                        << "Token  "
			<< Ink::yellow          << "`"
			<< Ink::brightYellow    << extract(token)
			<< Ink::yellow          << "`"
			<< Font::reset          << endl;
		}
	}
};