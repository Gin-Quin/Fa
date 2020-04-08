#pragma once
#include "../common.hpp"

struct Statement;

struct Body : public vector<Statement> {
	inline void push(const Statement& statement) {
		push_back(statement);
	}
};

#include "Statement.hpp"
