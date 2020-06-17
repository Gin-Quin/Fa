#pragma once
#include "TokenInfos.hpp"


struct Node {
	Token* token;
	vector<Node*> children {};

	Node() {
		token = NULL;
	}
	Node(Token* from) {
		token = from;
	}
	~Node() {
		for (auto child : children)
			delete child;
	}

	inline int priority() {
		return token ? token->priority() : 0;
	}
	inline int glue() {
		return token ? token->glue() : 0;
	}

	Node* lastChild() {
		return children.size()? children.back() : NULL;
	}

	Node* assimilate(Node* node) {
		children.push_back(node);
		return this;
	}

	Node* cuckoldedBy(Node* node) {
		if (children.size()) {
			node->assimilate(children[children.size() - 1]);
			children[children.size() - 1] = node;
		}
		else
			children.push_back(node);
		return node;
	}
};



