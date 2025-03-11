use crate::tokens::Token;

#[derive(Debug, Clone)]
pub enum Node {
	/* --------------------------------- Errors --------------------------------- */
	UnexpectedTokenError {
		token: Token,
	},

	/* ------------------------------- Primitives ------------------------------- */

	Identifier(&'static str),
	Integer(i32),
	// Float(f64),
	Boolean(bool),
	// String(&str),
	// Array(Vec<Node>),
	// Tuple(Vec<Node>),

	/* ------------------------------- Operations ------------------------------- */
	Negate {
		right: usize,
	},
	Not {
		right: usize,
	},
	Add {
		operands: Vec<usize>,
	},
	Subtract {
		operands: Vec<usize>,
	},
	Multiply {
		operands: Vec<usize>,
	},
	Divide {
		operands: Vec<usize>,
	},
	Modulo {
		operands: Vec<usize>,
	},
	Power {
		operands: Vec<usize>,
	},
	Equal {
		operands: Vec<usize>,
	},
	NotEqual {
		operands: Vec<usize>,
	},
	LessThan {
		operands: Vec<usize>,
	},
	LessThanOrEqual {
		operands: Vec<usize>,
	},
	GreaterThan {
		operands: Vec<usize>,
	},
	GreaterThanOrEqual {
		operands: Vec<usize>,
	},
	And {
		operands: Vec<usize>,
	},
	Or {
		operands: Vec<usize>,
	},
	Is {
		left: usize,
		right: usize,
	},
	Union {
		operands: Vec<usize>,
	},
	Pipe {
		operands: Vec<usize>,
	},
	Insert {
		left: usize,
		right: usize,
	},
	Extract {
		left: usize,
		right: usize,
	},

	/* --------------------------------- Groups --------------------------------- */
	Group {
		expression: usize,
	},
	Tuple {
		items: Vec<usize>,
	},

	/* -------------------------------- Functions ------------------------------- */
	// FunctionCall {
	// 	left: usize,
	// 	parameters: Vec<usize>,
	// },

	/* ------------------------------ Declarations ------------------------------ */
	// ValueDeclaration {
	// 	identifier: &'static str,
	// 	explicit_type: Option<usize>,
	// 	value: Option<usize>,
	// },
	// FunctionDeclaration {},
	// MethodDeclaration {},
	// TypeDeclaration {
	// 	identifier: &'static str,
	// 	parameters: Vec<usize>, // for generics
	// 	value: usize,
	// },
}
