use super::helpers::assert_statement;

#[test]
fn function_calls_no_parameters() {
	// Function calls with no parameters
	assert_statement("toto()", "toto()");
	assert_statement("toto( )", "toto()");
	assert_statement("getValue(\n)", "getValue()");
	assert_statement("calculateResult( \n\t\t\n  )", "calculateResult()");
}

#[test]
fn function_calls_single_parameter() {
	// Function calls with a single parameter
	assert_statement("toto(zabu)", "toto(zabu)");
	assert_statement("getValue(x)", "getValue(x)");
	assert_statement("calculateResult(42)", "calculateResult(42)");

	// Function calls with expressions as parameters
	assert_statement("toto(a + b)", "toto((a + b))");
	assert_statement("getValue(x * y)", "getValue((x * y))");
	assert_statement("calculateResult(a and b)", "calculateResult((a and b))");
}

#[test]
fn function_calls_multiple_parameters() {
	// Function calls with multiple parameters
	assert_statement("toto(zabu, tintin)", "toto(zabu, tintin)");
	assert_statement("getValue(x, y)", "getValue(x, y)");
	assert_statement("calculateResult(42, true)", "calculateResult(42, true)");

	// Function calls with expressions as parameters
	assert_statement("toto(a + b, c * d)", "toto((a + b), (c * d))");
	assert_statement("getValue(x and y, z or w)", "getValue((x and y), (z or w))");
	assert_statement(
		"calculateResult(a < b, c > d)",
		"calculateResult((a < b), (c > d))",
	);

	// Function calls with more than two parameters
	assert_statement("toto(a, b, c)", "toto(a, b, c)");
	assert_statement("getValue(x, y, z, w)", "getValue(x, y, z, w)");
}

#[test]
fn function_calls_labeled_parameters() {
	// Function calls with labeled parameters
	assert_statement("toto(x = zabu)", "toto(x = zabu)");
	assert_statement("getValue(key = value)", "getValue(key = value)");

	// Function calls with multiple labeled parameters
	assert_statement(
		"toto(x = zabu, machin = tintin)",
		"toto(x = zabu, machin = tintin)",
	);
	assert_statement(
		"getValue(first = a, second = b)",
		"getValue(first = a, second = b)",
	);

	// Function calls with mixed labeled and unlabeled parameters
	assert_statement("toto(zabu, machin = tintin)", "toto(zabu, machin = tintin)");
	assert_statement("getValue(a, b, key = value)", "getValue(a, b, key = value)");

	// Function calls with expressions in labeled parameters
	assert_statement("toto(x = a + b)", "toto(x = (a + b))");
	assert_statement("getValue(key = x and y)", "getValue(key = (x and y))");
}

#[test]
fn nested_function_calls() {
	// Nested function calls
	assert_statement("toto(getValue())", "toto(getValue())");
	assert_statement("outer(inner(x))", "outer(inner(x))");

	// Nested function calls with multiple parameters
	assert_statement("toto(getValue(x, y))", "toto(getValue(x, y))");
	assert_statement(
		"outer(inner(a, b), another(c, d))",
		"outer(inner(a, b), another(c, d))",
	);

	// Nested function calls with labeled parameters
	assert_statement("toto(x = getValue(y))", "toto(x = getValue(y))");
	assert_statement("outer(a = inner(x = value))", "outer(a = inner(x = value))");
}

#[test]
fn function_calls_in_expressions() {
	// Function calls in expressions
	assert_statement("toto() + getValue()", "(toto() + getValue())");
	assert_statement("a * func(b)", "(a * func(b))");

	// Function calls in complex expressions
	assert_statement(
		"toto(x) + getValue(y) * calculate(z)",
		"(toto(x) + (getValue(y) * calculate(z)))",
	);
	assert_statement("a and func(b) or func(c)", "((a and func(b)) or func(c))");

	// Function calls with expressions containing function calls
	assert_statement("toto(a + getValue(b))", "toto((a + getValue(b)))");
	assert_statement(
		"func(x = a and getValue(y))",
		"func(x = (a and getValue(y)))",
	);
}
