#pragma once
#include "../common.hpp"
#include "Glue.hpp"
#include "TokenInfos.hpp"

/**
 * List of all tokens
 */
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
		LeftBraceNoLeft,
		Backslash,
		DoubleBackslash,
		Equal,
		Colon,
		ColonBody,
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
		PlusRight,
		Minus,
		MinusRight,
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
		DoubleDotBody,
		TripleDot,
		TripleDotBody,
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
		IfComprehension,
		If,
		ElseComprehension,
		Else,
		ElseIf,
		Then,
		Do,
		WhileComprehension,
		While,
		RepeatComprehension,
		Repeat,
		ForComprehension,
		For,
		In,
		When,
		And,
		Or,
		Xor,
		Modulo,
		Is,
		IsStart,
		To,
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

		END,
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

	inline int glue() {
		return tokenInfos[type].glue;
	}

	inline int priority() {
		return tokenInfos[type].priority;
	}

	void incrementType() {
		type = static_cast<Type>(static_cast<int>(type) + 1);
	}

	/**
	 * {"type": xx, "position": xxxx, "length": xx}
	 */
	inline string toJson() {
		string json;
		json += '{';
			json += "\"type\":";
			json += to_string(type);

			json += ",\"position\":";
			json += to_string(position);

			json += ",\"length\":";
			json += to_string(length);
		json += '}';
		return json;
	}
};

