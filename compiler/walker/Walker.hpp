#pragma once
// #include "../../common.hpp"
#include "NodeCallback.hpp"
#include "../parser/Parser.hpp"


/**
 * A walker is a structure used to walk an AST.
 */
struct Walker {
	Parser* parser;
	Node* node { NULL };

	Walker(Parser& _parser) {
		parser = &_parser;
	}

	inline string value(Node*) {
		return parser->extract(node);
	}

	
	// virtual methods
	virtual void Unknown(Node* node) {}
	virtual void Integer(Node* node) {}


	// walker
	void walk(Node* node) {
		switch (node->token->type) {
			case Token::Type::Unknown: return Unknwown();
			
		}
	}
};
