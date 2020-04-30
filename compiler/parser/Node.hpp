#pragma once
#include "NodeType.hpp"

struct Node {
	// by default a terminal node (number, string, regex, ...)
	NodeType type { NodeType::Terminal };
	Token* token { NULL };

	Node() {}
	Node(Token& from) { token = &from; }
	Node(Token* from) { token = from; }

	virtual Node* nextChild() {
		return NULL;
	}
};


/**  The structure for every left + right operators **/
struct OperationNode : Node {
	NodeType type { NodeType::LeftRight };
	vector<Node*> arguments;

	OperationNode(Token* from, Node* leftNode) : Node(from) {
		arguments.push_back(leftNode);
	}

	~OperationNode() {
		for (Node* node : arguments)
			delete node;
	}

	Node* nextChild() {
		static int cursor = 0;
		if (cursor < arguments.size())
			return arguments[cursor++];
		cursor = 0;
		return NULL;
	}
};

/**  The structure for every left + right operators **/
struct SingleOperatioNode : Node {
	NodeType type { NodeType::SingleLeftRight };
	Node* left;
	Node* right;

	~SingleOperatioNode() {
		delete left;
		delete right;
	}

	Node* nextChild() {
		static int cursor = 0;
		cursor = (cursor++) % 3;
		return cursor ? cursor == 1 ? left : right : NULL;
	}
};

/**  The structure for every right operators **/
struct UnitNode : Node {
	NodeType type { NodeType::Right };
	Node* expression;

	~UnitNode() {
		delete expression;
	}
	Node* nextChild() {
		static bool visited = false;
		visited = !visited;
		return visited ? expression : NULL;
	}
};

/**  The structure for a user's custom function call **/
struct CallNode : Node {
	NodeType type { NodeType::Left };
	Node* expression;

	~CallNode() {
		delete expression;
	}

	Node* nextChild() {
		static bool visited = false;
		visited = !visited;
		return visited ? expression : NULL;
	}
};

/**  The structure for a user's custom function call **/
struct ExpressionNode : Node {
	NodeType type { NodeType::Group };
	Node* expression;

	~ExpressionNode() {
		delete expression;
	}

	Node* nextChild() {
		static bool visited = false;
		visited = !visited;
		return visited ? expression : NULL;
	}
};
