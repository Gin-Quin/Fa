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
		void visitNumber(Node* node) {
			emit = value(node);
		}

		void visitPlus(Node* node) {
			emit = joinChildren(node, " + ", "+");
		}

		void visitMinus(Node* node) {
			emit = joinChildren(node, " - ", "-");
		}

		void visitAsterisk(Node* node) {
			emit = joinChildren(node, " * ", "*");
		}

		void visitDivide(Node* node) {
			emit = joinChildren(node, " / ", "/");
		}

		void visitLeftParenthesis(Node* node) {
			if (node->children.size())
				visit(node->children[0]);
			emit = '(' + emit + ')';
		}

	};
}
