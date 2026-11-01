use crate::parsing::parse::parse;
use crate::parsing::parse_single_statement::parse_single_statement;
use crate::tokenize::tokenize_with_diagnostics;
use crate::typed_syntax_tree::TypedSyntaxTree;

/// Fail if the tree has a diagnostic.
pub fn assert_no_diagnostics(tree: &TypedSyntaxTree, input: &str) {
	let diagnostics = tree
		.diagnostics
		.iter()
		.map(|diagnostic| diagnostic.display(&tree.source_map).to_string())
		.collect::<Vec<_>>();
	assert!(
		diagnostics.is_empty(),
		"\nUnexpected diagnostics for:\n{input}\n\n{}",
		diagnostics.join("\n")
	);
}

/// Parse one statement, check that it has no diagnostic and that it renders as `expected`.
pub fn assert_statement(input: &str, expected: &str) {
	let tree = parse_single_statement(input);
	assert_no_diagnostics(&tree, input);
	assert_eq!(tree.to_string(), expected, "\nInput: {input}");
}

/// Parse a module, check that it has no diagnostic and that it renders as `expected`.
pub fn assert_module(input: &str, expected: &str) {
	let tree = parse(input);
	assert_no_diagnostics(&tree, input);
	assert_eq!(tree.to_string(), expected, "\nInput: {input}");
}

/// Check that parsing reports an error (as a statement and as a module), without panicking.
pub fn assert_has_error(input: &str) {
	let tree = parse_single_statement(input);
	assert!(
		tree.has_errors(),
		"Expected an error for `{input}`, got the tree `{}`",
		tree
	);
	let tree = parse(input);
	assert!(
		tree.has_errors(),
		"Expected an error for `{input}` (as a module), got the tree `{}`",
		tree
	);
}

/// Check that parsing reports an error whose message contains `fragment`.
pub fn assert_error_message(input: &str, fragment: &str) {
	let tree = parse(input);
	assert!(
		tree.diagnostics
			.iter()
			.any(|diagnostic| diagnostic.message.contains(fragment)),
		"Expected an error containing `{fragment}` for `{input}`, got {:#?}",
		tree.diagnostics
	);
}

/// Check that tokenizing reports an error.
pub fn assert_tokenizer_error(input: &str) {
	let (_, diagnostics) = tokenize_with_diagnostics(input);
	assert!(
		!diagnostics.is_empty(),
		"Expected a tokenizer error for `{input}`"
	);
}
