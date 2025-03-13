#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
	None = 0,
	Assignment,
	TypeAssignment,
	Pipe,
	Comma,
	Not,
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
