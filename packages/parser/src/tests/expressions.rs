use super::helpers::{assert_has_error, assert_statement};

#[test]
fn operations() {
	// Basic operations
	assert_statement("a+b", "(a + b)");
	assert_statement("a-b", "(a - b)");
	assert_statement("a*b", "(a * b)");

	// Operations with spaces
	assert_statement("a + b", "(a + b)");
	assert_statement("a - b", "(a - b)");
	assert_statement("a * b", "(a * b)");
	assert_statement("a / b", "(a / b)");
	assert_statement("a ** b", "(a ** b)");

	// Multiple operations of the same type
	assert_statement("a + b + c", "(a + b + c)");
	assert_statement("a - b - c", "(a - b - c)");
	assert_statement("a * b * c", "(a * b * c)");

	// Mixed operations
	assert_statement("a + b - c", "((a + b) - c)");
	assert_statement("a - b + c", "((a - b) + c)");
	assert_statement("a * b + c", "((a * b) + c)");
	assert_statement("a + b * c", "(a + (b * c))");

	// Operations with identifiers and literals
	assert_statement("a + 1", "(a + 1)");
	assert_statement("1 + a", "(1 + a)");
	assert_statement("1 + 2", "(1 + 2)");

	// Operations with boolean literals
	assert_statement("a + true", "(a + true)");
	assert_statement("false - b", "(false - b)");

	// Operations with union operator
	assert_statement("a | b", "(a | b)");
	assert_statement("a | b | c", "(a | b | c)");

	// Operations with composition keyword
	assert_statement("a then b", "(a then b)");
	assert_statement("a then b then c", "(a then b then c)");

	// Operations with remainder operator
	assert_statement("a %% b", "(a %% b)");
	assert_statement("a %% b %% c", "(a %% b %% c)");
	assert_statement("a + b %% c", "(a + (b %% c))");
	assert_statement("a%%b", "(a %% b)");
	assert_statement("50% + a %% 2", "(50% + (a %% 2))");

	// Fallback operator (optionals)
	assert_statement("a ?? b", "(a ?? b)");
	assert_statement(
		"user?.name ?? \"anonymous\"",
		"(user?.name ?? \"anonymous\")",
	);
	assert_statement("a ?? b or c", "(a ?? (b or c))");

	// Operations with relation operator
	assert_statement("a -> b", "(a -> b)");
}

#[test]
fn precedence() {
	assert_statement("  a+b *c", "(a + (b * c))");
	assert_statement("a*b+ c", "((a * b) + c)");
	assert_statement("a + b * c + d", "(a + (b * c) + d)");
	assert_statement("a * b + c * d", "((a * b) + (c * d))");
	assert_statement("a + b + c * d", "(a + b + (c * d))");
	assert_statement("a * b * c + d", "((a * b * c) + d)");
	assert_statement("a + b / c", "(a + (b / c))");
	assert_statement("a / b + c", "((a / b) + c)");
	assert_statement("a * b / c", "((a * b) / c)");
	assert_statement("a / b * c", "((a / b) * c)");
	assert_statement("a ** b * c", "((a ** b) * c)");
	assert_statement("a * b ** c", "(a * (b ** c))");

	// More complex precedence tests
	assert_statement("a + b * c - d", "((a + (b * c)) - d)");
	assert_statement("a * (b + c) * d", "(a * ((b + c)) * d)");

	// Operations with composition keyword
	assert_statement("a then b * c", "(a then (b * c))");
	assert_statement("a * b then c", "((a * b) then c)");
	assert_statement("a or b then c", "((a or b) then c)");
}

#[test]
fn prefix() {
	assert_statement("-a", "(-a)");
	assert_statement("-1", "-1");
	assert_statement("-true", "(-true)");
	assert_statement("...a", "...a");

	// Prefix with operations
	assert_statement("-a + b", "((-a) + b)");
	assert_statement("a + -b", "(a + (-b))");
	assert_statement("-a * b", "((-a) * b)");
	assert_statement("a * -b", "(a * (-b))");

	// Multiple prefix operators
	assert_statement("-(-a)", "(-((-a)))");

	// Prefix with parentheses
	assert_statement("-(a + b)", "(-((a + b)))");
	assert_statement("-a * -b", "((-a) * (-b))");
}

