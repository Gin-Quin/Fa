#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct Token {
	pub kind: TokenKind,
	pub start: usize,
	pub end: usize,
}

impl Token {
	pub fn new(kind: TokenKind, start: usize, end: usize) -> Self {
		Token { kind, start, end }
	}
}

pub const FIRST_CLOSING_TOKEN: isize = 48;
pub const FIRST_OPENING_TOKEN: isize = 64;
pub const FIRST_CHAINABLE_TOKEN: isize = 128;

/// The kind of a token. The numeric value of a kind encodes how the new lines around it are
/// handled (see the sections below), so the order of the variants matters.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
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
	// a character or a sequence that is not valid Fa, reported by the tokenizer
	Invalid,

	Return,
	Continue,
	Break,
	// contextual keyword (see `resolve_start_keywords`), so it keeps the following newline
	Start,
	// contextual keywords (see `resolve_contextual_keywords`), so they keep the following newline
	Type,
	Reactive,
	Derived,
	Namespace,

	Minus,
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
	// the `>` that closes a generic list (see `GenericOpen`)
	GenericClose,

	/* -------------------------------------------------------------------------- */
	/*                               Opening tokens                               */
	/*                            (Skip newlines after)                           */
	/* -------------------------------------------------------------------------- */
	If = FIRST_OPENING_TOKEN,
	Else,
	For,
	Parallel,
	While,
	Loop,
	Do,
	Watch,
	When,
	Export,
	Use,
	Reuse,
	Async,
	Await,
	Try,
	Throw,
	Provide,
	Assert,
	Not,
	Let,
	Function,
	Mutable,
	Static,
	Readonly,
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
	// a `<` that is not followed by a space: it opens a generic list, like `Array<Integer>`
	// (a `<` followed by a space, a tab or a new line is the comparison `LessThan`)
	GenericOpen,

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

impl TokenKind {
	/// A closing bracket: `)`, `}`, `]` or the `>` of a generic list.
	pub fn is_closing(self) -> bool {
		(FIRST_CLOSING_TOKEN..FIRST_OPENING_TOKEN).contains(&(self as isize))
	}

	/// A token after which new lines are ignored.
	pub fn skips_newlines_after(self) -> bool {
		(self as isize) >= FIRST_OPENING_TOKEN
	}

	/// A token before which new lines are ignored: the line continues the previous one.
	pub fn skips_newlines_before(self) -> bool {
		let kind = self as isize;
		kind >= FIRST_CHAINABLE_TOKEN || self.is_closing()
	}

	/// A literal or a name, i.e. a token that ends an operand: a `-` after it is a subtraction.
	pub fn ends_operand(self) -> bool {
		matches!(
			self,
			TokenKind::Integer
				| TokenKind::NegativeInteger
				| TokenKind::BinaryInteger
				| TokenKind::NegativeBinaryInteger
				| TokenKind::OctalInteger
				| TokenKind::NegativeOctalInteger
				| TokenKind::HexadecimalInteger
				| TokenKind::NegativeHexadecimalInteger
				| TokenKind::BigInteger
				| TokenKind::NegativeBigInteger
				| TokenKind::Number
				| TokenKind::True
				| TokenKind::False
				| TokenKind::String
				| TokenKind::None
				| TokenKind::Error
				| TokenKind::Identifier
				| TokenKind::Type
				| TokenKind::Reactive
				| TokenKind::Derived
				| TokenKind::Namespace
				| TokenKind::Start
				| TokenKind::QuestionMark
				| TokenKind::Percent
				| TokenKind::ParenthesisClose
				| TokenKind::BracesClose
				| TokenKind::BracketsClose
				| TokenKind::GenericClose
		)
	}

