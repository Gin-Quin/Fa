use crate::parsing::parse_single_statement::parse_single_statement;

#[cfg(test)]
// Helper function to check if tokenization results match expected tokens
fn assert_expression(input: &'static str, expected: &str) {
	let tree = parse_single_statement(input);
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
	assert_expression("a & b", "(a & b)");
	assert_expression("a & b & c", "(a & b & c)");

	// Operations with pipe operator
	assert_expression("a |> b", "(a |> b)");
	assert_expression("a |> b |> c", "(a |> b |> c)");

	// Operations with composition operator
	assert_expression("a ||> b", "(a ||> b)");
	assert_expression("a ||> b ||> c", "(a ||> b ||> c)");

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

	// Operations with relation operator
	assert_expression("a -> b", "(a -> b)");
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

	// Operations with composition operator
	assert_expression("a ||> b * c", "(a ||> (b * c))");
	assert_expression("a * b ||> c", "((a * b) ||> c)");
	assert_expression("a or b ||> c", "((a or b) ||> c)");
}

#[test]
fn prefix() {
	assert_expression("-a", "(-a)");
	assert_expression("-1", "-1");
	assert_expression("-true", "(-true)");
	assert_expression("...a", "(...a)");

	// Prefix with operations
	assert_expression("-a + b", "((-a) + b)");
	assert_expression("a + -b", "(a + (-b))");
	assert_expression("-a * b", "((-a) * b)");
	assert_expression("a * -b", "(a * (-b))");

	// Multiple prefix operators
	assert_expression("-(-a)", "(-((-a)))");

	// Prefix with parentheses
	assert_expression("-(a + b)", "(-((a + b)))");
	assert_expression("-a * -b", "((-a) * (-b))");
}

#[test]
fn list_literals() {
	assert_expression("[]", "[]");
	assert_expression("[a, b, c]", "[a, b, c]");
	assert_expression("@[1, 2, 3]", "@[1, 2, 3]");
	assert_expression("[[1, 2], [3, 4]]", "[[1, 2], [3, 4]]");
	assert_expression("[{ a = 1 }, { b = 2 }]", "[{\n\ta = 1\n}, {\n\tb = 2\n}]");
	assert_expression("{ a = [1, 2], b = @[3] }", "{\n\ta = [1, 2]\n\tb = @[3]\n}");
	assert_expression(
		"@{ a = [1, 2], b = @{ c = 3 } }",
		"@{\n\ta = [1, 2]\n\tb = @{\n\t\tc = 3\n\t}\n}",
	);
}

#[test]
fn control_flow_keywords() {
	assert_expression("continue", "continue");
	assert_expression("break", "break");
	assert_expression("break value", "break value");
	assert_expression("mutable value = other", "mutable value = other");
	assert_expression("static value", "static value");
}

#[test]
fn literal_null() {
	assert_expression("null", "null");
}

#[test]
fn for_loop_expression() {
	assert_expression("for values { continue }", "for values {\n\tcontinue\n}");
	assert_expression(
		"for values >> count { break count }",
		"for (values >> count) {\n\tbreak count\n}",
	);
	assert_expression("@for values { continue }", "@for values {\n\tcontinue\n}");
}

#[test]
fn while_and_loop() {
	assert_expression(
		"while condition { break value }",
		"while condition {\n\tbreak value\n}",
	);
	assert_expression("loop { continue }", "loop {\n\tcontinue\n}");
}

#[test]
fn do_expression() {
	assert_expression("do { continue }", "do {\n\tcontinue\n}");
	assert_expression("do { value } + other", "(do {\n\tvalue\n} + other)");
}

#[test]
fn if_else_chains() {
	assert_expression("if condition { continue }", "if condition {\n\tcontinue\n}");
	assert_expression(
		"if condition { continue } else { break }",
		"if condition {\n\tcontinue\n} else {\n\tbreak\n}",
	);
	assert_expression(
		"if a { continue } else if b { break } else { return }",
		"if a {\n\tcontinue\n} else if b {\n\tbreak\n} else {\n\treturn\n}",
	);
}

