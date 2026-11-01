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
fn unclosed_group_stops_at_the_end_of_its_line() {
	assert_eq!(error_count("(a\nlet y = 2"), 1);
	assert!(parse("(a\nlet y = 2").to_string().contains("let y = 2"));
	assert_eq!(error_count("x[1\nlet y = 2"), 1);
	assert!(parse("x[1\nlet y = 2").to_string().contains("let y = 2"));
}

#[test]
fn extra_token_in_a_group_gives_one_diagnostic() {
	assert_eq!(error_count("(a b)"), 1);
	assert_eq!(error_count("x[1 2]"), 1);
	assert_eq!(error_count("f((a b), c)"), 1);
	assert_statement("(a\n)", "(a)");
	assert_statement("x[\n1\n]", "x[1]");
}

#[test]
fn missing_separators_are_reported() {
	assert_error_message("[1, 2 3]", "Expected `,` or `]`");
	assert_error_message("f(a b)", "Expected `,` or `)`");
	assert_error_message("{ a b }", "after the member `a`");
}

#[test]
fn method_in_an_object_suggests_function() {
	assert_error_message("{ foo(x) }", "function foo = (self)");
}

#[test]
fn readonly_needs_a_field() {
	assert_statement("{ readonly id: Integer }", "{\n\treadonly id: Integer\n}");
	assert_error_message("{ readonly 1 }", "Expected a field after `readonly`");
}

#[test]
fn compile_time_tests_and_modifiers() {
	assert_statement("@test \"x\" { }", "@test \"x\" {\n\t\n}");
	assert_statement("test skip \"x\" { }", "test skip \"x\" {\n\t\n}");
	assert_statement("test only \"x\" { }", "test only \"x\" {\n\t\n}");
	assert_error_message("@test foo \"x\" { }", "Unknown test modifier `foo`");
	assert_error_message("@test { }", "Expected the name of the test");
}

#[test]
fn reuse_needs_a_name_and_a_value() {
	assert_statement("reuse best = player", "reuse best = player");
	assert_error_message(
		"reuse 1 = 2",
		"Expected the name of a `mutable use` reference",
	);
	assert_error_message("reuse x", "Expected `=` after `reuse x`");
}

#[test]
fn use_needs_a_value() {
	assert_statement("use r = x", "use r = x");
	assert_statement("mutable use r = x", "mutable use r = x");
	assert_error_message("use r", "A `use` declaration needs a value");
	assert_error_message("mutable use r", "A `mutable use` declaration needs a value");
}

#[test]
fn static_value_cannot_be_destructured() {
	assert_error_message("static { a } = b", "A `static` value can't be destructured");
}

#[test]
fn namespace_has_no_parameters() {
	assert_error_message("namespace foo(T) = {}", "A namespace has no parameters");
}

#[test]
fn empty_generic_parameters_are_reported() {
	assert_error_message("type T() = Integer", "Generic parameters can't be empty");
	assert_error_message("function f() = x", "Generic parameters can't be empty");
	assert_error_message("function f(x) = x", "needs a type");
}

#[test]
fn compile_time_and_runtime_else_are_not_mixed() {
	assert_error_message("if a { } @else { }", "An `if` continues with `else`");
	assert_error_message("@if a { } else { }", "continues with `@else`, not `else`");
	assert_statement(
		"@if a { } @else @if b { } @else { }",
		"@if a {\n\t\n} @else @if b {\n\t\n} @else {\n\t\n}",
	);
	assert_error_message("if a { } else 1", "Expected `if` or `{` after `else`");
}

#[test]
fn loops_need_a_condition_or_an_iterable() {
	assert_error_message("while { }", "Expected a condition after `while`");
	assert_error_message("for { }", "Expected what to iterate after `for`");
	assert_error_message("for x in { }", "Expected what to iterate after `for`");
	assert_error_message("for x, 1 in xs { }", "Expected the name of the index");
	assert_error_message("for x xs { }", "Expected `{` to start the for block");
}

#[test]
fn parallel_loops() {
	assert_statement(
		"for parallel(threads = 2) x in xs { }",
		"for parallel(threads = 2) x in xs {\n\t\n}",
	);
	assert_statement("for parallel x in xs { }", "for parallel x in xs {\n\t\n}");
}

#[test]
fn when_needs_a_value_is_and_branches() {
	assert_error_message("when { }", "Expected a value after `when`");
	assert_error_message("when x { }", "Expected `is` after the value of `when`");
	assert_error_message("when x is 1", "Expected `{` after `when ... is`");
	assert_error_message("when x is { 1 \"a\" }", "Expected `=>` after the pattern");
}

