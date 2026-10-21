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
}

#[test]
fn private_members() {
	assert_member_declaration(
		"{ #bag: Bag(Node), #secret = \"private\" }",
		"{\n\t#bag: Bag(Node)\n\t#secret = \"private\"\n}",
	);
}

#[test]
fn nested_members() {
	assert_member_declaration(
		"{\nouter = {\ninner\ninner_typed: Int\n}\n}",
		"{\n\touter = {\n\t\tinner\n\t\tinner_typed: Int\n\t}\n}",
	);
}

#[test]
fn member_methods() {
	assert_member_declaration(
		"{ function double = (self): Integer => self.value * 2 }",
		"{\n\tfunction double = (self): Integer => (self.value * 2)\n}",
	);
	assert_member_declaration(
		"{\nvalue: Integer\nfunction increment = (mutable self) => { self.value += 1 }\n}",
		"{\n\tvalue: Integer\n\tfunction increment = (mutable self) => {\n\t\tself.value += 1\n\t}\n}",
	);
	assert_member_declaration(
		"{ async function load = (self): Data => fetch(self.url) }",
		"{\n\tasync function load = (self): Data => fetch(self.url)\n}",
	);
}

#[test]
fn member_function_pointers() {
	assert_member_declaration(
		"{ format: (value: Integer) => String }",
		"{\n\tformat: (value: Integer) => String\n}",
	);
	assert_member_declaration(
		"{ format = (value) => value }",
		"{\n\tformat = (value) => value\n}",
	);
}

#[test]
#[should_panic]
fn member_methods_require_function_keyword() {
	parse_single_statement("{ foo(a) => a }");
}

#[test]
fn readonly_members() {
	assert_member_declaration(
		"{ readonly balance: Integer = 0 }",
		"{\n\treadonly balance: Integer = 0\n}",
	);
}

#[test]
fn member_spread() {
	assert_member_declaration("{ ...values }", "{\n\t...values\n}");
	assert_member_declaration("{ ...values, ...other }", "{\n\t...values\n\t...other\n}");
}

#[test]
#[should_panic]
fn member_disallows_literal() {
	parse_single_statement("{ 1 }");
}

#[test]
#[should_panic]
fn member_disallows_string_literal() {
	parse_single_statement("{ \"hello\" }");
}

#[test]
#[should_panic]
fn member_disallows_binary_expression() {
	parse_single_statement("{ foo + bar }");
}

#[test]
#[should_panic]
fn member_disallows_suffix_expression() {
	parse_single_statement("{ value? }");
}

#[test]
fn generic_types() {
	assert_member_declaration(
		"type Point(NumberType) = { x: NumberType, y: NumberType }",
		"type Point(NumberType) = {\n\tx: NumberType\n\ty: NumberType\n}",
	);
	assert_member_declaration(
		"type Pair(Left, Right) = { left: Left, right: Right }",
		"type Pair(Left, Right) = {\n\tleft: Left\n\tright: Right\n}",
	);
}

#[test]
fn typed_object_construction() {
	assert_member_declaration(
		"let point = Point(Integer) { x = 12, y = 13 }",
		"let point = Point(Integer) {\n\tx = 12\n\ty = 13\n}",
	);
	assert_member_declaration(
		"let point: Point(Integer) = { x = 12, y = 13 }",
		"let point: Point(Integer) = {\n\tx = 12\n\ty = 13\n}",
	);
}

#[test]
fn collection_creation() {
	assert_member_declaration(
		"let numbers: Set(Integer) = [1, 2, 3]",
		"let numbers: Set(Integer) = [1, 2, 3]",
	);
	assert_member_declaration(
		"let count = Set.from([1, 2, 2]).size()",
		"let count = Set.from([1, 2, 2]).size()",
	);
}

#[test]
#[should_panic]
fn collection_elements_are_not_members() {
	parse_single_statement("let numbers = Set(Integer) { 1, 2, 3 }");
}

#[test]
#[should_panic]
fn type_parameters_are_capitalized() {
	parse_single_statement("type Box(value) = { content: value }");
}

#[test]
fn nested_declarations() {
	assert_member_declaration(
		"type LinkedList(Data) = {\ntype Node = { data: Data }\n#bag: Bag(Node)\n}",
		"type LinkedList(Data) = {\n\ttype Node = {\n\t\tdata: Data\n\t}\n\t#bag: Bag(Node)\n}",
	);
	assert_member_declaration(
		"namespace Math = {\nnamespace constants = { pi = 3.14 }\n}",
		"namespace Math = {\n\tnamespace constants = {\n\t\tpi = 3.14\n\t}\n}",
	);
}
