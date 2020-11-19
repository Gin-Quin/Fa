
#include "Token.hpp"

/**
 * A symbol is a combination of punctuation characters.
 * They are separated from keywords because they have a different matching algorithm.
 */
struct Symbol {
	const char* value;
	Token::Type type;
};


/*
 * The list of the different symbols.
 * Can be extended by user-defined symbols.
**/
struct {
	vector<Symbol> table {
		${symbols.map(({key, token}) => `{ "${key.replace(/\\/g, '\\\\')}", Token::${token} }`).join(",\n\t\t")}
	};


	Token::Type find(const char* in, int& length) {
		char c;
		int i;

		for (auto& symbol : table) {
			i = 0;
			while ((c = symbol.value[i]) == in[i] && c) i++;

			if (c == 0) {  // they matched
				length = strlen(symbol.value);
				return symbol.type;
			}
		}

		length = 0;
		return Token::UnknownToken;
	}

} Symbols;
