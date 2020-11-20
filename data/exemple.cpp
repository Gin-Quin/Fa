namespace Is {
	bool Let(LetNode* node) {
		if (node->type != Token::Let) return false;
		Node* [_0] = node->children;
		node->identifier = NULL;
		node->type = NULL;
		node->value = NULL;

		if (
			Is::Identifier(_0, &node->identifier)
		|| Is::Colon_declare(_0, &node->identifier, &node->type)
		|| Is::Equal_assign(_0, &node->identifier, &node->type, &node->value)
		) return true;

		throw "Missing identifier after 'Let' statement"
	}


	void Equal_assign(Node* node, Node*& identifier, Node*& type, Node*& value) {
		if (node->type != Token::Identifier) return false;
		Node* [_0, _1] = node->children;

		if (
		   Is::Identifier_declare(_0, &node->identifier)
			&& (identifier = _0, true)
			&& (value = _1, true)
		|| Is::Colon_declare(_0, &node->identifier, &node->type)
			&& (value = _1, true)
		) return true;

		throw "Cannot deal with this shit";
	}


	void Identifier_declare(Node* node, Node*& identifier) {
		if (node->type != Token::Identifier) return false;
		Node* [_0, _1] = node->children;

	}
}

validate(Node* node) {
	node->value = new Let::Data();
}
}
bool Let(Node* node) {

}

bool Let(Node* node) {
	node->value = { NULL, NULL, NULL };
	Node* [child] = node->children;

	return TypedIdentifier(child, node->value[0], node->value[1])
	    || EqualAssign(child, node->value[0], node->value[1], node->value[2]));
}


bool TypedIdentifier(Node* node, Node*& id, Node*& type) {
	return Colon()
	if (node->token() == Token::Colon) {

	}

}

bool Colon()

void Type(Node* node) {
	Node* _0 = NULL;
	if (node->token() == Token::Identifier)

	node.value
}

Node* Type(Node* node) {
	if (node->token() == Token::Identifier) {
		type = node;
	}
	else if (node->token() == Token::Dot) {

	}

	return type;
}
