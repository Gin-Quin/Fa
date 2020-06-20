#destination "compiler/walker"
#pragma once
#include "../common.hpp"
#include "classes/NodeCallback.hpp"
#include "../parser/Parser.hpp"


/**
 * A walker is a structure used to walk an AST.
 */
struct Walker {
	Parser* parser;

	Walker(Parser& _parser) {
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
	${tokens.map(token => `virtual void visit${token}(Node* node) {std::cout << "Visit ${token} from Walker" << std::endl;}`).join('\n\t')}

	// node visiter
	virtual void visit(Node* node) {
		switch (node->token->type) {
			${tokens.map(token => `case Token::Type::${token}: return visit${token}(node);`).join('\n\t\t\t')}
		}
	}
};
