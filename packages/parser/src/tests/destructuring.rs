use crate::parsing::parse_single_statement::parse_single_statement;

#[cfg(test)]
fn assert_statement(input: &'static str, expected: &str) {
	let tree = parse_single_statement(input);
	println!("{:#?}", tree);
	assert_eq!(tree.to_string(), expected);
}

#[test]
fn object_destructuring() {
	assert_statement("let { a, b } = object", "let { a, b } = object");
	assert_statement("mutable { a, b } = object", "mutable { a, b } = object");
	assert_statement("use { a, b } = object", "use { a, b } = object");
	assert_statement("let { x, y } = getPoint()", "let { x, y } = getPoint()");
}

#[test]
fn renamed_fields() {
	assert_statement(
		"let { a as first, b as second } = object",
		"let { a as first, b as second } = object",
	);
	assert_statement(
		"use { Button as UIButton } = ui.components",
		"use { Button as UIButton } = ui.components",
	);
}

#[test]
fn default_values() {
	assert_statement("let { a, d = 4 } = object", "let { a, d = 4 } = object");
	assert_statement(
		"let { a as first = 1 + 2 } = object",
		"let { a as first = (1 + 2) } = object",
	);
}

#[test]
fn nested_destructuring() {
	assert_statement(
		"let { nested { c } } = object",
		"let { nested { c } } = object",
	);
	assert_statement(
		"use {\n\tcows\n\tmachines { tractor, harvester }\n} = farm",
		"use { cows, machines { tractor, harvester } } = farm",
	);
}

#[test]
fn array_and_tuple_destructuring() {
	assert_statement("let [first, second] = array", "let [first, second] = array");
	assert_statement("let (number, name) = tuple", "let (number, name) = tuple");
	assert_statement(
		"let [{ x }, [y, z]] = values",
		"let [{ x }, [y, z]] = values",
	);
}

#[test]
#[should_panic]
fn destructuring_has_no_type_annotation() {
	parse_single_statement("let { x, y }: Point = getPoint()");
}

#[test]
#[should_panic]
fn destructuring_requires_a_value() {
	parse_single_statement("let { x, y }");
}

#[test]
fn destructured_parameters() {
	assert_statement(
		"function draw = (prefix: String, { x, y }: Point) => x",
		"function draw = (prefix: String, { x, y }: Point) => x",
	);
	assert_statement(
		"function draw = ({ x as left, y as top }: Point): Integer => left",
		"function draw = ({ x as left, y as top }: Point): Integer => left",
	);
	assert_statement(
		"function sum = ([first, second]: Array(Integer)) => first + second",
		"function sum = ([first, second]: Array(Integer)) => (first + second)",
	);
}

#[test]
fn destructured_and_anonymous_structure_parameters() {
	assert_statement(
		"function greet = ({ name }: User, { greeting: String = \"Hello\" }) => name",
		"function greet = ({ name }: User, {\n\tgreeting: String = \"Hello\"\n}) => name",
	);
}
