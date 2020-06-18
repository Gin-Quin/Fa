void visit() {
	switch (node->token->type) {
		case Token::Type::Unknown: return Unknown();
		case Token::Type::Integer: return Integer();
	}
}
