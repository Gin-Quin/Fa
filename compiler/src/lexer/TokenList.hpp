#pragma once

#include "Token.hpp"

static string padding(int padding) {
	string str = "";
	while ((padding--) > 0)
		str += ' ';
	return str;
}


struct TokenList {
	const char* source { "" };
	int cursor { 0 };
	Token::Type lastType;
	vector<Token> list;

	TokenList() {}

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
		lastType = token.type;
	}

	void push(Token& token) {
		list.push_back(token);
		lastType = token.type;
	}

	inline Token& last() {
		return list.back();
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
			<< Ink::green           << padding(10 - to_string(token.type).length())
			                        << " position  "
			<< Ink::brightGreen     << token.position
			<< Ink::magenta         << padding(10 - to_string(token.position).length())
			                        << " length  "
			<< Ink::brightMagenta   << token.length

			<< padding(10 - to_string(token.length).length())
			<< coloredToken(token)
			<< Font::reset
			<< endl;
		}
	}

	string coloredToken(const Token& token) {
		if (token.type == Token::NewLine)
			return "";

		string content = extract(token);

		if (token.type > Token::KEYWORDS)  // keyword
			return Ink::red + content;

		if (token.type > Token::SYMBOLS)
			return Ink::yellow + content;

		if (token.type == Token::Number)
			return Ink::white + content;

		if (token.type == Token::Identifier)
			return Ink::white + content;

		if (token.type == Token::StringStart || token.type == Token::StringEnd)
			return Ink::green + content;

		if (token.type == Token::RawString)
			return Ink::green + content;

		if (token.type == Token::Comment)
			return Ink::cyan + content;
		if (token.type == Token::SubComment)
			return Ink::cyan + content;

		if (token.type == Token::Checkpoint)
			return Ink::cyan + content;

		if (token.type == Token::BlockStart)
			return Ink::blue + string("| ->");

		if (token.type == Token::BlockEnd)
			return Ink::blue + string("<- |");

		return content;
	}

};