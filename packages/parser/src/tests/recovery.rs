//! Diagnostics and error recovery: the parser never panics, always returns a tree, and keeps
//! parsing after a mistake.

use super::helpers::{assert_error_message, assert_has_error, assert_statement};
use crate::nodes::Node;
use crate::parsing::parse::parse;
use crate::tokenize::tokenize_with_diagnostics;

fn error_count(input: &str) -> usize {
	parse(input)
		.diagnostics
		.iter()
		.filter(|diagnostic| diagnostic.is_error())
		.count()
}

#[test]
fn statement_after_an_error_is_parsed() {
	let tree = parse("let = 1\nlet b = 2");
	assert!(tree.has_errors());
	assert!(tree.to_string().contains("let b = 2"));
}

#[test]
fn one_mistake_gives_one_diagnostic() {
	assert_eq!(error_count("let x = (1 + \nlet y = 2"), 1);
	assert_eq!(error_count("f(a, b"), 1);
	assert_eq!(error_count("let list = [1, 2"), 1);
}

#[test]
fn unclosed_block_is_reported_at_its_brace() {
	let tree = parse("if a {\n\tb");
	let diagnostic = &tree.diagnostics[0];
	assert!(diagnostic.message.contains("never closed"));
	assert_eq!(diagnostic.span.start, 5);
}

#[test]
fn missing_member_name_gives_an_access() {
	let tree = parse("user.");
	assert!(tree.has_errors());
	let Node::Module { statements, .. } = tree.at(tree.root) else {
		panic!("expected a module");
	};
	let Node::Access { operands } = tree.at(statements[0]) else {
		panic!("expected an access, for completions");
	};
	assert!(matches!(tree.at(operands[1]), Node::Invalid));
}

#[test]
fn integer_dot_suggests_a_float() {
	assert_error_message("let x = 5.", "write `5.0`");
}

#[test]
fn tuple_dot_index_suggests_brackets() {
	assert_error_message("pair.0", "write `pair[0]`");
}

#[test]
fn leading_dot_number_suggests_a_zero() {
	assert_error_message("let x = .5", "write `0.5`");
}

#[test]
fn two_expressions_on_one_line_are_reported() {
	assert_error_message("a b", "a statement ends at the end of its line");
}

#[test]
fn else_on_its_own_line_is_reported_and_attached() {
	let tree = parse("if a {\n\tb\n}\nelse {\n\tc\n}");
	assert!(tree.has_errors());
	assert_eq!(tree.to_string(), "if a {\n\tb\n} else {\n\tc\n};");
	assert_error_message(
		"if a {\n\tb\n}\nelse {\n\tc\n}",
		"must be on the line of the `}`",
	);
}

#[test]
fn let_with_a_lambda_suggests_function() {
	assert_error_message(
		"let addOne = (x: Integer) => x + 1",
		"write `function addOne = (x: Integer) => x + 1`",
	);
}

#[test]
fn mutable_type_is_reported_and_parsed() {
	let tree = parse("mutable type MyType = { foo: String }");
	assert!(tree.has_errors());
	assert!(tree.to_string().starts_with("type MyType"));
}

#[test]
fn integer_beyond_128_bits_suggests_a_big_integer() {
	assert_error_message(
		"let x = 999_999_999_999_999_999_999_999_999_999_999_999_999_999",
		"for a `BigInt`",
	);
}

#[test]
fn integer_beyond_64_bits_is_kept_for_the_checker() {
	assert_statement(
		"let x = 18446744073709551615",
		"let x = 18446744073709551615",
	);
	assert_statement(
		"let huge = 10_000_000_000_000_000_000",
		"let huge = 10000000000000000000",
	);
}

#[test]
fn largest_unsigned_128_bit_literal_is_kept() {
	assert_statement(
		"let x = 340282366920938463463374607431768211455",
		"let x = 340282366920938463463374607431768211455",
	);
}

#[test]
fn misplaced_underscores_are_reported() {
	assert_error_message("1_", "an underscore must be between two digits");
	assert_error_message("1__0", "an underscore must be between two digits");
	assert_error_message("0x_FF", "an underscore must be between two digits");
	assert_error_message("1_.5", "an underscore must be between two digits");
}

