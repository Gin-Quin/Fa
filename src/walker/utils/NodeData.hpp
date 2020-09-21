#include "Scope.hpp"

namespace NodeData {

	struct Module {
		string name;
		Scope scope;
	};

	struct Let {

	};

	struct Class {
		Scope scope;
	};
}
