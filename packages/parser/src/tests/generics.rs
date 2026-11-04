//! Generic lists written with `<` and `>`: applications of generic types and functions, and the
//! generic parameters of declarations. The parenthesized spelling of the transition is tested
//! next to the other calls and declarations.

use super::helpers::{assert_error_message, assert_has_error, assert_module, assert_statement};
use crate::nodes::Node;
use crate::parsing::parse::parse;
use crate::parsing::parse_single_statement::parse_single_statement;
use crate::typed_syntax_tree::TypedSyntaxTree;

fn error_count(input: &str) -> usize {
	parse(input)
		.diagnostics
		.iter()
		.filter(|diagnostic| diagnostic.is_error())
		.count()
}

fn root_node(tree: &TypedSyntaxTree) -> &Node {
	tree.at(tree.root)
}

/// The kind of the statement's root node.
fn root_kind(input: &str) -> &'static str {
	let tree = parse_single_statement(input);
	tree.at(tree.root).kind_name()
}

#[test]
fn applications_of_generic_types() {
	assert_statement("Array<Integer>", "Array<Integer>");
	assert_statement("Map<String, User>", "Map<String, User>");
	assert_statement("Map<String, Array<Integer>>", "Map<String, Array<Integer>>");
	assert_statement("A<B<C<D>>>", "A<B<C<D>>>");
	assert_statement("Matrix<Float, 4, 4>", "Matrix<Float, 4, 4>");
	assert_statement(
		"Omit<Entity, { password }>",
		"Omit<Entity, {\n\tpassword\n}>",
	);
	assert_statement("TypeFromSchema<schema>", "TypeFromSchema<schema>");
	assert_statement("Array<Integer>?", "Array<Integer>?");
	assert_statement("Array<Integer | Float>", "Array<(Integer | Float)>");
	assert_statement("Bag<in Self->friends>", "Bag<in (Self -> friends)>");
	assert_statement(
		"Handler<(event: Event) => Void>",
		"Handler<(event: Event) => Void>",
	);
	assert_eq!(root_kind("Array<Integer>"), "GenericApplication");
}

#[test]
fn ranged_integers_and_named_arguments() {
	assert_statement("Integer<min = 0, max = 200>", "Integer<min = 0, max = 200>");
	assert_statement(
		"Integer<min = 0, max = rows - 1>",
		"Integer<min = 0, max = (rows - 1)>",
	);
	assert_statement("Integer<min = 0>", "Integer<min = 0>");
}

#[test]
fn generic_lists_hold_compile_time_values() {
	assert_statement("Offset<-1>", "Offset<-1>");
	assert_statement("Repeat<\"ab\", 3>", "Repeat<\"ab\", 3>");
	assert_statement("Matrix<Float, 2 + 2, 4>", "Matrix<Float, (2 + 2), 4>");
	// a comparison goes in parentheses
	assert_statement("Storage<(size > 255)>", "Storage<((size > 255))>");
	assert_statement("Storage<(size >= 256)>", "Storage<((size >= 256))>");
}

#[test]
fn generic_applications_in_expressions() {
	// a typed literal
	assert_statement(
		"Pair<String, Integer> { left = \"age\", right = 42 }",
		"Pair<String, Integer> {\n\tleft = \"age\"\n\tright = 42\n}",
	);
	assert_eq!(
		root_kind("Pair<String, Integer> { left = \"age\", right = 42 }"),
		"MemberIndex"
	);
	// explicit arguments of a generic function, then the call
	assert_statement("first<Integer>(items)", "first<Integer>(items)");
	assert_statement("sum<Float>([1, 2, 3])", "sum<Float>([1, 2, 3])");
	assert_statement("empty<Integer>()", "empty<Integer>()");
	assert_eq!(root_kind("first<Integer>(items)"), "FunctionCall");
	// a chain
	assert_statement("Set<Float>.from([1, 2, 3])", "Set<Float>.from([1, 2, 3])");
	assert_statement(
		"Map<String, Integer>.from(entries).size",
		"Map<String, Integer>.from(entries).size",
	);
	assert_statement("shapes.Circle<Float>", "shapes.Circle<Float>");
	assert_statement(
		"let numbers = Array<Integer>.from([])",
		"let numbers = Array<Integer>.from([])",
	);
	assert_statement(
		"let count = try parse<Integer>(\"42\")",
		"let count = try parse<Integer>(\"42\")",
	);
	assert_statement(
		"select<User, { name, email }>(user)",
		"select<User, {\n\tname\n\temail\n}>(user)",
	);
}

