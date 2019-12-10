#include "../common/index.hpp"
#include "../lexer/IsNumber.hpp"


int main() {
	bool ok = true;

	struct TestString {
		string test;
		bool expected;
	};

	vector<TestString> testStrings = {
		{ "03265454321", true },
		{ "51.213", true },
		{ "321e321", true },
		{ "3211.123e321312", true },
		{ ".43132", true },
		{ "0b100101011", true },
		{ "0xA0f10F09187", true },
		{ "0o012345676543210.321", true },

		{ "2130a32azae", false },
		{ "321312_213", false },
		{ "32321e21.21", false },
		{ "0b012324", false },
		{ "0o0123456789", false },
		{ "0x0123456789abcdefgh", false },
		{ "51.21.321", false },
		{ "51e21e321", false }
	};

	for (auto [test, expected] : testStrings) {
		IsNumber isNumber;
		cout << "TEST : '" << test << "'" << endl;
		for (char c : test) {
			isNumber << c;
			cout << "   " << c << " -> " << (bool) isNumber << endl;
		}
		if (isNumber == expected)
			cout << "=> OK!!" << endl << endl;
		else {
			cout << "=> Bad expectation" << endl << endl;
			ok = false;
		}
	}

	if (ok) {
		cout << "Everything OK! :D" << endl;
		return 1;
	}

	cout << "An error occured :/" << endl;
	return 0;
}