#[test]
fn if_expression_usage() {
	assert_expression(
		"let result = if condition { value } else { other }",
		"let result = if condition {\n\tvalue\n} else {\n\tother\n}",
	);
	assert_expression(
		"if condition { value } + other",
		"(if condition {\n\tvalue\n} + other)",
	);
}

#[test]
fn when_expression() {
	assert_expression(
		"when foo is {\n1 => bar\n2 => { baz }\nelse => { other }\n}",
		"when foo is {\n\t1 => bar\n\t2 => {\n\t\tbaz\n\t}\n\telse => {\n\t\tother\n\t}\n}",
	);
	assert_expression(
		"when value is {\nNumber => one\nString => two\n}",
		"when value is {\n\tNumber => one\n\tString => two\n}",
	);
	assert_expression(
		"when foo is {\n1 => bar\n2 => baz\n3 => qux\n}",
		"when foo is {\n\t1 => bar\n\t2 => baz\n\t3 => qux\n}",
	);
	assert_expression(
		"when foo is {\n1 => { bar }\n2 => { baz }\nelse => other\n}",
		"when foo is {\n\t1 => {\n\t\tbar\n\t}\n\t2 => {\n\t\tbaz\n\t}\n\telse => other\n}",
	);
	assert_expression(
		"when foo is {\nMyUnion.SomeNumber >> value => value\nelse => { other }\n}",
		"when foo is {\n\t(MyUnion.SomeNumber >> value) => value\n\telse => {\n\t\tother\n\t}\n}",
	);
}

#[test]
fn member_access() {
	assert_expression("foo.bar", "foo.bar");
	assert_expression("foo.bar.baz", "foo.bar.baz");
}

#[test]
fn percentage_literals() {
	assert_expression("50%", "50%");
	assert_expression("-12%", "-12%");
	assert_expression("50.5%", "50.5%");
	assert_expression("123n%", "123n%");
}

#[test]
fn suffix_operators() {
	assert_expression("value?", "value?");
	assert_expression("value!", "value!");
	assert_expression("value? + other", "(value? + other)");
	assert_expression("value! + other", "(value! + other)");
}

#[test]
fn optional_access_and_suffix() {
	assert_expression("a?.b.c!.d", "a?.b.c!.d");
}

#[test]
fn optional_call_and_index() {
	assert_expression("foo?(bar)", "foo?(bar)");
	assert_expression("foo?[bar]", "foo?[bar]");
}

#[test]
fn strings() {
	assert_expression("\"hello\"", "\"hello\"");
	assert_expression("\"Hello {name}\"", "\"Hello {name}\"");
	assert_expression("\"Hello {{name}}\"", "\"Hello {{name}}\"");
	assert_expression(
		"\"Hello {name} and {count}\"",
		"\"Hello {name} and {count}\"",
	);
	assert_expression("\"line\\nfeed\"", "\"line\\nfeed\"");
	assert_expression("\"tab\\tseparated\"", "\"tab\\tseparated\"");
	assert_expression("\"quote: \\\"\"", "\"quote: \\\"\"");
	assert_expression("\"braces: \\{name\\}\"", "\"braces: {{name}}\"");
	assert_expression("\"Value {a + b * c}\"", "\"Value {(a + (b * c))}\"");
	assert_expression("\"Call {sum(a, b + c)}\"", "\"Call {sum(a, (b + c))}\"");
	assert_expression("\"Access {a?.b.c!.d}\"", "\"Access {a?.b.c!.d}\"");
	assert_expression("\"Relation {a -> b}\"", "\"Relation {(a -> b)}\"");
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
fn empty_group() {
	assert_expression("()", "()");
	assert_expression("(  )", "()");
	assert_expression("( \n )", "()");
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
