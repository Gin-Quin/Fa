#include "../common.hpp"
#include "../parser/Parser.cpp"
#include "../walker/generate/Js.hpp"

int main() {
	bool ok = true;

	#ifdef __WINDOWS__
		constexpr const char* filesample = "compiler\\test\\sample.fa";
	#else
		constexpr const char* filesample = "compiler/test/sample.fa";
	#endif

	auto melody = readFile(filesample);

	try {
		Parser parser(melody);
		
		parser.tokenize();
		parser.printTokens();

		parser.growTree();
		parser.printTree();

		Generate::Js walker(parser);
		walker.start();

		cout
		<< endl
		<< Ink::brightYellow
		// << Font::bold
		<< "The walker has emitted :"
		<< Font::reset
		<< endl
		<< walker.emit
		<< endl;
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
