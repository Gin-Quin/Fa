use lalrpop_util::lalrpop_mod;

mod ast;

lalrpop_mod!(pub fa);

#[cfg(test)]
mod tests {
	mod literals;
	mod operations;
	mod function_calling;
	mod declarations;
}
