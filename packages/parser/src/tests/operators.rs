use crate::fa;

use crate::ast::Expression::*;
use crate::ast::Operator;

#[test]
fn test_number() {
	let ast = fa::ExpressionParser::new().parse("12").unwrap();
	match *ast {
		Number(value) => assert_eq!(value, 12),
		_ => panic!("Expected a Number(12), got {:?}", ast),
	}
}

#[test]
fn test_add() {
	let ast = fa::ExpressionParser::new().parse("12+4").unwrap();
	match *ast {
		Operation(left, Operator::Add, right) => {
			assert_eq!(*left, Number(12));
			assert_eq!(*right, Number(4));
		}
		_ => panic!("Expected a Operation(12, Add, 4), got {:?}", ast),
	}
}

#[test]
fn test_subtract_nested() {
	let ast = fa::ExpressionParser::new().parse("12+4-7").unwrap();
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
