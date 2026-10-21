#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
	None = 0,
	PrefixKeyword,
	Comma,
	Assignment,
	Catch,
	TypeAssignment,
	Compose, // 'then'
	Not,
	Union,
	Or,
	And,
	Equality,
	Comparison,
	Range, // '..' and '..='
	Additive,
	Multiplicative,
	Exponentiation,
	Relation, // '->'
	Prefix,
	Postfix,
	Access,
}
