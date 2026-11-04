use super::helpers::{assert_has_error, assert_statement};

#[test]
fn single_member_declarations() {
	assert_statement("{ foo }", "{\n\tfoo\n}");
	assert_statement("{ foo: Int }", "{\n\tfoo: Int\n}");
	assert_statement("{ foo = 1 }", "{\n\tfoo = 1\n}");
	assert_statement("{ foo: Int = 1 }", "{\n\tfoo: Int = 1\n}");
}

#[test]
fn multiple_member_declarations() {
	assert_statement(
		"{\nfoo\nbar: Int\nbaz = 1\nqux: Int = 2\n}",
		"{\n\tfoo\n\tbar: Int\n\tbaz = 1\n\tqux: Int = 2\n}",
	);
}

#[test]
fn empty_members() {
	assert_statement("{}", "{}");
	assert_statement("{ }", "{}");
	assert_statement("{\t}", "{}");
	assert_statement("{\n}", "{}");
	assert_statement("{\n\t\n}", "{}");
}

#[test]
fn comma_separated_members() {
	assert_statement("{ foo, bar }", "{\n\tfoo\n\tbar\n}");
	assert_statement(
		"{ foo: Int, bar = 1, baz: Int = 2 }",
		"{\n\tfoo: Int\n\tbar = 1\n\tbaz: Int = 2\n}",
	);
}

#[test]
fn mixed_separator_members() {
	assert_statement(
		"{\nfoo, bar\nbaz\nqux: Int = 2, quux\n}",
		"{\n\tfoo\n\tbar\n\tbaz\n\tqux: Int = 2\n\tquux\n}",
	);
}

#[test]
fn private_members() {
	assert_statement(
		"{ #bag: Bag(Node), #secret = \"private\" }",
		"{\n\t#bag: Bag(Node)\n\t#secret = \"private\"\n}",
	);
}

#[test]
fn nested_members() {
	assert_statement(
		"{\nouter = {\ninner\ninner_typed: Int\n}\n}",
		"{\n\touter = {\n\t\tinner\n\t\tinner_typed: Int\n\t}\n}",
	);
}

#[test]
fn member_methods() {
	assert_statement(
		"{ function double = (self): Integer => self.value * 2 }",
		"{\n\tfunction double = (self): Integer => (self.value * 2)\n}",
	);
	assert_statement(
		"{\nvalue: Integer\nfunction increment = (self) mutates self => { self.value += 1 }\n}",
		"{\n\tvalue: Integer\n\tfunction increment = (self) mutates self => {\n\t\tself.value += 1\n\t}\n}",
	);
	assert_statement(
		"{ async function load = (self): Data => fetch(self.url) }",
		"{\n\tasync function load = (self): Data => fetch(self.url)\n}",
	);
}

#[test]
fn member_function_pointers() {
	assert_statement(
		"{ format: (value: Integer) => String }",
		"{\n\tformat: (value: Integer) => String\n}",
	);
	assert_statement(
		"{ format = (value) => value }",
		"{\n\tformat = (value) => value\n}",
	);
}

#[test]
fn member_methods_require_function_keyword() {
	assert_has_error("{ foo(a) => a }");
}

#[test]
fn readonly_members() {
	assert_statement(
		"{ readonly balance: Integer = 0 }",
		"{\n\treadonly balance: Integer = 0\n}",
	);
}

#[test]
fn member_spread() {
	assert_statement("{ ...values }", "{\n\t...values\n}");
	assert_statement("{ ...values, ...other }", "{\n\t...values\n\t...other\n}");
}

#[test]
fn member_disallows_literal() {
	assert_has_error("{ 1 }");
}

#[test]
fn member_disallows_string_literal() {
	assert_has_error("{ \"hello\" }");
}

#[test]
fn member_disallows_binary_expression() {
	assert_has_error("{ foo + bar }");
}

#[test]
fn member_disallows_suffix_expression() {
	assert_has_error("{ value? }");
}

