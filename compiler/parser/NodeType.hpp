#pragma once

enum NodeType {
	Terminal = 0,
	Group = 1,
	LeftRight = 2,
	SingleLeftRight = 3,
	Left = 4,
	Right = 8,
	Call,
	Forbidden,
};
