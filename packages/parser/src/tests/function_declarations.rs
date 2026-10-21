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
	assert_statement("(value) => value + 1", "(value) => (value + 1)");
}

#[test]
#[should_panic]
fn arrow_parameters_require_parentheses() {
	parse_single_statement("value => value + 1");
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
		"function foo = (value) => value + 1",
		"function foo = (value) => (value + 1)",
	);
}

#[test]
fn function_with_default_parameter() {
	assert_statement(
		"function greet = (name: String, greeting: String = \"Hello\") => name",
		"function greet = (name: String, greeting: String = \"Hello\") => name",
	);
}

#[test]
fn async_function() {
	assert_statement(
		"async function fetchData = (url: String): Response => get(url)",
		"async function fetchData = (url: String): Response => get(url)",
	);
}

#[test]
fn function_with_throws() {
	assert_statement(
		"function load = (): Integer throws ApiError => value",
		"function load = (): Integer throws ApiError => value",
	);
	assert_statement(
		"function save = (score: Integer) throws ApiError | StorageError => { save(score) }",
		"function save = (score: Integer) throws (ApiError | StorageError) => {\n\tsave(score)\n}",
	);
}

#[test]
fn function_with_requires() {
	assert_statement(
		"function greet = () requires session: Session => session.user",
		"function greet = () requires session: Session => session.user",
	);
	assert_statement(
		"function getUser = (id: String): User throws DbError requires database: Database, logger: Logger => user",
		"function getUser = (id: String): User throws DbError requires database: Database, logger: Logger => user",
	);
}

#[test]
fn multiline_signature() {
	assert_statement(
		"function getUser = (id: String): User\n\tthrows DbError\n\trequires database: Database\n=> user",
		"function getUser = (id: String): User throws DbError requires database: Database => user",
	);
}

#[test]
#[should_panic]
fn requires_before_throws_is_invalid() {
	parse_single_statement("function f = () requires a: A throws E => a");
}

#[test]
fn function_types() {
	assert_statement(
		"type Callback = (message: String) => Void",
		"type Callback = (message: String) => Void",
	);
	assert_statement(
		"type Loader = (id: String) => User throws LoadError requires database: Database",
		"type Loader = (id: String) => User throws LoadError requires database: Database",
	);
}

#[test]
fn function_expressions() {
	assert_statement(
		"function addAndMultiply = add then multiply",
		"function addAndMultiply = (add then multiply)",
	);
	assert_statement("function add3 = makeAdd(3)", "function add3 = makeAdd(3)");
}

#[test]
fn mutable_parameters() {
	assert_statement(
		"function increment = (mutable self, mutable channel: Channel(String)) => self",
		"function increment = (mutable self, mutable channel: Channel(String)) => self",
	);
}

#[test]
#[should_panic]
fn let_with_lambda_is_invalid() {
	parse_single_statement("let addOne = (x: Integer) => x + 1");
}

#[test]
fn let_with_function_call_is_valid() {
	assert_statement("let add = makeAdd(3)", "let add = makeAdd(3)");
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

#[test]
fn function_type_annotations() {
	assert_statement(
		"function run = (callback: (value: Integer) => String) => callback(1)",
		"function run = (callback: (value: Integer) => String) => callback(1)",
	);
	assert_statement(
		"mutable handler: (value: Integer) => String = format",
		"mutable handler: (value: Integer) => String = format",
	);
}

#[test]
fn anonymous_structure_parameter() {
	assert_statement(
		"function greet = ({ name: String, greeting: String = \"Hello\" }) => name",
		"function greet = ({\n\tname: String\n\tgreeting: String = \"Hello\"\n}) => name",
	);
}
