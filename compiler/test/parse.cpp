#include "../common.hpp"
#include "../parser/Parser.cpp"

int main() {
	bool ok = true;

	#ifdef __WINDOWS__
		constexpr const char* filesample = "test\\sample.fa";
	#else
		constexpr const char* filesample = "test/sample.fa";
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
		cout << "/!\\ " << message << endl;
		return 0;
	}
	catch (const char* message) {
		cout << "/!\\ " << message << endl;
		return 0;
	}

	if (ok) {
		cout << endl << "Everything OK 😀" << endl;
		return 1;
	}
	cout << "An error occured 🤨" << endl;
	return 0;
}
