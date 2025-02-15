use crate::recorder::Recorder;

pub enum Token {
	Stop,

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
	Slash,
	Percent,
	Equal,
	DoubleEqual,
	NotEqual,
	LessThan,
	LessThanOrEqual,
	GreaterThan,
	GreaterThanOrEqual,
	And,
	Or,
	Not,
	Pipe,
	FatArrow,
	Arrow,
	Is,
	IsNot,

	/* -------------------------------- Chainable ------------------------------- */
	Dot,
	Colon,
	DoubleColon,
	Comma,
	// Semicolon,

	/* --------------------------------- Groups --------------------------------- */
	Parenthesis(Recorder<Token>),
	Braces(Recorder<Token>),
	Brackets(Recorder<Token>),
}
