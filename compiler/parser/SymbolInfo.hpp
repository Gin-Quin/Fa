#pragma once
#include "NodeType.hpp"

struct SymbolInfo {
	NodeType type;
	int weight { 0 };  // 0 : not concerned
};



constexpr SymbolInfo symbolInfos[] = {
	{ NodeType::Group },      // LeftParenthesis
	{ NodeType::Group },      // RegexStart
	{ NodeType::Group },      // GlobStart
	{ NodeType::Group },      // LeftBrace
	{ NodeType::Operation, 30 }, // Backslash,
	{ NodeType::Operation, 30 }, // DoubleBackslash,
	{ NodeType::Operation, 6 }, // Equal,
	{ NodeType::SingleOperation, 5 }, // Colon,
	{ NodeType::Group }, // LeftCurlyBrace,
	{ NodeType::Operation, 40 }, // Dot,
	{ NodeType::Operation, 7 }, // Comma,
	{ NodeType::Group }, // Apostrophe,
	{ NodeType::Group }, // Quote,
	{ NodeType::Group }, // Accent,
	{ NodeType::Operation, 30 }, // Asterisk,
	{ NodeType::Operation, 30 }, // Divide,
	{ NodeType::Operation, 18 }, // Circumflex,
	{ NodeType::Operation, 28 }, // Plus,
	{ NodeType::Operation, 28 }, // Minus,
	{ NodeType::OperationUnit }, // QuestionMark,
	{ NodeType::Command }, // Tilde,
	{ NodeType::Operation, 22 }, // DoubleEqual,
	{ NodeType::Operation, 22 }, // NotEqual,
	{ NodeType::Operation, 22 }, // Equivalent,
	{ NodeType::Operation, 22 }, // LesserOrEqual,
	{ NodeType::Operation, 22 }, // GreaterOrEqual,
	{ NodeType::Operation, 8 }, // InputArrow,
	{ NodeType::Operation, 8 }, // OutputArrow,
	{ NodeType::OperationUnit }, // Percent,
	{ NodeType::Operation, 6 }, // Extract,
	{ NodeType::Operation, 6 }, // Insert,
	{ NodeType::Operation, 40 }, // DoubleDot,
	{ NodeType::Command }, // TripleDot,
	{ NodeType::Terminal }, // MultiLineString,
	{ NodeType::OperationUnit, 38 }, // PlusPlus,
	{ NodeType::OperationUnit, 38 }, // MinusMinus,
	{ NodeType::Operation, 32 }, // Power,
	{ NodeType::Operation }, // PlusEqual,
	{ NodeType::Operation }, // MinusEqual,
	{ NodeType::Operation }, // TimesEqual,
	{ NodeType::Operation }, // DivideEqual,
	{ NodeType::Operation }, // IntegerDivideEqual,
	{ NodeType::Operation }, // LesserThan,
	{ NodeType::Operation }, // GreaterThan,
	{ NodeType::Operation }, // SendTo,
	{ NodeType::Operation }, // Pipe,
	{ NodeType::Command }, // At,
}
