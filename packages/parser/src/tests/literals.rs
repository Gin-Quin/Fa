use crate::fa;

use crate::ast::Expression::*;

#[test]
fn number() {
	let ast = fa::ExpressionParser::new().parse("12.4").unwrap();
	match *ast {
		Number(value) => assert_eq!(value, 12.4),
		_ => panic!("Expected a Number(12.4), got {:?}", ast),
	}
}

#[test]
fn integer() {
	let ast = fa::ExpressionParser::new().parse("12").unwrap();
	match *ast {
		Integer(value) => assert_eq!(value, 12),
		_ => panic!("Expected a Integer(12), got {:?}", ast),
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

#[test]
fn boolean_true() {
	let ast = fa::ExpressionParser::new().parse("true").unwrap();
	match *ast {
		Boolean(value) => assert_eq!(value, true),
		_ => panic!("Expected a Boolean(true), got {:?}", ast),
	}
}

#[test]
fn boolean_false() {
	let ast = fa::ExpressionParser::new().parse("false").unwrap();
	match *ast {
		Boolean(value) => assert_eq!(value, false),
		_ => panic!("Expected a Boolean(false), got {:?}", ast),
	}
}

#[test]
fn string() {
	let ast = fa::ExpressionParser::new().parse("\"hello\"").unwrap();
	match *ast {
		String(value) => assert_eq!(value, "hello"),
		_ => panic!("Expected a String(hello), got {:?}", ast),
	}
}
