use crate::fa;

use crate::ast;
use ast::Expression::*;
use ast::Operator;

#[test]
fn add() {
	let ast = fa::ExpressionParser::new().parse("12+ 4").unwrap();
	match *ast {
		Operation { left, operator: Operator::Add, right } => {
			assert_eq!(*left, Integer(12));
			assert_eq!(*right, Integer(4));
		}
		_ => panic!("Expected a Operation(12, Add, 4), got {:?}", ast),
	}
}

#[test]
fn subtract_nested() {
	let ast = fa::ExpressionParser::new().parse("12 +4-7").unwrap();
	match *ast {
		Operation { left, operator: Operator::Subtract, right } => {
			match *left {
				Operation { left, operator: Operator::Add, right } => {
					assert_eq!(*left, Integer(12));
					assert_eq!(*right, Integer(4));
				}
				_ => panic!("Expected a Operation(12, Add, 4), got {:?}", left),
			}
			assert_eq!(*right, Integer(7));
		}
		_ => panic!("Expected a Operation(12, Add, 4), got {:?}", ast),
	}
}

#[test]
fn multiply() {
	let ast = fa::ExpressionParser::new().parse("12 * 4").unwrap();
	match *ast {
		Operation { left, operator: Operator::Multiply, right } => {
			assert_eq!(*left, Integer(12));
			assert_eq!(*right, Integer(4));
		}
		_ => panic!("Expected a Operation(12, Multiply, 4), got {:?}", ast),
	}
}

#[test]
fn divide() {
	let ast = fa::ExpressionParser::new().parse("12 / 4").unwrap();
	match *ast {
		Operation { left, operator: Operator::Divide, right } => {
			assert_eq!(*left, Integer(12));
			assert_eq!(*right, Integer(4));
		}
		_ => panic!("Expected a Operation(12, Divide, 4), got {:?}", ast),
	}
}

#[test]
fn add_with_parenthesis() {
	let ast = fa::ExpressionParser::new().parse("(12 + 4) * 5").unwrap();
	match *ast {
		Operation { left, operator: Operator::Multiply, right } => {
			match *left {
				Operation { left, operator: Operator::Add, right } => {
					assert_eq!(*left, Integer(12));
					assert_eq!(*right, Integer(4));
				}
				_ => panic!("Expected a Operation(12, Add, 4), got {:?}", left),
			}
			assert_eq!(*right, Integer(5));
		}
		_ => panic!("Expected a Operation(12, Add, 4), got {:?}", ast),
	}
}

#[test]
fn is_not() {
	let ast = fa::ExpressionParser::new().parse("12 is not 4").unwrap();
	match *ast {
		Operation { left, operator: Operator::IsNot, right } => {
			assert_eq!(*left, Integer(12));
			assert_eq!(*right, Integer(4));
		}
		_ => panic!("Expected a Operation(12, IsNot, 4), got {:?}", ast),
	}
}

#[test]
fn is() {
	let ast = fa::ExpressionParser::new().parse("12 is Number").unwrap();
	match *ast {
		Operation { left, operator: Operator::Is, right } => {
			assert_eq!(*left, Integer(12));
			assert_eq!(*right, Identifier("Number".to_string()));
		}
		_ => panic!("Expected a Operation(12, Is, Number), got {:?}", ast),
	}
}

#[test]
fn equal() {
	let ast = fa::ExpressionParser::new().parse("12 == 4").unwrap();
	match *ast {
		Operation { left, operator: Operator::Equal, right } => {
			assert_eq!(*left, Integer(12));
			assert_eq!(*right, Integer(4));
		}
		_ => panic!("Expected a Operation(12, Equal, 4), got {:?}", ast),
	}
}

#[test]
fn not_equal() {
	let ast = fa::ExpressionParser::new().parse("12 != 4").unwrap();
	match *ast {
		Operation { left, operator: Operator::NotEqual, right } => {
			assert_eq!(*left, Integer(12));
			assert_eq!(*right, Integer(4));
		}
		_ => panic!("Expected a Operation(12, NotEqual, 4), got {:?}", ast),
	}
}

#[test]
fn less_than() {
	let ast = fa::ExpressionParser::new().parse("12 < 4").unwrap();
	match *ast {
		Operation { left, operator: Operator::LessThan, right } => {
			assert_eq!(*left, Integer(12));
			assert_eq!(*right, Integer(4));
		}
		_ => panic!("Expected a Operation(12, LessThan, 4), got {:?}", ast),
	}
}

#[test]
fn less_than_or_equal() {
	let ast = fa::ExpressionParser::new().parse("12 <= 4").unwrap();
	match *ast {
		Operation { left, operator: Operator::LessThanOrEqual, right } => {
			assert_eq!(*left, Integer(12));
			assert_eq!(*right, Integer(4));
		}
		_ => panic!("Expected a Operation(12, LessThanOrEqual, 4), got {:?}", ast),
	}
}

#[test]
fn greater_than() {
	let ast = fa::ExpressionParser::new().parse("12 > 4").unwrap();
	match *ast {
		Operation { left, operator: Operator::GreaterThan, right } => {
			assert_eq!(*left, Integer(12));
			assert_eq!(*right, Integer(4));
		}
		_ => panic!("Expected a Operation(12, GreaterThan, 4), got {:?}", ast),
	}
}

#[test]
fn greater_than_or_equal() {
	let ast = fa::ExpressionParser::new().parse("12 >= 4").unwrap();
	match *ast {
		Operation { left, operator: Operator::GreaterThanOrEqual, right } => {
			assert_eq!(*left, Integer(12));
			assert_eq!(*right, Integer(4));
		}
		_ => panic!("Expected a Operation(12, GreaterThanOrEqual, 4), got {:?}", ast),
	}
}

#[test]
fn or() {
	let ast = fa::ExpressionParser::new().parse("12 or 4").unwrap();
	match *ast {
		Operation { left, operator: Operator::Or, right } => {
			assert_eq!(*left, Integer(12));
			assert_eq!(*right, Integer(4));
		}
		_ => panic!("Expected a Operation(12, Or, 4), got {:?}", ast),
	}
}

#[test]
fn and() {
	let ast = fa::ExpressionParser::new().parse("12 and 4").unwrap();
	match *ast {
		Operation { left, operator: Operator::And, right } => {
			assert_eq!(*left, Integer(12));
			assert_eq!(*right, Integer(4));
		}
		_ => panic!("Expected a Operation(12, And, 4), got {:?}", ast),
	}
}

#[test]
fn path() {
	let ast = fa::ExpressionParser::new().parse("12::4").unwrap();
	match *ast {
		Operation { left, operator: Operator::Path, right } => {
			assert_eq!(*left, Integer(12));
			assert_eq!(*right, Integer(4));
		}
		_ => panic!("Expected a Operation(12, Path, 4), got {:?}", ast),
	}
}

#[test]
fn member_access() {
	let ast = fa::ExpressionParser::new().parse("hello.you").unwrap();
	match *ast {
		Operation { left, operator: Operator::MemberAccess, right } => {
			assert_eq!(*left, Identifier("hello".to_string()));
			assert_eq!(*right, Identifier("you".to_string()));
		}
		_ => panic!("Expected a Operation(12, MemberAccess, 4), got {:?}", ast),
	}
}
