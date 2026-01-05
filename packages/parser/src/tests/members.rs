use crate::parsing::parse_single_statement::parse_single_statement;

#[cfg(test)]
fn assert_member_declaration(input: &'static str, expected: &str) {
	let tree = parse_single_statement(input);
	println!("{:#?}", &tree);
	assert_eq!(tree.to_string(), expected);
}

#[test]
fn single_member_declarations() {
	assert_member_declaration("{ foo }", "{\n\tfoo\n}");
	assert_member_declaration("{ foo: Int }", "{\n\tfoo: Int\n}");
	assert_member_declaration("{ foo = 1 }", "{\n\tfoo = 1\n}");
	assert_member_declaration("{ foo: Int = 1 }", "{\n\tfoo: Int = 1\n}");
}

#[test]
fn multiple_member_declarations() {
	assert_member_declaration(
		"{\nfoo\nbar: Int\nbaz = 1\nqux: Int = 2\n}",
		"{\n\tfoo\n\tbar: Int\n\tbaz = 1\n\tqux: Int = 2\n}",
	);
}

#[test]
fn empty_members() {
	assert_member_declaration("{}", "{}");
	assert_member_declaration("{ }", "{}");
	assert_member_declaration("{\t}", "{}");
	assert_member_declaration("{\n}", "{}");
	assert_member_declaration("{\n\t\n}", "{}");
	assert_member_declaration("@{}", "@{}");
}

#[test]
fn comma_separated_members() {
	assert_member_declaration("{ foo, bar }", "{\n\tfoo\n\tbar\n}");
	assert_member_declaration(
		"{ foo: Int, bar = 1, baz: Int = 2 }",
		"{\n\tfoo: Int\n\tbar = 1\n\tbaz: Int = 2\n}",
	);
}

#[test]
fn mixed_separator_members() {
	assert_member_declaration(
		"{\nfoo, bar\nbaz\nqux: Int = 2, quux\n}",
		"{\n\tfoo\n\tbar\n\tbaz\n\tqux: Int = 2\n\tquux\n}",
	);
	assert_member_declaration(
		"@{\nfoo, bar\nbaz\nqux: Int = 2, quux\n}",
		"@{\n\tfoo\n\tbar\n\tbaz\n\tqux: Int = 2\n\tquux\n}",
	);
}

#[test]
fn nested_members() {
	assert_member_declaration(
		"{\nouter = {\ninner\ninner_typed: Int\n}\n}",
		"{\n\touter = {\n\t\tinner\n\t\tinner_typed: Int\n\t}\n}",
	);
}
