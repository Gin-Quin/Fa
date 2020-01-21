#pragma once

#include "../common/index.hpp"
#include "../lexer/Tokenizer.cpp"

struct Node {
	enum Type {
		Terminal,
		Operation,
		UnitOperation,
		Command,
		Call,
	};

	Type type;
	Token* token { NULL };

	Node(Token& from) { token = &from; }
	Node(Token* from) { token = from; }
};

/**  The structure for every terminal token (number, string, regex, ...) **/
struct Terminal : Node {
	Type type { Node::Type::Terminal };
};

/**  The structure for every left + right operators **/
struct Operation : Node {
	Type type { Node::Type::Operation };
	vector<Node*> arguments;
};

/**  The structure for every right operators **/
struct UnitOperation : Node {
	Type type { Node::Type::UnitOperation };
	Node* expression;
};

/**  The structure for commands (special left operators) **/
struct Command : Node {
	Type type { Node::Type::Command };
	Node* expression;
};

/**  The structure for a user's custom function call **/
struct Call : Node {
	Type type { Node::Type::Call };
	Node* expression;
};

// struct Expression : Node {
// 	Type type { Node::Type::Call };
// 	Node* expression;
// 	body;
// 	comprehension;
// };


/**  The structure for a user's custom function call **/
Node* parseExpression (
	TokenList tokens,
	int start = 0,
	int stopAtWeight = 0,
	vector<Token::Type> stopAtTokens = {}
) {
	cout << "Parsing expression with " << tokens.size() << " tokens" << endl;
	return NULL;
}
