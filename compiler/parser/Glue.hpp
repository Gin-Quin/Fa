#pragma once
/**
 * Describe how a token should glue to the others.
 * One token can 
 */

namespace Glue {
	constexpr unsigned int
		NotConcerned = 0,  // Not concerned by gluing (for special cases)
		Assimilable = 1,  // Can be glued but do not glue others
		Group = 2,  // Glue inside (Parenthesis, brackets, ...)
		Left = 4,  // Glue a left token
		Right = 8,  // Glue a right token
		Body = 16,  // Glue a following body

		// -- modifiers
		Single = 32,  // When glue left and right, cannot chain gluing
		WeakLeft = 64,  // When left is not possible, increment token type by 1
		WeakRight = 128,  // No error thrown when right is not possible
		OptionalBody = 256,  // The token can glue a body but not mandatory (ex: comments)
		TransformAtStartOfStatement = 512,  // When at start of statement, increment token type
		TransformAtEndOfStatement = 1024;  // When at end of statement, increment token type by 1
}