#[test]
fn list_literals() {
	assert_statement("[]", "[]");
	assert_statement("[a, b, c]", "[a, b, c]");
	assert_statement("[[1, 2], [3, 4]]", "[[1, 2], [3, 4]]");
	assert_statement("[{ a = 1 }, { b = 2 }]", "[{\n\ta = 1\n}, {\n\tb = 2\n}]");
	assert_statement("{ a = [1, 2], b = [3] }", "{\n\ta = [1, 2]\n\tb = [3]\n}");
}

#[test]
fn compile_time_values() {
	assert_statement("let @foo = 12", "let @foo = 12");
	assert_statement("let z = @add(2, 4)", "let z = @add(2, 4)");
	assert_statement("console.log(@type(x))", "console.log(@type(x))");
	assert_statement(
		"use weather = @import(\"https://api.weather.com/fa/types.fa\")",
		"use weather = @import(\"https://api.weather.com/fa/types.fa\")",
	);
	assert_statement(
		"@for fruit in @fruits { console.log(fruit) }",
		"@for fruit in @fruits {\n\tconsole.log(fruit)\n}",
	);
}

#[test]
fn private_names() {
	assert_statement("self.#bag.delete(node)", "self.#bag.delete(node)");
	assert_statement("Self in #bag", "(Self in #bag)");
}

#[test]
fn mutable_use_and_reuse() {
	assert_statement(
		"mutable use best = players[0]",
		"mutable use best = players[0]",
	);
	assert_statement(
		"mutable use best: Player in players = players[0]",
		"mutable use best: (Player in players) = players[0]",
	);
	assert_statement("reuse best = player", "reuse best = player");
	assert_statement(
		"reuse best = players[index + 1]",
		"reuse best = players[(index + 1)]",
	);
}

#[test]
fn control_flow_keywords() {
	assert_statement("continue", "continue");
	assert_statement("break", "break");
	assert_statement("break value", "break value");
	assert_statement("mutable value = other", "mutable value = other");
	assert_statement("use value = other", "use value = other");
	assert_statement("use value: Number = other", "use value: Number = other");
	assert_statement("static version = 12", "static version = 12");
	assert_statement(
		"static version: String = @readFile(\"version.txt\")",
		"static version: String = @readFile(\"version.txt\")",
	);
}

#[test]
fn literal_none() {
	assert_statement("none", "none");
}

#[test]
fn for_loop_expression() {
	assert_statement("for values { continue }", "for values {\n\tcontinue\n}");
	assert_statement("@for values { continue }", "@for values {\n\tcontinue\n}");
	assert_statement(
		"for value in values { continue }",
		"for value in values {\n\tcontinue\n}",
	);
	assert_statement(
		"for item, index in items { continue }",
		"for item, index in items {\n\tcontinue\n}",
	);
	assert_statement(
		"for { name, age as years } in columnar { continue }",
		"for { name, age as years } in columnar {\n\tcontinue\n}",
	);
	assert_statement(
		"for (id, name) in usersById { continue }",
		"for (id, name) in usersById {\n\tcontinue\n}",
	);
	assert_statement(
		"for i in 1..=10 { continue }",
		"for i in (1..=10) {\n\tcontinue\n}",
	);
	assert_statement("for 1..10 { continue }", "for (1..10) {\n\tcontinue\n}");
	assert_statement(
		"@for fruit in fruits { continue }",
		"@for fruit in fruits {\n\tcontinue\n}",
	);
	assert_statement(
		"for item in getItems(filter = { active = true }) { continue }",
		"for item in getItems(filter = {\n\tactive = true\n}) {\n\tcontinue\n}",
	);
}

#[test]
fn parallel_for_loop() {
	assert_statement(
		"for parallel number in numbers { continue }",
		"for parallel number in numbers {\n\tcontinue\n}",
	);
	assert_statement(
		"for parallel(threads = 2) number in numbers { continue }",
		"for parallel(threads = 2) number in numbers {\n\tcontinue\n}",
	);
}

#[test]
fn parallel_is_a_keyword() {
	assert_has_error("for parallel in parallels { continue }");
}

#[test]
fn for_loop_index_must_be_a_name() {
	assert_has_error("for item, { index } in items { continue }");
}

#[test]
fn while_and_loop() {
	assert_statement(
		"while condition { break value }",
		"while condition {\n\tbreak value\n}",
	);
	assert_statement("loop { continue }", "loop {\n\tcontinue\n}");
	assert_statement(
		"while let value = stack.pop() { continue }",
		"while let value = stack.pop() {\n\tcontinue\n}",
	);
}

