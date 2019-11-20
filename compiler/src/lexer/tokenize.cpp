#include "../global.hpp"

#include "TokenList.hpp"
#include "Symbols.hpp"
#include "Keywords.hpp"
#include "Worktop.hpp"

using namespace Worktop;

enum IndentType {
	Unknown,
	Space,
	Tab
};

/**
* Transform some fa code (a melody) into a list of tokens.
* The token list will be parsed by the parser.
*/
TokenList tokenize(const char* _melody) {
	melody = _melody;  // defined in Worktop namespace
	position = length = 0;
	TokenList tokens(melody);
	Token nextToken;
	int indentUnit = 0;  // the number of spaces to make indentation (must be constant through the melody)
	IndentType indentType = IndentType::Unknown;  // space or tab (if tabs, indentUnit must be 1)
	int indentLevel = getIndentLevel(melody, position);
	
	tokens.push({ Token::NewLineIndent, 0, indentLevel });  // every line starts with
	bool isWord = false;

	do {
		c = melody[position + length];

		if (isWordCharacter(c))
			length++;
		
		else {  // special symbol
			// if there was a word before
			if (length) {
				auto type = Keywords.find(melody + position, length);
				tokens.push({type, position, length});
				position += length;
				length = 0;
			}

			// next we add the symbol
			if (c == '#') {  // comment
				int lineLength = getLineLength(melody, position);
				tokens.push({ Token::Comment, position, lineLength });
				position += lineLength;
				c = melody[position];  // next is 0 or '\n'
			}

			if (c == 0)
				break;
			if (isEndOfLine(c)) {
				// tokens.push({Token::NewLine, position, 1});
				int nextIndentLevel = getIndentLevel(melody, ++position);
				tokens.push({Token::NewLineIndent, position - nextIndentLevel, nextIndentLevel});
			}
			else if (isSpace(c)) {
				position++;
				length = 0;
			}
			else if (isControlCharacter(c)) {
				throw error("Forbidden control character");
			}
			else {
				auto type = Symbols.find(melody + position, length);
				if (type == Token::UnknownToken)
					throw error("Unexpected symbol");
				tokens.push({type, position, length});
				position += length;
				length = 0;
			}
		}
	} while (c);


	return tokens;
}


int main() {
	try {
		auto tokenList = tokenize("x = 12 if y == 11\nz:String=call zabu+3");
		tokenList.print();
	}
	catch (string message) {
		cout << "The tokenizer encountered an error." << endl;
	}
	return 0;
}