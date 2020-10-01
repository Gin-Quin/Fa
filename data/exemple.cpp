/* ----------------------------------------


Let(id, type, value):
  - Colon: [id, type]
  - Equal: [TypedOrNotIdentifier(id, type), /Any(value)]

/Type(type):
  - Identifier(type)
  - Dot(type): [/Identifier...]

/TypedOrNotIdentifier(id, type):
  - Identifier(id)
  - Colon(id, type)

 ---------------------------------------- */




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