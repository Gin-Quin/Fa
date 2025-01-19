use crate::fa;

use crate::ast;
use ast::Expression::*;

#[test]
fn empty_function() {
	let ast = fa::ExpressionParser::new().parse("() => {}").unwrap();
	match *ast {
		Function { parameters, body } => {
			assert_eq!(parameters.len(), 0);
			assert_eq!(body.len(), 0);
		}
		_ => panic!("Expected a Function([], []), got {:?}", ast),
	}
}

#[test]
fn function_with_parameters() {
	let ast = fa::ExpressionParser
		::new()
		.parse("(x: String, y = 12, z: Number = 123) => {}")
		.unwrap();

	match *ast {
		Function { parameters, body } => {
			assert_eq!(parameters.len(), 3);
			assert_eq!(body.len(), 0);

			assert_eq!(parameters[0].identifier, "x");
			assert_eq!(
				**parameters[0].type_expression.as_ref().unwrap(),
				Identifier("String".to_string())
			);
			assert_eq!(parameters[0].expression, None);

			assert_eq!(parameters[1].identifier, "y");
			assert_eq!(parameters[1].type_expression, None);
			assert_eq!(**parameters[1].expression.as_ref().unwrap(), Integer(12));

			assert_eq!(parameters[2].identifier, "z");
			assert_eq!(
				**parameters[2].type_expression.as_ref().unwrap(),
				Identifier("Number".to_string())
			);
			assert_eq!(**parameters[2].expression.as_ref().unwrap(), Integer(123));
		}
		_ =>
			panic!(
				"Expected a Function([x: String, y = 12, z: Number = 123], []), got {:?}",
				ast
			),
	}
}

#[test]
fn function_with_body() {
	let ast = fa::ExpressionParser::new().parse("() => { x = 12 }").unwrap();
	match *ast {
		Function { parameters, body } => {
			assert_eq!(parameters.len(), 0);
			assert_eq!(body.len(), 1);

			match body[0] {
				ast::Statement::Declaration(ref declaration) => {
					assert_eq!(declaration.identifier, "x");
					assert_eq!(declaration.type_expression, None);
					assert_eq!(**declaration.expression.as_ref().unwrap(), Integer(12));
				}
				_ => panic!("Expected a Declaration 'x = 12', got {:?}", body[0]),
			}
		}
		_ => panic!("Expected a Function([], [x = 12]), got {:?}", ast),
	}
}
