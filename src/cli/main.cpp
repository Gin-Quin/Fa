#include <utility>

#include "../../lib/fa.hpp"

#include "compile.hpp"
#include "parse.hpp"


int main(int argc, const char* argv[]) {
	string firstArgument = argc > 1 ? argv[1] : "";

	vector<string> arguments;
	for (int i = 2; i < argc; i++)
		arguments.push_back(argv[i]);
	
	if (argc == 1 || argc == 2 && (firstArgument == "-v" || firstArgument == "--version")) {
		std::cout << "Fa compiler v" << version() << std::endl;
		if (argc == 1) {
			cout << endl << "Usage" << endl << "─────" << endl;
			help();
		}
		return 0;
	}

	if (argc == 2 && (firstArgument == "-h" || firstArgument == "--help")) {
		help();
		return 0;
	}

	if (argc == 2) {  // we run a single entry file
		// TODO : run the file
		cout << "TODO : run " << arguments[1] << endl;
		return 0;
	}


	string action = argv[1];

	if (action == "parse")
		return parse(arguments);
	else if (action == "compile" || action == "c")
		return compile(arguments);
		
	cout << "'" << action << "' is not a valid action" << endl;
	cout << endl << "Usage" << "──────────" << endl;
	help();
	return 1;
}
