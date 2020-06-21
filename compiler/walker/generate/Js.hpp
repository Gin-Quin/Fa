#pragma once
#include "../BaseWalker.hpp"

namespace Walker {
	namespace Generate {

		struct Js : BaseWalker {
			bool prettyPrint { true };
			string emit;

			// -- Constructor
			Js(Parser& parser) : BaseWalker(parser) {}

			// -- Visiter
			void visit(Node* node) {
				emit = "";
				BaseWalker::visit(node);
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

}
