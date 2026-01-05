use crate::parsing::parse::parse;
use crate::parsing::parse_single_statement::parse_single_statement;

#[cfg(test)]
fn assert_statement(input: &'static str, expected: &str) {
	let tree = parse_single_statement(input);
	println!("{:#?}", &tree);
	assert_eq!(tree.to_string(), expected);
}

#[test]
fn function_alias() {
	assert_statement(
		"function foo = someOtherFunction",
		"function foo = someOtherFunction",
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
fn arrow_expression_body_with_return_type() {
	assert_statement("(): Result => expression", "(): Result => expression");
}

#[test]
fn arrow_with_parameters() {
	assert_statement(
		"(left: Number, right: Number) => left * right",
		"(left: Number, right: Number) => (left * right)",
	);
}

#[test]
fn arrow_with_parameters_and_return_type() {
	assert_statement(
		"(left: Number, right: Number): Number => left * right",
		"(left: Number, right: Number): Number => (left * right)",
	);
}

#[test]
fn function_with_arrow_empty_block() {
	assert_statement("function foo = () => { }", "function foo = () => {\n\t\n}");
}

#[test]
fn function_with_arrow_block_with_return_type() {
	assert_statement(
		"function foo = (): Result => { return value }",
		"function foo = (): Result => {\n\treturn value\n}",
	);
}

#[test]
fn function_with_arrow_expression_body() {
	assert_statement(
		"function foo = value => value + 1",
		"function foo = value => (value + 1)",
	);
}

#[test]
fn function_with_arrow_expression_body_and_return_type() {
	assert_statement(
		"function foo = (): Result => expression",
		"function foo = (): Result => expression",
	);
}

#[test]
fn function_with_arrow_with_parameters() {
	assert_statement(
		"function multiply = (left: Number, right: Number) => left * right",
		"function multiply = (left: Number, right: Number) => (left * right)",
	);
}

#[test]
fn function_with_arrow_with_parameters_and_return_type() {
	assert_statement(
		"function multiply = (left: Number, right: Number): Number => left * right",
		"function multiply = (left: Number, right: Number): Number => (left * right)",
	);
}

#[test]
fn function_followed_by_let_statement() {
	let tree = parse("function foo = () => { }\nlet value = 1");
	assert_eq!(
		tree.to_string(),
		"function foo = () => {\n\t\n};\nlet value = 1;",
	);
}
