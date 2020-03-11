#pragma once
#include "NodeType.hpp"

struct Node {
	NodeType type;
	Token* token { NULL };

	Node(Token& from) { token = &from; }
	Node(Token* from) { token = from; }
};

/**  The structure for every terminal token (number, string, regex, ...) **/
struct Terminal : Node {
	NodeType type { NodeType::Terminal };
};

/**  The structure for every left + right operators **/
struct Operation : Node {
	NodeType type { NodeType::Operation };
	vector<Node*> arguments;
};

/**  The structure for every right operators **/
struct UnitOperation : Node {
	NodeType type { NodeType::UnitOperation };
	Node* expression;
};

/**  The structure for commands (special left operators) **/
struct Command : Node {
	NodeType type { NodeType::Command };
	Node* expression;
};

/**  The structure for a user's custom function call **/
struct Call : Node {
	NodeType type { NodeType::Call };
	Node* expression;
};
