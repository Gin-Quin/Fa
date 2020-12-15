



void help() {
	cout
	<< Ink::brightBlue << "fa " << Ink::yellow << '{' << Font::reset << "filename" << Ink::yellow << '}' << endl
	<< Ink::white << "   Execute a .fa file" << endl << endl

	<< Ink::brightBlue << "fa " << Font::reset << "compile "
		<< Ink::yellow << '{' << Font::reset << "inputFile" << Ink::yellow << '}'
		<< Ink::yellow << '{' << Font::reset << "outputFile1 outputFile2 ..." << Ink::yellow << '}'
		<< Ink::white << "[--bundle]"
		<< endl
	<< Ink::white << "   Compile one or more output files from a single input file" << endl << endl

	<< Ink::brightBlue << "fa " << Font::reset << "parse "
		<< Ink::yellow << '{' << Font::reset << "inputFile" << Ink::yellow << '}'
		<< Ink::white << "[--tokens] [--raw-ast] [--ast] [--dependencies] [--declarations] [--js] [--cpp] [--wasm] [--ts]"
		<< endl
	<< Ink::white << "   Parse a single input file and output Json" << endl
	;
}