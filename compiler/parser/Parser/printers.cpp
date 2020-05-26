


static string padding(int padding) {
	string str = "";
	while ((padding--) > 0)
		str += ' ';
	return str;
}



// print a statement
void Parser::print(Statement* statement, int depth) {
	// we print all the tokens of the statement
	for (auto& token : *statement) {
		cout
		<< string(depth * 3, ' ')  // indentation
		<< Ink::cyan            << "type  "
		<< Ink::brightCyan      << token.type
		<< Ink::green           << padding(10 - to_string(token.type).length())
		                        << " position  "
		<< Ink::brightGreen     << token.position
		<< Ink::magenta         << padding(10 - to_string(token.position).length())
		                        << " length  "
		<< Ink::brightMagenta   << token.length

		<< padding(10 - to_string(token.length).length())
		<< coloredToken(token)
		<< Font::reset
		<< endl;
	}

	cout << endl;

	// and we print the body
	for (auto statement : statement->body)
		print(statement, depth + 1);
}


// print a node (and all its subnodes if it has)
void Parser::print(Node* node, int depth) {
	if (!node) return;
	
	if (node->token)
		cout << coloredToken(node->token) << endl;

	Node* child;
	while ((child = node->nextChild())) {
		cout << string(depth * 2, ' ') << "| ";
		print(child, depth + 1);
	}
}


// print a colored token
string Parser::coloredToken(const Token& token) {
	string content = extract(token);

	if (token.type > Token::KEYWORDS)  // keyword
		return Ink::red + content + Font::reset;

	if (token.type > Token::SYMBOLS)
		return Ink::yellow + content + Font::reset;

	switch (token.type) {
		case Token::Number :
			return Ink::white + content + Font::reset;

		case Token::Identifier :
			return Ink::white + content + Font::reset;

		case Token::String :
		case Token::RawString :
		case Token::StringEnd :
			return Ink::green + content + Font::reset;

		case Token::Comment :
		case Token::SubComment :
		case Token::Checkpoint :
			return Ink::cyan + content + Font::reset;
		
		default:
			return content;
	}
}


string Parser::coloredToken(Token* token) {
	return coloredToken(*token);
}


void Parser::printTokens() {  // print all tokens
	for (Statement* statement : *scope[0])
		print(statement);
}

void Parser::printTree() {  // print all tokens
	print(tree);
}

string Parser::error(string msg) {
	return prettyError(melody, msg, position, length);
};

string Parser::extract(const Token& token) {
	return string(melody + token.position, token.length);
}
