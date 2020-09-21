#include "Glue.hpp"

struct TokenInfo {
	int glue;
	int priority { 0 };  // 0 : not concerned
};

constexpr TokenInfo tokenInfos[] = {
	{ /* UnknownToken */ },
	{ Glue::Assimilable },                       // Number
	{ Glue::Body | Glue::OptionalBody },         // Comment
	{ /* SubComment */ },
	{ /* Checkpoint */ },
	{ Glue::Assimilable },                       // Identifiers (glue depends on definition)
	{ Glue::Group },                             // String
	{ Glue::Assimilable },                       // RawString
	{ /* StringEnd */ },

	{ /** -- SYMBOLS -- **/  },
	{ Glue::Group | Glue::Assimilable },          // LeftParenthesis
	{ Glue::Group | Glue::Assimilable },          // RegexStart
	{ Glue::Group | Glue::Assimilable },          // GlobStart
	{ Glue::Group | Glue::Left | Glue::WeakLeft },          // LeftBrace
	{ Glue::Group | Glue::Assimilable },                    // LeftBrace (no left)
	{ Glue::Left | Glue::Right, 30 },             // Backslash,
	{ Glue::Left | Glue::Right, 30 },             // DoubleBackslash,
	{ Glue::Left | Glue::Right, 6 },              // Equal,
	{ Glue::Left | Glue::Right | Glue::TransformAtEndOfStatement, 7 },       // Colon,
	{ Glue::Left | Glue::Body, 0 },                                          // Colon (end of statement),
	{ Glue::Group | Glue::Assimilable },          // LeftCurlyBrace,
	{ Glue::Left | Glue::Right, 40 },             // Dot,
	{ Glue::Left | Glue::Right, 2 },              // Comma,
	{ Glue::Group | Glue::Assimilable },          // Apostrophe,
	{ Glue::Group | Glue::Assimilable },          // Quote,
	{ Glue::Group | Glue::Assimilable },          // Accent,
	{ Glue::Left | Glue::Right, 30 },             // Asterisk,
	{ Glue::Left | Glue::Right, 30 },             // Divide,
	{ Glue::Left | Glue::Right, 18 },             // Circumflex,
	{ Glue::Left | Glue::Right | Glue::WeakLeft, 28 },          // Plus,
	{ Glue::Right, 31 },                                        // Plus (only right),
	{ Glue::Left | Glue::Right | Glue::WeakLeft, 29 },          // Minus,
	{ Glue::Right, 31 },                                        // Minus (only right),
	{ Glue::Left },                               // QuestionMark,
	{ Glue::Right },                              // Tilde,
	{ Glue::Left | Glue::Right, 22 },             // DoubleEqual,
	{ Glue::Left | Glue::Right, 22 },             // NotEqual,
	{ Glue::Left | Glue::Right, 22 },             // Equivalent,
	{ Glue::Left | Glue::Right, 22 },             // LesserOrEqual,
	{ Glue::Left | Glue::Right, 22 },             // GreaterOrEqual,
	{ Glue::Left | Glue::Right, 8 },              // InputArrow,
	{ Glue::Left | Glue::Right, 8 },              // OutputArrow,
	{ Glue::Left, 31 },                           // Percent,
	{ Glue::Left | Glue::Right, 6 },              // Extract,
	{ Glue::Left | Glue::Right, 6 },              // Insert,
	{ Glue::Left | Glue::Right | Glue::TransformAtEndOfStatement, 40 },     // DoubleDot
	{ Glue::Left | Glue::TransformAtEndOfStatement, 40 },                   // DoubleDot (end of statement)
	{ Glue::Right | Glue::TransformAtEndOfStatement },                      // TripleDot
	{ Glue::Body },                                                         // TripleDot (end of statement)
	{ Glue::Assimilable },                        // MultiLineString,
	{ Glue::Left, 38 },                           // PlusPlus,
	{ Glue::Left, 38 },                           // MinusMinus,
	{ Glue::Left | Glue::Right, 32 },             // Power,
	{ Glue::Left | Glue::Right, 6 },                 // PlusEqual,
	{ Glue::Left | Glue::Right, 6 },                 // MinusEqual,
	{ Glue::Left | Glue::Right, 6 },                 // TimesEqual,
	{ Glue::Left | Glue::Right, 6 },                 // DivideEqual,
	{ Glue::Left | Glue::Right, 6 },                 // IntegerDivideEqual,
	{ Glue::Left | Glue::Right, 24 },                 // LesserThan,
	{ Glue::Left | Glue::Right, 24 },                 // GreaterThan,
	{ Glue::Left | Glue::Right },                 // SendTo,
	{ Glue::Left | Glue::Right },                 // Pipe,
	{ Glue::Right, 50 },                          // At,
	{ /* Semicolon */ },
	{ /* RightParenthesis */ },
	{ /* RegexOrGlobContent */ },
	{ /* RegexOrGlobEnd */ },
	{ /* RegexOrGlobOption */ },
	{ /* RightBrace */ },
	{ /* RightCurlyBrace */ },
	{ /* UserDefinedSymbol */ },

	{ /** -- KEYWORDS -- **/  },
	{ Glue::Right },                   // Let
	{ Glue::Assimilable },             // Super
	{ Glue::Right },                   // Print
	{ Glue::Right },                   // Use
	{ Glue::Right },                   // Import
	{ Glue::Right },                   // Export
	{ Glue::Left | Glue::Right },      // From
	{ Glue::Left | Glue::Right },      // Extends
	{ Glue::Left | Glue::Right | Glue::TransformAtStartOfStatement }, // If (comprehension)
	{ Glue::Right | Glue::Body },                   // If
	{ Glue::Left | Glue::Right | Glue::TransformAtStartOfStatement }, // Else (comprehension)
	{ Glue::Body },                   // Else
	{ Glue::Right | Glue::Body },                   // ElseIf
	{ Glue::Right },                   // Then
	{ Glue::Right },                   // Do
	{ Glue::Left | Glue::Right | Glue::TransformAtStartOfStatement }, // While (comprehension)
	{ Glue::Right | Glue::Body | Glue::OptionalBody },   // While
	{ Glue::Left | Glue::Right | Glue::TransformAtStartOfStatement }, // Repeat (comprehension)
	{ Glue::Right | Glue::Body },                        // Repeat
	{ Glue::Left | Glue::Right | Glue::TransformAtStartOfStatement }, // For (comprehension)
	{ Glue::Right | Glue::Body },                        // For
	{ Glue::Left | Glue::Right },      // In
	{ Glue::Right | Glue::Body },      // When
	{ Glue::Left | Glue::Right, 12 },      // And
	{ Glue::Left | Glue::Right, 10 },      // Or
	{ Glue::Left | Glue::Right },      // Xor
	{ Glue::Left | Glue::Right },      // Modulo
	{ Glue::Left | Glue::Right | Glue::TransformAtStartOfStatement }, // Is
	{ Glue::Right | Glue::Body | Glue::OptionalBody },      // Is (start of statement)
	{ Glue::Left | Glue::Right },      // To
	{ Glue::Right },                   // Not
	{ Glue::Left | Glue::Right },      // Isnt
	{ Glue::Right },                   // Return
	{ Glue::Right },                   // Continue
	{ Glue::Right },                   // Break
	{ Glue::Right },                   // Try
	{ Glue::Right },                   // Catch
	{ Glue::Right },                   // Finally
	{ Glue::Right },                   // Throw
	{ Glue::Right },                   // Async
	{ Glue::Right },                   // Await
	{ Glue::Right },                   // Yield
	{ Glue::Assimilable },             // Nil
	{ Glue::Assimilable },             // True
	{ Glue::Assimilable },             // False
	{ Glue::Assimilable },             // Infinity
	{ Glue::Right },                   // Global
	{ Glue::Right },                   // Override
	{ Glue::Right },                   // Final
	{ Glue::Right },                   // Const
	{ Glue::Right },                   // Private
	{ Glue::Right },                   // Static
	{ Glue::Right | Glue::Body | Glue::OptionalBody },                   // Class
	{ Glue::Right | Glue::Body | Glue::OptionalBody },                   // Type
	{ Glue::Right | Glue::Body | Glue::OptionalBody },                   // Abstract
	{ Glue::Right | Glue::Body | Glue::OptionalBody },                   // Interface
	{ Glue::Right | Glue::Body | Glue::OptionalBody },                   // Structure
	{ Glue::Right | Glue::Body | Glue::OptionalBody },                   // Unique
	{ Glue::Right },                   // Exclamation
	{ Glue::Assimilable },             // Self
};
