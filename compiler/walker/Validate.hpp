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

		Validate(Parser& parser) : BaseWalker(parser) {
			auto moduleData = new NodeData::Module();
			parser.tree->data = (void*) moduleData;
			scopes.push_back(moduleData->scope);
		}

		// -- Visiters
		void visitLet(Node* node) {
			cout << "Visiting let" << endl;
		}


	};
}