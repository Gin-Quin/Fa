#pragma once

#include "Tokenizer.hpp"
#include "Symbols.hpp"
#include "Keywords.hpp"


/**
* Transform some fa code into a list of tokens.
* The token list will be parsed by the parser.
*/
TokenList Tokenizer::tokenize() {
	char c;
	reset();

	if (getIndentLevel())
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
				int newIndent = getIndentLevel();
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
					if (indentLevel < indentLock)
						indentLock = 0;
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
						indentLock = indentLevel + 1;
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
						parseString(c);

					// maybe end of template
					else if (type == Token::RightCurlyBrace) {
						if (stringDepth) {
							if (curlyBraceDepth)
								curlyBraceDepth--;
							else {  // end of template
								stringDepth--;
								char opener = stringOpeners.back();
								stringOpeners.pop_back();
								parseString(opener);
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






/**
* Parse the string starting at the given location and push the right tokens
*/
void Tokenizer::parseString(char opener) {
	bool isTemplate = (opener == '"' || opener == '\'');
	char c;
	while (c = melody[position+length]) {
		if (isEndOfLine(c))
			throw error(forbiddenEolInString);
		else if (c == '\\') {
			if (isEndOfLine(melody[position+length+1]))
				throw error(forbiddenEolInString);
			length++;
		}
		else if (c == opener) {  // end of string
			if (length) {
				tokens.push({Token::RawString, position, length});
				position += length;
			}
			tokens.push({Token::StringEnd, position, 1});
			position++;
			length = 0;
			return;
		}
		else if (isTemplate && c == '{') {  // start of template
			tokens.push({Token::RawString, position, length});
			position += length;
			tokens.push({Token::LeftCurlyBrace, position, 1});
			position++;
			length = 0;
			stringOpeners += opener;
			stringDepth++;
			return;
		}
		else if (isControlCharacter(c))
			throw error("Forbidden control character");
		length++;
	}

	throw error(forbiddenEolInString);
}







/**
* Skip all empty lines and return the number of spaces/tabs before the first non-empty line
* The position will be automatically updated by this function
*/
int Tokenizer::getIndentLevel() {
	int indent = 0;
	char c;

	while (c = melody[position]) {
		if (isSpace(c)) {
			if (!indentType) {
				indentType = c;
				if (c == '\t')
					indentUnit = 1;
			}
			else if (c != indentType) {
				if (c == ' ')
					throw error("Unexpected space as indentation character. The file previously used tabulations.");
				else
					throw error("Unexpected tabulation as indentation character. The file previously used spaces.");
			}
			indent++;
		}
		else if (isBlank(c)) {}
		else if (isEndOfLine(c)) indent = 0;
		else if (isControlCharacter(c)) throw error("Forbidden control character");
		else {
			// first character met : we return the calculated indent
			if (indentType == ' ') {
				if (indentUnit) {
					if (indent % indentUnit) {
						string msg = "Incorrect number of spaces for indentation. The file previously used ";

						if (indentUnit == 1) msg += "1 space";
						else msg += to_string(indentUnit) + " spaces";
						
						msg += " as an indentation unit. You indented with ";
						
						if (indent == 1) msg += "1 space";
						else msg += to_string(indent) + " spaces";
						
						msg += ".";
						throw error(msg);
					}
				}
				else indentUnit = indent;
			}
			return indent;
		}

		position++;
	}

	// we check the up-indentation is correct
	if (indent > indentLevel) {
		int up = (indent - indentLevel) / indentUnit;
		if (up > 1) {
			string msg =
				"Too strong indentation. You indented by "
				+ to_string(up)
				+" levels.";
			throw error(msg);
		}
	}

	return indent;
}

