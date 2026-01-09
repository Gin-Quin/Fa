#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct Token {
	pub kind: TokenKind,
	pub start: usize,
	pub end: usize,
}

pub const FIRST_CLOSING_TOKEN: isize = 32;
pub const FIRST_OPENING_TOKEN: isize = 64;
pub const FIRST_CHAINABLE_TOKEN: isize = 97;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum TokenKind {
	None = 0,
	Stop,
	End,

	InlineComment,
	BlockComment,

	/* -------------------------------------------------------------------------- */
	/*                              Primitive tokens                              */
	/*                           (Don't skip newlines)                            */
	/* -------------------------------------------------------------------------- */
	Integer,
	NegativeInteger,
	BinaryInteger,
	NegativeBinaryInteger,
	OctalInteger,
	NegativeOctalInteger,
	HexadecimalInteger,
	NegativeHexadecimalInteger,
	BigInteger,
	NegativeBigInteger,
	Number,
	True,
	False,
	String,
	Null,
	Identifier,

	Return,
	Continue,
	Break,

	MinusWithoutSpaceAfter,
	DoubleDot,
	TripleDot,

	ExclamationMark,
	QuestionMark,
	Percent,

	/* -------------------------------------------------------------------------- */
	/*                               Closing tokens                               */
	/*                           (Skip newlines before)                           */
	/* -------------------------------------------------------------------------- */
	ParenthesisClose = FIRST_CLOSING_TOKEN,
	BracesClose,
	BracketsClose,

	/* -------------------------------------------------------------------------- */
	/*                               Opening tokens                               */
	/*                            (Skip newlines after)                           */
	/* -------------------------------------------------------------------------- */
	If = FIRST_OPENING_TOKEN,
	Else,
	For,
	While,
	Loop,
	Do,
	When,
	Export,
	Use,
	Async,
	Await,
	Yield,
	Not,
	Let,
	Function,
	Mutable,
	Static,
	Type,
	UnionKeyword,
	Enum,
	Fields,
	Reactive,
	Derived,
	Namespace,
	AtFor,

	Colon,
	Comma,

	ParenthesisOpen,
	ParenthesisOpenFunctionDeclaration,
	BracesOpen,
	BracketsOpen,
	AtBracketsOpen,
	AtBracesOpen,

	/* -------------------------------------------------------------------------- */
	/*                              Chainable tokens                              */
	/*                      (Skip newlines before and after)                      */
	/* -------------------------------------------------------------------------- */
	Plus = FIRST_CHAINABLE_TOKEN,
	Star,
	DoubleStar,
	Slash,
	DoubleSlash,
	Modulo,
	Equal,
	DoubleEqual,
	NotEqual,
	LessThan,
	LessThanOrEqual,
	GreaterThan,
	GreaterThanOrEqual,
	FatArrow,
	Union,
	Intersection,
	And,
	Or,
	Is,
	Extract,
	Insert,
	Arrow,
	Dot,
	QuestionMarkDot,
	Pipe,
	Compose,
}
