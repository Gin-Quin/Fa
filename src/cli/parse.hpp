
#pragma once

#include "Help.hpp"

enum Options {
	none = 0,
	tokens = 1,
	rawAst = 2,
	ast = 4,
	js = 8,
	cpp = 16,
	ts = 32,
	wasm = 64,
	dependencies = 128,
	declarations = 256,
	verbose = 512,
};


// parse an input file with the given options
static int _parse(string input, Options options) {

	try {
		auto melody = readFile(input);
		Fa::Parser parser(melody);

		parser.tokenize();

		auto& body = parser.body();
		string json = options & verbose ? body.toJson() : body.toVerboseJson();
		cout << "Tokens :" << endl << json << endl;

		// if (options & Options::tokens) {
		// 	cout << "[";
		// 	for (Token& token : parser)
		// 	cout << parser.tokens
		// }


		// parser.growTree();
		// parser.printTree();

		// Walkers::Validate walker(parser);
		// walker.start();

		// cout
		// << endl
		// << Ink::brightYellow
		// // << Font::bold
		// << "The walker has emitted :"
		// << Font::reset
		// << endl
		// << walker.emit
		// << endl;
	}
	catch (string message) {
		cout << "/!\\ " << message << endl;
		return 1;
	}
	catch (const char* message) {
		cout << "/!\\ " << message << endl;
		return 1;
	}

	return 0;
}



// read the arguments and call _parse()
int parse(vector<string> arguments) {
	string input = "";
	Options options = Options::none;

	for (int i=0; i < arguments.size(); i++) {
		string argument = arguments[i];

		if (argument == "--ast")
			options = static_cast<Options>(options | Options::ast);
		else if (argument == "--raw-ast")
			options = static_cast<Options>(options | Options::ast);
		else if (argument == "--tokens")
			options = static_cast<Options>(options | Options::ast);
		else if (argument == "--js")
			options = static_cast<Options>(options | Options::ast);
		else if (argument == "--cpp")
			options = static_cast<Options>(options | Options::ast);
		else if (argument == "--ts")
			options = static_cast<Options>(options | Options::ast);
		else if (argument == "--wasm")
			options = static_cast<Options>(options | Options::ast);
		else if (argument == "--dependencies")
			options = static_cast<Options>(options | Options::ast);
		else if (argument == "--declarations")
			options = static_cast<Options>(options | Options::ast);
		else if (arguments[i][0] == '-' && arguments[i][1] == '-' ) {  // bad option
			cout << Font::bold << Ink::red << "Unknwon option " << Font::bold << "'" << arguments[i] << "'" << Font::reset << endl;
			Help::usage();
			return 1;
		}
		else if (input.length()) {  // input file already defined
			cout << Font::bold << Ink::red << "Unexepected parameter '" << arguments[i] << "'" << Font::reset << endl;
			Help::usage();
			return 1;
		}
		else
			input = argument;
	}

	if (options == Options::none) {
		cout << Font::bold << Ink::red << "At least one paremeter option is expected" << Font::reset << endl;
		Help::usage();
		return 1;
	}

	if (!input.length()) {
		cout << Font::bold << Ink::red << "No input file" << Font::reset << endl;
		Help::usage();
		return 1;
	}

	return _parse(input, options);
}
