#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Token {
	pub kind: TokenKind,
	pub start: usize,
	pub end: usize,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum TokenKind {
	None,
	Stop, // end of line
	End, // end of file

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

	/* ------------------------------- Chainable tokens ------------------------------- */
	Plus, // <-- Must be the first chainable token
	Minus,
	Star,
	DoubleStar,
	Slash,
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
	DoubleDot,
	Pipe, // <-- Must be the last chainable token

	/* --------------------------------- Groups --------------------------------- */
	ParenthesisOpen,
	ParenthesisClose,
	ParametersStart, // special case for function parameters
	ParametersEnd,
	BracesOpen,
	BracesClose,
	BracketsOpen,
	BracketsClose,
	QuestionMarkParenthesisOpen,
	QuestionMarkBracketsOpen,

	/* -------------------------------- Comments -------------------------------- */
	DoubleSlash,

	/* --------------------------------- Others --------------------------------- */
	TripleDot,
	Colon,
	Comma,
	ExclamationMark,
	QuestionMark,
}
