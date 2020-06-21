#destination "compiler/walker"
#pragma once
#include "../common.hpp"
#include "utils/NodeCallback.hpp"
#include "../parser/Parser.hpp"

namespace Walker {
	/**
	* The abstract walker class every walker should inherit from
	*/
	struct BaseWalker {
		Parser* parser;

		BaseWalker(Parser& _parser) {
			parser = &_parser;
		}

		inline string value(Node* node) {
			return parser->extract(node);
		}

		virtual void start() {
			if (!parser->tree)
				throw "No syntax tree to walk through";
			walk(parser->tree);
		}


		virtual void walk(Node* node) {
			for (Node* child : parser->tree->children)
				visit(child);
		}

		// virtual methods
		${tokens.map(token => `virtual void visit${token}(Node* node) {std::cout << "Visit ${token} from BaseWalker" << std::endl;}`).join('\n\t\t')}

		// node visiter
		virtual void visit(Node* node) {
			switch (node->token->type) {
				${tokens.map(token => `case Token::Type::${token}: return visit${token}(node);`).join('\n\t\t\t\t')}
			}
		}
	};
}
