use lalrpop_util::lalrpop_mod;

mod ast;

lalrpop_mod!(pub fa);

#[cfg(test)]
mod tests {
	mod literals;
	mod operations;
	mod declarations;
	mod objects;
	mod arrays;
	mod indexes;
	mod function_calling;
	mod function_declaration;
}
