#pragma once

struct Statement;

struct Body : public vector<Statement> {
	inline void push(const Statement& statement) {
		push_back(statement);
	}
};

#include "Statement.hpp"
