use crate::parsing::parse_single_statement::parse_single_statement;

#[cfg(test)]
// Helper function to check if statement parsing results match expected output
fn assert_statement(input: &'static str, expected: &str) {
	let tree = parse_single_statement(input);
	println!("{:#?}", &tree);
	assert_eq!(tree.to_string(), expected);
}

#[test]
fn keyword_initializers() {
	assert_statement("mutable count = 1", "mutable count = 1");
	assert_statement("type User = Id", "type User = Id");
	assert_statement("static version = 1", "static version = 1");
	assert_statement("reactive value = source()", "reactive value = source()");
	assert_statement("derived total = a + b", "derived total = (a + b)");
	assert_statement("namespace Ui = styles()", "namespace Ui = styles()");
}

#[test]
fn type_constructors() {
	assert_statement(
		"type Color = Enum { red, green, blue }",
		"type Color = Enum {\n\tred\n\tgreen\n\tblue\n}",
	);
	assert_statement(
		"type Status = Enum { pending = 0, archived = 9 }",
		"type Status = Enum {\n\tpending = 0\n\tarchived = 9\n}",
	);
	assert_statement(
		"type Shape = Union { Circle = { radius: Float }, Point }",
		"type Shape = Union {\n\tCircle = {\n\t\tradius: Float\n\t}\n\tPoint\n}",
	);
	assert_statement(
		"type ApiError = Errors { NotFound = { id: String }, Timeout }",
		"type ApiError = Errors {\n\tNotFound = {\n\t\tid: String\n\t}\n\tTimeout\n}",
	);
	assert_statement(
		"type Permissions = Fields { read, write }",
		"type Permissions = Fields {\n\tread\n\twrite\n}",
	);
}
