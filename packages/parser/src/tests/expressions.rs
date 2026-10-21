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

	// Operations with composition keyword
	assert_expression("a then b", "(a then b)");
	assert_expression("a then b then c", "(a then b then c)");

	// Operations with modulo operator
	assert_expression("a modulo b", "(a modulo b)");
	assert_expression("a modulo b modulo c", "(a modulo b modulo c)");

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

	// Operations with composition keyword
	assert_expression("a then b * c", "(a then (b * c))");
	assert_expression("a * b then c", "((a * b) then c)");
	assert_expression("a or b then c", "((a or b) then c)");
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
	assert_expression("[[1, 2], [3, 4]]", "[[1, 2], [3, 4]]");
	assert_expression("[{ a = 1 }, { b = 2 }]", "[{\n\ta = 1\n}, {\n\tb = 2\n}]");
	assert_expression("{ a = [1, 2], b = [3] }", "{\n\ta = [1, 2]\n\tb = [3]\n}");
}

#[test]
fn compile_time_values() {
	assert_expression("let @foo = 12", "let @foo = 12");
	assert_expression("let z = @add(2, 4)", "let z = @add(2, 4)");
	assert_expression("console.log(@type(x))", "console.log(@type(x))");
	assert_expression(
		"use weather = @import(\"https://api.weather.com/fa/types.fa\")",
		"use weather = @import(\"https://api.weather.com/fa/types.fa\")",
	);
	assert_expression(
		"@for fruit in @fruits { console.log(fruit) }",
		"@for fruit in @fruits {\n\tconsole.log(fruit)\n}",
	);
}

#[test]
fn private_names() {
	assert_expression("self.#bag.delete(node)", "self.#bag.delete(node)");
	assert_expression("Self in #bag", "(Self in #bag)");
}

#[test]
fn control_flow_keywords() {
	assert_expression("continue", "continue");
	assert_expression("break", "break");
	assert_expression("break value", "break value");
	assert_expression("mutable value = other", "mutable value = other");
	assert_expression("use value = other", "use value = other");
	assert_expression("use value: Number = other", "use value: Number = other");
	assert_expression("static value", "static value");
}

#[test]
fn literal_none() {
	assert_expression("none", "none");
}

#[test]
fn for_loop_expression() {
	assert_expression("for values { continue }", "for values {\n\tcontinue\n}");
	assert_expression("@for values { continue }", "@for values {\n\tcontinue\n}");
	assert_expression(
		"for value in values { continue }",
		"for value in values {\n\tcontinue\n}",
	);
	assert_expression(
		"for item, index in items { continue }",
		"for item, index in items {\n\tcontinue\n}",
	);
	assert_expression(
		"for { name, age as years } in columnar { continue }",
		"for { name, age as years } in columnar {\n\tcontinue\n}",
	);
	assert_expression(
		"for (id, name) in usersById { continue }",
		"for (id, name) in usersById {\n\tcontinue\n}",
	);
	assert_expression(
		"for i in 1..=10 { continue }",
		"for i in (1..=10) {\n\tcontinue\n}",
	);
	assert_expression("for 1..10 { continue }", "for (1..10) {\n\tcontinue\n}");
	assert_expression(
		"@for fruit in fruits { continue }",
		"@for fruit in fruits {\n\tcontinue\n}",
	);
	assert_expression(
		"for item in getItems(filter = { active = true }) { continue }",
		"for item in getItems(filter = {\n\tactive = true\n}) {\n\tcontinue\n}",
	);
}

#[test]
fn parallel_for_loop() {
	assert_expression(
		"for parallel number in numbers { continue }",
		"for parallel number in numbers {\n\tcontinue\n}",
	);
	assert_expression(
		"for parallel(threads = 2) number in numbers { continue }",
		"for parallel(threads = 2) number in numbers {\n\tcontinue\n}",
	);
	// `parallel` can still be used as a name
	assert_expression(
		"for parallel in parallels { continue }",
		"for parallel in parallels {\n\tcontinue\n}",
	);
}

#[test]
#[should_panic]
fn for_loop_index_must_be_a_name() {
	parse_single_statement("for item, { index } in items { continue }");
}

#[test]
fn while_and_loop() {
	assert_expression(
		"while condition { break value }",
		"while condition {\n\tbreak value\n}",
	);
	assert_expression("loop { continue }", "loop {\n\tcontinue\n}");
	assert_expression(
		"while let value = stack.pop() { continue }",
		"while let value = stack.pop() {\n\tcontinue\n}",
	);
	assert_expression(
		"while let { name } = queue.next() { continue }",
		"while let { name } = queue.next() {\n\tcontinue\n}",
	);
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
}

