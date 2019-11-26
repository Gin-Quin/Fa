
#include "TokenList.hpp"
#include "Symbols.hpp"
#include "Keywords.hpp"
#include "Workspace.hpp"
#include "IsNumber.hpp"

using namespace Workspace;

/**
* Transform some fa code (a melody) into a list of tokens.
* The token list will be parsed by the parser.
*/
TokenList tokenize(const char* _melody) {
	melody = _melody;  // defined in Workspace namespace
	position = length = 0;
	TokenList tokens(melody);
	Token nextToken;
	IsNumber isNumber;
	bool isWord = false;
	int stringLevel = 0;
	
	getIndentLevel(melody, position);
	if (indentLevel)
		throw error("Indentation at the beginning of the file");

	do {
		c = melody[position + length];

		if (isWordCharacter(c)) {
			isNumber << c;
			length++;
		}
		
		else {  // special symbol
			// if there was a word before
			if (length) {
				if (isNumber)
					tokens.push({Token::Number, position, length});
				else {
					auto type = Keywords.find(melody + position, length);
					tokens.push({type, position, length});
				}
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

			// end of string
			if (c == 0) break;

			// new line
			if (isEndOfLine(c)) {
				position++;
				int newIndent = getIndentLevel(melody, position);
				tokens.push({Token::NewLine, position - indentLevel});

				if (newIndent > indentLevel) {
					indentLevel++;
					tokens.push({Token::BlockStart});
				}
				else while (newIndent < indentLevel) {
					indentLevel--;
					tokens.push({Token::BlockEnd});
				}
			}
			else if (isSpace(c) || isBlank(c)) {
				position++;
				length = 0;
			}

			// string
			else if (c == '`' || c == '"' || c == '\'') {
				parseString(c);
			}

			// forbidden control character
			else if (isControlCharacter(c)) {
				throw error("Forbidden control character");
			}

			// other symbol
			else {
				auto type = Symbols.find(melody + position, length);
				if (type == Token::UnknownToken)
					throw error("Unexpected symbol");
				tokens.push({type, position, length});
				position += length;
				length = 0;
			}

			isNumber.reset();
		}
	} while (c);


	return tokens;
}


// int main() {
// 	try {
// 		auto tokenList = tokenize("x = 12 if y == 11\nz:String=call zabu+3\nCoucou");
// 		tokenList.print();
// 	}
// 	catch (string message) {
// 		cout << "The tokenizer encountered an error." << endl;
// 	}
// 	return 0;
// }