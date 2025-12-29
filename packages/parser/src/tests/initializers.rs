use crate::parse::parse_single_statement;

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
	assert_statement("union Result = Ok | Err", "union Result = (Ok | Err)");
	assert_statement(
		"enum Color = Red | Green | Blue",
		"enum Color = (Red | Green | Blue)",
	);
	assert_statement("fields User = schema", "fields User = schema");
	assert_statement("reactive value = source()", "reactive value = source()");
	assert_statement("derived total = a + b", "derived total = (a + b)");
	assert_statement("namespace Ui = styles()", "namespace Ui = styles()");
}
