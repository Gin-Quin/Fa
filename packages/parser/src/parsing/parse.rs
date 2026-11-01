use std::sync::Arc;

use crate::context::Context;
use crate::nodes::Node;
use crate::parsing::parse_block_body::parse_statements;
use crate::tokens::TokenKind;
use crate::typed_syntax_tree::TypedSyntaxTree;

/// Parse a file. The root of the tree is a `Node::Module`. Never panics: the problems found are
/// in `tree.diagnostics`, and the code that could not be parsed is `Node::Invalid`.
/// The tree keeps a copy of the source (see `parse_shared` to share it instead).
pub fn parse(source: &str) -> TypedSyntaxTree {
	parse_shared(Arc::from(source))
}

/// Parse a file whose text is shared, like a document of a language server: the tree keeps the
/// `Arc` instead of a copy.
pub fn parse_shared(source: Arc<str>) -> TypedSyntaxTree {
	let mut context = Context::new(&source, source.clone());
	let statements = parse_statements(&mut context, TokenKind::End);
	let hoisted_symbols = context.exit_hoisted_scope();

	let (start, end) = match (statements.first(), statements.last()) {
		(Some(first), Some(last)) => (context.span(*first).start, context.span(*last).end),
		_ => (0, 0),
	};
	let root = context.insert(
		Node::Module {
			statements,
			hoisted_symbols,
		},
		start,
		end,
	);

	let mut tree = context.tree;
	tree.root = root;
	tree
}
