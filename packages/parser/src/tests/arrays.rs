use crate::parser;
use crate::ast::Expression::*;

#[test]
fn anonymous_array() {
	let ast = parser::ExpressionParser::new().parse("[1, hello, 3]").unwrap();

	match *ast {
		Array(fields) => {
			assert_eq!(fields.len(), 3);
			assert_eq!(*fields[0], Integer(1));
		}
		_ =>
			panic!(
				"Expected an anonymous array with fields 1, hello and 3, got {:?}",
				ast
			),
	}
}

#[test]
fn nested_array() {
	let ast = parser::ExpressionParser::new().parse("[1, [2, 3], 4]").unwrap();

	match *ast {
		Array(fields) => {
			assert_eq!(fields.len(), 3);
			assert_eq!(*fields[0], Integer(1));
			assert_eq!(*fields[2], Integer(4));

			match *fields[1] {
				Array(ref nested_fields) => {
					assert_eq!(nested_fields.len(), 2);
					assert_eq!(*nested_fields[0], Integer(2));
					assert_eq!(*nested_fields[1], Integer(3));
				}
				_ =>
					panic!(
						"Expected a nested array with fields 2 and 3, got {:?}",
						*fields[1]
					),
			}
		}
		_ =>
			panic!(
				"Expected an anonymous array with fields 1, [2, 3] and 4, got {:?}",
				ast
			),
	}
}

#[test]
fn empty_array() {
	let ast = parser::ExpressionParser::new().parse("[]").unwrap();

	match *ast {
		Array(fields) => assert_eq!(fields.len(), 0),
		_ =>
			panic!(
				"Expected an anonymous array with fields 1, [2, 3] and 4, got {:?}",
				ast
			),
	}
}
