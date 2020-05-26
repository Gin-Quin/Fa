#pragma once

#include "Glue.hpp"

struct TokenInfo {
	int glue;
	int priority { 0 };  // 0 : not concerned
};

constexpr TokenInfo tokenInfos[] = {
	{ /* UnknownToken */ },
	{ Glue::Assimilable },  // Number
	{ /* Comment */ },
	{ /* SubComment */ },
	{ /* Checkpoint */ },
	{ Glue::Assimilable },  // Identifiers (glue depends on definition)
	{ Glue::Assimilable },  // String
	{ Glue::Assimilable },  // RawString
	{ /* StringEnd */ },


	{ /** -- SYMBOLS -- **/  },
	{ Glue::Group | Glue::Assimilable },          // LeftParenthesis
	{ Glue::Group | Glue::Assimilable },          // RegexStart
	{ Glue::Group | Glue::Assimilable },          // GlobStart
	{ Glue::Group | Glue::Assimilable },          // LeftBrace
	{ Glue::Left | Glue::Right, 30 },             // Backslash,
	{ Glue::Left | Glue::Right, 30 },             // DoubleBackslash,
	{ Glue::Left | Glue::Right, 6 },              // Equal,
	{ Glue::Left | Glue::Right | Glue::Body, 5 }, // Colon,
	{ Glue::Group | Glue::Assimilable },          // LeftCurlyBrace,
	{ Glue::Left | Glue::Right, 40 },             // Dot,
	{ Glue::Left | Glue::Right, 7 },              // Comma,
	{ Glue::Group | Glue::Assimilable },          // Apostrophe,
	{ Glue::Group | Glue::Assimilable },          // Quote,
	{ Glue::Group | Glue::Assimilable },          // Accent,
	{ Glue::Left | Glue::Right, 30 },             // Asterisk,
	{ Glue::Left | Glue::Right, 30 },             // Divide,
	{ Glue::Left | Glue::Right, 18 },             // Circumflex,
	{ Glue::Left | Glue::Right | Glue::WeakLeft, 28 }, // Plus,
	{ Glue::Right, 31 },                               // Plus (only right),
	{ Glue::Left | Glue::Right | Glue::WeakLeft, 29 }, // Minus,
	{ Glue::Right, 31 },                               // Minus (only right),
	{ Glue::Left },                               // QuestionMark,
	{ Glue::Right },                              // Tilde,
	{ Glue::Left | Glue::Right, 22 },             // DoubleEqual,
	{ Glue::Left | Glue::Right, 22 },             // NotEqual,
	{ Glue::Left | Glue::Right, 22 },             // Equivalent,
	{ Glue::Left | Glue::Right, 22 },             // LesserOrEqual,
	{ Glue::Left | Glue::Right, 22 },             // GreaterOrEqual,
	{ Glue::Left | Glue::Right, 8 },              // InputArrow,
	{ Glue::Left | Glue::Right, 8 },              // OutputArrow,
	{ Glue::Left },                               // Percent,
	{ Glue::Left | Glue::Right, 6 },              // Extract,
	{ Glue::Left | Glue::Right, 6 },              // Insert,
	{ Glue::Left | Glue::Right, 40 },             // DoubleDot,
	{ Glue::Right },                              // TripleDot,
	{ Glue::Assimilable },                        // MultiLineString,
	{ Glue::Left, 38 },                           // PlusPlus,
	{ Glue::Left, 38 },                           // MinusMinus,
	{ Glue::Left | Glue::Right, 32 },             // Power,
	{ Glue::Left | Glue::Right },                 // PlusEqual,
	{ Glue::Left | Glue::Right },                 // MinusEqual,
	{ Glue::Left | Glue::Right },                 // TimesEqual,
	{ Glue::Left | Glue::Right },                 // DivideEqual,
	{ Glue::Left | Glue::Right },                 // IntegerDivideEqual,
	{ Glue::Left | Glue::Right },                 // LesserThan,
	{ Glue::Left | Glue::Right },                 // GreaterThan,
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
	{ Glue::Right },                   // If
	{ Glue::Right },                   // Else
	{ Glue::Right },                   // ElseIf
	{ Glue::Right },                   // Then
	{ Glue::Right },                   // Do
	{ Glue::Right },                   // While
	{ Glue::Right },                   // Repeat
	{ Glue::Right },                   // For
	{ Glue::Left | Glue::Right },      // In
	{ Glue::Right },                   // When
	{ Glue::Left | Glue::Right },      // And
	{ Glue::Left | Glue::Right },      // Or
	{ Glue::Left | Glue::Right },      // Xor
	{ Glue::Left | Glue::Right },      // Modulo
	{ Glue::Left | Glue::Right },      // Is
	{ Glue::Left | Glue::Right },      // As
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
	{ Glue::Right },                   // Class
	{ Glue::Right },                   // Enum
	{ Glue::Right },                   // Abstract
	{ Glue::Right },                   // Interface
	{ Glue::Right },                   // Structure
	{ Glue::Right },                   // Unique
	{ Glue::Right },                   // Exclamation
	{ Glue::Assimilable },             // Self
	{ Glue::Left | Glue::Right },      // Self
};
