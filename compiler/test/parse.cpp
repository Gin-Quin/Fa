#include "../common.hpp"
#include "../parser/Parser.cpp"

int main() {
	bool ok = true;

	#ifdef __WINDOWS__
		constexpr const char* filesample = "test\\files\\expressions.fx";
	#else
		constexpr const char* filesample = "test/files/expressions.fx";
	#endif

	// cout << "File :" << filesample << endl;
	auto melody = readFile(filesample);
	// cout << "Melody :" << endl << melody << endl;

	try {
		Parser parser(melody.data());
		parser.tokenize();
		parser.printTokens();

		parser.growTree();
		parser.printTree();
	}
	catch (string message) {
		cout << "The Parser encountered an error : " << message << endl;
		return 0;
	}

	if (ok) {
		cout << "Everything OK! :D" << endl;
		return 1;
	}
	cout << "An error occured :/" << endl;
	return 0;
}
