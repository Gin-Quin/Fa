use super::helpers::assert_statement;

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
fn start_is_a_name_after_a_declaration_keyword() {
	assert_statement("reactive start = 0", "reactive start = 0");
	assert_statement("derived start = count * 2", "derived start = (count * 2)");
	assert_statement("namespace start = {}", "namespace start = {}");
	assert_statement("let start = 1", "let start = 1");
	assert_statement("let task = start load()", "let task = (start load())");
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
