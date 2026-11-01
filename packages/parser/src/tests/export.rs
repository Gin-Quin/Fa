use super::helpers::assert_statement;

#[test]
fn export_value_declaration() {
	assert_statement("export = value", "export = value");
	assert_statement("export: Int = 42", "export: Int = 42");
}

#[test]
fn export_function_declaration() {
	assert_statement("export function = add", "export function = add");
	assert_statement(
		"export function: Function = add",
		"export function: Function = add",
	);
}

#[test]
fn export_type_declaration() {
	assert_statement("export type = Foo", "export type = Foo");
}

#[test]
fn export_namespace_declaration() {
	assert_statement("export namespace = Foo", "export namespace = Foo");
}

#[test]
fn export_type_constructor() {
	assert_statement(
		"export type = Enum { red, green }",
		"export type = Enum {\n\tred\n\tgreen\n}",
	);
}
