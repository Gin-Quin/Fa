use crate::fa;
use crate::ast::Expression::*;

#[test]
fn anonymous_object() {
	let ast = fa::ExpressionParser::new().parse("{a = 1, b = true, c }").unwrap();

	match *ast {
		Object(fields) => {
			assert_eq!(fields.len(), 3);

			assert_eq!(fields[0].identifier, "a");
			assert_eq!(fields[0].type_expression.is_none(), true);
			assert_eq!(**fields[0].expression.as_ref().unwrap(), Integer(1));

			assert_eq!(fields[1].identifier, "b");
			assert_eq!(fields[1].type_expression.is_none(), true);
			assert_eq!(**fields[1].expression.as_ref().unwrap(), Boolean(true));

			// assert_eq!(fields[2].identifier, "c");
			// assert_eq!(fields[2].expression.is_none(), true);
			// assert_eq!(
			// 	**fields[2].type_expression.as_ref().unwrap(),
			// 	String("Integer".to_string())
			// );
		}
		_ => panic!("Expected an anonymous object with fields a and b, got {:?}", ast),
	}
}

#[test]
fn empty_object() {
	let ast = fa::ExpressionParser::new().parse("{}").unwrap();

	match *ast {
		Object(fields) => assert_eq!(fields.len(), 0),
		_ => panic!("Expected an anonymous object with fields a, b and c, got {:?}", ast),
	}
}

#[test]
fn empty_object_with_line_breaks() {
	let ast = fa::ExpressionParser::new().parse("{\n\n}").unwrap();

	match *ast {
		Object(fields) => assert_eq!(fields.len(), 0),
		_ => panic!("Expected an anonymous object with fields a, b and c, got {:?}", ast),
	}
}

#[test]
fn empty_object_with_commmas() {
	let ast = fa::ExpressionParser::new().parse("{,,}");
	assert!(ast.is_err());
}

#[test]
fn nested_object() {
	let ast = fa::ExpressionParser
		::new()
		.parse("{a = {b = 1, c = 2}, d = ({ e = 4})}")
		.unwrap();

	match *ast {
		Object(fields) => {
			assert_eq!(fields.len(), 2);

			let a = fields[0].expression.as_ref().unwrap();
			let d = fields[1].expression.as_ref().unwrap();

			match **a {
				Object(ref fields_a) => {
					assert_eq!(fields_a.len(), 2);

					assert_eq!(fields_a[0].identifier, "b");
					assert_eq!(**fields_a[0].expression.as_ref().unwrap(), Integer(1));

					assert_eq!(fields_a[1].identifier, "c");
					assert_eq!(**fields_a[1].expression.as_ref().unwrap(), Integer(2));
				}
				_ =>
					panic!(
						"Expected an anonymous object with fields a, b and c, got {:?}",
						a
					),
			}

			match **d {
				Object(ref fields_d) => {
					assert_eq!(fields_d.len(), 1);

					assert_eq!(fields_d[0].identifier, "e");
					assert_eq!(**fields_d[0].expression.as_ref().unwrap(), Integer(4));
				}
				_ =>
					panic!(
						"Expected an anonymous object with fields a, b and c, got {:?}",
						d
					),
			}
		}
		_ => panic!("Expected an anonymous object with fields a, b and c, got {:?}", ast),
	}
}
