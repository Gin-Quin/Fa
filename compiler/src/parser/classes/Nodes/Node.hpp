#pragma once


struct Node {
	// many different types !!
	enum Type {
		NUMBER,
		STRING,
		BOOLEAN,
		CLASS,
		UNIQUE,
		OPERATOR,
		EXPRESSION,
	};

	Type type;
	struct Position {
		unsigned int start;
		unsigned int end;
	} position;
};