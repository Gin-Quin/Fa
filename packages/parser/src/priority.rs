/// The binding power of the operators, from the loosest to the tightest. Parsing the right side
/// of an expression with a priority stops at the first operator whose priority is lower or equal.
/// See the order of operations in `documentation/operations/arithmetic.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
	None = 0,
	// the value of a declaration, and the right side of `return`, `throw`, `break`
	PrefixKeyword,
	Comma,
	// `=`, `+=` and the other compound assignments, `.=`
	Assignment,
	// `catch`
	Catch,
	// `:`, and the clauses of a function signature (`throws`, `requires`, `mutates`...)
	TypeAssignment,
	// the right side of `try`, `await` and `start`, which stops at `catch`
	TryAwaitStart,
	// `then`
	Compose,
	// `??`
	Fallback,
	Or,
	And,
	// the right side of `not`
	Not,
	// `==`, `!=`, `is`, `is not`, `is in`, `is not in`
	Equality,
	// `|`, so that `value is "y" | "yes"` is `value is ("y" | "yes")`
	Union,
	// `<`, `<=`, `>`, `>=`, `in`
	Comparison,
	// `..`, `..=`
	Range,
	Additive,
	Multiplicative,
	// `**`
	Exponentiation,
	// `->`
	Relation,
	// the right side of the prefix `-`, `...` and `in`
	Prefix,
	// `%` and the type suffix `?`
	Postfix,
	// `a.b`, `a?.b`, `a(b)`, `a[b]`, `a with (b = 1)`
	Access,
}
