#include "../lib/fa.hpp"

int main() {
	bool ok = true;

	#ifdef __WINDOWS__
		constexpr const char* filesample = "tests\\sample.fa";
	#else
		constexpr const char* filesample = "tests/sample.fa";
	#endif

	auto melody = readFile(filesample);

	// cout << "Parsed file : " << filesample << endl;
	// cout << melody << endl;

	try {
		Fa::Parser parser(melody);

		parser.tokenize();
		parser.printTokens();

		parser.growTree();
		parser.printTree();

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
		return 0;
	}
	catch (const char* message) {
		cout << "/!\\ " << message << endl;
		return 0;
	}

	return !ok;
}
