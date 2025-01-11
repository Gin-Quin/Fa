use crate::fa;

use crate::ast::Expression::*;

#[test]
fn number() {
	let ast = fa::ExpressionParser::new().parse("12").unwrap();
	match *ast {
		Number(value) => assert_eq!(value, 12),
		_ => panic!("Expected a Number(12), got {:?}", ast),
	}
}

#[test]
fn identifier() {
	let ast = fa::ExpressionParser::new().parse("myVariable").unwrap();
	match *ast {
		Identifier(value) => assert_eq!(value, "myVariable"),
		_ => panic!("Expected a Identifier(myVariable), got {:?}", ast),
	}
}

#[test]
fn emoji_identifier() {
	let ast = fa::ExpressionParser::new().parse("🍎").unwrap();
	match *ast {
		Identifier(value) => assert_eq!(value, "🍎"),
		_ => panic!("Expected a Identifier(🍎), got {:?}", ast),
	}
}
