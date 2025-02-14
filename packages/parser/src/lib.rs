#[cfg(target_arch = "wasm32")]
#[global_allocator]
// When exporting to wasm, we use a wasm-optimized allocator
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

use lalrpop_util::lalrpop_mod;

pub mod ast;

lalrpop_mod!(pub parser);

#[cfg(test)]
mod tests {
	mod literals;
	mod operations;
	mod declarations;
	mod blocks;
	mod arrays;
	mod indexes;
	mod function_calling;
	mod function_declaration;
	mod program;
}

/* -------------------------------------------------------------------------- */
/*                                Wasm library                                */
/* -------------------------------------------------------------------------- */

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
#[allow(non_snake_case)]
pub fn parseFaProgram(input: &str) -> bool {
	let program = parser::ProgramParser::new().parse(input);
	program.is_ok()
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
#[allow(non_snake_case)]
pub fn parseFaExpression(input: &str) -> bool {
	let expression = parser::ExpressionParser::new().parse(input);
	expression.is_ok()
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
#[allow(non_snake_case)]
pub fn parseFaStatement(input: &str) -> bool {
	let statement = parser::StatementParser::new().parse(input);
	statement.is_ok()
}
