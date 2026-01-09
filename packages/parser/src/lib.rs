pub mod analysis;
pub mod context;
pub mod nodes;
pub mod parsing;
pub mod priority;
pub mod scope;
pub mod source;
pub mod tokenize;
pub mod tokens;
pub mod type_error;
pub mod typed_syntax_tree;
pub mod types;

#[cfg(test)]
mod tests {
	mod assignment;
	mod export;
	mod expressions;
	mod function_declarations;
	mod functions;
	mod initializers;
	mod members;
	mod source_map;
	mod spans;
	mod symbols;
	mod tokenize;
}