#[test]
fn catch_needs_a_name_and_an_arrow() {
	assert_error_message(
		"a catch => 1",
		"Expected the name of the error after `catch`",
	);
	assert_error_message("a catch e 1", "Expected `=>` after `catch e`");
	assert_statement("a catch e => { 1 }", "(a catch e => {\n\t1\n})");
}

#[test]
fn partial_application_needs_arguments() {
	assert_error_message("f with 1", "Expected the bound parameters after `with`");
}

#[test]
fn assignments_are_not_chained() {
	assert_error_message("x = 1 = 2", "Assignments can't be chained");
}

#[test]
fn malformed_exports_are_reported() {
	assert_error_message("export async x", "Expected `function` after `export async`");
	assert_error_message("export foo", "An export has no name");
	assert_error_message(
		"export 1",
		"Expected `=`, `mutable`, `function`, `type` or `namespace`",
	);
	assert_error_message("export static x = 1", "A file exports one anonymous value");
}

#[test]
fn provide_needs_a_value() {
	assert_statement(
		"provide Database = Postgres(url)",
		"provide Database = Postgres(url)",
	);
	assert_error_message("provide Database", "Expected `=` after the provided type");
}

#[test]
fn async_needs_a_function() {
	assert_statement(
		"async (url: String) => fetch(url)",
		"async (url: String) => fetch(url)",
	);
	assert_error_message("async 1", "`async` must be followed by `function`");
	assert_error_message(
		"async function load = 1",
		"An `async` function is declared with its parameters",
	);
	assert_error_message("function 1 = 2", "Expected the name of the function");
}

#[test]
fn misplaced_signature_clauses_are_reported() {
	assert_error_message("(a) throws E: T => a", "are written between parentheses");
	assert_error_message(
		"function f = (self) invalidates self mutates self => self",
		"`mutates` must be written once",
	);
}

#[test]
fn missing_field_names_are_reported() {
	assert_error_message("a?.", "Expected a field name after `?.`");
	assert_error_message("a?.1", "write `a[1]`");
}

#[test]
fn invalid_characters_are_reported() {
	assert_error_message("'a'", "Strings are written between double quotes");
	assert_error_message("`a`", "Strings are written between double quotes");
	assert_error_message("a ^ b", "`^` is not an operator");
	assert_error_message("a ~ b", "`~` is not an operator");
	assert_error_message("@", "Expected a name after `@`");
	assert_error_message("#", "Expected a name after `#`");
	assert_statement("#private", "#private");
	assert_statement("@readFile(\"x\")", "@readFile(\"x\")");
}

#[test]
fn malformed_numbers_are_reported() {
	assert_error_message("0b", "has no digits");
	assert_error_message("0o9", "is not a valid number");
	assert_error_message("0x1g", "is not a valid number");
	assert_error_message("1e", "is not a valid number");
	assert_error_message("1.5n", "is not a valid number");
	assert_error_message("2.", "write `2.0`");
}

#[test]
fn valid_number_forms() {
	assert_statement("1.5e3", "1500");
	assert_statement("2.5e-3", "0.0025");
	assert_statement("1_000.000_1", "1000.0001");
	assert_statement("-0x10", "-16");
	assert_statement("0o17", "15");
	assert_statement("0b1010", "10");
	assert_statement("-0b1n", "-0b1n");
	assert_statement("12n", "12n");
}

#[test]
fn malformed_escapes_are_reported() {
	assert_error_message("\"\\xé1\"", "`\\x` is followed by two hexadecimal digits");
	assert_error_message("\"\\x4\"", "`\\x` is followed by two hexadecimal digits");
	assert_error_message("\"\\u{}\"", "`\\u` is followed by a Unicode scalar value");
	assert_error_message("\"\\u1234\"", "`\\u` is followed by a Unicode scalar value");
	assert_error_message(
		"\"\\u{1234567}\"",
		"`\\u` is followed by a Unicode scalar value",
	);
	assert_error_message("\"\\", "A string can't end with a single `\\`");
	assert_statement("\"\\x41\\u{1F600}\\t\"", "\"A😀\\t\"");
}

#[test]
fn malformed_interpolations_are_reported() {
	assert_error_message("\"{a b}\"", "a string interpolates one expression");
	assert_error_message("x = \"{a\"", "This string is never closed");
	assert_statement("\"{{}}\"", "\"{{}}\"");
	assert_statement("\"a {b} c {d}\"", "\"a {b} c {d}\"");
}

#[test]
fn spans_of_invalid_nodes_are_in_the_source() {
	let source = "let x = (1 +\n";
	let tree = parse(source);
	for span in &tree.spans {
		assert!(span.start <= span.end && span.end <= source.len());
	}
}