	/// How the token is written, for diagnostics.
	pub fn describe(self) -> &'static str {
		match self {
			TokenKind::Empty => "nothing",
			TokenKind::Stop => "a new line",
			TokenKind::End => "the end of the file",
			TokenKind::InlineComment | TokenKind::BlockComment => "a comment",
			TokenKind::Integer
			| TokenKind::NegativeInteger
			| TokenKind::BinaryInteger
			| TokenKind::NegativeBinaryInteger
			| TokenKind::OctalInteger
			| TokenKind::NegativeOctalInteger
			| TokenKind::HexadecimalInteger
			| TokenKind::NegativeHexadecimalInteger
			| TokenKind::BigInteger
			| TokenKind::NegativeBigInteger
			| TokenKind::Number => "a number",
			TokenKind::True => "`true`",
			TokenKind::False => "`false`",
			TokenKind::String => "a string",
			TokenKind::None => "`none`",
			TokenKind::Error => "`Error`",
			TokenKind::Identifier => "a name",
			TokenKind::Invalid => "an invalid character",
			TokenKind::Return => "`return`",
			TokenKind::Continue => "`continue`",
			TokenKind::Break => "`break`",
			TokenKind::Start => "`start`",
			TokenKind::Type => "`type`",
			TokenKind::Reactive => "`reactive`",
			TokenKind::Derived => "`derived`",
			TokenKind::Namespace => "`namespace`",
			TokenKind::Minus => "`-`",
			TokenKind::DoubleDot => "`..`",
			TokenKind::DoubleDotEqual => "`..=`",
			TokenKind::TripleDot => "`...`",
			TokenKind::QuestionMark => "`?`",
			TokenKind::Percent => "`%`",
			TokenKind::ParenthesisClose => "`)`",
			TokenKind::BracesClose => "`}`",
			TokenKind::BracketsClose => "`]`",
			TokenKind::GenericClose => "`>`",
			TokenKind::If => "`if`",
			TokenKind::Else => "`else`",
			TokenKind::For => "`for`",
			TokenKind::Parallel => "`parallel`",
			TokenKind::While => "`while`",
			TokenKind::Loop => "`loop`",
			TokenKind::Do => "`do`",
			TokenKind::Watch => "`watch`",
			TokenKind::When => "`when`",
			TokenKind::Export => "`export`",
			TokenKind::Use => "`use`",
			TokenKind::Reuse => "`reuse`",
			TokenKind::Async => "`async`",
			TokenKind::Await => "`await`",
			TokenKind::Try => "`try`",
			TokenKind::Throw => "`throw`",
			TokenKind::Provide => "`provide`",
			TokenKind::Assert => "`assert`",
			TokenKind::Not => "`not`",
			TokenKind::Let => "`let`",
			TokenKind::Function => "`function`",
			TokenKind::Mutable => "`mutable`",
			TokenKind::Static => "`static`",
			TokenKind::Readonly => "`readonly`",
			TokenKind::AtFor => "`@for`",
			TokenKind::AtIf => "`@if`",
			TokenKind::AtElse => "`@else`",
			TokenKind::AtTest => "`@test`",
			TokenKind::Colon => "`:`",
			TokenKind::Comma => "`,`",
			TokenKind::ParenthesisOpen | TokenKind::ParenthesisOpenFunctionDeclaration => "`(`",
			TokenKind::BracesOpen => "`{`",
			TokenKind::BracketsOpen => "`[`",
			TokenKind::GenericOpen => "`<`",
			TokenKind::Plus => "`+`",
			TokenKind::Star => "`*`",
			TokenKind::DoubleStar => "`**`",
			TokenKind::Slash => "`/`",
			TokenKind::DoubleSlash => "`//`",
			TokenKind::DoublePercent => "`%%`",
			TokenKind::DoubleQuestionMark => "`??`",
			TokenKind::Equal => "`=`",
			TokenKind::PlusEqual => "`+=`",
			TokenKind::MinusEqual => "`-=`",
			TokenKind::StarEqual => "`*=`",
			TokenKind::DoubleStarEqual => "`**=`",
			TokenKind::SlashEqual => "`/=`",
			TokenKind::DoubleSlashEqual => "`//=`",
			TokenKind::DoublePercentEqual => "`%%=`",
			TokenKind::DotEqual => "`.=`",
			TokenKind::DoubleEqual => "`==`",
			TokenKind::NotEqual => "`!=`",
			TokenKind::LessThan => "`<`",
			TokenKind::LessThanOrEqual => "`<=`",
			TokenKind::GreaterThan => "`>`",
			TokenKind::GreaterThanOrEqual => "`>=`",
			TokenKind::FatArrow => "`=>`",
			TokenKind::Union => "`|`",
			TokenKind::And => "`and`",
			TokenKind::Or => "`or`",
			TokenKind::Is => "`is`",
			TokenKind::In => "`in`",
			TokenKind::As => "`as`",
			TokenKind::Then => "`then`",
			TokenKind::With => "`with`",
			TokenKind::Throws => "`throws`",
			TokenKind::Requires => "`requires`",
			TokenKind::Mutates => "`mutates`",
			TokenKind::Invalidates => "`invalidates`",
			TokenKind::Catch => "`catch`",
			TokenKind::Arrow => "`->`",
			TokenKind::Dot => "`.`",
			TokenKind::QuestionMarkDot => "`?.`",
		}
	}
}
