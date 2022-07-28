
#include "Token.hpp"

/**
 * A keyword is made of alphanumeric characters
 * and has a special meaning in Fa's language.
 */
struct Keyword {
	const char* value;
	Token::Type type;
};


/**
 * The list of the different keywords.
 * Keywords are sorted by size.
 * Maximum size for a keyword is 32 characters.
 * Minimum size is 2 characters.
 * Can be extended by user-defined keywords.
 */
struct {
	vector<Keyword> table[32] {
		${Array(30).fill(0).map((_, length) =>
		`{  // ${length+2} characters
			${keywords
				.filter(({key}) => (key.length == length + 2))
				.map(({key, token}) => `{ "${key}", Token::${token} }`)
				.join(",\n\t\t\t")
			}
		}`).join(",\n\t\t")
		}

		//
		// {	// 2 characters
		// 	{ "if", Token::IfComprehension },
		// 	{ "in", Token::In },
		// 	{ "or", Token::Or },
		// 	{ "is", Token::Is },
		// 	{ "to", Token::To },
		// 	{ "do", Token::Do }
		// },
		//
		// {	// 3 characters
		// 	{ "let", Token::Let },
		// 	{ "use", Token::Use },
		// 	{ "for", Token::ForComprehension },
		// 	{ "and", Token::And },
		// 	{ "xor", Token::Xor },
		// 	{ "not", Token::Not },
		// 	{ "try", Token::Try },
		// 	{ "nil", Token::Nil }
		// },
		//
		//
		// {	// 4 characters
		// 	{ "else", Token::ElseComprehension },
		// 	{ "from", Token::From },
		// 	{ "self", Token::Self },
		// 	{ "then", Token::Then },
		// 	{ "when", Token::When },
		// 	{ "isnt", Token::Isnt },
		// 	{ "true", Token::True },
		// 	{ "enum", Token::Enum }
		// },
		//
		// {	// 5 characters
		// 	{ "while", Token::WhileComprehension },
		// 	{ "super", Token::Super },
		// 	{ "break", Token::Break },
		// 	{ "catch", Token::Catch },
		// 	{ "throw", Token::Throw },
		// 	{ "async", Token::Async },
		// 	{ "await", Token::Await },
		// 	{ "yield", Token::Yield },
		// 	{ "false", Token::False },
		// 	{ "final", Token::Final },
		// 	{ "const", Token::Const },
		// 	{ "class", Token::Class },
		// 	{ "print", Token::Print },
		// },
		//
		// {	// 6 characters
		// 	{ "import", Token::Import },
		// 	{ "export", Token::Export },
		// 	{ "modulo", Token::Modulo },
		// 	{ "return", Token::Return },
		// 	{ "elseif", Token::ElseIf },
		// 	{ "repeat", Token::RepeatComprehension },
		// 	{ "global", Token::Global },
		// 	{ "static", Token::Static },
		// 	{ "unique", Token::Unique }
		// },
		//
		// {	// 7 characters
		// 	{ "extends", Token::Extends },
		// 	{ "finally", Token::Finally },
		// 	{ "private", Token::Private }
		// },
		//
		// {	// 8 characters
		// 	{ "continue", Token::Continue },
		// 	{ "infinity", Token::Infinity },
		// 	{ "override", Token::Override },
		// 	{ "abstract", Token::Abstract }
		// },
		//
		// {	// 9 characters
		// 	{ "interface", Token::Interface },
		// 	{ "structure", Token::Structure },
		// },

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
