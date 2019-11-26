#include "../src/global.hpp"
#include "../src/lexer/tokenize.hpp"


int main() {
	bool ok = true;

	auto melody = readFile("test\\sample-A.fx");
	// cout << "Melody :" << endl << melody << endl;

	try {
		auto tokenList = tokenize(melody.data());
		// cout << "Tokens :" << tokenList.size() << endl;
		tokenList.print();
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
