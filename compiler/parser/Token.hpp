#pragma once
#include "../common.hpp"
#include "NodeType.hpp"

struct Token {
	enum Type {
		UnknownToken,
		Number,
		Comment,
		SubComment,
		Checkpoint,
		Identifier,
		String,
		RawString,
		StringEnd,

		SYMBOLS,
		LeftParenthesis,
		RegexStart,
		GlobStart,
		LeftBrace,
		Backslash,
		DoubleBackslash,
		Equal,
		Colon,
		LeftCurlyBrace,
		Dot,
		Comma,
		Apostrophe,
		Quote,
		Accent,
		Asterisk,
		Divide,
		Circumflex,
		Plus,
		Minus,
		QuestionMark,
		Tilde,
		DoubleEqual,
		NotEqual,
		Equivalent,
		LesserOrEqual,
		GreaterOrEqual,
		InputArrow,
		OutputArrow,
		Percent,
		Extract,
		Insert,
		DoubleDot,
		TripleDot,
		MultiLineString,
		PlusPlus,
		MinusMinus,
		Power,
		PlusEqual,
		MinusEqual,
		TimesEqual,
		DivideEqual,
		IntegerDivideEqual,
		LesserThan,
		GreaterThan,
		SendTo,
		Pipe,
		At,
		// DotColon,
		// IntegerDivide,
		// Swap,
		// StreamFrom,
		// StreamTo,

		Semicolon,
		RightParenthesis,
		RegexOrGlobContent,
		RegexOrGlobEnd,
		RegexOrGlobOption,
		RightBrace,
		RightCurlyBrace,
		UserDefinedSymbol,

		KEYWORDS,
		Let,
		Super,
		Print,
		Use,
		Import,
		Export,
		From,
		Extends,
		If,
		Else,
		ElseIf,
		Then,
		Do,
		While,
		Repeat,
		For,
		In,
		When,
		And,
		Or,
		Xor,
		Modulo,
		Is,
		As,
		Not,
		Isnt,
		Return,
		Continue,
		Break,
		Try,
		Catch,
		Finally,
		Throw,
		Async,
		Await,
		Yield,
		Nil,
		True,
		False,
		Infinity,
		Global,
		Override,
		Final,
		Const,
		Private,
		Static,
		Class,
		Enum,
		Abstract,
		Interface,
		Structure,
		Unique,
		Exclamation,
		Self,
		// FatArrow,		// =>
		// DoubleColon,	// ::
	};

	Type type { UnknownToken };
	int position { 0 };
	int length { 0 };


	void print() {
		cout
			<< " { "
			<< "type: " << type << ", "
			<< "position: " << position << ", "
			<< "length: " << length
			<< " } "
		;
	}

	inline bool isSymbol() {
		return type > SYMBOLS && type < KEYWORDS;
	}

	inline bool isWord() {
		return type == Identifier;
	}

	inline bool isKeyword() {
		return type > KEYWORDS;
	}

	inline NodeType getNodeType() {
		return nodeInfosFromTokenType[type].type;
	}
};

using TokenList = vector<Token>;
