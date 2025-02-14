use crate::parser;
use crate::ast::Expression::*;
use crate::ast::Statement;

#[test]
fn anonymous_object() {
	let ast = parser::ExpressionParser::new().parse("{a = 1, b = true, c }").unwrap();

	match *ast {
		Block(statements) => {
			assert_eq!(statements.len(), 3);

			match &statements[0] {
				Statement::Declaration(declaration) => {
					assert_eq!(declaration.identifier, "a");
					assert_eq!(declaration.type_expression.is_none(), true);
					assert_eq!(**declaration.expression.as_ref().unwrap(), Integer(1));
				}
				_ => panic!("Expected declaration, got {:?}", statements[0]),
			}

			match &statements[1] {
				Statement::Declaration(declaration) => {
					assert_eq!(declaration.identifier, "b");
					assert_eq!(declaration.type_expression.is_none(), true);
					assert_eq!(**declaration.expression.as_ref().unwrap(), Boolean(true));
				}
				_ => panic!("Expected declaration, got {:?}", statements[1]),
			}

			match &statements[2] {
				Statement::Expression(expression) => {
					match **expression {
						Identifier(ref identifier) => {
							assert_eq!(identifier, "c");
						}
						_ => panic!("Expected identifier, got {:?}", expression),
					}
				}
				_ => panic!("Expected declaration, got {:?}", statements[2]),
			}
		}
		_ =>
			panic!("Expected an anonymous object with statements a and b, got {:?}", ast),
	}
}

#[test]
fn empty_object() {
	let ast = parser::ExpressionParser::new().parse("{}").unwrap();

	match *ast {
		Block(statements) => assert_eq!(statements.len(), 0),
		_ =>
			panic!(
				"Expected an anonymous object with statements a, b and c, got {:?}",
				ast
			),
	}
}

#[test]
fn empty_object_with_line_breaks() {
	let ast = parser::ExpressionParser::new().parse("{\n\n}").unwrap();

	match *ast {
		Block(statements) => assert_eq!(statements.len(), 0),
		_ =>
			panic!(
				"Expected an anonymous object with statements a, b and c, got {:?}",
				ast
			),
	}
}

#[test]
fn empty_object_with_commmas() {
	let ast = parser::ExpressionParser::new().parse("{,,}");
	assert!(ast.is_err());
}

// #[test]
// fn nested_object() {
// 	let ast = parser::ExpressionParser
// 		::new()
// 		.parse("{a = {b = 1, c = 2}, d = ({ e = 4})}")
// 		.unwrap();

// 	match *ast {
// 		Block(statements) => {
// 			assert_eq!(statements.len(), 2);

// 			let a = statements[0].expression.as_ref().unwrap();
// 			let d = statements[1].expression.as_ref().unwrap();

// 			match **a {
// 				Block(ref statements_a) => {
// 					assert_eq!(statements_a.len(), 2);

// 					assert_eq!(statements_a[0].identifier, "b");
// 					assert_eq!(
// 						**statements_a[0].expression.as_ref().unwrap(),
// 						Integer(1)
// 					);

// 					assert_eq!(statements_a[1].identifier, "c");
// 					assert_eq!(
// 						**statements_a[1].expression.as_ref().unwrap(),
// 						Integer(2)
// 					);
// 				}
// 				_ =>
// 					panic!(
// 						"Expected an anonymous object with statements a, b and c, got {:?}",
// 						a
// 					),
// 			}

// 			match **d {
// 				Block(ref statements_d) => {
// 					assert_eq!(statements_d.len(), 1);

// 					assert_eq!(statements_d[0].identifier, "e");
// 					assert_eq!(
// 						**statements_d[0].expression.as_ref().unwrap(),
// 						Integer(4)
// 					);
// 				}
// 				_ =>
// 					panic!(
// 						"Expected an anonymous object with statements a, b and c, got {:?}",
// 						d
// 					),
// 			}
// 		}
// 		_ =>
// 			panic!(
// 				"Expected an anonymous object with statements a, b and c, got {:?}",
// 				ast
// 			),
// 	}
// }
