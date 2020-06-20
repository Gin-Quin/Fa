

// push a token to the current statement
void Parser::push(Token::Type type) {
	currentStatement->push({ type, position, length });
	position += length;
	length = 0;
}

// push the current statement to the current scope
void Parser::pushStatement() {
	if (currentStatement->size())
		scope.back()->push_back(currentStatement);
}

// indent by one
void Parser::indent() {
	indentLevel++;
	scope.push_back( &(scope.back()->back()->body) );
}

// unindent by one
void Parser::unindent() {
	indentLevel--;
	scope.pop_back();
}


/**
* Transform some fa code into a list of tokens.
* The token list will be parsed by the parser.
*/
void Parser::tokenize() {
	char c;
	position = 0;
	currentStatement = new Statement();

	if (getIndentLevel())
		throw error("Indentation at the beginning of the file");

	do {
		c = melody[position + length];

		if (isWordCharacter(c)) {
			if (isNumber) {
				isNumber << c;
				if (length && !isNumber) push(Token::Number);
			}
			length++;
		}

		else {  // special symbol
			// if there was a word before
			if (length) {
				if (isNumber)  push(Token::Number);
				else           push(Keywords.find(melody + position, length));
				startOfLine = false;
			}

			// new line
			if (!c || isEndOfLine(c)) {
				if (stringDepth)
					throw error(forbiddenEolInString);

				// we add the new line token
				position++;
				pushStatement();
				startOfLine = true;

				// if the line ends with ... it's a multiline string
				if (currentStatement->lastType == Token::TripleDot) {
					currentStatement->back().type = Token::MultiLineString;
					lockType = LockType::Multiline;
					indent();
					indentLock = indentLevel;
				}

				if (c) getIndentLevel();
			}

			// space
			else if (isSpace(c) || isBlank(c)) {
				position++;
				length = 0;
			}

			// forbidden control character
			else if (isControlCharacter(c)) {
				cout << "zabu ??" << endl;
				throw error("Forbidden control character");
			}


			// other symbol
			else {
				auto type = Symbols.find(melody + position, length);

				if (type == Token::UnknownToken)
					throw error("Unexpected symbol");

				// comment
				if (type == Token::Comment) {
					pushRawLine(Token::Comment);
					length = 0;
					if (startOfLine) {
						pushStatement();
						indent();
						indentLock = indentLevel;
						lockType = LockType::Comment;
						getIndentLevel();
					}
				}

				// checkpoint
				else if (startOfLine && type == Token::GreaterThan) {
					pushRawLine(Token::Checkpoint);
					length = 0;
				}

				// regular symbol
				else {
					push(type);
					startOfLine = false;

					switch (type) {
						// start of string
						case Token::String:
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

						default:
						break;
					}
				}
			}

			isNumber.reset();
		}
	} while (c);
	// we arrived at the end of the string

	// if everything is not closed
	if (stringDepth)
		throw error(forbiddenEolInString);

	pushStatement();
	hasTokenized = true;
}


/**
* Parse the string starting at the given location and push the right tokens
*/
void Parser::parseString(char opener) {
	bool isTemplate = (opener == '"' || opener == '\'');
	char c;
	while ((c = melody[position+length])) {
		if (isEndOfLine(c))
			throw error(forbiddenEolInString);
		else if (c == '\\') {
			if (isEndOfLine(melody[position+length+1]))
				throw error(forbiddenEolInString);
			length++;
		}
		else if (c == opener) {  // end of string
			if (length)
				push(Token::RawString);
			length = 1;
			push(Token::StringEnd);
			return;
		}
		else if (isTemplate && c == '{') {  // start of template
			push(Token::RawString);
			length = 1;
			push(Token::LeftCurlyBrace);

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
* This function is called after every end of line
*/
int Parser::getIndentLevel() {
	currentStatement = new Statement();
	int indentValue = 0;
	char c;

	while ((c = melody[position])) {
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
			indentValue++;

			// if indent is locked we get the line and continue
			if (indentLock && indentUnit && indentLock == indentValue / indentUnit) {
				position++;
				pushLockedLine();
				indentValue = 0;
				continue;
			}
		}
		else if (isBlank(c)) {}
		else if (isEndOfLine(c)) indentValue = 0;
		else if (isControlCharacter(c)) {
			cout << "coco ??" << endl;
			throw error("Forbidden control character!");
		}
		else {
			// first character met : we return the calculated indent
			if (indentUnit == 0) {
				indentUnit = indentValue;
				if (indentLock == 1) {  // if locked indent at 1 unit
					position++;
					pushLockedLine();
					indentValue = 0;
					continue;
				}
			}
			break;
		}

		position++;
	}

	// we check space indent is correct
	if (indentValue && (indentValue % indentUnit)) {
		string msg = "Incorrect number of spaces for indentation. The file previously used ";

		if (indentUnit == 1) msg += "1 space";
		else msg += to_string(indentUnit) + " spaces";

		msg += " as an indentation unit. You indented with ";

		if (indentValue == 1) msg += "1 space";
		else msg += to_string(indentValue) + " spaces";

		msg += ".";
		throw error(msg);
	}

	// we update indent
	if (indentValue)
		indentValue /= indentUnit;
	if (indentLock)
		indentLock = 0;

	if (indentValue > indentLevel) {
		if (indentValue > indentLevel + 1) {  // we check the indentation-up is correct
			position--;
			throw error("Too strong indentation. You indented by "
				+ to_string(indentValue)
				+" levels."
			);
		}

		indent();
	}
	else while (indentValue < indentLevel)
		unindent();
	
	return indentValue;
}



/**
* Push from the current position to the end of line
*/
void Parser::pushRawLine(Token::Type tokenType) {
	length = getLineLength(melody, position);
	push(tokenType);
}


/**
* Push from the current position to the end of line
*/
void Parser::pushLockedLine() {
	pushRawLine(lockType == LockType::Comment ? Token::SubComment : Token::RawString);
	pushStatement();
	currentStatement = new Statement();
	position++;
}


/**
* Parse a glob or a regular expression.
* Example of regex : //.../opt?/
* Example of glob : ||...|opt?|
*/
void Parser::parseRegexOrGlob(Token::Type type) {
	const bool isRegex = (type == Token::RegexStart);
	const char closer = (isRegex ? '/' : '|');
	char c;

	// main regex/glob content
	while ((c = melody[position+length])) {
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
	push(Token::RegexOrGlobContent);
	length = 1;
	push(Token::RegexOrGlobEnd);

	// regex/glob option
	while ((c = melody[position+length])) {
		if (c == closer) break;
		else if (isEndOfLine(c))
			throw error(forbiddenEolInRegex);
		else if (!isBlank(c) && isControlCharacter(c))
			throw error("Forbidden control character in regex or glob");
		length++;
	}
	if (!c) throw error(forbiddenEolInRegex);
	push(Token::RegexOrGlobOption);
	length = 1;
	push(Token::LeftParenthesis);
}
