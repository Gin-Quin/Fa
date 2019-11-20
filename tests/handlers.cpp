
//  g++ handlers.cpp ../src/Tools.cpp -I ../include -o handlerTest -std=c++14

#include <iostream>
#include <string>
#include "Handler.hpp"
#include "Tools.hpp"


using namespace std;



/**
* Handler simple qui gère les opérations suivantes : +, -, *, /
* Retourne l'entier final
*/
class SimpleMathHandler : public Handler<int> {
	int operatorLeft(std::string op, int exprLeft) {
		cout << "OperatorLeft ?? Impossible !!" << endl;
	};

	int operatorRight(std::string op, int exprLeft) {
		cout << "OperatorRight ?? Impossible !!" << endl;
	};

	int operatorLeftAndRight(std::string op, int exprLeft, int exprRight) {
		int result;

		switch (op) {
			case "+":
				result = exprLeft + exprRight;
				break;

			case "-":
				result = exprLeft - exprRight;
				break;

			case "*":
				result = exprLeft * exprRight;
				break;

			case "/":
				result = exprLeft / exprRight;
				break;

			default:
				cout << "ERROR ! Operator " << op << " unknown !";
				result = 0;
		}

		cout << ">   " << exprLeft << op << exprRight << "   ~>   " << result << endl;
		return result;
	};
};





int evalFaCode(string code) {
	unsigned int offset = 0;
	CodeElements stack;  // La pile des éléments. Se construit et se réduit dynamiquement.
	CodeElement elt;
	std::vector<int> operatorPositions;


	// Tant qu'il y a un prochain élément
	while ((elt = nextCodeElement(code, offset)).value != "") {
		
		// si c'est un opérateur
		if (elt.op) {
			
			// si c'est un container
			if (elt.op->connexion & CONTAINER) {

			}

			// si c'est un opérateur normal
			else {

			}

		}

		// si c'est une expression
		else {

		}
	}
}



int main() {

	return 1;
}