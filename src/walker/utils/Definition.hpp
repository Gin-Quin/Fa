#pragma once

struct Definition {
	Node* node;
	Node* identifier;

	struct {
		bool shared { false };
		bool constant { false };
	} is;
};