#[test]
fn generic_applications_in_type_annotations() {
	assert_statement(
		"let names: Array<String> = []",
		"let names: Array<String> = []",
	);
	assert_statement(
		"mutable cache: Map<String, Array<Integer>> = Map<String, Array<Integer>> {}",
		"mutable cache: Map<String, Array<Integer>> = Map<String, Array<Integer>> {}",
	);
	assert_statement(
		"function first = <Item>(items: Array<Item>): Item? => items.get(0)",
		"function first = <Item>(items: Array<Item>): Item? => items.get(0)",
	);
	assert_statement(
		"function sum = <Value: Integer | Float>(values: Array<Value>): Value => 0",
		"function sum = <Value: (Integer | Float)>(values: Array<Value>): Value => 0",
	);
	assert_statement(
		"let f: (items: Array<Integer>) => Map<String, Integer> = g",
		"let f: (items: Array<Integer>) => Map<String, Integer> = g",
	);
}

#[test]
fn comparisons_next_to_generic_applications() {
	assert_statement("a < b", "(a < b)");
	assert_statement("a > b", "(a > b)");
	assert_statement("0 <= index < length", "(0 <= index < length)");
	assert_statement("Array<Integer> > b", "(Array<Integer> > b)");
	assert_statement("count<Integer>() < limit", "(count<Integer>() < limit)");
	assert_statement("x >= y", "(x >= y)");
	assert_statement("[a > b, c < d]", "[(a > b), (c < d)]");
	assert_statement("f(a > b, c >= d)", "f((a > b), (c >= d))");
	assert_statement("a <\n b", "(a < b)");
}

#[test]
fn generic_parameters_of_declarations() {
	assert_statement(
		"type Pair<Left, Right> = { left: Left, right: Right }",
		"type Pair<Left, Right> = {\n\tleft: Left\n\tright: Right\n}",
	);
	assert_statement(
		"type Point<NumberType: Integer | Float> = { x: NumberType }",
		"type Point<NumberType: (Integer | Float)> = {\n\tx: NumberType\n}",
	);
	assert_statement(
		"type Collection<Item, Index = Integer> = { get: (index: Index) => Item? }",
		"type Collection<Item, Index = Integer> = {\n\tget: (index: Index) => Item?\n}",
	);
	assert_statement(
		"type Matrix<Element, rows: Integer, columns: Integer> = { values: Array<Element> }",
		"type Matrix<Element, rows: Integer, columns: Integer> = {\n\tvalues: Array<Element>\n}",
	);
	assert_statement(
		"type Storage<size: Integer> = if size <= 255 { UInt8 } else { UInt64 }",
		"type Storage<size: Integer> = if (size <= 255) {\n\tUInt8\n} else {\n\tUInt64\n}",
	);
	assert_statement(
		"function select = <Entity, keys: Keys<Entity>>(entity: Entity): Pick<Entity, keys> => pick(entity, keys)",
		"function select = <Entity, keys: Keys<Entity>>(entity: Entity): Pick<Entity, keys> => pick(entity, keys)",
	);
	assert_statement(
		"async function load = <Item>(url: String): Item => fetch(url)",
		"async function load = <Item>(url: String): Item => fetch(url)",
	);
	assert!(matches!(
		root_node(&parse_single_statement("type Pair<Left> = {}")),
		Node::Type {
			angle_brackets: true,
			..
		}
	));
}

#[test]
fn generic_parameters_written_over_several_lines() {
	assert_statement(
		"type Pair<Left,\n\tRight\n> = {}",
		"type Pair<Left, Right> = {}",
	);
	assert_statement(
		"type Pair<Left\n\tRight> = {}",
		"type Pair<Left, Right> = {}",
	);
}

