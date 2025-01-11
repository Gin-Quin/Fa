use std::fmt::{Debug, Error, Formatter};

pub enum Expression {
    Number(i32),
	 Identifier(String),
    Operation(Box<Expression>, Operator, Box<Expression>),
	 List(Vec<Box<Expression>>),
	 Call(Box<Expression>, Vec<Box<Expression>>),
	//  Declaration()
    Error,
}

#[derive(Copy, Clone)]
pub enum Operator {
    Multiply,
    Divide,
    Add,
    Subtract,
}

impl Debug for Expression {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> Result<(), Error> {
        use self::Expression::*;
        match *self {
            Number(number) => write!(formatter, "{:?}", number),
            Operation(ref left, operation, ref right) => write!(formatter, "({:?} {:?} {:?})", left, operation, right),
				List(ref list) => write!(formatter, "{:?}", list),
				Identifier(ref identifier) => write!(formatter, "{:?}", identifier),
				Call(ref expression, ref parameters) => write!(formatter, "{:?}({:?})", expression, parameters),
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
        }
    }
}
