#pragma once

#include "Location.hpp"
#include "Clio.hpp"


/**
* Display the error msg with position information
*/
struct CodeError {
	const char* source { "" };
	const char* filename { "" };

	string operator() (string message, int position, int length=0) {
		auto [line, column, lineStart, lineLength] = Location(source, position, length);
		string lineContent(source + lineStart, lineLength);

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
};