#[test]
fn generic_types() {
	assert_statement(
		"type Point<NumberType> = { x: NumberType, y: NumberType }",
		"type Point<NumberType> = {\n\tx: NumberType\n\ty: NumberType\n}",
	);
	assert_statement(
		"type Pair<Left, Right> = { left: Left, right: Right }",
		"type Pair<Left, Right> = {\n\tleft: Left\n\tright: Right\n}",
	);
}

#[test]
fn typed_object_construction() {
	assert_statement(
		"let point = Point<Integer> { x = 12, y = 13 }",
		"let point = Point<Integer> {\n\tx = 12\n\ty = 13\n}",
	);
	assert_statement(
		"let point: Point<Integer> = { x = 12, y = 13 }",
		"let point: Point<Integer> = {\n\tx = 12\n\ty = 13\n}",
	);
}

#[test]
fn collection_creation() {
	assert_statement(
		"let numbers: Set<Integer> = [1, 2, 3]",
		"let numbers: Set<Integer> = [1, 2, 3]",
	);
	assert_statement(
		"let count = Set.from([1, 2, 2]).size()",
		"let count = Set.from([1, 2, 2]).size()",
	);
}

#[test]
fn collection_elements_are_not_members() {
	assert_has_error("let numbers = Set(Integer) { 1, 2, 3 }");
}

#[test]
fn type_parameters_are_capitalized() {
	assert_has_error("type Box(value) = { content: value }");
}

#[test]
fn nested_declarations() {
	assert_statement(
		"type LinkedList<Data> = {\ntype Node = { data: Data }\n#bag: Bag<Node>\n}",
		"type LinkedList<Data> = {\n\ttype Node = {\n\t\tdata: Data\n\t}\n\t#bag: Bag<Node>\n}",
	);
	assert_statement(
		"namespace Math = {\nnamespace constants = { pi = 3.14 }\n}",
		"namespace Math = {\n\tnamespace constants = {\n\t\tpi = 3.14\n\t}\n}",
	);
}

#[test]
fn generic_parameters() {
	assert_statement(
		"type Point<NumberType: Integer | Float> = { x: NumberType }",
		"type Point<NumberType: (Integer | Float)> = {\n\tx: NumberType\n}",
	);
	assert_statement(
		"type Matrix<Element, rows: Integer, columns: Integer> = { cells: Array<Element> }",
		"type Matrix<Element, rows: Integer, columns: Integer> = {\n\tcells: Array<Element>\n}",
	);
	assert_statement(
		"type Collection<Item, Index = Integer> = { items: Array<Item> }",
		"type Collection<Item, Index = Integer> = {\n\titems: Array<Item>\n}",
	);
	assert_statement(
		"function first = <Item>(items: Array<Item>): Item? => items[0]",
		"function first = <Item>(items: Array<Item>): Item? => items[0]",
	);
	assert_statement(
		"function sum = <Value: Integer | Float>(values: Array<Value>): Value => values.length",
		"function sum = <Value: (Integer | Float)>(values: Array<Value>): Value => values.length",
	);
	assert_statement(
		"function get = <Object, Key: Keys<Object>>(object: Object, key: Key) => object[key]",
		"function get = <Object, Key: Keys<Object>>(object: Object, key: Key) => object[key]",
	);
	assert_statement(
		"let total = sum(Float)([1, 2, 3])",
		"let total = sum(Float)([1, 2, 3])",
	);
}

#[test]
fn value_parameters_need_a_type() {
	assert_has_error("type Matrix(Element, rows) = { cells: Array(Element) }");
}

#[test]
fn nested_key_sets() {
	assert_statement(
		"let picked = Pick(User, { name, address { city } })",
		"let picked = Pick(User, {\n\tname\n\taddress {\n\t\tcity\n\t}\n})",
	);
}

#[test]
fn namespace_declarations() {
	assert_statement(
		"export namespace = {\nlet version = 1\nuse User = User\nstatic answer = 42\ntest \"adds\" { assert 1 + 1 == 2 }\n}",
		"export namespace = {\n\tlet version = 1\n\tuse User = User\n\tstatic answer = 42\n\ttest \"adds\" {\n\t\tassert ((1 + 1) == 2)\n\t}\n}",
	);
}
