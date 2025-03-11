use crate::parse::parse_expression;

#[cfg(test)]
// Helper function to check if tokenization results match expected tokens
fn assert_expression(input: &'static str, expected: &str) {
	let tree = parse_expression(input);
	println!("{:#?}", &tree);
	assert_eq!(tree.to_string(), expected);
}

#[test]
fn operations() {
	// Basic operations
	assert_expression("a+b", "(a + b)");
	assert_expression("a-b", "(a - b)");
	assert_expression("a*b", "(a * b)");

	// Operations with spaces
	assert_expression("a + b", "(a + b)");
	assert_expression("a - b", "(a - b)");
	assert_expression("a * b", "(a * b)");
	assert_expression("a / b", "(a / b)");
	assert_expression("a ** b", "(a ** b)");

	// Multiple operations of the same type
	assert_expression("a + b + c", "(a + b + c)");
	assert_expression("a - b - c", "(a - b - c)");
	assert_expression("a * b * c", "(a * b * c)");

	// Mixed operations
	assert_expression("a + b - c", "((a + b) - c)");
	assert_expression("a - b + c", "((a - b) + c)");
	assert_expression("a * b + c", "((a * b) + c)");
	assert_expression("a + b * c", "(a + (b * c))");

	// Operations with identifiers and literals
	assert_expression("a + 1", "(a + 1)");
	assert_expression("1 + a", "(1 + a)");
	assert_expression("1 + 2", "(1 + 2)");

	// Operations with boolean literals
	assert_expression("a + true", "(a + true)");
	assert_expression("false - b", "(false - b)");

	// Operations with union operator
	assert_expression("a | b", "(a | b)");
	assert_expression("a | b | c", "(a | b | c)");

	// Operations with pipe operator
	assert_expression("a |> b", "(a |> b)");
	assert_expression("a |> b |> c", "(a |> b |> c)");

	// Operations with modulo operator
	assert_expression("a modulo b", "(a modulo b)");
	assert_expression("a modulo b modulo c", "(a modulo b modulo c)");

	// Operations with insert and extract operators
	assert_expression("a << b", "(a << b)");
	assert_expression("a << b << c", "((a << b) << c)");
	assert_expression("a >> b", "(a >> b)");
	assert_expression("a >> b >> c", "((a >> b) >> c)");
	assert_expression("a >> b << c", "((a >> b) << c)");
	assert_expression("a << b >> c", "((a << b) >> c)");
}

#[test]
fn precedence() {
	assert_expression("  a+b *c", "(a + (b * c))");
	assert_expression("a*b+ c", "((a * b) + c)");
	assert_expression("a + b * c + d", "(a + (b * c) + d)");
	assert_expression("a * b + c * d", "((a * b) + (c * d))");
	assert_expression("a + b + c * d", "(a + b + (c * d))");
	assert_expression("a * b * c + d", "((a * b * c) + d)");
	assert_expression("a + b / c", "(a + (b / c))");
	assert_expression("a / b + c", "((a / b) + c)");
	assert_expression("a * b / c", "((a * b) / c)");
	assert_expression("a / b * c", "((a / b) * c)");
	assert_expression("a ** b * c", "((a ** b) * c)");
	assert_expression("a * b ** c", "(a * (b ** c))");

	// More complex precedence tests
	assert_expression("a + b * c - d", "((a + (b * c)) - d)");
	assert_expression("a * (b + c) * d", "(a * ((b + c)) * d)");

	// Operations with pipe operator
	assert_expression("a |> b * c", "(a |> (b * c))");
	assert_expression("a * b |> c", "((a * b) |> c)");
	assert_expression("a or b |> c", "((a or b) |> c)");
}

#[test]
fn prefix() {
	assert_expression("-a", "(-a)");
	assert_expression("- a", "(-a)");
	assert_expression("-1", "(-1)");
	assert_expression("-true", "(-true)");

	// Prefix with operations
	assert_expression("-a + b", "((-a) + b)");
	assert_expression("a + -b", "(a + (-b))");
	assert_expression("-a * b", "((-a) * b)");
	assert_expression("a * -b", "(a * (-b))");

	// Multiple prefix operators
	assert_expression("--a", "(-(-a))");
	assert_expression("---a", "(-(-(-a)))");

	// Prefix with parentheses
	assert_expression("-(a + b)", "(-((a + b)))");
	assert_expression("-a * -b", "((-a) * (-b))");
}

#[test]
fn groups() {
	assert_expression("( a   )", "(a)");
	assert_expression("(a + b) * c", "(((a + b)) * c)");
	assert_expression("a * (b * c)", "(a * ((b * c)))");
	assert_expression("a * (b + c)", "(a * ((b + c)))");

	// Nested groups
	assert_expression("((a))", "((a))");
	assert_expression("(a + (b + c))", "((a + ((b + c))))");

	// Groups with prefix operators
	assert_expression("-(a + b)", "(-((a + b)))");
	assert_expression("(-a + b)", "(((-a) + b))");
}

// Check that empty groups are forbidden
#[test]
#[should_panic(expected = "Empty groups are not allowed")]
fn empty_group_should_panic() {
	parse_expression("()");
}

#[test]
fn logical_operations() {
	// Logical operations
	assert_expression("a and b", "(a and b)");
	assert_expression("a and b and c", "(a and b and c)");
	assert_expression("a or b", "(a or b)");
	assert_expression("not a", "(not a)");
	assert_expression("a is b", "(a is b)");

	// Logical operations with precedence
	assert_expression("a and b or c", "((a and b) or c)");
	assert_expression("a or b and c", "(a or (b and c))");
	assert_expression("not a and b", "(not (a and b))");
	assert_expression("(not a) and b", "(((not a)) and b)");
}

#[test]
fn comparison_operations() {
	// Comparison operations
	assert_expression("a == b", "(a == b)");
	assert_expression("a != b", "(a != b)");
	assert_expression("a < b", "(a < b)");
	assert_expression("a <= b", "(a <= b)");
	assert_expression("a <= b <= c", "(a <= b <= c)");
	assert_expression("a >= b >= c", "(a >= b >= c)");
	assert_expression("a > b", "(a > b)");
	assert_expression("a >= b", "(a >= b)");

	// Comparison operations with other operations
	assert_expression("a + b == c", "((a + b) == c)");
	assert_expression("a == b == c", "(a == b == c)");
	assert_expression("a == b + c", "(a == (b + c))");
	assert_expression("a < b and c > d", "((a < b) and (c > d))");
}
