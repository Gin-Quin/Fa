pub mod context;
pub mod nodes;
pub mod parse;
pub mod parse_arrow_function;
pub mod parse_expression;
pub mod parse_function_declaration;
pub mod parse_statement;
pub mod priority;
pub mod symbols;
pub mod tokenize;
pub mod tokens;
pub mod typed_syntax_tree;
pub mod types;

#[cfg(test)]
mod tests {
	mod assignment;
	mod expressions;
	mod function_declarations;
	mod functions;
	mod members;
	mod tokenize;
}
