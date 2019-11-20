#pragma once
/**
Vector of the different keywords.
Keywords are sorted by size.
Maximum size for a keyword is 32 characters.
Minimum size is 2 characters.
Can be extended by user-defined keywords.
**/

#include "Token.hpp"

struct Keyword {
	const char* value;
	Token::Type type;
};

// we create the singleton
struct {
	vector<Keyword> table[32] {
		{	// 2 characters
			{ "if", Token::If },
			{ "in", Token::In },
			{ "or", Token::Or },
			{ "is", Token::Is },
			{ "as", Token::As }
		},

		{	// 3 characters
			{ "use", Token::Use },
			{ "for", Token::For },
			{ "and", Token::And },
			{ "xor", Token::Xor },
			{ "not", Token::Not },
			{ "try", Token::Try },
			{ "nil", Token::Nil }
		},


		{	// 4 characters
			{ "else", Token::Else },
			{ "from", Token::From },
			{ "self", Token::Self },
			{ "then", Token::Then },
			{ "when", Token::When },
			{ "isnt", Token::Isnt },
			{ "true", Token::True }
		},

		{	// 5 characters
			{ "while", Token::While },
			{ "super", Token::Super },
			{ "break", Token::Break },
			{ "catch", Token::Catch },
			{ "throw", Token::Throw },
			{ "async", Token::Async },
			{ "await", Token::Await },
			{ "yield", Token::Yield },
			{ "false", Token::False },
			{ "final", Token::Final },
			{ "const", Token::Const },
			{ "class", Token::Class },
			{ "enum", Token::Enum }
		},

		{	// 6 characters
			{ "import", Token::Import },
			{ "export", Token::Export },
			{ "modulo", Token::Modulo },
			{ "return", Token::Return },
			{ "elseIf", Token::ElseIf },
			{ "repeat", Token::Repeat },
			{ "global", Token::Global },
			{ "static", Token::Static },
			{ "unique", Token::Unique }
		},

		{	// 7 characters
			{ "extends", Token::Extends },
			{ "finally", Token::Finally },
			{ "private", Token::Private }
		},

		{	// 8 characters
			{ "continue", Token::Continue },
			{ "infinity", Token::Infinity },
			{ "override", Token::Override },
			{ "abstract", Token::Abstract }
		},

		{	// 9 characters
			{ "interface", Token::Interface },
			{ "structure", Token::Structure },
		},

		// 10-32 characters (may be defined by the user)
		// {{}}, {{}}, {{}}, {{}},
		// {{}}, {{}}, {{}}, {{}},
		// {{}}, {{}}, {{}}, {{}},
		// {{}}, {{}}, {{}}, {{}},
		// {{}}, {{}}, {{}}, {{}},
		// {{}}, {{}}, {{}}
	};

	Token::Type find(const char* in, const int length) {
		if (length == 1 || length > 32)
			return Token::Identifier;

		int i;
		char c;

		for (auto& keyword : table[length-2]) {
			i = -1;
			while (++i < length && keyword.value[i] == in[i]);
			if (i == length)
				return keyword.type;
		}

		return Token::Identifier;
	}

} Keywords;


