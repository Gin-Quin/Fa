/**
* The worktop for tokenizing.
* Define semi-global variables.
*/

namespace Workspace {
	const char*  melody;
	int position, length, cursor;  // cursor = position + length
	char c;
	// the number of spaces to make indentation (must be constant through the melody)
	int indentUnit = 0;
	// space or tab (if tabs, indentUnit must be 1)
	char indentType = 0;
	int indentLevel = 0;


	/**
	* Character check functions
	*/
	inline bool isSpace(char c) {
		return c == ' ' || c == '\t';
	}
	inline bool isBlank(char c) {  // ignored characters
		return c == '\r';
	}
	inline bool isEndOfLine(char c) {
		return c == '\n';
	}
	inline bool isControlCharacter(char c) {
		// (valid control characters for Fa files are : 0, \t, \n, \r)
		return c < 32;
	}

	/**
	* Extract a string from the melody
	*/
	inline string extract(int position, int length) {
		return string(melody + position, length);
	}


	/**
	* Return the number of characters before the end of line
	*/
	int getLineLength(const char* str, int offset) {
		char c;
		int length = 0;
		while ((c = str[offset+length]) && !isEndOfLine(c))
			length++;
		return length;
	}


	/**
	* Return the line, column and lineStart of the message.
	*/
	struct Location {
		int line       { 1 };  // line number
		int column     { 1 };  // column number
		int lineStart  { 0 };  // position of the first non-space character of the line
		int lineLength { 0 };  // number of characters before the next end-of-line character
	};
	Location getLocation() {
		Location location;
		int padding = 0, offset = 0;
		int end = position + length;

		while (offset < end && (c = melody[offset++])) {
			if (isEndOfLine(c)) {
				location.line++;
				location.lineStart = offset;
				location.column = 1;
				padding = 0;
			}
			else {
				if (padding == location.column && isBlank(c))
					location.lineStart++, padding++;
				location.column++;
			}
		}

		location.lineLength = getLineLength(melody, location.lineStart);
		return location;
	}



	/**
	* Display the error msg with position information
	*/
	string error(string message) {
		auto [line, column, lineStart, lineLength] = getLocation();
		string lineContent(melody + lineStart, lineLength);

		cout
		<< Ink::brightRed
		<< Font::bold
		<< utf8("[!] ", "ERROR: ")
		<< message
		<< Font::reset
		<< Font::italic
		// << endl
		
		<< " (at position "
		<< Ink::white
		<< position + length
		
		<< Font::reset
		<< Font::italic
		<< ", line "
		<< Ink::brightYellow
		<< line

		<< Font::reset
		<< Font::italic
		<< ", column "
		<< Ink::brightPurple
		<< column

		<< Font::reset
		<< Font::italic
		<< ")"
		<< endl
		<< endl
		<< Font::reset
		<< Ink::brightYellow
		<< Font::bold
		<< line
		<< " |  "
		<< Font::reset
		<< Font::italic
		<< Ink::white
		<< lineContent
		<< endl
		<< "    ";

		for (int i=0; i < to_string(line).length(); i++)
			cout << ' ';

		for (int i=0; i < column; i++)
			cout << (lineContent[i] == '\t' ? '\t' : ' ');

		cout
		<< Font::reset
		<< Font::bold
		<< Ink::brightPurple
		<< '^'
		<< Font::reset
		<< endl
		<< endl;

		return message;
	}



	/**
	* Check if the given character is a word-breaker (and is a symbol too)
	*/
	inline bool doBreakWord(char c) {
		return (
			!(128 & c) && !(
				c == '$'
				|| c == '_'
				|| (c >= 'a' && c <= 'z')
				|| (c >= 'A' && c <= 'Z')
			)
		);
	}

	inline bool isWordCharacter(char c) {
		return (128 & c)
			|| c == '$'
			|| c == '_'
			|| (c >= 'a' && c <= 'z')
			|| (c >= 'A' && c <= 'Z')
			|| (c >= '0' && c <= '9')
		;
	}



	/**
	* Skip all empty lines and return the number of spaces/tabs before the first non-empty line
	* Return -1 if the end of string has been reached
	* Offset is a reference, so the position will be automatically updated by this function
	*/
	int getIndentLevel(const char* str, int& offset) {
		int indent = 0;
		char c;
		while (c = str[offset]) {
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

			offset++;
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

}