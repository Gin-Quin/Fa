//  g++ elements.cpp ../src/CodeElements.cpp -I ../include -o elts -std=c++14

#include <iostream>
#include <string>
#include "CodeElements.hpp"


using namespace std;

int main() {

	CodeElements elements;
	elements.push_back({"Coucou", false});


	cout << elements.size() << endl;
	cout << elements[0].value << endl;
	cout << elements[0].isOperator << endl;


	return 1;
}