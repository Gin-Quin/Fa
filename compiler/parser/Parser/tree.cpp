
/**
 * Parse the current statement at the current position
 * Return a new node
 */
Node* Parser::parseExpression (Token::Type groupType) {
	if (position >= currentStatement->size())
		return NULL;
	
	Node* leftNode;
	Node* rightNode = NULL;
	Token* token;
	Token* lastToken;
	int leftGlue;
	int rightGlue;
	Token::Type stopAtToken = getStopToken(groupType);
	vector<Node*> stack;

	if (position == 0)
		cout
			<< "Parsing expression with "
			<< currentStatement->size()
			<< " tokens"
			<< endl
		;


	// we create the left node from the first token
	token = &currentStatement->at(position++);
	checkChaining(NULL, token);
	leftGlue = token->glue();
	leftNode = Node::from(token);

	cout << " | "
		<< position << " | "
		<< "token : " << token->type << " | "
	<< endl;

	// if the first node is a group...
	if (leftGlue & Glue::Group)
		leftNode->assimilate(parseExpression(token->type));
		


	// we loop through all the tokens
	while (position < currentStatement->size()) {
		lastToken = token;
		token = &currentStatement->at(position++);
		checkChaining(lastToken, token);

		cout << " | "
			<< position << " | "
			<< "token : " << token->type << " | "
		<< endl;

		// we stop here if we have reached the end of a group
		if (stopAtToken == token->type)
			return stack.size()? stack[0] : leftNode;
		
		rightGlue = token->glue();
		rightNode = Node::from(token);

		// if we have a group we find its content
		if (rightGlue & Glue::Group)
			rightNode->assimilate(parseExpression(token->type));

		// both left and right glue to each other...
		if ((rightGlue & Glue::Left) && (leftGlue & Glue::Right)) {
			cout << "Priority fight..." << endl;
			int rightPriority = token->priority();
			Node* parent = leftNode;
			stack.push_back(leftNode);
			bool isSingle = rightGlue & Glue::Single;

			while (rightPriority <= parent->priority()) {
				if (rightPriority == parent->priority()) {
					if (isSingle || token->type != parent->token->type)
						break;

					// fusion
					cout << "Fusion!" << endl;
					stack.pop_back();
					delete rightNode;
					leftNode = parent;
					goto next;
				}

				cout << "    Cukolding : up 1 parent" << endl;
				stack.pop_back();
				if (!stack.size()) {  // lowest priority of all stack
					cout << "Ultimate cuckolding!" << endl;
					leftNode = rightNode->assimilate(parent);
					goto next;
				}
				parent = stack.back();
			}

			cout << "Cuckolding!" << endl;
			leftNode = parent->cuckoldedBy(rightNode);
		}

		// only right is gluing
		else if (rightGlue & Glue::Left) {
			leftNode = rightNode->assimilate(leftNode);
		}

		// only left is gluing
		else if (leftGlue & Glue::Right) {
			leftNode->assimilate(rightNode);
			if (!(rightGlue & Glue::Assimilable)) {
				stack.push_back(leftNode);
				leftNode = rightNode;
			}
		}

		// none of them glue to each other : error
		else {
			throw "The left and right nodes don't glue";
		}

		next:
		leftGlue = leftNode->glue();
	}


	checkChaining(token, NULL);
	return stack.size()? stack[0] : leftNode;
}



Node* Parser::growTree() {
	if (!hasTokenized)
		tokenize();
	
	// we reset the cursor and the current statement
	position = 0;
	currentStatement = scope[0]->front();

	try {
		tree = parseExpression();
	}
	catch (string message) {
		cout << endl << "//!!\\\\" << endl << message << endl << endl;
	}
	catch (const char* message) {
		cout << endl << "//!\\\\" << endl << message << endl << endl;
	}
	
	hasGrownTree = true;
	return tree;
}



Token::Type Parser::getStopToken(Token::Type type) {
	if (type == Token::Type::UnknownToken)
		return Token::Type::UnknownToken;
	
	if (type == Token::Type::LeftParenthesis)  // (...)
		return Token::Type::RightParenthesis;

	if (type == Token::Type::RegexStart)  // //...//
		return Token::Type::RegexOrGlobEnd;
	
	if (type == Token::Type::GlobStart)  // ||...||
		return Token::Type::RegexOrGlobEnd;
	
	if (type == Token::Type::LeftCurlyBrace)  // {...}
		return Token::Type::RightCurlyBrace;
	
	if (type == Token::Type::LeftBrace)  // [...]
		return Token::Type::RightBrace;
	
	if (type == Token::Type::String)  // "..."
		return Token::Type::StringEnd;

	return Token::Type::UnknownToken;
}



/**
 * Check if two consecutive tokens can be next to each other
 */
void Parser::checkChaining(Token* left, Token* right) {
	if (!left) {
		int glue = right->glue();
		if (glue & Glue::Left) {
			if (glue & Glue::WeakLeft)
				right->type = static_cast<Token::Type>(static_cast<int>(right->type) + 1);
			else
				throw "Expected assimilable value before first token";
		}
	}
	else if (!right) {
		int glue = left->glue();
		if (glue & Glue::Right)
			throw "Expected assimilable value after last token";
	}
	else {
		int leftGlue = left->glue();
		int rightGlue = right->glue();
		if ((leftGlue & Glue::Right) && (rightGlue & Glue::Left)) {
			if (rightGlue & Glue::WeakLeft) 
				right->type = static_cast<Token::Type>(static_cast<int>(right->type) + 1);
			else
				throw "Glue conflict : both tokens glue to each other";
		}
	}
}
