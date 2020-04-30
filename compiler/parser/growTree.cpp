


/**  The structure for a user's custom function call **/
Node* Parser::parseExpression(Node* parent) {
	if (!parent) parent = tree;

	cout
		<< "Parsing expression with "
		// << statement.size()
		<< " tokens"
		<< endl
	;
	Node* node = NULL;

	while (position < currentStatement->size()) {
		Token* token = &(currentStatement->at(position++));
		NodeType nodeType = token->getNodeType();

		if (nodeType == NodeType::Terminal) {
			node = new Node(token);
		}
		else if (nodeType == NodeType::Left) {

		}
		else if (nodeType == NodeType::Right) {

		}
		else if (nodeType == NodeType::LeftRight) {
			node = new OperationNode(token, node);
			parseExpression(node);
		}
		else if (nodeType == NodeType::SingleLeftRight) {

		}
	}

	return parent;
}



Node* Parser::growTree() {
	if (!hasTokenized)
		tokenize();
	
	// we reset the cursor and the current statement
	position = 0;
	currentStatement = scope[0]->front();
	parseExpression();
	
	hasGrownTree = true;
	return tree;
}