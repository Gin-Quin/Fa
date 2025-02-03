use std::fmt::{ Debug, Error, Formatter };

#[derive(PartialEq, Debug)]
pub struct Declaration {
	pub identifier: String,
	pub type_expression: Option<Box<Expression>>,
	pub expression: Option<Box<Expression>>,
}

#[derive(PartialEq, Debug)]
pub enum Expression {
	Integer(i64),
	Number(f64),
	Identifier(String),
	Boolean(bool),
	String(String),
	Null,

	Operation {
		left: Box<Expression>,
		operator: Operator,
		right: Box<Expression>,
	},

	Call {
		function: Box<Expression>,
		parameters: Vec<Box<Expression>>,
	},

	Index {
		expression: Box<Expression>,
		index: Box<Expression>,
	},

	Object(Vec<Declaration>),
	Array(Vec<Box<Expression>>),

	Function {
		parameters: Vec<Declaration>,
		body: Vec<Statement>,
	},

	Error,
}

#[derive(PartialEq)]
pub enum Statement {
	TypeDeclaration(String, Box<Expression>),
	Declaration(Declaration),
	Expression(Box<Expression>),
	Use(Box<Expression>),
}

pub type Program = Vec<Statement>;

#[derive(PartialEq, Copy, Clone)]
pub enum Operator {
	Multiply,
	Divide,
	Add,
	Subtract,
	Union,
	Or,
	And,
	Equal,
	NotEqual,
	LessThan,
	LessThanOrEqual,
	GreaterThan,
	GreaterThanOrEqual,
	Remainder,
	Is,
	IsNot,
	MemberAccess,
	Path,
}

// impl Debug for Expression {
// 	fn fmt(&self, formatter: &mut Formatter<'_>) -> Result<(), Error> {
// 		use self::Expression::*;
// 		match *self {
// 			Integer(integer) => write!(formatter, "{:?}", integer),
// 			Number(number) => write!(formatter, "{:?}", number),
// 			OperationExpression(ref operation) =>
// 				write!(
// 					formatter,
// 					"({:?} {:?} {:?})",
// 					operation.left,
// 					operation.operator,
// 					operation.right
// 				),
// 			Identifier(ref identifier) => write!(formatter, "{:?}", identifier),
// 			CallExpression(ref call) =>
// 				write!(formatter, "{:?}({:?})", call.function, call.parameters),
// 			Boolean(boolean) => write!(formatter, "{:?}", boolean),
// 			String(ref string) => write!(formatter, "{:?}", string),
// 			Object(ref fields) => write!(formatter, "{{ {:?} }}", fields),
// 			Null => write!(formatter, "null"),
// 			Error => write!(formatter, "error"),
// 		}
// 	}
// }

impl Debug for Operator {
	fn fmt(&self, formatter: &mut Formatter<'_>) -> Result<(), Error> {
		use self::Operator::*;
		match *self {
			Multiply => write!(formatter, "*"),
			Divide => write!(formatter, "/"),
			Add => write!(formatter, "+"),
			Subtract => write!(formatter, "-"),
			Union => write!(formatter, "|"),
			Or => write!(formatter, "or"),
			And => write!(formatter, "and"),
			Equal => write!(formatter, "=="),
			NotEqual => write!(formatter, "!="),
			LessThan => write!(formatter, "<"),
			LessThanOrEqual => write!(formatter, "<="),
			GreaterThan => write!(formatter, ">"),
			GreaterThanOrEqual => write!(formatter, ">="),
			Remainder => write!(formatter, "%"),
			Is => write!(formatter, "is"),
			IsNot => write!(formatter, "is not"),
			Path => write!(formatter, "::"),
			MemberAccess => write!(formatter, "."),
		}
	}
}

impl Debug for Statement {
	fn fmt(&self, formatter: &mut Formatter<'_>) -> Result<(), Error> {
		use self::Statement::*;
		match self {
			TypeDeclaration(identifier, expression) =>
				write!(formatter, "type {:?} = {:?}", identifier, expression),
			Declaration(declaration) => write!(formatter, "{:?}", declaration),
			Expression(expression) => write!(formatter, "{:?}", expression),
			Use(expression) => write!(formatter, "use {:?}", expression),
		}
	}
}
