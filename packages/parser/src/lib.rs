pub mod context;
pub mod diagnostic;
pub mod nodes;
pub mod parsing;
pub mod priority;
pub mod source;
pub mod tokenize;
pub mod tokens;
pub mod typed_syntax_tree;

pub use diagnostic::{Diagnostic, Severity};
pub use nodes::{Name, Node};
pub use parsing::parse::{parse, parse_shared};
pub use parsing::parse_single_statement::parse_single_statement;
pub use source::{SourceMap, SourceSpan};
pub use typed_syntax_tree::TypedSyntaxTree;

#[cfg(test)]
mod tests {
	mod assignment;
	mod destructuring;
	mod effects;
	mod export;
	mod expressions;
	mod fixtures;
	mod follow_ups;
	mod function_declarations;
	mod functions;
	mod generics;
	mod helpers;
	mod initializers;
	mod members;
	mod precedence;
	mod recovery;
	mod source_map;
	mod spans;
	mod tokenize;
	mod tree_api;
}
