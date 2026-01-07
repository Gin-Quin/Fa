use crate::parsing::parse_single_statement::parse_single_statement;

#[cfg(test)]
fn assert_statement(input: &'static str, expected: &str) {
	let tree = parse_single_statement(input);
	println!("{:#?}", &tree);
	assert_eq!(tree.to_string(), expected);
}

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
fn export_union_declaration() {
	assert_statement("export union = Foo", "export union = Foo");
}

#[test]
fn export_fields_declaration() {
	assert_statement("export fields = Foo", "export fields = Foo");
}

#[test]
fn export_enum_declaration() {
	assert_statement("export enum = Foo", "export enum = Foo");
}
