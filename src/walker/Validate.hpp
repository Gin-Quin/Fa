#pragma once
#include "BaseWalker.hpp"
#include "utils/NodeData.hpp"
#include "utils/Scope.hpp"

namespace Walker {

	/**
	 * This walker make sure the given tree is valid.
	 * Plus it feeds the `data` value of every node.
	 */
	struct Validate : BaseWalker {
		vector<Scope> scopes;
		inline const Scope& context() { return scopes.back(); } 

		Validate(Parser& parser) : BaseWalker(parser) {
			auto moduleData = new NodeData::Module();
			parser.tree->data = (void*) moduleData;
			scopes.push_back(moduleData->scope);
		}


		// -- VISITERS
		void visitLet(Node* node, bool shared=false, bool constant=false) {
			// Syntax : a let expect a ':' or a '='
			cout << "Visiting let" << endl;
			auto [child] = node->children;
			auto type = child->type();

			// assignment
			if (type == Token::Equal) {
				auto [left, right] = child->children;

				if (left->type() == Token::Colon) {
					auto [identifier, type] = left->children;
				}
				else if (left->type() == Token::Identifier) {
					context()[value(left)] = {
						node, left,
						shared, constant
					};
				}
				else {
					throw "Invalid identifier in let statement";
				}
			}

			// declaration
			else if (type == Token::Colon) {
				auto [identifier] = child->children;
				if (identifier->type() != Token::Identifier)
					throw "A let statement expects an identifier";

				context()[] = { node, identifier };
			}

			// errors
			else if (type == Token::Identifier) {
				throw "A let statement expect a type or a value after the identifier";
			}
			else {
				throw "Unexpected token after let statement";
			}
		}

	};
}