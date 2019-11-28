#include "../src/common.hpp"
#include "../src/lexer/Tokenizer.cpp"


int main() {
	bool ok = true;

	#if __WINDOWS__
		constexpr const char* filesample = "test\\sample-A.fx";
	#else
		constexpr const char* filesample = "test/sample-A.fx";
	#endif

	// cout << "File :" << filesample << endl;
	auto melody = readFile(filesample);
	// cout << "Melody :" << endl << melody << endl;

	try {
		Tokenizer tokenizer(melody.data());
		auto tokenList = tokenizer.tokenize();
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