#[test]
fn while_let_binds_a_name_only() {
	assert_has_error("while let { name } = queue.next() { continue }");
}

#[test]
fn do_expression() {
	assert_statement("do { continue }", "do {\n\tcontinue\n}");
	assert_statement("do { value } + other", "(do {\n\tvalue\n} + other)");
}

#[test]
fn if_else_chains() {
	assert_statement("if condition { continue }", "if condition {\n\tcontinue\n}");
	assert_statement(
		"if condition { continue } else { break }",
		"if condition {\n\tcontinue\n} else {\n\tbreak\n}",
	);
	assert_statement(
		"if a { continue } else if b { break } else { return }",
		"if a {\n\tcontinue\n} else if b {\n\tbreak\n} else {\n\treturn\n}",
	);
}

#[test]
fn if_expression_usage() {
	assert_statement(
		"let result = if condition { value } else { other }",
		"let result = if condition {\n\tvalue\n} else {\n\tother\n}",
	);
	assert_statement(
		"if condition { value } + other",
		"(if condition {\n\tvalue\n} + other)",
	);
}

#[test]
fn when_expression() {
	assert_statement(
		"when foo is {\n1 => bar\n2 => { baz }\nelse => { other }\n}",
		"when foo is {\n\t1 => bar\n\t2 => {\n\t\tbaz\n\t}\n\telse => {\n\t\tother\n\t}\n}",
	);
	assert_statement(
		"when value is {\nNumber => one\nString => two\n}",
		"when value is {\n\tNumber => one\n\tString => two\n}",
	);
	assert_statement(
		"when foo is {\n1 => bar\n2 => baz\n3 => qux\n}",
		"when foo is {\n\t1 => bar\n\t2 => baz\n\t3 => qux\n}",
	);
	assert_statement(
		"when foo is {\n1 => { bar }\n2 => { baz }\nelse => other\n}",
		"when foo is {\n\t1 => {\n\t\tbar\n\t}\n\t2 => {\n\t\tbaz\n\t}\n\telse => other\n}",
	);
}

#[test]
fn when_branch_bindings() {
	assert_statement(
		"when shape is {\nShape.Point => 0\nShape.Circle as circle => circle.radius\n}",
		"when shape is {\n\tShape.Point => 0\n\tShape.Circle as circle => circle.radius\n}",
	);
	assert_statement(
		"when shape is {\nShape.Rectangle as { width, height } => width * height\n}",
		"when shape is {\n\tShape.Rectangle as { width, height } => (width * height)\n}",
	);
	assert_statement(
		"when shape is {\nShape.Segment as (start, end) => start\nelse => none\n}",
		"when shape is {\n\tShape.Segment as (start, end) => start\n\telse => none\n}",
	);
}

#[test]
fn when_else_branch_cannot_be_bound() {
	assert_has_error("when shape is {\nelse as other => other\n}");
}

#[test]
fn member_access() {
	assert_statement("foo.bar", "foo.bar");
	assert_statement("foo.bar.baz", "foo.bar.baz");
}

#[test]
fn member_index() {
	assert_statement("foo { bar }", "foo {\n\tbar\n}");
	assert_statement("foo { a = 1, b }", "foo {\n\ta = 1\n\tb\n}");
}

#[test]
fn percentage_literals() {
	assert_statement("50%", "50%");
	assert_statement("-12%", "-12%");
	assert_statement("50.5%", "50.5%");
	assert_statement("123n%", "123n%");
}

#[test]
fn optional_suffix() {
	assert_statement("value?", "value?");
	assert_statement("Array(Integer)?", "Array(Integer)?");
}

#[test]
fn optional_access() {
	assert_statement("a?.b.c", "a?.b.c");
	assert_statement("a.b?.c.d", "a.b?.c.d");
}

#[test]
fn ranges() {
	assert_statement("a..b", "(a..b)");
	assert_statement("1..10", "(1..10)");
	assert_statement("1..=10", "(1..=10)");
	assert_statement("start..", "(start..)");
	assert_statement("..end", "(..end)");
	assert_statement("..=end", "(..=end)");
	assert_statement("..", "(..)");
	assert_statement("0..count + 1", "(0..(count + 1))");
	assert_statement("array[1..4]", "array[(1..4)]");
	assert_statement("array[1..]", "array[(1..)]");
	assert_statement("x is in 1..10", "(x is in (1..10))");
}

