use crate::tokens::Token;

#[derive(Debug, Clone)]
pub enum ArrowFunctionBody {
	Block(Vec<usize>),
	Expression(usize),
}

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
	Integer(i64),
	Number(f64),
	Boolean(bool),
	// String(&str),

	/* ------------------------------- Composed ------------------------------- */
	// List(Vec<Node>),
	Tuple {
		// ex: `(a, b, c)`
		items: Vec<usize>,
	},
	Members {
		// an "object literal", i.e. member declarations inside brackets
		// ex: `{ a = 12, b: String = "Hello", #x, foo }`
		items: Vec<usize>,
	},

	/* ------------------------------- Operations ------------------------------- */
	Negate {
		right: usize,
	},
	Not {
		right: usize,
	},
	Return {
		expression: Option<usize>,
	},
	Break {
		expression: Option<usize>,
	},
	Continue,
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
	IntegerDivide {
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
	EmptyGroup,
	Group {
		expression: usize,
	},

	/* ------------------------------ Declarations ------------------------------ */
	Let {
		right: usize,
	},
	Mutable {
		right: usize,
	},
	Static {
		right: usize,
	},
	Type {
		right: usize,
	},
	UnionDeclaration {
		right: usize,
	},
	Enum {
		right: usize,
	},
	Fields {
		right: usize,
	},
	Reactive {
		right: usize,
	},
	Derived {
		right: usize,
	},
	Namespace {
		right: usize,
	},
	Function {
		name: &'static str,
		value: usize,
	},
	ArrowFunction {
		parameters: Option<usize>,
		parenthesized_parameters: bool,
		return_type_expression: Option<usize>,
		body: ArrowFunctionBody,
	},
	Assignment {
		// an equal statement
		name: usize,
		type_expression: Option<usize>,
		expression: Option<usize>,
	},

	/* -------------------------------- Functions ------------------------------- */
	FunctionCall {
		function: usize,
		parameters: Vec<usize>,
	},
}
