#pragma once


struct Slice {
	int start;
	int end;

	inline int length() {
		return end - start;
	}
};

