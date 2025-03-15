#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Token {
	pub kind: TokenKind,
	pub start: usize,
	pub end: usize,
}

pub const FIRST_CLOSING_TOKEN: isize = 8;
pub const FIRST_OPENING_TOKEN: isize = 32;
pub const FIRST_CHAINABLE_TOKEN: isize = 64;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum TokenKind {
	None = 0,
	Stop,
	End,

	InlineComment,
	BlockComment,

	/* -------------------------------------------------------------------------- */
	/*                               Closing tokens                               */
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

	Exit,
	Continue,

	ParenthesisClose,
	ParametersEnd,
	BracesClose,
	BracketsClose,

	MinusWithoutSpaceAfter,
	DoubleDot,
	TripleDot,

	ExclamationMark,
	QuestionMark,

	/* -------------------------------------------------------------------------- */
	/*                               Opening tokens                               */
	/*                            (Skip newlines after)                           */
	/* -------------------------------------------------------------------------- */

	Return = FIRST_OPENING_TOKEN,
	If,
	Else,
	For,
	While,
	When,
	Use,
	Async,
	Await,
	Yield,

	Colon,
	Comma,

	ParenthesisOpen,
	ParametersStart, // special case for function parameters
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
	Not,
	Is,
	Extract,
	Insert,
	Dot,
	QuestionMarkDot,
	Pipe,
}
