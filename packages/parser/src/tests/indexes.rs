use crate::parser;
use crate::ast::Expression::*;

#[test]
fn indexing_an_array() {
	let ast = parser::ExpressionParser::new().parse("[0, 1, 2][1]").unwrap();

	match *ast {
		Index { expression, index } => {
			match *expression {
				Array(fields) => {
					assert_eq!(fields.len(), 3);
					assert_eq!(*fields[0], Integer(0));
					assert_eq!(*fields[1], Integer(1));
					assert_eq!(*fields[2], Integer(2));
				}
				_ =>
					panic!(
						"Expected an array with fields 0, 1 and 2, got {:?}",
						expression
					),
			}
			assert_eq!(*index, Integer(1));
		}
		_ =>
			panic!(
				"Expected an index expression with identifier 'array' and index 1, got {:?}",
				ast
			),
	}
}

#[test]
fn indexes_dont_have_commas() {
	let ast = parser::ExpressionParser::new().parse("[0, 1, 2][1, 2]");
	assert!(ast.is_err());
}
