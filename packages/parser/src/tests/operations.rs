use crate::fa;

use crate::ast::Expression::*;
use crate::ast::Operator;

#[test]
fn add() {
	let ast = fa::ExpressionParser::new().parse("12+ 4").unwrap();
	match *ast {
		Operation(left, Operator::Add, right) => {
			assert_eq!(*left, Number(12));
			assert_eq!(*right, Number(4));
		}
		_ => panic!("Expected a Operation(12, Add, 4), got {:?}", ast),
	}
}

#[test]
fn subtract_nested() {
	let ast = fa::ExpressionParser::new().parse("12 +4-7").unwrap();
	match *ast {
		Operation(left, Operator::Subtract, right) => {
			match *left {
				Operation(left, Operator::Add, right) => {
					assert_eq!(*left, Number(12));
					assert_eq!(*right, Number(4));
				}
				_ => panic!("Expected a Operation(12, Add, 4), got {:?}", left),
			}
			assert_eq!(*right, Number(7));
		}
		_ => panic!("Expected a Operation(12, Add, 4), got {:?}", ast),
	}
}

#[test]
fn multiply() {
	let ast = fa::ExpressionParser::new().parse("12 * 4").unwrap();
	match *ast {
		Operation(left, Operator::Multiply, right) => {
			assert_eq!(*left, Number(12));
			assert_eq!(*right, Number(4));
		}
		_ => panic!("Expected a Operation(12, Multiply, 4), got {:?}", ast),
	}
}

#[test]
fn divide() {
	let ast = fa::ExpressionParser::new().parse("12 / 4").unwrap();
	match *ast {
		Operation(left, Operator::Divide, right) => {
			assert_eq!(*left, Number(12));
			assert_eq!(*right, Number(4));
		}
		_ => panic!("Expected a Operation(12, Divide, 4), got {:?}", ast),
	}
}

#[test]
fn add_with_parenthesis() {
	let ast = fa::ExpressionParser::new().parse("(12 + 4) * 5").unwrap();
	match *ast {
		Operation(left, Operator::Multiply, right) => {
			match *left {
				Operation(left, Operator::Add, right) => {
					assert_eq!(*left, Number(12));
					assert_eq!(*right, Number(4));
				}
				_ => panic!("Expected a Operation(12, Add, 4), got {:?}", left),
			}
			assert_eq!(*right, Number(5));
		}
		_ => panic!("Expected a Operation(12, Add, 4), got {:?}", ast),
	}
}
