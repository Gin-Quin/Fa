
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
	tokens = melody;
	IsNumber isNumber;
	stringOpeners = "";
	stringDepth = 0;
	curlyBraceDepth = 0;
	commentIndent = 0;
	startOfLine = true;

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
				startOfLine = false;
			}

			// end of string
			if (c == 0) {
				if (stringDepth)
					throw error(forbiddenEolInString);
				break;
			}

			// new line
			if (isEndOfLine(c)) {
				if (stringDepth)
					throw error(forbiddenEolInString);
				position++;
				int newIndent = getIndentLevel(melody, position);
				tokens.push({Token::NewLine, position - indentLevel});

				if (newIndent > indentLevel) {
					indentLevel++;
					tokens.push({Token::BlockStart});
				}
				else {
					while (newIndent < indentLevel) {
						indentLevel--;
						tokens.push({Token::BlockEnd});
					}
					if (indentLevel < commentIndent)
						commentIndent = 0;
				}
				startOfLine = true;
			}
			else if (isSpace(c) || isBlank(c)) {
				position++;
				length = 0;
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

				// comment
				if (type == Token::Comment) {
					int lineLength = getLineLength(melody, position);
					tokens.push({ Token::Comment, position, lineLength });
					position += lineLength;
					length = 0;
					if (startOfLine)
						commentIndent = indentLevel + 1;
				}
				
				// checkpoint
				else if (startOfLine && type == Token::GreaterThan) {
					int lineLength = getLineLength(melody, position);
					tokens.push({ Token::Checkpoint, position, lineLength });
					position += lineLength;
					length = 0;
				}

				// regular symbol
				else {
					tokens.push({type, position, length});
					position += length;
					length = 0;
					startOfLine = false;

					// start of string
					if (type == Token::StringStart)
						parseRawString(c);

					// maybe end of template
					else if (type == Token::RightCurlyBrace) {
						if (stringDepth) {
							if (curlyBraceDepth)
								curlyBraceDepth--;
							else {  // end of template
								stringDepth--;
								char opener = stringOpeners.back();
								stringOpeners.pop_back();
								parseRawString(opener);
							}
						}
					}

					// curly brace
					else if (type == Token::LeftCurlyBrace)
						if (stringDepth) curlyBraceDepth++;
				}
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