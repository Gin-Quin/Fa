

/**
 * Parse a body (vector of statements) and return a BodyNode
 */
Node* Parser::parseBody(const Body& body) {
	auto root = new Node();
	for (auto statement : body) {
		position = 0;
		root->assimilate(parseStatement(statement));
	}
	return root;
}


/**
 * Parse the current statement at the current position
 * Return a new node, the root of the statement
 */
Node* Parser::parseStatement(Statement* statement, Token::Type groupType) {
	if (position >= statement->size())
		return NULL;

	Node* leftNode;
	Node* rightNode = NULL;
	Node* bodyNode = NULL;
	Token* token;
	Token* lastToken;
	int leftGlue;
	int rightGlue;
	Token::Type stopAtToken = getStopToken(groupType);
	vector<Node*> stack;

	// we check if the last token needs to be transformed
	lastToken = &statement->back();
	if (lastToken->glue() & Glue::TransformAtEndOfStatement)
		lastToken->incrementType();

	// we create the left node from the first token
	token = &statement->at(position++);

	// we check if the first token needs to be transformed
	if (position == 1 && (token->glue() & Glue::TransformAtStartOfStatement))
		token->incrementType();

	checkChaining(NULL, token);
	leftGlue = token->glue();
	leftNode = Node::from(token);

	if (leftGlue & Glue::Body)  // if the node can have a body
		bodyNode = leftNode;

	if (token->type == Token::Type::String)  // if the node is a template string
		parseTemplateString(statement, leftNode);
	else if (leftGlue & Glue::Group)  // if the first node is a group...
		leftNode->assimilate(parseStatement(statement, token->type));

	rightGlue = leftGlue;

	// we loop through all the tokens
	while (position < statement->size()) {
		lastToken = token;
		token = &statement->at(position++);
		checkChaining(lastToken, token);

		// we stop here if we have reached the end of a group
		if (stopAtToken == token->type) {
			checkChaining(lastToken, NULL);
			return stack.size()? stack[0] : leftNode;
		}

		rightGlue = token->glue();
		rightNode = Node::from(token);

		// if the node can have a body
		if (rightGlue & Glue::Body) {
			if (!bodyNode)
				bodyNode = rightNode;
			else {
				if (bodyNode->glue() & Glue::OptionalBody)
					bodyNode = rightNode;
				else if (!(rightGlue & Glue::OptionalBody))
					throw "Two tokens in the same statement need a body";
			}
		}

		// if we have a group we find its content
		if (token->type == Token::Type::String)  // if the node is a template string
			parseTemplateString(statement, rightNode);
		else if (rightGlue & Glue::Group)
			rightNode->assimilate(parseStatement(statement, token->type));

		// both left and right glue to each other...
		if ((rightGlue & Glue::Left) && (leftGlue & Glue::Right)) {
			int rightPriority = token->priority();
			Node* parent = leftNode;
			stack.push_back(leftNode);
			bool isSingle = rightGlue & Glue::Single;

			while (rightPriority <= parent->priority()) {
				if (rightPriority == parent->priority()) {
					if (isSingle || token->type != parent->token->type)
						break;

					// fusion
					stack.pop_back();
					delete rightNode;
					leftNode = parent;
					goto next;
				}

				stack.pop_back();
				if (!stack.size()) {  // lowest priority of all stack
					leftNode = rightNode->assimilate(parent);
					goto next;
				}
				parent = stack.back();
			}

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
			cout << "Left token : " << leftNode->token->type << endl;
			cout << "Right token : " << rightNode->token->type << endl;
			throw "The left and right nodes don't glue";
		}

		next:
		leftGlue = leftNode->glue();
	}

	checkChaining(token, NULL);

	// we check if there is a body and if it can be consumed
	if (statement->hasBody()) {
		if (!bodyNode)
			throw "The body cannot be consumed";
		bodyNode->assimilate(parseBody(statement->body));
	}
	else if (bodyNode && !(bodyNode->glue() & Glue::OptionalBody)) {
		throw "A body is expected!";
	}

	return stack.size()? stack[0] : leftNode;
}


/**
 * Parse a template string
 * Return a single node containing the string and all its children
 */
void Parser::parseTemplateString(Statement* statement, Node* root) {
	Token* token;

	// the tokenization ensured there is a StringEnd token
	while ((token = &statement->at(position++))) switch (token->type) {
		case Token::Type::StringEnd:
			return;

		case Token::Type::RawString:
			root->assimilate(Node::from(token));
			break;

		case Token::Type::LeftCurlyBrace:
			root->assimilate(parseStatement(statement, Token::Type::LeftCurlyBrace));
			break;

		default:
			throw "Unexpected token in string template";
	}
}



/**
 * Parse the whole melody
 */
Node* Parser::growTree() {
	if (!hasTokenized)
		tokenize();
	tree = parseBody(body());
	hasGrownTree = true;
	return tree;
}



/**
 * Return the stop token corresponding to a token group
 */
Token::Type Parser::getStopToken(Token::Type type) {
	switch (type) {
		case Token::Type::UnknownToken :
			return Token::Type::UnknownToken;

		case Token::Type::LeftParenthesis :  // (...)
			return Token::Type::RightParenthesis;

		case Token::Type::LeftCurlyBrace :  // {...}
			return Token::Type::RightCurlyBrace;

		case Token::Type::LeftBrace :  // [...]
		case Token::Type::LeftBraceNoLeft :
			return Token::Type::RightBrace;

		case Token::Type::String :  // "..." or '...'
			return Token::Type::StringEnd;

		case Token::Type::RegexStart :  // //...//
		case Token::Type::GlobStart :   // ||...||
			return Token::Type::RegexOrGlobEnd;

		default:
			return Token::Type::UnknownToken;
	}
}



/**
 * Check if two consecutive tokens can be next to each other
 */
void Parser::checkChaining(Token* left, Token* right) {
	if (!left) {
		int glue = right->glue();
		if (glue & Glue::Left) {
			if (glue & Glue::WeakLeft)
				right->incrementType();
			else
				throw "Missing expression before token";
		}
	}
	else if (!right) {
		int glue = left->glue();
		if ((glue & Glue::Right) && !(glue & Glue::WeakRight)) {
			cout << "Token : " << left->type << endl;
			throw "Missing expression after token";
		}
	}
	else {
		int leftGlue = left->glue();
		int rightGlue = right->glue();
		if ((leftGlue & Glue::Right) && (rightGlue & Glue::Left)) {
			if (rightGlue & Glue::WeakLeft)
				right->incrementType();
			else
				throw "Glue conflict : both tokens glue to each other";
		}
	}
}
