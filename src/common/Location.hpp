
#include "StringUtilities.hpp"


/**
* Structure returned by the getLocation method.
*/
struct Location {
	int line       { 1 };  // line number
	int column     { 1 };  // column number
	int lineStart  { 0 };  // position of the first non-space character of the line
	int lineLength { 0 };  // number of characters before the next end-of-line character

	Location() {}
	Location(const char* str, int position, int length=0) {
		char c;
		int padding = 0, offset = 0;
		int end = position + length;

		while (offset < end && (c = str[offset++])) {
			if (StringUtilities::isEndOfLine(c)) {
				line++;
				lineStart = offset;
				column = 1;
				padding = 0;
			}
			else {
				if (padding == column && StringUtilities::isBlank(c))
					lineStart++, padding++;
				column++;
			}
		}

		lineLength = StringUtilities::getLineLength(str, lineStart);
	}
};

