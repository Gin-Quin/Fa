use crate::parser;
use crate::ast::Expression::*;
use crate::ast::*;

#[test]
fn type_declaration() {
	let ast = parser::StatementParser::new().parse("type MyType = expression").unwrap();
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
fn declaration_without_type() {
	let ast = parser::StatementParser::new().parse("myVar = 12").unwrap();
	match ast {
		Statement::Declaration(
			Declaration { identifier, type_expression, expression },
		) => {
			assert_eq!(identifier, "myVar");
			assert_eq!(*expression.unwrap(), Integer(12));
			assert_eq!(type_expression, None);
		}
		_ =>
			panic!(
				"Expected a Declaration {{ identifier: myVar, type_expression: None, expression: Some(12) }}, got {:?}",
				ast
			),
	}
}

#[test]
fn declaration_with_type() {
	let ast = parser::StatementParser::new().parse("myVar: MyType = 12").unwrap();
	match ast {
		Statement::Declaration(
			Declaration { identifier, type_expression, expression },
		) => {
			assert_eq!(identifier, "myVar");
			assert_eq!(*type_expression.unwrap(), Identifier("MyType".to_string()));
			assert_eq!(*expression.unwrap(), Integer(12));
		}
		_ =>
			panic!(
				"Expected a Declaration {{ identifier: myVar, type_expression: Some(MyType), expression: Some(12) }}, got {:?}",
				ast
			),
	}
}
