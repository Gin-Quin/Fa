pub mod context;
pub mod nodes;
pub mod parse;
pub mod priority;
pub mod tokenize;
pub mod tokens;
pub mod typed_syntax_tree;
pub mod types;
pub mod symbols;
pub mod parsers {
	pub mod expression;
	pub mod statement;
}

#[cfg(test)]
mod tests {
	mod tokenize;
	mod expressions;
	mod assignment;
	// mod literals;
	// mod operations;
	// mod declarations;
	// mod blocks;
	// mod arrays;
	// mod indexes;
	// mod function_calling;
	// mod function_declaration;
	// mod program;
}
