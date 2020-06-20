#pragma once
#include "../Walker.hpp"


namespace Generate {
	struct Js : Walker {
		bool prettyPrint { true };
		string emit;

		// -- Constructor
		Js(Parser& parser) : Walker(parser) {}

		// -- Visiter
		void visit(Node* node) {
			std::cout << "Visit from Generate::JS" << std::endl;
			emit = "";
			Walker::visit(node);
		}

		// -- Utils
		string joinChildren(Node* node, const char* pretty, const char* ugly) {
			string output;
			bool first = true;
			for (auto child : node->children) {
				if (first) first = false;
				else if (prettyPrint) output += pretty;
				else output += ugly;
				visit(child);
				output += emit;
			}
			return output;
		}


		// ------------- EMITTERS ---------------------
		void Number(Node* node) {
			emit = value(node);
		}

		void Plus(Node* node) {
			emit = joinChildren(node, " + ", "+");
		}

		void Minus(Node* node) {
			emit = joinChildren(node, " - ", "-");
		}

		void Asterisk(Node* node) {
			emit = joinChildren(node, " * ", "*");
		}

		void Divide(Node* node) {
			emit = joinChildren(node, " / ", "/");
		}

		void LeftParenthesis(Node* node) {
			if (node->children.size())
				visit(node->children[0]);
			emit = '(' + emit + ')';
		}

	};
}
