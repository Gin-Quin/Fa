//  g++ splitCode.cpp ../src/Tools.cpp -I ../include -o split -std=c++14

#include "Tools.hpp"

using namespace std;

int main() {

	CodeElements elts = splitCodeString("x+7*(24-3)");

	for (int i = 0; i < elts.size(); i++) {
		cout << elts[i].value << (elts[i].isOperator ? " -> C'est un opérateur !":"") << endl;
	}


	// cout << endl << endl << matchOperator("==") << endl;


	return 1;
}