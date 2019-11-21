#pragma once


struct Token {
	enum Type {
		UnknownToken,
		Number,
		Comment,
		Identifier,
		BlockStart,
		BlockEnd,
		StringStart,
		StringEnd,
		RawString,

		SYMBOLS,
		LeftParenthesis,
		RightParenthesis,
		NewLine,
		LeftBrace,
		RightBrace,
		Backslash,
		Equal,
		Colon,
		LeftCurlyBrace,
		RightCurlyBrace,
		Dot,
		Comma,
		Apostrophe,
		Quote,
		Accent,
		Asterisk,
		Divide,
		UserDefinedSymbol,
		Semicolon,
		Swap,
		LesserThan,
		Circumflex,
		Plus,
		Minus,
		QuestionMark,
		Tilde,
		DoubleEqual,
		IsNotEqual,
		Equivalent,
		LesserOrEqual,
		GreaterOrEqual,
		Arrow,
		Extract,
		Insert,
		StreamFrom,
		StreamTo,
		DoubleDot,
		TripleDot,
		PlusPlus,
		MinusMinus,
		Power,
		IntegerDivide,
		PlusEqual,
		MinusEqual,
		TimesEqual,
		DivideEqual,
		IntegerDivideEqual,
		DotColon,
		GreaterThan,

		KEYWORDS,
		Super,
		Use,
		Import,
		Export,
		From,
		Extends,
		If,
		Else,
		ElseIf,
		Then,
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
		At,
		Exclamation,
		Self,
		// FatArrow,		// =>
		// DoubleColon,	// ::
	};

	Type type;
	int position;
	int length;

	Token(Type _type=UnknownToken, int _position=0, int _length=0) {
		type = _type;
		position = _position;
		length = _length;
	}

	void print() {
		cout << " { "
			<< "type: " << type << ", "
			<< "position: " << position << ", "
			<< "length: " << length
		<< " } ";
	}

	bool isSymbol() {
		return type > SYMBOLS && type < KEYWORDS;
	}

	bool isWord() {
		return type == Identifier;
	}

	bool isKeyword() {
		return type == Identifier;
	}
};

