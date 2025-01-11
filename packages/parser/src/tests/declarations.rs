use crate::fa;
use crate::ast::Expression::*;
use crate::ast::Statement;

#[test]
fn type_declaration() {
	let ast = fa::StatementParser::new().parse("type MyType = expression").unwrap();
	match ast {
		Statement::TypeDeclaration(identifier, expression) => {
			assert_eq!(identifier, "MyType");
			assert_eq!(*expression, Identifier("expression".to_string()));
		}
		_ =>
			panic!(
				"Expected a Statement::TypeDeclaration(MyType, expression), got {:?}",
				ast
			),
	}
}

#[test]
fn assignment() {
	let ast = fa::StatementParser::new().parse("myVar = 12").unwrap();
	match ast {
		Statement::Assignment(identifier, type_expression, expression) => {
			assert_eq!(identifier, "myVar");
			assert_eq!(*expression, Integer(12));
			assert_eq!(type_expression, None);
		}
		_ => panic!("Expected a Statement::Assignment(myVar, None, 12), got {:?}", ast),
	}
}

#[test]
fn assignment_with_type() {
	let ast = fa::StatementParser::new().parse("myVar: MyType = 12").unwrap();
	match ast {
		Statement::Assignment(identifier, type_expression, expression) => {
			assert_eq!(identifier, "myVar");
			assert_eq!(*type_expression.unwrap(), Identifier("MyType".to_string()));
			assert_eq!(*expression, Integer(12));
		}
		_ =>
			panic!(
				"Expected a Statement::Assignment(myVar, Some(MyType), 12), got {:?}",
				ast
			),
	}
}
