/**
* The worktop for tokenizing.
* Define semi-global variables.
*/

namespace Worktop {
	const char*  melody;
	int position, length, cursor;  // cursor = position + length
	char c;

	/**
	* Character check functions
	*/
	inline bool isSpace(char c) {
		return c == ' ' || c == '\t' || c == '\r';
	}
	inline bool isEndOfLine(char c) {
		return c == '\n';
	}
	inline bool isControlCharacter(char c) {
		return  c < 32;  // (valid control characters for Fa files are : 0, \t, \n, \r)
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
		while (c = str[offset+length] && !isEndOfLine(c))
			length++;
		return length;
	}


	/**
	* Return the line, column and lineStart of the message.
	*/
	struct Location {
		int line			{ 1 };  // line number
		int column		{ 1 };  // column number
		int lineStart	{ 0 };  // position of the first non-space character of the line
		int lineLength	{ 0 };  // number of characters before the next end-of-line character
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
				if (padding == location.column && isSpace(c))
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
		<< Ink::red
			<< utf8("⚠ ", "ERROR: ")
		<< Ink::brightRed << Font::bold
			<< message
		<< Ink::red
			<< " at position "
		<< Ink::brightRed
			<< position + length
		<< endl
		<< endl
		<< Ink::brightYellow
			<< line
		<< Ink::yellow
			<< " |  "
			<< lineContent
		<< endl
			<< "    ";

		for (int i=1; i < column + to_string(line).length(); i++)
			cout << ' ';

		cout
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
	inline int getIndentLevel(const char* str, int& offset) {
		int indent = 0;
		char c;
		while (c = str[offset]) {
			if (isSpace(c)) indent++;
			else if (isEndOfLine(c)) indent = 0;
			else if (isControlCharacter(c)) throw error("Forbidden control character");
			else return indent;  // first character met : we return the calculated indent
			offset++;
		}
		return indent;
	}

}