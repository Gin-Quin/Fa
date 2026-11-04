//! The syntax decided in the documentation that the parser didn't support (see the parser
//! follow-ups of `design/spec-review.md`).

use super::helpers::{assert_has_error, assert_module, assert_statement};

#[test]
fn export_mutable_value() {
	assert_statement(
		"export mutable: Pool<Person> = []",
		"export mutable: Pool<Person> = []",
	);
}

#[test]
fn export_mutable_value_without_type() {
	assert_statement("export mutable = 0", "export mutable = 0");
}

#[test]
fn export_async_function() {
	assert_statement(
		"export async function = (url: String): String throws HttpError => url",
		"export async function = (url: String): String throws HttpError => url",
	);
}

#[test]
fn export_generic_type() {
	assert_statement(
		"export type<First, Second> = { first: First }",
		"export type<First, Second> = {\n\tfirst: First\n}",
	);
}

#[test]
fn export_generic_function() {
	assert_statement(
		"export function = <Item>(items: Array<Item>): Item? => items.get(0)",
		"export function = <Item>(items: Array<Item>): Item? => items.get(0)",
	);
}

#[test]
fn export_requires_a_known_form() {
	assert_has_error("export let x = 1");
	assert_has_error("export name = 1");
}

#[test]
fn mutable_member_of_a_namespace() {
	assert_statement(
		"namespace counters = { mutable count = 0 }",
		"namespace counters = {\n\tmutable count = 0\n}",
	);
}

#[test]
fn effect_path_on_every_element() {
	assert_statement(
		"(players: Array<Player>) mutates players[..].score => none",
		"(players: Array(Player)) mutates players[(..)].score => none",
	);
}

#[test]
fn effect_path_with_an_index_then_a_field() {
	assert_statement(
		"(c: C, i: Integer) mutates c[i].d => none",
		"(c: C, i: Integer) mutates c[i].d => none",
	);
}

#[test]
fn effect_path_through_a_private_field() {
	assert_statement(
		"(self) mutates self.#nodes[..].next, self.firstChild => none",
		"(self) mutates self.#nodes[(..)].next, self.firstChild => none",
	);
}

#[test]
fn async_function_type() {
	assert_statement(
		"type Handler = async (request: Request) => Response",
		"type Handler = async (request: Request) => Response",
	);
}

#[test]
fn async_lambda() {
	assert_statement(
		"run(async () => await load())",
		"run(async () => (await load()))",
	);
}

#[test]
fn async_requires_a_function() {
	assert_has_error("let x = async 12");
}

#[test]
fn function_declaration_with_a_type() {
	assert_statement(
		"function ascending: Comparator = (a, b) => a - b",
		"function ascending: Comparator = (a, b) => (a - b)",
	);
}

#[test]
fn function_declaration_with_a_function_type() {
	assert_statement(
		"function close: (database: Pointer<Sqlite3>) => c.Int mutates database = @extern(\"sqlite3_close\")",
		"function close: (database: Pointer<Sqlite3>) => c.Int mutates database = @extern(\"sqlite3_close\")",
	);
}

#[test]
fn object_rest_pattern() {
	assert_statement(
		"let { name, ...details } = person",
		"let { name, ...details } = person",
	);
}

#[test]
fn array_rest_pattern() {
	assert_statement(
		"let [first, ...others] = items",
		"let [first, ...others] = items",
	);
}

#[test]
fn rest_pattern_must_be_last() {
	assert_has_error("let [...others, last] = items");
}

#[test]
fn rest_pattern_is_unique() {
	assert_has_error("let { ...a, ...b } = items");
}

#[test]
fn tuple_pattern_has_no_rest() {
	assert_has_error("let (first, ...others) = tuple");
}

#[test]
fn nested_string_in_interpolation() {
	assert_statement(
		"\"Name: {user.name ?? \"anonymous\"}\"",
		"\"Name: {(user.name ?? \"anonymous\")}\"",
	);
}

#[test]
fn object_literal_in_interpolation() {
	assert_statement(
		"\"Origin: {Point { x = 0 }}\"",
		"\"Origin: {Point {\n\tx = 0\n}}\"",
	);
}

#[test]
fn interpolation_nested_twice() {
	assert_statement("\"a {f(\"b {c}\")} d\"", "\"a {f(\"b {c}\")} d\"");
}

#[test]
fn parallel_is_a_keyword_of_for() {
	assert_statement(
		"for parallel item in items { continue }",
		"for parallel item in items {\n\tcontinue\n}",
	);
}

#[test]
fn yield_is_a_name() {
	assert_statement("yield + 1", "(yield + 1)");
}

#[test]
fn placeholder_in_an_object() {
	assert_statement("type Config = { ... }", "type Config = {\n\t...\n}");
}

#[test]
fn placeholder_in_a_block() {
	assert_statement(
		"function f = () => { ... }",
		"function f = () => {\n\t...\n}",
	);
}

#[test]
fn spread_is_not_a_placeholder() {
	assert_statement("{ ...other }", "{\n\t...other\n}");
}

#[test]
fn implicit_enumeration_member() {
	assert_statement(
		"let favoriteColor: Color = .red",
		"let favoriteColor: Color = .red",
	);
}

#[test]
fn when_branches_separated_by_commas() {
	assert_statement(
		"when x is { 1 => \"a\", 2 => \"b\", else => \"c\" }",
		"when x is {\n\t1 => \"a\"\n\t2 => \"b\"\n\telse => \"c\"\n}",
	);
}

#[test]
fn when_branch_with_a_qualified_member() {
	assert_statement(
		"when result is { Result.Ok => 1, Result.Error as { message } => 2 }",
		"when result is {\n\tResult.Ok => 1\n\tResult.Error as { message } => 2\n}",
	);
}

#[test]
fn optional_type_written_twice() {
	assert_statement("let n: Integer?? = none", "let n: Integer?? = none");
}

#[test]
fn optional_type_twice_in_a_call() {
	assert_statement("Array(Integer??)", "Array(Integer??)");
}

#[test]
fn mutable_use_and_reuse() {
	assert_module(
		"mutable use best = players[0]\nreuse best = players[1]",
		"mutable use best = players[0];\nreuse best = players[1];",
	);
}

#[test]
fn try_before_a_destructuring() {
	assert_statement(
		"try let [first, second] = values",
		"try let [first, second] = values",
	);
}
