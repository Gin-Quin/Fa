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
		${tokens.map(({name}) => `virtual void visit${name}(Node* node) {std::cout << "Visit ${name} from BaseWalker" << std::endl;}`).join('\n\t\t')}

		// node visiter
		virtual void visit(Node* node) {
			switch (node->token->type) {
				${tokens.map(({name}) => `case Token::Type::${name}: return visit${name}(node);`).join('\n\t\t\t\t')}
			}
		}
	};
}
