use crate::parse::parse_single_statement;

#[cfg(test)]
// Helper function to check if statement parsing results match expected output
fn assert_expression(input: &'static str, expected: &str) {
	let tree = parse_single_statement(input);
	println!("{:#?}", &tree);
	assert_eq!(tree.to_string(), expected);
}

#[test]
fn function_calls_no_parameters() {
	// Function calls with no parameters
	assert_expression("toto()", "toto()");
	assert_expression("toto( )", "toto()");
	assert_expression("getValue(\n)", "getValue()");
	assert_expression("calculateResult( \n\t\t\n  )", "calculateResult()");
}

#[test]
fn function_calls_single_parameter() {
	// Function calls with a single parameter
	assert_expression("toto(zabu)", "toto(zabu)");
	assert_expression("getValue(x)", "getValue(x)");
	assert_expression("calculateResult(42)", "calculateResult(42)");

	// Function calls with expressions as parameters
	assert_expression("toto(a + b)", "toto((a + b))");
	assert_expression("getValue(x * y)", "getValue((x * y))");
	assert_expression("calculateResult(a and b)", "calculateResult((a and b))");
}

#[test]
fn function_calls_multiple_parameters() {
	// Function calls with multiple parameters
	assert_expression("toto(zabu, tintin)", "toto(zabu, tintin)");
	assert_expression("getValue(x, y)", "getValue(x, y)");
	assert_expression("calculateResult(42, true)", "calculateResult(42, true)");

	// Function calls with expressions as parameters
	assert_expression("toto(a + b, c * d)", "toto((a + b), (c * d))");
	assert_expression("getValue(x and y, z or w)", "getValue((x and y), (z or w))");
	assert_expression(
		"calculateResult(a < b, c > d)",
		"calculateResult((a < b), (c > d))"
	);

	// Function calls with more than two parameters
	assert_expression("toto(a, b, c)", "toto(a, b, c)");
	assert_expression("getValue(x, y, z, w)", "getValue(x, y, z, w)");
}

#[test]
fn function_calls_labeled_parameters() {
	// Function calls with labeled parameters
	assert_expression("toto(x = zabu)", "toto(let x = zabu)");
	assert_expression("getValue(key = value)", "getValue(let key = value)");

	// Function calls with multiple labeled parameters
	assert_expression(
		"toto(x = zabu, machin = tintin)",
		"toto(let x = zabu, let machin = tintin)"
	);
	assert_expression(
		"getValue(first = a, second = b)",
		"getValue(let first = a, let second = b)"
	);

	// Function calls with mixed labeled and unlabeled parameters
	assert_expression("toto(zabu, machin = tintin)", "toto(zabu, let machin = tintin)");
	assert_expression("getValue(a, b, key = value)", "getValue(a, b, let key = value)");

	// Function calls with expressions in labeled parameters
	assert_expression("toto(x = a + b)", "toto(let x = (a + b))");
	assert_expression("getValue(key = x and y)", "getValue(let key = (x and y))");
}

#[test]
fn nested_function_calls() {
	// Nested function calls
	assert_expression("toto(getValue())", "toto(getValue())");
	assert_expression("outer(inner(x))", "outer(inner(x))");

	// Nested function calls with multiple parameters
	assert_expression("toto(getValue(x, y))", "toto(getValue(x, y))");
	assert_expression(
		"outer(inner(a, b), another(c, d))",
		"outer(inner(a, b), another(c, d))"
	);

	// Nested function calls with labeled parameters
	assert_expression("toto(x = getValue(y))", "toto(let x = getValue(y))");
	assert_expression(
		"outer(a = inner(x = value))",
		"outer(let a = inner(let x = value))"
	);
}

#[test]
fn function_calls_in_expressions() {
	// Function calls in expressions
	assert_expression("toto() + getValue()", "(toto() + getValue())");
	assert_expression("a * func(b)", "(a * func(b))");

	// Function calls in complex expressions
	assert_expression(
		"toto(x) + getValue(y) * calculate(z)",
		"(toto(x) + (getValue(y) * calculate(z)))"
	);
	assert_expression("a and func(b) or func(c)", "((a and func(b)) or func(c))");

	// Function calls with expressions containing function calls
	assert_expression("toto(a + getValue(b))", "toto((a + getValue(b)))");
	assert_expression("func(x = a and getValue(y))", "func(let x = (a and getValue(y)))");
}