#[test]
fn inclusive_range_requires_an_end() {
	assert_has_error("1..=");
}

#[test]
fn compound_assignments() {
	assert_statement("count += 1", "count += 1");
	assert_statement("count -= 2", "count -= 2");
	assert_statement("count *= a + b", "count *= (a + b)");
	assert_statement("count **= 2", "count **= 2");
	assert_statement("count /= 4", "count /= 4");
	assert_statement("count //= 4", "count //= 4");
	assert_statement("count %%= 4", "count %%= 4");
	assert_statement("player.score += 1", "player.score += 1");
}

#[test]
fn index_access() {
	assert_statement("foo[bar]", "foo[bar]");
	assert_statement("foo[bar][baz]", "foo[bar][baz]");
}

#[test]
fn strings() {
	assert_statement("\"hello\"", "\"hello\"");
	assert_statement("\"Hello {name}\"", "\"Hello {name}\"");
	assert_statement("\"Hello {{name}}\"", "\"Hello {{name}}\"");
	assert_statement(
		"\"Hello {name} and {count}\"",
		"\"Hello {name} and {count}\"",
	);
	assert_statement("\"line\\nfeed\"", "\"line\\nfeed\"");
	assert_statement("\"tab\\tseparated\"", "\"tab\\tseparated\"");
	assert_statement("\"quote: \\\"\"", "\"quote: \\\"\"");
	assert_statement("\"braces: {{name}}\"", "\"braces: {{name}}\"");
	assert_has_error("\"braces: \\{name\\}\"");
	assert_statement("\"carriage\\rreturn\"", "\"carriage\\rreturn\"");
	assert_statement("\"none\\0byte\"", "\"none\\0byte\"");
	assert_statement("\"hex: \\x41\"", "\"hex: A\"");
	assert_statement("\"unicode: \\u{2603}\"", "\"unicode: \u{2603}\"");
	assert_statement("\"emoji: \u{1F600}\"", "\"emoji: \u{1F600}\"");
	assert_statement("\"Value {a + b * c}\"", "\"Value {(a + (b * c))}\"");
	assert_statement("\"Call {sum(a, b + c)}\"", "\"Call {sum(a, (b + c))}\"");
	assert_statement("\"Access {a?.b.c.d}\"", "\"Access {a?.b.c.d}\"");
	assert_statement("\"Relation {a -> b}\"", "\"Relation {(a -> b)}\"");
}

#[test]
fn groups() {
	assert_statement("( a   )", "(a)");
	assert_statement("(a + b) * c", "(((a + b)) * c)");
	assert_statement("a * (b * c)", "(a * ((b * c)))");
	assert_statement("a * (b + c)", "(a * ((b + c)))");

	// Nested groups
	assert_statement("((a))", "((a))");
	assert_statement("(a + (b + c))", "((a + ((b + c))))");

	// Groups with prefix operators
	assert_statement("-(a + b)", "(-((a + b)))");
	assert_statement("(-a + b)", "(((-a) + b))");
}

// Check that empty groups are forbidden
#[test]
fn empty_group() {
	assert_statement("()", "()");
	assert_statement("(  )", "()");
	assert_statement("( \n )", "()");
}

#[test]
fn logical_operations() {
	// Logical operations
	assert_statement("a and b", "(a and b)");
	assert_statement("a and b and c", "(a and b and c)");
	assert_statement("a or b", "(a or b)");
	assert_statement("not a", "(not a)");
	assert_statement("a is b", "(a is b)");
	assert_statement("a is not b", "(a is not b)");
	assert_statement("a is none", "(a is none)");
	assert_statement("a is not none", "(a is not none)");
	assert_statement("a is in b", "(a is in b)");
	assert_statement("a is not in b", "(a is not in b)");
	assert_statement("a is not none and b", "((a is not none) and b)");
	assert_statement("Person in persons", "(Person in persons)");

	// Logical operations with precedence
	assert_statement("a and b or c", "((a and b) or c)");
	assert_statement("a or b and c", "(a or (b and c))");
	assert_statement("not a and b", "((not a) and b)");
	assert_statement("(not a) and b", "(((not a)) and b)");
}

