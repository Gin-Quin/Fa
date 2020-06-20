#pragma once

using TypeId = unsigned long long;

/**
 * A variable type is identified by a number and a number of sub types.
 */
struct Type {
	static TypeId nextId;
	TypeId id;
	vector<Type> subs;  // sub types

	Type() {
		id = nextId++;
	}

	/**
	 * Check if two types are the same
	 */
	bool operator==(const Type& other) {
		// first we check they have the same id
		if (id != other.id)
			return false;
		
		// then we check they have the same subs
		for (int i=0; i < subs.size(); i++)
			if (subs[i] != other.subs[i])
				return false;
		return true;
	}

	inline bool operator!=(const Type& other) {
		return !(*this == other);
	}
};

TypeId Type::nextId = 1;