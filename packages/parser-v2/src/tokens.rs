#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Token {
	pub kind: TokenKind,
	pub length: u8,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum TokenKind {
	Stop,
	Space,

	/* ------------------------------- Primitives ------------------------------- */
	Integer,
	Number,
	True,
	False,
	String,
	Symbol,
	Null,
	Identifier,

	/* ------------------------------- Keywords ------------------------------- */
	Return,
	If,
	Else,
	For,
	While,
	When,
	Use,
	Async,
	Await,
	Yield,
	Exit,
	Continue,

	/* ------------------------------- Operators ------------------------------- */
	Plus,
	Minus,
	Star,
	DoubleStar,
	Slash,
	DoubleSlash,
	Percent,
	Equal,
	DoubleEqual,
	NotEqual,
	LessThan,
	LessThanOrEqual,
	GreaterThan,
	GreaterThanOrEqual,
	FatArrow,
	Arrow,
	Pipe,
	And,
	Or,
	Not,
	Is,

	/* -------------------------------- Chainable ------------------------------- */
	Dot,
	DoubleDot,
	TripleDot,
	Colon,
	DoubleColon,
	Comma,
	// Semicolon,

	/* --------------------------------- Groups --------------------------------- */
	ParenthesisOpen,
	ParenthesisClose,
	BracesOpen,
	BracesClose,
	BracketsOpen,
	BracketsClose,
}