#[test]
fn when_branch_bindings() {
	assert_expression(
		"when shape is {\nShape.Point => 0\nShape.Circle as circle => circle.radius\n}",
		"when shape is {\n\tShape.Point => 0\n\tShape.Circle as circle => circle.radius\n}",
	);
	assert_expression(
		"when shape is {\nShape.Rectangle as { width, height } => width * height\n}",
		"when shape is {\n\tShape.Rectangle as { width, height } => (width * height)\n}",
	);
	assert_expression(
		"when shape is {\nShape.Segment as (start, end) => start\nelse => none\n}",
		"when shape is {\n\tShape.Segment as (start, end) => start\n\telse => none\n}",
	);
}

#[test]
#[should_panic]
fn when_else_branch_cannot_be_bound() {
	parse_single_statement("when shape is {\nelse as other => other\n}");
}

#[test]
fn member_access() {
	assert_expression("foo.bar", "foo.bar");
	assert_expression("foo.bar.baz", "foo.bar.baz");
}

#[test]
fn member_index() {
	assert_expression("foo { bar }", "foo {\n\tbar\n}");
	assert_expression("foo { a = 1, b }", "foo {\n\ta = 1\n\tb\n}");
}

#[test]
fn percentage_literals() {
	assert_expression("50%", "50%");
	assert_expression("-12%", "-12%");
	assert_expression("50.5%", "50.5%");
	assert_expression("123n%", "123n%");
}

#[test]
fn optional_suffix() {
	assert_expression("value?", "value?");
	assert_expression("Array(Integer)?", "Array(Integer)?");
}

#[test]
fn optional_access() {
	assert_expression("a?.b.c", "a?.b.c");
	assert_expression("a.b?.c.d", "a.b?.c.d");
}

#[test]
fn ranges() {
	assert_expression("a..b", "(a..b)");
	assert_expression("1..10", "(1..10)");
	assert_expression("1..=10", "(1..=10)");
	assert_expression("start..", "(start..)");
	assert_expression("..end", "(..end)");
	assert_expression("..=end", "(..=end)");
	assert_expression("..", "(..)");
	assert_expression("0..count + 1", "(0..(count + 1))");
	assert_expression("array[1..4]", "array[(1..4)]");
	assert_expression("array[1..]", "array[(1..)]");
	assert_expression("x is in 1..10", "(x is in (1..10))");
}

#[test]
#[should_panic]
fn inclusive_range_requires_an_end() {
	parse_single_statement("1..=");
}

#[test]
fn compound_assignments() {
	assert_expression("count += 1", "count += 1");
	assert_expression("count -= 2", "count -= 2");
	assert_expression("count *= a + b", "count *= (a + b)");
	assert_expression("count **= 2", "count **= 2");
	assert_expression("count /= 4", "count /= 4");
	assert_expression("count //= 4", "count //= 4");
	assert_expression("player.score += 1", "player.score += 1");
}

#[test]
fn index_access() {
	assert_expression("foo[bar]", "foo[bar]");
	assert_expression("foo[bar][baz]", "foo[bar][baz]");
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
	assert_expression("\"right brace: \\}\"", "\"right brace: }}\"");
	assert_expression("\"carriage\\rreturn\"", "\"carriage\\rreturn\"");
	assert_expression("\"none\\0byte\"", "\"none\\0byte\"");
	assert_expression("\"hex: \\x41\"", "\"hex: A\"");
	assert_expression("\"unicode: \\u{2603}\"", "\"unicode: \u{2603}\"");
	assert_expression("\"emoji: \u{1F600}\"", "\"emoji: \u{1F600}\"");
	assert_expression("\"Value {a + b * c}\"", "\"Value {(a + (b * c))}\"");
	assert_expression("\"Call {sum(a, b + c)}\"", "\"Call {sum(a, (b + c))}\"");
	assert_expression("\"Access {a?.b.c.d}\"", "\"Access {a?.b.c.d}\"");
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
	assert_expression("a is not b", "(a is not b)");
	assert_expression("a is none", "(a is none)");
	assert_expression("a is not none", "(a is not none)");
	assert_expression("a is in b", "(a is in b)");
	assert_expression("a is not in b", "(a is not in b)");
	assert_expression("a is not none and b", "((a is not none) and b)");
	assert_expression("Person in persons", "(Person in persons)");

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
