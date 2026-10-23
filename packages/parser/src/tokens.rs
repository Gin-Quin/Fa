#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct Token {
	pub kind: TokenKind,
	pub start: usize,
	pub end: usize,
}

pub const FIRST_CLOSING_TOKEN: isize = 32;
pub const FIRST_OPENING_TOKEN: isize = 64;
pub const FIRST_CHAINABLE_TOKEN: isize = 128;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum TokenKind {
	Empty = 0,
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
	None,
	Error,
	Identifier,

	Return,
	Continue,
	Break,
	// contextual keyword (see `resolve_start_keywords`), so it keeps the following newline
	Start,

	MinusWithoutSpaceAfter,
	DoubleDot,
	DoubleDotEqual,
	TripleDot,

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
	Watch,
	When,
	Export,
	Use,
	Async,
	Await,
	Try,
	Throw,
	Provide,
	Assert,
	Yield,
	Not,
	Let,
	Function,
	Mutable,
	Static,
	Type,
	Readonly,
	Reactive,
	Derived,
	Namespace,
	AtFor,
	AtIf,
	AtElse,
	AtTest,

	Colon,
	Comma,

	ParenthesisOpen,
	ParenthesisOpenFunctionDeclaration,
	BracesOpen,
	BracketsOpen,

	/* -------------------------------------------------------------------------- */
	/*                              Chainable tokens                              */
	/*                      (Skip newlines before and after)                      */
	/* -------------------------------------------------------------------------- */
	Plus = FIRST_CHAINABLE_TOKEN,
	Star,
	DoubleStar,
	Slash,
	DoubleSlash,
	DoublePercent,
	DoubleQuestionMark,
	Equal,
	PlusEqual,
	MinusEqual,
	StarEqual,
	DoubleStarEqual,
	SlashEqual,
	DoubleSlashEqual,
	DoublePercentEqual,
	DotEqual,
	DoubleEqual,
	NotEqual,
	LessThan,
	LessThanOrEqual,
	GreaterThan,
	GreaterThanOrEqual,
	FatArrow,
	Union,
	And,
	Or,
	Is,
	In,
	As,
	Then,
	With,
	Throws,
	Requires,
	Mutates,
	Invalidates,
	Catch,
	Arrow,
	Dot,
	QuestionMarkDot,
}
