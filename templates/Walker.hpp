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

	void start() {
		if (!parser->tree)
			throw "No syntax tree to walk through";
		walk(parser->tree);
	}


	void walk(Node* node) {
		for (Node* child : parser->tree->children)
			visit(child);
	}

	// virtual methods
	${tokens.map(token => `virtual void ${token}(Node*) {}`).join('\n\t')}

	// node visiter
	void visit(Node* node) {
		switch (node->token->type) {
			${tokens.map(token => `case Token::Type::${token}: return ${token}(node);`).join('\n\t\t\t')}
		}
	}
};
