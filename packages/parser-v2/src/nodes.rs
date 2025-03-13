use crate::tokens::Token;

#[derive(Debug, Clone)]
pub enum Node {
	Module {
		statements: Vec<usize>,
	},
	DanglingToken {
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

	/* ------------------------------ Declarations ------------------------------ */
	Assignment {
		name: Result<&'static str, usize>,
		type_expression: Option<usize>,
		expression: Option<usize>,
	},
	// FunctionDeclaration { // declared with the `function` keyword
	// 	name: &'static str,
	// 	parameters: Vec<usize>,
	// 	return_type_expression: Option<usize>,
	// 	body: usize,
	// },
	// TypeDeclaration { // declared with the `type` keyword
	// 	name: &'static str,
	// 	expression: usize,
	// },
	// EnumerationDeclaration { // declared with the `enumeration` keyword
	// 	name: &'static str,
	// 	variants: Vec<usize>,
	// },
	// UnionDeclaration { // declared with the `union` keyword
	// 	name: &'static str,
	// 	variants: Vec<usize>,
	// },
	// FieldsDeclaration { // declared with the `fields` keyword
	// 	name: &'static str,
	// 	fields: Vec<usize>,
	// },

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
