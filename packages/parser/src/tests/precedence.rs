//! The order of operations of `documentation/operations/arithmetic.md`.

use super::helpers::{assert_has_error, assert_module, assert_statement};
use crate::nodes::Node;
use crate::parsing::parse_single_statement::parse_single_statement;

#[test]
fn not_binds_tighter_than_or() {
	assert_statement("not a or b", "((not a) or b)");
}

#[test]
fn not_binds_tighter_than_and() {
	assert_statement("not a and b", "((not a) and b)");
}

#[test]
fn not_binds_looser_than_equality() {
	assert_statement("not a == b", "(not (a == b))");
}

#[test]
fn is_pattern_with_alternatives() {
	assert_statement("x is A | B", "(x is (A | B))");
}

#[test]
fn is_pattern_with_literal_alternatives() {
	assert_statement("answer is \"y\" | \"yes\"", "(answer is (\"y\" | \"yes\"))");
}

#[test]
fn is_pattern_with_ranges() {
	assert_statement("x is 1 | 2..5", "(x is (1 | (2..5)))");
}

#[test]
fn is_binds_tighter_than_and() {
	assert_statement("x is none and y", "((x is none) and y)");
}

#[test]
fn fallback_binds_looser_than_or() {
	assert_statement("a ?? b or c", "(a ?? (b or c))");
}

#[test]
fn then_binds_looser_than_fallback() {
	assert_statement("a ?? b then c", "((a ?? b) then c)");
}

#[test]
fn power_is_right_associative() {
	assert_statement("2 ** 3 ** 2", "(2 ** (3 ** 2))");
}

#[test]
fn prefix_minus_binds_tighter_than_power() {
	assert_statement("-x ** 2", "((-x) ** 2)");
}

#[test]
fn negative_literal_is_a_constant() {
	assert_statement("-2 ** 2", "(-2 ** 2)");
	let tree = parse_single_statement("-2 ** 2");
	let Node::Power { operands } = tree.at(tree.root) else {
		panic!("expected a power");
	};
	assert!(matches!(tree.at(operands[0]), Node::Integer(value) if value.to_i64() == Some(-2)));
}

#[test]
fn minus_after_a_name_is_a_subtraction() {
	assert_statement("a-1", "(a - 1)");
	assert_statement("a -1", "(a - 1)");
	assert_statement("a - 1", "(a - 1)");
	assert_statement("f(x)-1", "(f(x) - 1)");
}

#[test]
fn minus_after_an_operator_is_a_negative_literal() {
	assert_statement("a * -1", "(a * -1)");
	assert_statement("[1, -2]", "[1, -2]");
}

#[test]
fn ranges_bind_looser_than_arithmetic() {
	assert_statement("0..count + 1", "(0..(count + 1))");
}

#[test]
fn chained_comparison_is_one_node() {
	let tree = parse_single_statement("0 < x <= 100");
	assert_eq!(tree.to_string(), "(0 < x <= 100)");
	let Node::Comparison {
		operands,
		operators,
	} = tree.at(tree.root)
	else {
		panic!("expected a comparison");
	};
	assert_eq!(operands.len(), 3);
	assert_eq!(operators.len(), 2);
}

#[test]
fn parenthesized_comparison_is_not_a_chain() {
	assert_statement("(a < b) < c", "(((a < b)) < c)");
}

#[test]
fn equality_is_not_chainable() {
	assert_has_error("a == b == c");
	assert_has_error("a != b == c");
}

#[test]
fn parenthesized_equality_can_be_compared() {
	assert_statement("(a == b) == c", "(((a == b)) == c)");
}

#[test]
fn try_covers_the_whole_expression() {
	assert_statement("try 10 // divisor", "try (10 // divisor)");
}

#[test]
fn try_covers_an_assignment() {
	assert_statement("try scores[index] = 0", "try scores[index] = 0");
}

#[test]
fn await_covers_the_whole_expression() {
	assert_statement("await a + b", "(await (a + b))");
}

#[test]
fn await_stops_at_catch() {
	assert_statement(
		"await load() catch error => 0",
		"((await load()) catch error => 0)",
	);
}

#[test]
fn catch_handler_extends_to_the_right() {
	assert_statement(
		"parse(read(path)) catch error => fallback + 1",
		"(parse(read(path)) catch error => (fallback + 1))",
	);
}

#[test]
fn try_and_catch_on_the_same_expression_is_an_error() {
	assert_has_error("try a // b catch error => 0");
}

#[test]
fn assignment_binds_looser_than_catch() {
	assert_statement(
		"x = load() catch error => 0",
		"x = (load() catch error => 0)",
	);
}

#[test]
fn compound_assignments_are_not_chainable() {
	assert_has_error("x += y += z");
}

#[test]
fn partial_application_binds_like_a_call() {
	assert_statement("a.b with (c = 1)", "(a.b with (c = 1))");
}

#[test]
fn percentage_binds_tighter_than_prefix_minus() {
	assert_statement("-(a)%", "(-(a)%)");
}

#[test]
fn percentage_requires_a_literal_or_parentheses() {
	assert_has_error("fifty%");
	assert_statement("(fifty)%", "(fifty)%");
}

#[test]
fn call_applies_to_the_whole_access() {
	let tree = parse_single_statement("a.b(c).d");
	assert_eq!(tree.to_string(), "a.b(c).d");
	let Node::Access { operands } = tree.at(tree.root) else {
		panic!("expected an access");
	};
	assert!(matches!(tree.at(operands[0]), Node::FunctionCall { .. }));
}

#[test]
fn index_applies_to_the_whole_access() {
	let tree = parse_single_statement("a.b[0]");
	assert!(matches!(tree.at(tree.root), Node::Index { .. }));
}

#[test]
fn object_of_a_qualified_type() {
	let tree = parse_single_statement("Shape.Circle { radius = 1 }");
	let Node::MemberIndex { target, .. } = tree.at(tree.root) else {
		panic!("expected an object of a named type");
	};
	assert!(matches!(tree.at(*target), Node::Access { .. }));
}

#[test]
fn line_starting_with_a_dot_continues() {
	assert_module("items\n\t.map(f)\n\t.sum()", "items.map(f).sum();");
}

#[test]
fn line_starting_with_minus_is_a_new_statement() {
	assert_module("a\n-1", "a;\n-1;");
}

#[test]
fn line_starting_with_an_operator_continues() {
	assert_module("a\n\t+ b", "(a + b);");
}
