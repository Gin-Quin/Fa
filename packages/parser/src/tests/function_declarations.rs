use crate::parse::{parse, parse_single_statement};

#[cfg(test)]
// Helper function to check if statement parsing results match expected output
fn assert_statement(input: &'static str, expected: &str) {
	let tree = parse_single_statement(input);
	println!("{:#?}", &tree);
	assert_eq!(tree.to_string(), expected);
}

#[test]
fn empty_block() {
	assert_statement(
		"function myFunction(): Result { }",
		"function myFunction(): Result {\n\t\n}",
	);
}

#[test]
fn empty_block_no_return_type() {
	assert_statement(
		"function myFunction() { }",
		"function myFunction() {\n\t\n}",
	);
}

#[test]
fn with_return_type() {
	// Functions with return type and block body
	assert_statement(
		"function myFunction(): Result { return value }",
		"function myFunction(): Result {\n\treturn value\n}",
	);
}

#[test]
fn with_parameters() {
	// Functions with parameters and return type
	assert_statement(
		"function multiply(a: Number, b: Number): Number { return a * b }",
		"function multiply(a: Number, b: Number): Number {\n\treturn (a * b)\n}",
	);
}

#[test]
fn with_parameters_no_return_type() {
	// Functions with parameters and no return type
	assert_statement(
		"function multiply(a: Number, b: Number) { return a * b }",
		"function multiply(a: Number, b: Number) {\n\treturn (a * b)\n}",
	);
}

#[test]
fn with_complex_bodies() {
	// Functions with multi-statement bodies
	assert_statement(
		"function process(): Result { \n\t\tintermediate = transform(input)\n\t\tresult = calculate(intermediate)\n\t\treturn result\n\t}",
		"function process(): Result {\n\tintermediate = transform(input)\n\tresult = calculate(intermediate)\n\treturn result\n}",
	);
}

#[test]
fn function_followed_by_let_statement() {
	let tree = parse("function foo(): Int { }\nlet value = 1");
	assert_eq!(
		tree.to_string(),
		"function foo(): Int {\n\t\n};\nlet value = 1;\n",
	);
}

#[test]
fn function_no_return_type_followed_by_let_statement() {
	let tree = parse("function foo() { }\nlet value = 1");
	assert_eq!(
		tree.to_string(),
		"function foo() {\n\t\n};\nlet value = 1;\n",
	);
}

#[test]
fn arrow_empty_block() {
	assert_statement("() => { }", "() => {\n\t\n}");
}

#[test]
fn arrow_block_with_return_type() {
	assert_statement(
		"(): Result => { return value }",
		"(): Result => {\n\treturn value\n}",
	);
}

#[test]
fn arrow_expression_body() {
	assert_statement("value => value + 1", "value => (value + 1)");
}

#[test]
fn arrow_with_parameters() {
	assert_statement(
		"(left: Number, right: Number) => left * right",
		"(left: Number, right: Number) => (left * right)",
	);
}