#[test]
fn generic_methods_and_exports() {
	assert_statement(
		"type Stack<Item> = {\n\t#items: Array<Item>\n\tfunction map = <Result>(self, transform: (item: Item) => Result) => Stack<Result> { #items = self.#items.map(transform) }\n}",
		"type Stack<Item> = {\n\t#items: Array<Item>\n\tfunction map = <Result>(self, transform: (item: Item) => Result) => Stack<Result> {\n\t\t#items = self.#items.map(transform)\n\t}\n}",
	);
	assert_module(
		"export type<First, Second> = { first: First, second: Second }",
		"export type<First, Second> = {\n\tfirst: First\n\tsecond: Second\n};",
	);
	assert_module(
		"export function = <Item>(items: Array<Item>): Item? => items.get(0)",
		"export function = <Item>(items: Array<Item>): Item? => items.get(0);",
	);
	assert_module(
		"export async function = <Item>(url: String): Item => fetch(url)",
		"export async function = <Item>(url: String): Item => fetch(url);",
	);
}

#[test]
fn transition_parenthesized_generics_still_parse() {
	assert_statement("Array(Integer)", "Array(Integer)");
	assert_statement(
		"type Pair(Left, Right) = { left: Left }",
		"type Pair(Left, Right) = {\n\tleft: Left\n}",
	);
	assert_statement(
		"function first(Item) = (items: Array(Item)): Item? => items.get(0)",
		"function first(Item) = (items: Array(Item)): Item? => items.get(0)",
	);
	assert_eq!(root_kind("Array(Integer)"), "FunctionCall");
	assert!(matches!(
		root_node(&parse_single_statement("type Pair(Left) = {}")),
		Node::Type {
			angle_brackets: false,
			..
		}
	));
	// both spellings in one type
	assert_statement("Map<String, Array(Integer)>", "Map<String, Array(Integer)>");
}

#[test]
fn unclosed_generic_lists_are_reported_where_they_open() {
	let tree = parse("let small = a <b");
	assert_eq!(tree.diagnostics.len(), 1, "{:#?}", tree.diagnostics);
	let diagnostic = &tree.diagnostics[0];
	assert!(diagnostic.message.contains("never closed"));
	assert!(diagnostic.message.contains("put a space after `<`"));
	assert_eq!(diagnostic.span.start, 14);

	assert_error_message("1<2", "put a space after `<` for a comparison");
	assert_error_message("if a <b {\n\tc\n}", "never closed");
	assert_eq!(error_count("f(a <b)"), 1);
	assert_eq!(error_count("f(a <b)\nlet c = d"), 1);
	assert!(
		parse("f(a <b)\nlet c = d")
			.to_string()
			.contains("let c = d")
	);
	assert_eq!(error_count("let x = Array<Integer"), 1);
}

#[test]
fn spaced_generic_lists_are_reported() {
	assert_error_message(
		"let items: Array< Integer> = []",
		"Remove the space after `<`",
	);
	assert_eq!(error_count("let items: Array< Integer> = []"), 1);
	assert_eq!(
		parse("let items: Array< Integer> = []").to_string(),
		"let items: Array<Integer> = [];"
	);
	assert_error_message("type Pair< Left, Right> = {}", "Remove the space after `<`");
	assert_eq!(error_count("type Pair< Left, Right> = {}"), 1);
	assert_error_message("type Box< Item> = { item: Item }", "Remove the space");
	// a `<` followed by a new line is a comparison too: the list starts on the line of its `<`
	assert_error_message("type Pair<\n\tLeft\n\tRight\n> = {}", "Remove the space");
	assert_eq!(error_count("type Pair<\n\tLeft\n\tRight\n> = {}"), 1);
	// a comparison has spaces on both sides, and doesn't compare types
	assert_eq!(error_count("let small = a< b"), 0);
	assert_eq!(error_count("Array< Integer"), 0);
}

#[test]
fn greater_or_equal_after_a_generic_list_is_reported() {
	let tree = parse("let box: Box<Integer>= Box<Integer> { value = 1 }");
	assert_eq!(tree.diagnostics.len(), 1, "{:#?}", tree.diagnostics);
	assert!(tree.diagnostics[0].message.contains("`Box<T> = value`"));
	assert_eq!(
		tree.to_string(),
		"let box: Box<Integer> = Box<Integer> {\n\tvalue = 1\n};"
	);
	assert_error_message("Storage<size >= 256>", "goes in parentheses");
}

