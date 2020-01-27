#pragma once

#include "../common/index.hpp"
#include "../lexer/Tokenizer.cpp"

#include "Node.hpp"
#include "SymbolInfo.hpp"
#include "ParseInfo.hpp"

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
