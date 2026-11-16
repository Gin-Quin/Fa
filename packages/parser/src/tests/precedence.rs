//! The order of operations of `documentation/operations/arithmetic.md`.

use super::helpers::{assert_has_error, assert_module, assert_statement};
use crate::nodes::Node;
use crate::parsing::parse_single_statement::parse_single_statement;

#[test]
fn explicit_copy_binds_after_access_and_before_arithmetic() {
	assert_statement("copy values[0]", "(copy values[0])");
	assert_statement("copy source.items + other", "((copy source.items) + other)");
	assert_statement("copy (left ?? right)", "(copy ((left ?? right)))");
	assert_statement("copy copy value", "(copy (copy value))");
}

#[test]
fn reserved_copy_accepts_every_operand_spelling() {
	assert_statement("copy(value)", "(copy (value))");
	assert_statement("copy (value)", "(copy (value))");
	assert_statement("copy[0]", "(copy [0])");
	assert_statement("copy [1, 2]", "(copy [1, 2])");
	assert_statement("copy -value", "(copy (-value))");
	assert_statement("copy -2", "(copy -2)");
	assert_statement("copy - 2", "(copy (-2))");
	assert_statement("copy\nvalues[0]", "(copy values[0])");
	assert_statement("copy source.items()[0]", "(copy source.items()[0])");
}

#[test]
fn copy_member_names_follow_await() {
	for keyword in ["await", "copy"] {
		let member = format!("source.{keyword}(value)");
		assert_statement(&member, &member);
		let optional = format!("source?.{keyword}");
		assert_statement(&optional, &optional);
		assert_has_error(&format!("let source = {{ {keyword} = 1 }}"));
	}
}

#[test]
fn reserved_copy_cannot_be_a_declared_name() {
	for source in [
		"let copy = 1",
		"mutable copy = 1",
		"use copy = source",
		"static copy = 1",
		"function copy = () => 1",
		"type copy = Integer",
		"namespace copy = {}",
		"reactive copy = 1",
		"derived copy = 1",
		"function apply = (copy: Integer) => 1",
		"function apply = (copy) => 1",
		"function apply = (...copy: Integer) => 1",
		"let [copy] = [1]",
		"let { field as copy } = source",
		"for copy in values {}",
		"for item, copy in values {}",
		"read() catch copy => 0",
		"reuse copy = source",
		"type Record = { copy: Integer }",
		"function generic = <copy>(item: Integer) => item",
	] {
		let tree = crate::parse(source);
		assert!(
			tree.diagnostics.iter().any(|diagnostic| {
				diagnostic.code == Some("E-LEX-3")
					&& diagnostic.message.contains("`copy items`")
					&& diagnostic.message.contains("choose another name")
			}),
			"{source}: {:?}",
			tree.diagnostics
		);
	}
}

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

// Each level of the precedence table of `documentation/reference/language-reference.md`
// against the next one, from the tightest to the loosest.

#[test]
fn level_1_access_binds_tighter_than_prefix_minus() {
	assert_statement("-a.b", "(-a.b)");
	assert_statement("-a(b)", "(-a(b))");
	assert_statement("-a[0]", "(-a[0])");
	assert_statement("-a with (b = 1)", "(-(a with (b = 1)))");
}

#[test]
fn level_1_access_chains_from_left_to_right() {
	assert_statement("a.b.c(d)[e].f", "a.b.c(d)[e].f");
	assert_statement("a with (b = 1).c", "(a with (b = 1)).c");
	assert_statement("x?.y.z?.w", "x?.y.z?.w");
}

#[test]
fn level_2_percentage_applies_to_a_literal_before_its_access() {
	assert_statement("-5%", "-5%");
	assert_statement("(a)%.value", "(a)%.value");
}

#[test]
fn level_3_prefix_minus_binds_tighter_than_power_on_both_sides() {
	assert_statement("a ** -b", "(a ** (-b))");
	assert_statement("-a ** -b", "((-a) ** (-b))");
}

