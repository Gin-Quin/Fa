#pragma once

#include "../common/index.hpp"
#include "../lexer/Tokenizer.cpp"

#include "Node.hpp"
#include "getNodeType.hpp"


/**  The structure for a user's custom function call **/
Node* parseExpression (
	Statement statement,
	int start = 0,
	int stopAtWeight = 0,
	vector<Token::Type> stopAtTokens = {}
) {
	cout << "Parsing expression with " << tokens.size() << " tokens" << endl;
	int cursor = 0;
	Token* leftToken;

	while (cursor < statement.size()) {
		Token* token = &statement[cursor++];
		NodeType nodeType = getNodeType(token);

		if (nodeType == NodeType::Terminal) {
			leftToken = token;
		}
		else if (nodeType == NodeType::Left) {

		}
		else if (nodeType == NodeType::Right) {

		}
		else if (nodeType == NodeType::LeftRight) {

		}
		else if (nodeType == NodeType::SingleLeftRight) {

		}
	}


	return NULL;
}
