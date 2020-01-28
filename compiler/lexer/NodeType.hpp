#pragma once

enum NodeType {
	NaN = 0,  // Not a Node (default)
	Terminal = 1,
	Group = 2,
	LeftRight = 4,
	SingleLeftRight = 8,
	Left = 16,
	Right = 32,
	// Ignored = 64,  // ignored nodes (like comments)
};


struct NodeInfo {
	NodeType type;
	int weight { 0 };  // 0 : not concerned
};

constexpr NodeInfo nodeInfosFromTokenType[] = {
	{ },  // UnknownToken
	{ NodeType::Terminal },  // Number
	{ },  // Comment
	{ },  // SubComment
	{ },  // Checkpoint
	{ NodeType::Terminal },  // Identifier
	{ NodeType::Terminal },  // String
	{ NodeType::Terminal },  // RawString
	{ },  // StringEnd


	{ /** SYMBOLS **/  },
	{ NodeType::Group },              // LeftParenthesis
	{ NodeType::Group },              // RegexStart
	{ NodeType::Group },              // GlobStart
	{ NodeType::Group },              // LeftBrace
	{ NodeType::LeftRight, 30 },      // Backslash,
	{ NodeType::LeftRight, 30 },      // DoubleBackslash,
	{ NodeType::LeftRight, 6 },       // Equal,
	{ NodeType::SingleLeftRight, 5 }, // Colon,
	{ NodeType::Group },              // LeftCurlyBrace,
	{ NodeType::LeftRight, 40 },      // Dot,
	{ NodeType::LeftRight, 7 },       // Comma,
	{ NodeType::Group },              // Apostrophe,
	{ NodeType::Group },              // Quote,
	{ NodeType::Group },              // Accent,
	{ NodeType::LeftRight, 30 },      // Asterisk,
	{ NodeType::LeftRight, 30 },      // Divide,
	{ NodeType::LeftRight, 18 },      // Circumflex,
	{ NodeType::LeftRight, 28 },      // Plus,
	{ NodeType::LeftRight, 28 },      // Minus,
	{ NodeType::Right },              // QuestionMark,
	{ NodeType::Left },               // Tilde,
	{ NodeType::LeftRight, 22 },      // DoubleEqual,
	{ NodeType::LeftRight, 22 },      // NotEqual,
	{ NodeType::LeftRight, 22 },      // Equivalent,
	{ NodeType::LeftRight, 22 },      // LesserOrEqual,
	{ NodeType::LeftRight, 22 },      // GreaterOrEqual,
	{ NodeType::LeftRight, 8 },       // InputArrow,
	{ NodeType::LeftRight, 8 },       // OutputArrow,
	{ NodeType::Right },              // Percent,
	{ NodeType::LeftRight, 6 },       // Extract,
	{ NodeType::LeftRight, 6 },       // Insert,
	{ NodeType::LeftRight, 40 },      // DoubleDot,
	{ NodeType::Left },               // TripleDot,
	{ NodeType::Terminal },           // MultiLineString,
	{ NodeType::Right, 38 },          // PlusPlus,
	{ NodeType::Right, 38 },          // MinusMinus,
	{ NodeType::LeftRight, 32 },      // Power,
	{ NodeType::LeftRight },          // PlusEqual,
	{ NodeType::LeftRight },          // MinusEqual,
	{ NodeType::LeftRight },          // TimesEqual,
	{ NodeType::LeftRight },          // DivideEqual,
	{ NodeType::LeftRight },          // IntegerDivideEqual,
	{ NodeType::LeftRight },          // LesserThan,
	{ NodeType::LeftRight },          // GreaterThan,
	{ NodeType::LeftRight },          // SendTo,
	{ NodeType::LeftRight },          // Pipe,
	{ NodeType::Left },               // At,
	{ },  // Semicolon
	{ },  // RightParenthesis
	{ },  // RegexOrGlobContent
	{ },  // RegexOrGlobEnd
	{ },  // RegexOrGlobOption
	{ },  // RightBrace
	{ },  // RightCurlyBrace
	{ },  // UserDefinedSymbol


	{ /** KEYWORDS **/  },
	{NodeType::Left },          // Let
	{NodeType::Terminal },      // Super
	{NodeType::Left },          // Print
	{NodeType::Left },          // Use
	{NodeType::Left },          // Import
	{NodeType::Left },          // Export
	{NodeType::LeftRight },     // From
	{NodeType::LeftRight },     // Extends
	{NodeType::Left },          // If
	{NodeType::Left },          // Else
	{NodeType::Left },          // ElseIf
	{NodeType::Left },          // Then
	{NodeType::Left },          // Do
	{NodeType::Left },          // While
	{NodeType::Left },          // Repeat
	{NodeType::Left },          // For
	{NodeType::LeftRight },     // In
	{NodeType::Left },          // When
	{NodeType::LeftRight },     // And
	{NodeType::LeftRight },     // Or
	{NodeType::LeftRight },     // Xor
	{NodeType::LeftRight },     // Modulo
	{NodeType::LeftRight },     // Is
	{NodeType::LeftRight },     // As
	{NodeType::Left },          // Not
	{NodeType::LeftRight },     // Isnt
	{NodeType::Left },          // Return
	{NodeType::Left },          // Continue
	{NodeType::Left },          // Break
	{NodeType::Left },          // Try
	{NodeType::Left },          // Catch
	{NodeType::Left },          // Finally
	{NodeType::Left },          // Throw
	{NodeType::Left },          // Async
	{NodeType::Left },          // Await
	{NodeType::Left },          // Yield
	{NodeType::Terminal },      // Nil
	{NodeType::Terminal },      // True
	{NodeType::Terminal },      // False
	{NodeType::Terminal },      // Infinity
	{NodeType::Left },          // Global
	{NodeType::Left },          // Override
	{NodeType::Left },          // Final
	{NodeType::Left },          // Const
	{NodeType::Left },          // Private
	{NodeType::Left },          // Static
	{NodeType::Left },          // Class
	{NodeType::Left },          // Enum
	{NodeType::Left },          // Abstract
	{NodeType::Left },          // Interface
	{NodeType::Left },          // Structure
	{NodeType::Left },          // Unique
	{NodeType::Left },          // Exclamation
	{NodeType::Terminal },      // Self
}