#[test]
fn comparison_operations() {
	// Comparison operations
	assert_statement("a == b", "(a == b)");
	assert_statement("a != b", "(a != b)");
	assert_statement("a < b", "(a < b)");
	assert_statement("a <= b", "(a <= b)");
	assert_statement("a <= b <= c", "(a <= b <= c)");
	assert_statement("a >= b >= c", "(a >= b >= c)");
	assert_statement("a > b", "(a > b)");
	assert_statement("a >= b", "(a >= b)");

	// Comparison operations with other operations
	assert_statement("a + b == c", "((a + b) == c)");
	assert_statement("a == b + c", "(a == (b + c))");
	assert_statement("a < b and c > d", "((a < b) and (c > d))");
}

#[test]
fn static_requires_a_value() {
	assert_has_error("static version");
}

#[test]
fn compile_time_if() {
	assert_statement(
		"@if target is \"native\" { a } @else { b }",
		"@if (target is \"native\") {\n\ta\n} @else {\n\tb\n}",
	);
	assert_statement(
		"@if debug { a } @else @if verbose { b }",
		"@if debug {\n\ta\n} @else @if verbose {\n\tb\n}",
	);
}

#[test]
fn compile_time_if_requires_compile_time_else() {
	assert_has_error("@if debug { a } else { b }");
}

#[test]
fn test_declarations() {
	assert_statement(
		"test \"adds numbers\" { assert add(1, 2) == 3 }",
		"test \"adds numbers\" {\n\tassert (add(1, 2) == 3)\n}",
	);
	assert_statement(
		"test skip \"slow\" { run() }",
		"test skip \"slow\" {\n\trun()\n}",
	);
	assert_statement(
		"test only \"focus\" { run() }",
		"test only \"focus\" {\n\trun()\n}",
	);
	assert_statement(
		"@test \"at compile time\" { assert sum([1, 2]) == 3 }",
		"@test \"at compile time\" {\n\tassert (sum([1, 2]) == 3)\n}",
	);
	// `test` remains usable as a name
	assert_statement("test = 2", "test = 2");
	assert_statement("test(value)", "test(value)");
}

#[test]
fn digit_separators() {
	assert_statement("8_100_000_000", "8100000000");
	assert_statement("0b1111_0000", "240");
	assert_statement("0xFF_88_00", "16746496");
	assert_statement("1_000.5", "1000.5");
}

#[test]
fn relations() {
	assert_statement("Bag(Self->bestFriend)", "Bag((Self -> bestFriend))");
	assert_statement("Bag(in Self->bestFriends)", "Bag(in (Self -> bestFriends))");
	assert_statement("(Self->bestFriend)?", "((Self -> bestFriend))?");
}

#[test]
fn unexpected_token_in_call_is_an_error() {
	assert_has_error("f(})");
}

#[test]
fn watch_blocks() {
	assert_statement(
		"watch { console.log(count) }",
		"watch {\n\tconsole.log(count)\n}",
	);
	assert_statement(
		"mutable subscription = watch { console.log(count) }",
		"mutable subscription = watch {\n\tconsole.log(count)\n}",
	);
}

#[test]
fn reactive_fields() {
	assert_statement(
		"type Counter = { reactive count: Integer = 0, derived double = count * 2 }",
		"type Counter = {\n\treactive count: Integer = 0\n\tderived double = (count * 2)\n}",
	);
}

#[test]
fn partial_update() {
	assert_statement(
		"settings .= { port = 8080, host = \"localhost\" }",
		"settings .= {\n\tport = 8080\n\thost = \"localhost\"\n}",
	);
	assert_statement(
		"settings.build .= { minify = false }",
		"settings.build .= {\n\tminify = false\n}",
	);
}

#[test]
fn partial_update_expects_an_object() {
	assert_has_error("settings .= other");
}

#[test]
fn type_functions_with_logic() {
	assert_statement(
		"type Optionalize<Value> = when Value is {\nNone => None\nelse => Value?\n}",
		"type Optionalize<Value> = when Value is {\n\tNone => None\n\telse => Value?\n}",
	);
	assert_statement(
		"type Storage<size: Integer> = if size <= 255 { UInt8 } else { UInt64 }",
		"type Storage<size: Integer> = if (size <= 255) {\n\tUInt8\n} else {\n\tUInt64\n}",
	);
	assert_statement(
		"type Capitalized<Input> = do {\nlet schema = @typeToSchema(Input)\nTypeFromSchema<schema>\n}",
		"type Capitalized<Input> = do {\n\tlet schema = @typeToSchema(Input)\n\tTypeFromSchema<schema>\n}",
	);
}
