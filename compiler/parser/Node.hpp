#pragma once
#include "TokenInfos.hpp"

/**
 * The base structure for a Node Fa's syntax tree.
 * The Fa parser uses its own algorithm to generate the tree.
 * This algorithm is only based on two operations :
 - `assimilate` : add a child node
 - `cuckold` : a node cuckolds another node when :
	1. It becomes the father of the node's last child
	2. He then becomes the last child of the node
 * This algorithm makes Fa sort of a `grammar-less` language.
 * The grammar is checked during the validation phase, each node having its own validation rules.
 */
struct Node {
	Token* token;
	vector<Node*> children;
	void* data { NULL };  // the arbitrary data that brings the node. Used and defined by tree walkers.

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

	/**
	 * {"type": xx, "position": xxxx, "length": xx}
	 */
	inline string toJson() {
		string json;
		json += '{';
			json += "\"token\":";
			json += token ? token->toJson() : "null,";

			json += "\"children\":[";
				for (int i=0; i < children.size(); i++) {
					if (i) json += ',';
					json += children[i]->toJson();
				}
			json += ']';
		json += '}';
		return json;
	}
};



