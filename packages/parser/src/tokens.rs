#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Token {
	pub kind: TokenKind,
	pub start: usize,
	pub end: usize,
}

pub const FIRST_CLOSING_TOKEN: isize = 32;
pub const FIRST_OPENING_TOKEN: isize = 64;
pub const FIRST_CHAINABLE_TOKEN: isize = 96;

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
	Number,
	True,
	False,
	String,
	Symbol,
	Null,
	Identifier,

	Return,
	Exit,
	Continue,

	MinusWithoutSpaceAfter,
	DoubleDot,
	TripleDot,

	ExclamationMark,
	QuestionMark,

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
	When,
	Use,
	Async,
	Await,
	Yield,
	Not,
	Let,
	Function,
	Mutable,
	Type,
	UnionKeyword,
	Enum,
	Fields,
	Reactive,
	Derived,
	Namespace,

	Colon,
	Comma,

	ParenthesisOpen,
	BracesOpen,
	BracketsOpen,
	QuestionMarkParenthesisOpen,
	QuestionMarkBracketsOpen,

	/* -------------------------------------------------------------------------- */
	/*                              Chainable tokens                              */
	/*                      (Skip newlines before and after)                      */
	/* -------------------------------------------------------------------------- */
	Plus = FIRST_CHAINABLE_TOKEN,
	MinusWithSpaceAfter,
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
	Dot,
	QuestionMarkDot,
	Pipe,
}
