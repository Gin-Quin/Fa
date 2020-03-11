#include "../common/index.hpp"
#include "../parser/parseExpression.hpp"

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
		Tokenizer tokenizer(melody.data());
		parseExpression(tokenizer[0]);
		// auto tokenList = tokenizer.tokenize();
		// cout << "Tokens :" << tokenList.size() << endl;
		// tokenizer.print();
	}
	catch (string message) {
		cout << "The tokenizer encountered an error : " << message << endl;
		return 0;
	}

	if (ok) {
		cout << "Everything OK! :D" << endl;
		return 1;
	}
	cout << "An error occured :/" << endl;
	return 0;
}
