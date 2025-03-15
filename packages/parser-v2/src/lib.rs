pub mod context;
pub mod nodes;
pub mod parse;
pub mod priority;
pub mod tokenize;
pub mod tokens;
pub mod typed_syntax_tree;
pub mod types;
pub mod symbols;
pub mod parse_expression;
pub mod parse_statement;

#[cfg(test)]
mod tests {
	mod tokenize;
	mod expressions;
	mod assignment;
	mod functions;
}