#[test]
fn level_4_power_binds_tighter_than_multiplication() {
	assert_statement("a * b ** c", "(a * (b ** c))");
	assert_statement("a ** b * c", "((a ** b) * c)");
}

#[test]
fn level_5_multiplicative_operators_are_left_associative() {
	assert_statement("a // b %% c * d / e", "((((a // b) %% c) * d) / e)");
	assert_statement("a - b - c", "(a - b - c)");
}

#[test]
fn level_7_ranges_bind_looser_than_both_sides() {
	assert_statement("a - 1..b * 2", "((a - 1)..(b * 2))");
}

#[test]
fn level_8_comparisons_bind_looser_than_ranges() {
	assert_statement("a < b..c", "(a < (b..c))");
	assert_statement("a..b < c", "((a..b) < c)");
	assert_statement("a + 1 in b", "((a + 1) in b)");
}

#[test]
fn level_9_equality_binds_looser_than_comparisons() {
	assert_statement("a < b == c < d", "((a < b) == (c < d))");
	assert_statement("a in b == c", "((a in b) == c)");
	assert_statement("a is b < c", "(a is (b < c))");
}

#[test]
fn level_9_is_forms() {
	assert_statement("x is not in list", "(x is not in list)");
	assert_statement("x is in list and y", "((x is in list) and y)");
	assert_statement("x is not none", "(x is not none)");
}

#[test]
fn level_10_not_is_a_prefix_of_itself() {
	assert_statement("not not a", "(not (not a))");
	assert_statement("not -a", "(not (-a))");
}

#[test]
fn level_11_and_binds_tighter_than_or() {
	assert_statement("a and b and c or d", "((a and b and c) or d)");
	assert_statement("a or b and c", "(a or (b and c))");
}

#[test]
fn level_13_fallback_chains_flat() {
	assert_statement("a ?? b ?? c", "(a ?? b ?? c)");
	assert_statement("f then g ?? h", "(f then (g ?? h))");
}

#[test]
fn level_15_try_await_start_cover_composition() {
	assert_statement("try a then b", "try (a then b)");
	assert_statement("await a ?? b", "(await (a ?? b))");
	assert_statement("start fetch(url) then b", "(start (fetch(url) then b))");
}

#[test]
fn level_16_catch_covers_its_left_operand() {
	assert_statement("a + b catch e => 0", "((a + b) catch e => 0)");
	assert_statement("f(a catch e => 0, b)", "f((a catch e => 0), b)");
}

#[test]
fn level_17_return_throw_break_cover_everything_on_their_right() {
	assert_statement("return a catch e => 0", "return (a catch e => 0)");
	assert_statement("throw a or b", "throw (a or b)");
	assert_statement("break a ?? b", "break (a ?? b)");
}

#[test]
fn level_18_assignments_bind_loosest() {
	assert_statement("x = a ?? b", "x = (a ?? b)");
	assert_statement("f = a then b", "f = (a then b)");
	assert_statement("a: Integer = 1 + 2", "a: Integer = (1 + 2)");
	assert_statement("a.b += 1", "a.b += 1");
	assert_statement("items[0] = 1", "items[0] = 1");
}

#[test]
fn relation_binds_tighter_than_power_and_looser_than_access() {
	assert_statement("a->b.c", "(a -> b.c)");
	assert_statement("a.b->c", "(a.b -> c)");
	assert_statement("a ** b->c", "(a ** (b -> c))");
}

#[test]
fn ranges_without_bounds() {
	assert_statement("..", "(..)");
	assert_statement("..b", "(..b)");
	assert_statement("..=b", "(..=b)");
	assert_statement("a..", "(a..)");
	assert_has_error("a..=");
	assert_has_error("x = ..=");
}

#[test]
fn optional_type_suffixes() {
	assert_statement("a.b?", "a.b?");
	assert_statement("f(x)??", "f(x)??");
	let tree = parse_single_statement("Integer??");
	let Node::Optional { value } = tree.at(tree.root) else {
		panic!("expected an optional");
	};
	assert!(matches!(tree.at(*value), Node::Optional { .. }));
}
