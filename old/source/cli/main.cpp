#include <utility>

#include "../../lib/fa.hpp"

#include "compile.hpp"
#include "parse.hpp"
#include "Help.hpp"


int main(int argc, const char* argv[]) {
	cout.sync_with_stdio(false);

	string firstArgument = argc > 1 ? argv[1] : "";

	vector<string> arguments;
	for (int i = 2; i < argc; i++)
		arguments.push_back(argv[i]);

	if (argc == 1 || argc == 2 && (firstArgument == "-v" || firstArgument == "--version")) {
		Help::infos();
		// std::cout << "Fa compiler v" << version() << std::endl;
		if (argc == 1)
			Help::usage();
		return 0;
	}

	if (argc == 2 && (firstArgument == "-h" || firstArgument == "--help")) {
		Help::usage();
		return 0;
	}

	string action = argv[1];

	if (argc == 2) {  // we run a single entry file
		// TODO : run the file
		cout << "TODO : run " << action << endl;
		return 0;
	}

	if (action == "parse")
		return parse(arguments);
	else if (action == "compile" || action == "c")
		return compile(arguments);

	cout << "'" << action << "' is not a valid action" << endl;
	Help::usage();
	return 1;
}
