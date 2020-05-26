#pragma once
#include "TokenInfos.hpp"


struct Node {
	Token* token;

	Node(Token* from) {
		token = from;
	}

	inline int priority() {
		return token->priority();
	}
	inline int glue() {
		return token->glue();
	}

	virtual Node* nextChild() {
		return NULL;
	}
	
	virtual Node* lastChild() {
		return NULL;
	}

	virtual Node* assimilate(Node* node) {
		throw "A terminal Node cannot assimilate another Node!";
		return this;
	}

	virtual Node* cuckoldedBy(Node* node) {
		throw "A terminal Node cannot get cuckolded by another Node!";
		return this;
	}

	/**
	 * Create a node from the given token
	 */
	static Node* from(Token* token);
};




/** -- Subclasses -- **/
// A generic node structure containing a body of nodes
struct BodyNode : Node {
	vector<Node*> body {};
	int cursor { 0 };

	BodyNode(Token* from) : Node(from) {}
	~BodyNode() {
		for (auto child : body)
			delete child;
	}

	Node* nextChild() {
		if (cursor < body.size())
			return body[cursor++];
		cursor = 0;
		return NULL;
	}

	Node* lastChild() {
		return body.size()? body.back() : NULL;
	}

	Node* assimilate(Node* node) {
		body.push_back(node);
		return this;
	}

	Node* cuckoldedBy(Node* node) {
		if (body.size()) {
			node->assimilate(body[body.size() - 1]);
			body[body.size() - 1] = node;
		}
		else
			body.push_back(node);
		return node;
	}
};


// An operation node
struct OperationNode : BodyNode {
	OperationNode(Token* from) : BodyNode(from) {}
};


// A module node
struct ModuleNode : BodyNode {
	string name;
	ModuleNode() : BodyNode(NULL) {}
};


/**  The structure for every left + right operators **/
struct SingleOperationNode : Node {
	Node* left { NULL };
	Node* right { NULL };
	int cursor { 0 };

	SingleOperationNode(Token* from) : Node(from) {}
	~SingleOperationNode() {
		delete left;
		delete right;
	}

	Node* nextChild() {
		cursor = (cursor + 1) % 3;
		return cursor ? (cursor == 1 ? left : right) : NULL;
	}

	Node* lastChild() {
		return right? right : (left? left : NULL);
	}

	Node* assimilate(Node* node) {
		if (!left) left = node;
		else if (!right) right = node;
		else throw "A SingleOperationNode can only assimilate twice!";
		return this;
	}

	Node* cuckoldedBy(Node* node) {
		if (right) {
			node->assimilate(right);
			right = node;
		}
		else if (left) {
			node->assimilate(left);
			left = node;
		}
		else
			left = node;
		return node;
	}
};


/**  The common structure for commands, units and groups **/
struct ExpressionNode : Node {
	Node* expression { NULL };
	bool visited { false };

	ExpressionNode(Token* from) : Node(from) {}
	~ExpressionNode() {
		delete expression;
	}

	Node* nextChild() {
		visited = !visited;
		return visited ? expression : NULL;
	}

	Node* lastChild() {
		return expression? expression : NULL;
	}

	Node* assimilate(Node* node) {
		if (!expression) expression = node;
		else throw "An ExpressionNode can only assimilate once!";
		return this;
	}

	Node* cuckoldedBy(Node* node) {
		if (expression)
			node->assimilate(expression);
		expression = node;
		return node;
	}
};



/** -- New node -- **/
Node* Node::from(Token* token) {
	int glue = token->glue();

	if (glue & Glue::Single)
		return new SingleOperationNode(token);

	if (glue & Glue::Group)
		return new ExpressionNode(token);

	if (glue & Glue::Right) {
		if (glue & Glue::Left)
			return new OperationNode(token);
		return new ExpressionNode(token);
	}

	if (glue & Glue::Left)
		return new ExpressionNode(token);

	if (glue != Glue::Assimilable) {
		throw string("A non-assimilable-only token is created as terminal! Token type : ") + to_string(token->type);
	}
	
	return new Node(token);  // terminal node
}