#[test]
fn uppercase_number_prefixes_are_reported() {
	assert_has_error("0XFF");
	assert_has_error("1E6");
	assert_has_error("0b102");
	assert_has_error("12.5n");
}

#[test]
fn number_without_digits_is_reported() {
	assert_has_error("0x");
}

#[test]
fn unknown_escape_is_reported() {
	assert_error_message("\"a\\qb\"", "Unknown escape sequence `\\q`");
}

#[test]
fn non_ascii_hexadecimal_escape_is_reported() {
	assert_has_error("\"\\x80\"");
	assert_has_error("\"\\xé\"");
}

#[test]
fn surrogate_unicode_escape_is_reported() {
	assert_has_error("\"\\u{D800}\"");
	assert_has_error("\"\\u{110000}\"");
	assert_statement("\"\\u{1F600}\"", "\"😀\"");
}

#[test]
fn single_closing_brace_in_a_string_is_reported() {
	assert_error_message("\"a } b\"", "must be doubled");
}

#[test]
fn empty_interpolation_is_reported() {
	assert_error_message("\"a {} b\"", "Empty `{}`");
}

#[test]
fn line_feeds_of_a_string_are_normalized() {
	let tree = parse("\"a\r\nb\"");
	let Node::Module { statements, .. } = tree.at(tree.root) else {
		panic!("expected a module");
	};
	assert_eq!(
		tree.at(statements[0]),
		&Node::StringLiteral("a\nb".to_string())
	);
}

#[test]
fn carriage_returns_are_spaces() {
	assert_eq!(error_count("let a = 1\r\nlet b = 2\r\n"), 0);
}

#[test]
fn unclosed_string_is_reported() {
	assert_error_message("let s = \"abc", "never closed");
}

#[test]
fn unclosed_block_comment_is_reported() {
	assert_error_message("--- abc", "never closed");
}

#[test]
fn exclamation_mark_suggests_not() {
	assert_error_message("if !ready { }", "use `not`");
}

#[test]
fn semicolon_is_reported() {
	assert_error_message("a; b", "new lines");
}

#[test]
fn non_ascii_name_is_reported() {
	let (_, diagnostics) = tokenize_with_diagnostics("let café = 1");
	assert_eq!(diagnostics.len(), 1);
}

#[test]
fn deep_nesting_is_reported_without_overflowing_the_stack() {
	for (open, close) in [
		("(", ")"),
		("[", "]"),
		("{ a = ", " }"),
		("- ", ""),
		("not ", ""),
	] {
		let source = format!("{}x{}", open.repeat(5000), close.repeat(5000));
		let tree = parse(&source);
		assert!(tree.has_errors(), "{open}");
	}
	let source = format!("x{}", " ** x".repeat(5000));
	assert!(parse(&source).has_errors());
	let source = format!("{}x{}", "\"{".repeat(2000), "}\"".repeat(2000));
	assert!(parse(&source).has_errors());
}

#[test]
fn reasonable_nesting_is_accepted() {
	let source = format!("{}x{}", "(".repeat(40), ")".repeat(40));
	assert!(!parse(&source).has_errors());
}

#[test]
fn garbage_never_panics() {
	let pieces = [
		"{", "}", "(", ")", "[", "]", "\"", "=>", "=", ".", ",", "\n", "@", "#", "é", "\\", "if",
		"else", "let", "function", "when", "is", "catch", "try", "..", "...", "-", "?", "%", ":",
		"x", "1", "type", "export", "mutable", "for", "in", "{{", "}}", "---", "--",
	];
	let mut state: u64 = 0x2545F4914F6CDD1D;
	for _ in 0..2000 {
		let mut source = String::new();
		for _ in 0..12 {
			state ^= state << 13;
			state ^= state >> 7;
			state ^= state << 17;
			source.push_str(pieces[(state % pieces.len() as u64) as usize]);
			source.push(' ');
		}
		parse(&source);
	}
}

#[test]
fn spans_of_invalid_nodes_are_in_the_source() {
	let source = "let x = (1 +\n";
	let tree = parse(source);
	for span in &tree.spans {
		assert!(span.start <= span.end && span.end <= source.len());
	}
}
