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
				if (isNumber)  pushToken(Token::Number);
				else           pushToken(Keywords.find(melody + position, length));
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

				// if the line ends with ... it's a multiline string
				if (tokens.lastType == Token::TripleDot) {
					tokens.last().type = Token::MultiLineString;
					indentLock = indentLevel + 1;
					lockType = LockType::Multiline;
				}

				// we add the new line token
				position++;
				pushToken(Token::NewLine);
				getIndentLevel();
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
				else if (tokens.lastType == Token::LeftParenthesis) {
					if (type == Token::Divide)
						type = Token::RegexStart;
					else if (type == Token::Pipe)
						type = Token::GlobStart;
				}

				// comment
				if (type == Token::Comment) {
					pushRawLine(Token::Comment);
					length = 0;
					if (startOfLine) {
						indentLock = indentLevel + 1;
						lockType = LockType::Comment;
					}
				}
				
				// checkpoint
				else if (startOfLine && type == Token::GreaterThan) {
					pushRawLine(Token::Checkpoint);
					length = 0;
				}

				// regular symbol
				else {
					pushToken(type);
					startOfLine = false;

					switch (type) {
						// start of string
						case Token::StringStart:
							parseString(c);
						break;

						// maybe end of template
						case Token::RightCurlyBrace:
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
						break;

						// curly brace : increment depth
						case Token::LeftCurlyBrace:
							if (stringDepth)
								curlyBraceDepth++;
						break;

						// regex / glob
						case Token::RegexStart:
						case Token::GlobStart:
							parseRegexOrGlob(type);
						break;
					}
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
			if (length)
				pushToken(Token::RawString);
			length = 1;
			pushToken(Token::StringEnd);
			return;
		}
		else if (isTemplate && c == '{') {  // start of template
			pushToken(Token::RawString);
			length = 1;
			pushToken(Token::LeftCurlyBrace);

			stringOpeners += opener;
			stringDepth++;
			return;
		}
		else if (!isBlank(c) && isControlCharacter(c))
			throw error("Forbidden control character in string");
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
					throw error("Unexpected use of space as indentation character. The file previously used tabulations.");
				else
					throw error("Unexpected use of tabulation as indentation character. The file previously used spaces.");
			}
			indent++;
			
			// if indent is locked we get the line and continue
			if (indentLock && indentUnit && indentLock == indent / indentUnit) {
				position++;
				pushLockedLine();
				indent = 0;
				continue;
			}
		}
		else if (isBlank(c)) {}
		else if (isEndOfLine(c)) indent = 0;
		else if (isControlCharacter(c)) throw error("Forbidden control character!");
		else {
			// first character met : we return the calculated indent
			if (indentUnit == 0) {
				indentUnit = indent;
				if (indentLock == 1) {  // if locked indent at 1 unit
					position++;
					pushLockedLine();
					indent = 0;
					continue;
				}
			}
			break;
		}

		position++;
	}

	// we check space indent is correct
	if (indent && indent % indentUnit) {
		string msg = "Incorrect number of spaces for indentation. The file previously used ";

		if (indentUnit == 1) msg += "1 space";
		else msg += to_string(indentUnit) + " spaces";
		
		msg += " as an indentation unit. You indented with ";
		
		if (indent == 1) msg += "1 space";
		else msg += to_string(indent) + " spaces";
		
		msg += ".";
		throw error(msg);
	}

	// we update indent
	if (indent)
		indent /= indentUnit;
	if (indentLock)
		indentLock = 0;

	if (indent > indentLevel) {
		if (indent > indentLevel + 1) {  // we check the indentation-up is correct
			position--;
			throw error("Too strong indentation. You indented by "
				+ to_string(indent)
				+" levels."
			);
		}

		indentLevel++;
		tokens.push({Token::BlockStart});
	}

	else {
		while (indent < indentLevel) {
			indentLevel--;
			tokens.push({Token::BlockEnd});
		}
	}
	return indent;
}



/**
* Push from the current position to the end of line
*/
void Tokenizer::pushRawLine(Token::Type tokenType) {
	length = getLineLength(melody, position);
	pushToken(tokenType);
}


/**
* Push from the current position to the end of line
*/
void Tokenizer::pushLockedLine() {
	pushRawLine(lockType == LockType::Comment ? Token::SubComment : Token::RawString);
	pushToken(Token::NewLine);
	position++;
}


/**
* Parse a glob or a regular expression.
* Example of regex : (/.../opt)
* Example of glob : (|...|opt)
*/
void Tokenizer::parseRegexOrGlob(Token::Type type) {
	const bool isRegex = (type == Token::RegexStart);
	const char closer = (isRegex ? '/' : '|');
	char c;

	cout << "Start part 1. " << melody[position+length] << endl;
	cout << "closer = " << closer << endl;

	// main regex/glob content
	while (c = melody[position+length]) {
		if (c == closer)
			break;
		else if (isEndOfLine(c))
			throw error(forbiddenEolInRegex);
		else if (c == '\\') {
			if (isEndOfLine(melody[position+length+1]))
				throw error(forbiddenEolInRegex);
			length++;
		}
		else if (!isBlank(c) && isControlCharacter(c))
			throw error("Forbidden control character in regex or glob");
		length++;
	}
	if (!c) throw error(forbiddenEolInRegex);
	pushToken(Token::RegexOrGlobContent);
	length = 1;
	pushToken(Token::RegexOrGlobEnd);

	cout << "Part 1 done. " << endl;
	cout << "Start part 2. " << melody[position+length] << endl;

	// regex/glob option
	while (c = melody[position+length]) {
		if (c == ')') break;
		else if (isEndOfLine(c))
			throw error(forbiddenEolInRegex);
		else if (!isBlank(c) && isControlCharacter(c))
			throw error("Forbidden control character in regex or glob");
		length++;
	}
	if (!c) throw error(forbiddenEolInRegex);
	pushToken(Token::RegexOrGlobOption);
	length = 1;
	pushToken(Token::LeftParenthesis);
}