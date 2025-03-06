#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Token {
	pub kind: TokenKind,
	pub length: u8,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum TokenKind {
	None,
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
	Union,
	And,
	Or,
	Not,
	Is,
	Pipe,

	/* -------------------------------- Chainable ------------------------------- */
	Dot,
	DoubleDot,
	TripleDot,
	Colon,
	DoubleColon,
	Comma,
	QuestionDot,
	// Semicolon,

	/* --------------------------------- Groups --------------------------------- */
	ParenthesisOpen,
	ParenthesisClose,
	ParametersStart, // special case for function parameters
	ParametersEnd,
	QuestionParenthesisOpen,
	BracesOpen,
	BracesClose,
	BracketsOpen,
	BracketsClose,
	QuestionBracketsOpen,

	/* --------------------------------- Postfix -------------------------------- */
	ExclamationMark,
	QuestionMark,
}
