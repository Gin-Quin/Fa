use super::helpers::{assert_has_error, assert_statement};
use crate::parsing::parse::parse;

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
fn arrow_parameters_require_parentheses() {
	assert_has_error("value => value + 1");
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
fn requires_before_throws_is_invalid() {
	assert_has_error("function f = () requires a: A throws E => a");
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
fn mutation_effects() {
	assert_statement(
		"function increment = (self) mutates self => { self.value += 1 }",
		"function increment = (self) mutates self => {\n\tself.value += 1\n}",
	);
	assert_statement(
		"function set = (self, index: Integer, item: Item) mutates self[index] => item",
		"function set = (self, index: Integer, item: Item) mutates self[index] => item",
	);
	assert_statement(
		"function deposit = (self, amount: Integer): Integer throws Overflow requires log: Logger mutates self.balance, log => amount",
		"function deposit = (self, amount: Integer): Integer throws Overflow requires log: Logger mutates self.balance, log => amount",
	);
	assert_statement(
		"function push = (self, item: Item) invalidates self => item",
		"function push = (self, item: Item) invalidates self => item",
	);
	assert_statement(
		"function swap = (self, a: Integer, b: Integer) mutates self[a], self[b] invalidates other => self",
		"function swap = (self, a: Integer, b: Integer) mutates self[a], self[b] invalidates other => self",
	);
	assert_statement(
		"type Push = (item: Integer) => Void invalidates self",
		"type Push = (item: Integer) => Void invalidates self",
	);
}

#[test]
fn fine_grained_invalidations() {
	assert_statement(
		"function delete = (self, index: Integer) invalidates self[index..] => index",
		"function delete = (self, index: Integer) invalidates self[(index..)] => index",
	);
	assert_statement(
		"function pop = (self) invalidates self[self.length - 1] => self",
		"function pop = (self) invalidates self[(self.length - 1)] => self",
	);
	assert_statement(
		"function delete = (self, key: Key) invalidates self[key] => key",
		"function delete = (self, key: Key) invalidates self[key] => key",
	);
	assert_statement(
		"function delete = (self, element: Element in self) invalidates element => element",
		"function delete = (self, element: (Element in self)) invalidates element => element",
	);
	assert_statement(
		"function clearFirst = (self) invalidates self[0] => self",
		"function clearFirst = (self) invalidates self[0] => self",
	);
}

#[test]
fn effect_index_must_be_a_path() {
	assert_has_error("function f = (self) invalidates self[compute()] => self");
}

#[test]
fn mutable_parameters_are_invalid() {
	assert_has_error("function increment = (mutable self) => self");
}

#[test]
fn effects_order_is_enforced() {
	assert_has_error("function f = (self) mutates self requires a: A => self");
}

#[test]
fn effect_paths_are_simple() {
	assert_has_error("function f = (self) mutates self.items.map(x) => self");
}

#[test]
fn variadic_parameters() {
	assert_statement(
		"function sum = (...values: Integer): Integer => values.length",
		"function sum = (...values: Integer): Integer => values.length",
	);
}

#[test]
fn let_with_lambda_parses() {
	// a semantic rule: the checker reports it (E-DECL-6)
	assert_statement(
		"let addOne = (x: Integer) => x + 1",
		"let addOne = (x: Integer) => (x + 1)",
	);
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

#[test]
fn comments_are_ignored() {
	let tree = parse(
		"-- a comment\nlet a = 1 -- after a value\n--- a block\ncomment ---\nfunction f = () => { -- inside\n\ta -- end of line\n}",
	);
	assert_eq!(
		tree.to_string(),
		"let a = 1;\nfunction f = () => {\n\ta\n};",
	);
}

#[test]
fn generic_return_types() {
	assert_statement(
		"function f = (self, data: Array<Integer>): Array<Integer> => data",
		"function f = (self, data: Array<Integer>): Array<Integer> => data",
	);
}

#[test]
fn partial_application() {
	assert_statement(
		"function xz = xyz with (y = 0)",
		"function xz = (xyz with (y = 0))",
	);
	assert_statement(
		"function f = a then b with (x = 1)",
		"function f = (a then (b with (x = 1)))",
	);
}

#[test]
fn spread_arguments() {
	assert_statement("let total = sum(...numbers)", "let total = sum(...numbers)");
}

#[test]
fn signature_clauses_in_a_type_annotation() {
	assert_statement(
		"let f: (a: Integer) throws E => Integer = g",
		"let f: (a: Integer) throws E => Integer = g",
	);
	assert_statement(
		"type T = { handler: (a: Integer) throws E => Integer }",
		"type T = {\n\thandler: (a: Integer) throws E => Integer\n}",
	);
}

#[test]
fn signature_clauses_in_a_parameter_type() {
	assert_statement(
		"function run = (callback: () throws E => Void) => callback()",
		"function run = (callback: () throws E => Void) => callback()",
	);
}

#[test]
fn signature_clauses_in_an_assigned_lambda() {
	assert_statement("handler = (x) throws E => 1", "handler = (x) throws E => 1");
	assert_statement(
		"handler = (x) requires db: Db => 1",
		"handler = (x) requires db: Db => 1",
	);
	assert_statement(
		"f(callback = (x) mutates x => x)",
		"f(callback = (x) mutates x => x)",
	);
}

#[test]
fn signature_clauses_after_a_requirement() {
	assert_statement(
		"function f = () requires a: A mutates a => a",
		"function f = () requires a: A mutates a => a",
	);
	assert_has_error("function f = () requires a: A requires b: B => a");
}
