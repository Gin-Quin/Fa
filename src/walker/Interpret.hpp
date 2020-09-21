#include "Validate.hpp"

namespace Walker {

	/**
	 * This walker make sure the given tree is valid.
	 * Plus it feeds the `data` value of every node.
	 */
	struct Interpret : Validate {
		Interpret(Parser& parser) : Validate(parser) {}
	};
}