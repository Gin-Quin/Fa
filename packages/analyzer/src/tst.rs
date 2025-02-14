/**
 * TST stands for "Typed Symbols Tree".
 * It is a tree structure built from the Fa parser's AST that represents all symbols used in a program.
 */

use fa_parser::*;

pub type Body<'ast> = Vec<Definition<'ast>>;

#[derive(Debug)]
pub enum Definition<'ast> {
	Variable {
		declaration: &'ast ast::Declaration,
	},
	Function {
		declaration: &'ast ast::Declaration,
		body: Body<'ast>,
	},
	FunctionParameter {
		declaration: &'ast ast::Declaration,
	},
}

#[derive(Debug)]
pub struct TypeSymbolTree<'ast> {
	pub body: Body<'ast>,
}

impl<'ast> TypeSymbolTree<'ast> {
	pub fn new(ast: &'ast ast::Program) -> Self {
		let body: Vec<Definition<'ast>> = ast
			.iter()
			.map(|statement| {
				match statement {
					ast::Statement::Declaration(declaration) => {
						Self::parse_statement(statement)
					}
					_ => None,
				}
			})
			.flatten()
			.collect();

		TypeSymbolTree { body }
	}

	pub fn parse_statement(statement: &'ast ast::Statement) -> Option<Definition<'ast>> {
		match statement {
			ast::Statement::Declaration(declaration) => {
				match &declaration.expression {
					Some(expression) => {
						match **expression {
							ast::Expression::Function { parameters: _, body: _ } => {
								Some(Definition::Function {
									declaration,
									body: Vec::new(),
								})
							}
							_ => Some(Definition::Variable { declaration }),
						}
					}
					None => Some(Definition::Variable { declaration }),
				}
			}
			_ => None,
		}
	}
}
