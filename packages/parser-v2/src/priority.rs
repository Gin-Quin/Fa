#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
	None = 0,
	Pipe,
	Comma,
	Not,
	Assignment,
	Union,
	Or,
	And,
	Equality,
	Comparison,
	Additive,
	Multiplicative,
	Exponentiation,
	InsertExtract,
	Transfer, // '<<' and '>>'
	Prefix,
	Postfix,
	Access,
}
