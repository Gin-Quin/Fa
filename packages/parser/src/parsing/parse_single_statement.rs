use std::sync::Arc;

use crate::context::Context;
use crate::parsing::parse_expression::{ExpressionContext, parse_expression};
use crate::priority::Priority;
use crate::tokens::TokenKind;
use crate::typed_syntax_tree::TypedSyntaxTree;

/// Parse a single statement: the root of the tree is the statement itself. Anything after it
/// is reported. Useful for tests and for evaluating a line of code.
pub fn parse_single_statement(source: &str) -> TypedSyntaxTree {
	let shared: Arc<str> = Arc::from(source);
	let mut context = Context::new(source, shared);
	context.skip_stops();
	let root = parse_expression(
		&mut context,
		Priority::None,
		ExpressionContext::new(true, false),
		&[TokenKind::End],
	);
	context.skip_stops();
	if !context.done() {
		let found = context.describe_token();
		context.error_at_token(format!("Unexpected {found} after the end of the statement"));
	}

	let mut tree = context.tree;
	tree.root = root;
	tree
}