#[test]
fn comparisons_in_generic_lists_go_in_parentheses() {
	assert_error_message("Storage<size < 256>", "`Storage<(size < 256)>`");
	assert_error_message("Storage<a <= b>", "`Storage<(size <= 256)>`");
	assert_eq!(error_count("Storage<(size < 256)>"), 0);
}

#[test]
fn malformed_generic_lists() {
	assert_error_message("Array<>", "can't be empty");
	assert_error_message("type Pair<> = {}", "can't be empty");
	assert_error_message("Map<String Integer>", "Expected `,` or `>`");
	assert_error_message("type Grid<size> = {}", "needs a type");
	assert_error_message("type Grid<1> = {}", "Expected a generic parameter");
	assert_error_message("namespace Tools<T> = {}", "A namespace has no parameters");
	assert_has_error("<T>");
	assert_has_error("a + <T>");
	assert_has_error("Map<String,");
	assert_has_error("Map<(String>");
}

#[test]
fn generic_lists_count_in_the_depth() {
	let source = format!("{}T{}", "A<".repeat(400), ">".repeat(400));
	let tree = parse(&source);
	assert!(tree.has_errors());
	assert!(
		tree.diagnostics
			.iter()
			.any(|diagnostic| diagnostic.message.contains("nested too deeply"))
	);
	let source = format!("{}T{}", "A<".repeat(30), ">".repeat(30));
	assert!(!parse(&source).has_errors());
	let source = format!("f{}", "<T>".repeat(5000));
	assert!(parse(&source).has_errors());
}

#[test]
fn generic_functions_carry_their_list_on_the_value() {
	assert_statement(
		"function first = <Item>(items: Array<Item>): Item? => items.get(0)",
		"function first = <Item>(items: Array<Item>): Item? => items.get(0)",
	);
	assert_statement(
		"let identity = <T>(value: T) => value",
		"let identity = <T>(value: T) => value",
	);
	assert_statement(
		"type Identity = <T>(value: T) => T",
		"type Identity = <T>(value: T) => T",
	);
	assert_statement(
		"function f: Identity = <T>(value: T): T => value",
		"function f: Identity = <T>(value: T): T => value",
	);
	assert_statement(
		"function load = async <Item>(url: String): Item => fetch(url)",
		"function load = async <Item>(url: String): Item => fetch(url)",
	);
	assert_statement(
		"items.map(<T>(value: T) => value)",
		"items.map(<T>(value: T) => value)",
	);
	assert_statement(
		"function sum = <Value: Integer | Float, Index = Integer>(values: Array<Value>): Value throws E => 0",
		"function sum = <Value: (Integer | Float), Index = Integer>(values: Array<Value>): Value throws E => 0",
	);
	let tree = parse_single_statement("function first = <Item>(items: Array<Item>) => items");
	let Node::Function {
		parameters,
		expression,
		..
	} = root_node(&tree)
	else {
		panic!("a function");
	};
	assert!(parameters.is_empty());
	let Node::ArrowFunction { signature, .. } = tree.at(*expression) else {
		panic!("a lambda");
	};
	assert_eq!(signature.generics.len(), 1);
	assert_eq!(tree.span(*expression).start, 17);
}

#[test]
fn generic_parameters_after_a_function_name_are_reported() {
	assert_error_message(
		"function first<Item> = (items: Array<Item>) => items",
		"write `function first = <Item>(items: Array<Item>) => …`",
	);
	assert_eq!(
		error_count("function first<Item> = (items: Array<Item>) => items"),
		1
	);
	assert_error_message(
		"export function<Item> = (items: Array<Item>) => items",
		"write `export function = <Item>(items: Array<Item>) => …`",
	);
	// the parenthesized spelling of the transition stays accepted
	assert_eq!(
		error_count("function first(Item) = (items: Array(Item)) => items"),
		0
	);
}

#[test]
fn malformed_generic_functions() {
	assert_error_message(
		"let f = <T>",
		"Expected the parameters of a generic function",
	);
	assert_error_message("let f = <T>(x)", "is followed by a function");
	assert_error_message(
		"let f = <T>x => x",
		"Expected the parameters of a generic function",
	);
	assert_has_error("let f = <>(x) => x");
	assert_has_error("let f = <T(x) => x");
}
