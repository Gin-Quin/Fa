#pragma once


struct Token {
	enum Type {
		UnknownToken,			/*--  0  --*/
		Number,					/*--  1  --*/
		Comment,					/*--  2  --*/
		MultiLineComment,		/*--  3  --*/
		Identifier,				/*--  4  --*/
		
		SYMBOLS,					/*--  5  --*/
		Block,					/*--  6  --*/
		Parenthesis,			/*--  7  --*/
		NewLineIndent,			/*--  8  --*/
		Brace,					/*--  9  --*/
		Backslash,				/*--  10  --*/
		Equal,					/*--  11  --*/
		Colon,					/*--  12  --*/
		CurlyBrace,				/*--  13  --*/
		Dot,						/*--  14  --*/
		Comma,					/*--  15  --*/
		Apostrophe,				/*--  16  --*/
		Quote,					/*--  17  --*/
		Accent,					/*--  18  --*/
		Asterisk,				/*--  19  --*/
		Divide,					/*--  20  --*/
		UserDefinedSymbol,	/*--  21  --*/
		Semicolon,				/*--  22  --*/
		Swap,						/*--  23  --*/
		LesserThan,				/*--  24  --*/
		Circumflex,				/*--  25  --*/
		Plus,						/*--  26  --*/
		Minus,					/*--  27  --*/
		QuestionMark,			/*--  28  --*/
		Tilde,					/*--  29  --*/
		DoubleEqual,			/*--  30  --*/
		IsNotEqual,				/*--  31  --*/
		Equivalent,				/*--  32  --*/
		LesserOrEqual,			/*--  33  --*/
		GreaterOrEqual,		/*--  34  --*/
		Arrow,					/*--  35  --*/
		Extract,					/*--  36  --*/
		Insert,					/*--  37  --*/
		StreamFrom,				/*--  38  --*/
		StreamTo,				/*--  39  --*/
		DoubleDot,				/*--  40  --*/
		TripleDot,				/*--  41  --*/
		PlusPlus,				/*--  42  --*/
		MinusMinus,				/*--  43  --*/
		Power,					/*--  44  --*/
		IntegerDivide,			/*--  45  --*/
		PlusEqual,				/*--  46  --*/
		MinusEqual,				/*--  47  --*/
		TimesEqual,				/*--  48  --*/
		DivideEqual,			/*--  49  --*/
		IntegerDivideEqual,	/*--  50  --*/
		DotColon,				/*--  51  --*/
		GreaterThan,			/*--  52  --*/

		KEYWORDS,				/*--  53  --*/
		Super,					/*--  54  --*/
		Use,						/*--  55  --*/
		Import,					/*--  56  --*/
		Export,					/*--  57  --*/
		From,						/*--  58  --*/
		Extends,					/*--  59  --*/
		If,						/*--  60  --*/
		Else,						/*--  61  --*/
		ElseIf,					/*--  62  --*/
		Then,						/*--  63  --*/
		While,					/*--  64  --*/
		Repeat,					/*--  65  --*/
		For,						/*--  66  --*/
		In,						/*--  67  --*/
		When,						/*--  68  --*/
		And,						/*--  69  --*/
		Or,						/*--  70  --*/
		Xor,						/*--  71  --*/
		Modulo,					/*--  72  --*/
		Is,						/*--  73  --*/
		As,						/*--  74  --*/
		Not,						/*--  75  --*/
		Isnt,						/*--  76  --*/
		Return,					/*--  77  --*/
		Continue,				/*--  78  --*/
		Break,					/*--  79  --*/
		Try,						/*--  80  --*/
		Catch,					/*--  81  --*/
		Finally,					/*--  82  --*/
		Throw,					/*--  83  --*/
		Async,					/*--  84  --*/
		Await,					/*--  85  --*/
		Yield,					/*--  86  --*/
		Nil,						/*--  87  --*/
		True,						/*--  88  --*/
		False,					/*--  89  --*/
		Infinity,				/*--  90  --*/
		Global,					/*--  91  --*/
		Override,				/*--  92  --*/
		Final,					/*--  93  --*/
		Const,					/*--  94  --*/
		Private,					/*--  95  --*/
		Static,					/*--  96  --*/
		Class,					/*--  97  --*/
		Enum,						/*--  98  --*/
		Abstract,				/*--  99  --*/
		Interface,				/*--  100  --*/
		Structure,				/*--  101  --*/
		Unique,					/*--  102  --*/
		At,						/*--  103  --*/
		Exclamation,			/*--  104  --*/
		Self,						/*--  105  --*/
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

