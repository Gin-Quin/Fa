#include "../common.hpp"
#include "../parser/Parser.cpp"
#include "../walker/generate/js/generator.hpp"

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
		Parser parser(melody);
		parser.growTree();
		parser.printTree();

		Generate::Js walker(parser);
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
