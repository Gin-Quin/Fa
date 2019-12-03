#pragma once


struct Token {
	enum Type {
		UnknownToken,
		Number,
		Comment,
		SubComment,
		Checkpoint,
		Identifier,
		StringStart,
		StringEnd,
		RawString,
		NewLine,

		SYMBOLS,
		LeftParenthesis,
		RightParenthesis,
		RegexStart,
		GlobStart,
		RegexOrGlobContent,
		RegexOrGlobEnd,
		RegexOrGlobOption,
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
		NotEqual,
		Equivalent,
		LesserOrEqual,
		GreaterOrEqual,
		InputArrow,
		OutputArrow,
		Extract,
		Insert,
		StreamFrom,
		StreamTo,
		DoubleDot,
		TripleDot,
		MultiLineString,
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
		SendTo,
		Pipe,

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
		return type > KEYWORDS;
	}
};


using TokenList = vector<Token>;
