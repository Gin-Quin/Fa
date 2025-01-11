use std::fmt::{ Debug, Error, Formatter };

#[derive(PartialEq)]
pub enum Expression {
	Integer(i64),
	Number(f64),
	Identifier(String),
	Boolean(bool),
	String(String),
	Null,

	Operation(Box<Expression>, Operator, Box<Expression>),
	List(Vec<Box<Expression>>),
	Call(Box<Expression>, Vec<Box<Expression>>),
	//  Declaration()

	Error,
}

#[derive(PartialEq)]
pub enum Statement {
	TypeDeclaration(String, Box<Expression>),
	Assignment(String, Option<Box<Expression>>, Box<Expression>),
	Expression(Box<Expression>),
}

#[derive(PartialEq)]
pub struct Program(pub Vec<Statement>);

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
}

impl Debug for Expression {
	fn fmt(&self, formatter: &mut Formatter<'_>) -> Result<(), Error> {
		use self::Expression::*;
		match *self {
			Integer(integer) => write!(formatter, "{:?}", integer),
			Number(number) => write!(formatter, "{:?}", number),
			Operation(ref left, operation, ref right) =>
				write!(formatter, "({:?} {:?} {:?})", left, operation, right),
			List(ref list) => write!(formatter, "{:?}", list),
			Identifier(ref identifier) => write!(formatter, "{:?}", identifier),
			Call(ref expression, ref parameters) =>
				write!(formatter, "{:?}({:?})", expression, parameters),
			Boolean(boolean) => write!(formatter, "{:?}", boolean),
			String(ref string) => write!(formatter, "{:?}", string),
			Null => write!(formatter, "null"),
			Error => write!(formatter, "error"),
		}
	}
}

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
		}
	}
}

impl Debug for Statement {
	fn fmt(&self, formatter: &mut Formatter<'_>) -> Result<(), Error> {
		use self::Statement::*;
		match self {
			TypeDeclaration(identifier, expression) =>
				write!(formatter, "type {:?} = {:?}", identifier, expression),
			Assignment(identifier, type_expression, expression) =>
				match type_expression {
					Some(type_expression) =>
						write!(
							formatter,
							"{:?}: {:?} = {:?}",
							identifier,
							type_expression,
							expression
						),
					None => write!(formatter, "{:?} = {:?}", identifier, expression),
				}
			Expression(expression) => write!(formatter, "{:?}", expression),
		}
	}
}
